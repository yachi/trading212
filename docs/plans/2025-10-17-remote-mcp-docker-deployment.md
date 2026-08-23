# Remote MCP Server & Docker Deployment Plan

## Status: ✅ **Phases 1-2 Complete** | PR: [#15](https://github.com/yachi/trading212/pull/15)

### What Was Built

**HTTP Server**
- Remote MCP server over HTTP (JSON-RPC 2.0)
- Dual authentication: Bearer token + custom X-Trading212-API-Key header
- Shared cache architecture with Arc for thread-safety
- Request ID tracking, CORS, timeouts, graceful shutdown

**Docker**
- Multi-stage Dockerfile (rust:1.83-slim → debian:bookworm-slim)
- Health checks, non-root user, optimized caching
- docker-compose.yml for local testing
- Comprehensive .dockerignore

**Testing**
- 49 integration tests covering auth, MCP protocol, tools, edge cases
- 93.14% test coverage, zero clippy warnings
- Successfully tested with real Trading212 API

**Key Differences from Plan**
- Used axum 0.8.6 (latest stable) instead of 0.7
- No SSE - direct HTTP POST is simpler and sufficient
- No separate transport module - all in remote-server.rs binary
- Feature-gated HTTP dependencies under `http-server` feature

---

## Phase 1: Remote MCP Server Architecture ✅

### 1.1 HTTP Server Implementation ✅
- [x] Add HTTP dependencies (axum 0.8.6, tower-http, hyper, axum-extra)
- [x] Create `src/bin/remote-server.rs` binary (505 lines)
- [x] Implement JSON-RPC 2.0 over HTTP POST
- [x] Health check endpoint `/health`
- [x] MCP endpoint `/mcp/v1`

### 1.2 Authentication Middleware ✅
- [x] Dual authentication: Bearer token + X-Trading212-API-Key header
- [x] Per-request Trading212Config creation
- [x] CORS, request timeouts (60s), request ID tracking

### 1.3 Configuration ✅
- [x] Environment variables: MCP_HTTP_HOST, MCP_HTTP_PORT
- [x] HTTP client config: TRADING212_POOL_SIZE, TRADING212_TIMEOUT_SECS
- [x] Backward compatible - stdio mode unchanged

---

## Phase 2: Docker Containerization ✅

### 2.1 Multi-stage Dockerfile ✅
- [x] Builder stage: rust:1.83-slim with dependency caching
- [x] Runtime stage: debian:bookworm-slim
- [x] Non-root user (mcp, UID 1000)
- [x] Health checks, stripped binary
- [x] Environment defaults

### 2.2 Docker Compose ✅
- [x] Created docker-compose.yml for local testing
- [x] Port mapping, env vars, health checks

### 2.3 .dockerignore ✅
- [x] Comprehensive ignore patterns (44 lines)
- [x] Excludes target/, tests/, .git/, secrets

---

## Phase 3: GitHub Container Registry (GHCR) Setup

### 3.1 GitHub Actions Workflow
**Goal:** Automated Docker image builds and GHCR publishing

**Tasks:**
- [ ] Create `.github/workflows/docker-publish.yml`

- [ ] Configure workflow triggers
  - Push to main branch
  - Push tags (v*.*.*)
  - Manual workflow dispatch

- [ ] Build multi-platform images
  - linux/amd64
  - linux/arm64 (optional)

- [ ] Push to GHCR with proper tags
  - `ghcr.io/<owner>/trading212-mcp:latest`
  - `ghcr.io/<owner>/trading212-mcp:<version>`
  - `ghcr.io/<owner>/trading212-mcp:<commit-sha>`

**Example Workflow:**
```yaml
name: Build and Push Docker Image

on:
  push:
    branches:
      - main
    tags:
      - 'v*.*.*'
  workflow_dispatch:

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Log in to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=ref,event=branch
            type=semver,pattern={{version}}
            type=semver,pattern={{major}}.{{minor}}
            type=sha,prefix={{branch}}-

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Build and push Docker image
        uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
          platforms: linux/amd64,linux/arm64
```

### 3.2 Repository Configuration
**Goal:** Enable GHCR access and package visibility

**Tasks:**
- [ ] Enable GitHub Packages in repository settings
- [ ] Set package visibility (public or private)
- [ ] Configure package permissions
- [ ] Link package to repository
- [ ] Add package description and README

---

## Phase 4: Deployment & Documentation

### 4.1 Deployment Options Documentation
**Goal:** Provide multiple deployment guides

**Tasks:**
- [ ] Create `docs/deployment/` directory

- [ ] Write deployment guides:
  - [ ] `docker-local.md` - Local Docker deployment
  - [ ] `kubernetes.md` - Kubernetes deployment
  - [ ] `cloud-run.md` - Google Cloud Run deployment
  - [ ] `fly-io.md` - Fly.io deployment
  - [ ] `railway.md` - Railway deployment

### 4.2 Update README
**Goal:** Document new remote MCP usage

**Tasks:**
- [ ] Update README.md with:
  - Remote MCP server usage
  - Docker deployment instructions
  - GHCR image pulling
  - API key header authentication
  - Health check endpoint
  - Example HTTP requests

**Example README Section:**
```markdown
## Remote MCP Server

### Docker Deployment

Pull and run the Docker image from GHCR:

\`\`\`bash
docker pull ghcr.io/<owner>/trading212-mcp:latest
docker run -p 3000:3000 ghcr.io/<owner>/trading212-mcp:latest
\`\`\`

### API Authentication

Pass your Trading212 API key via HTTP header:

\`\`\`bash
curl -X POST http://localhost:3000/mcp/v1 \
  -H "Authorization: Bearer YOUR_TRADING212_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list",
    "params": {}
  }'
\`\`\`

### Claude Desktop Configuration

Configure Claude Desktop to use remote MCP:

\`\`\`json
{
  "mcpServers": {
    "trading212-remote": {
      "type": "http",
      "url": "http://localhost:3000/mcp/v1",
      "headers": {
        "Authorization": "Bearer YOUR_TRADING212_API_KEY"
      }
    }
  }
}
\`\`\`
```

### 4.3 API Documentation
**Goal:** Document HTTP API

**Tasks:**
- [ ] Create `docs/api.md`
  - Endpoints documentation
  - Request/response examples
  - Authentication details
  - Error responses
  - Rate limiting information

---

## Phase 5: Testing & Quality Assurance ✅

### 5.1 Integration Tests ✅
- [x] Created `tests/remote_server_tests.rs` (577 lines, 49 tests)
- [x] Authentication tests (Bearer + custom header)
- [x] MCP protocol tests (JSON-RPC validation)
- [x] Tool execution tests
- [x] Request validation, edge cases
- [x] Cache thread-safety tests

### 5.2 Live Testing ✅
- [x] Tested with real Trading212 API
- [x] Verified both auth methods work
- [x] Confirmed rate limiting behavior
- [x] Validated timeout protection (60s)

### 5.3 CI/CD ✅
- [x] All existing CI checks pass (clippy, tests, fmt, audit)
- [x] Mutation testing: 10 survived (expected in infrastructure code)
- [x] Build verified with `--features http-server`

---

## Phase 6: Production Readiness

### 6.1 Observability
**Goal:** Production monitoring and debugging

**Tasks:**
- [ ] Add structured logging
  - Request ID tracking
  - API key masking in logs
  - Request duration metrics

- [ ] Add metrics endpoint
  - `/metrics` for Prometheus
  - Request count, latency, error rate

- [ ] Add health check details
  - `/health` - Simple UP/DOWN
  - `/health/ready` - Readiness check
  - `/health/live` - Liveness check

### 6.2 Security Hardening
**Goal:** Secure production deployment

**Tasks:**
- [ ] Implement rate limiting
  - Per API key
  - Global rate limit

- [ ] Add request validation
  - Request size limits
  - Timeout configuration

- [ ] Security headers
  - Content-Security-Policy
  - X-Content-Type-Options
  - X-Frame-Options

- [ ] API key validation
  - Format validation
  - Length checks
  - Character restrictions

### 6.3 Configuration Management
**Goal:** Flexible deployment configuration

**Tasks:**
- [ ] Environment variable documentation
  - Required vs optional
  - Default values
  - Valid ranges

- [ ] Configuration file support (optional)
  - YAML configuration
  - Override priority: CLI > ENV > Config file > Defaults

---

## Phase 7: Migration Path

### 7.1 Backward Compatibility
**Goal:** Support both stdio and HTTP modes

**Tasks:**
- [ ] Keep existing stdio mode working
- [ ] Document migration path
- [ ] Provide transition guide

### 7.2 Version Strategy
**Goal:** Clear versioning for users

**Tasks:**
- [ ] Use semantic versioning
- [ ] Tag releases properly
- [ ] Maintain CHANGELOG.md
- [ ] Document breaking changes

---

## Technical Architecture Summary

### Components

```
┌─────────────────────────────────────┐
│         Claude Desktop              │
│  (or any MCP client)                │
└─────────────┬───────────────────────┘
              │
              │ HTTP + JSON-RPC 2.0
              │ Header: Authorization: Bearer <API_KEY>
              │
┌─────────────▼───────────────────────┐
│     Remote MCP Server (HTTP)        │
│  ┌───────────────────────────────┐  │
│  │  Auth Middleware              │  │
│  │  - Extract API key            │  │
│  │  - Validate format            │  │
│  │  - Rate limiting              │  │
│  └───────────┬───────────────────┘  │
│              │                       │
│  ┌───────────▼───────────────────┐  │
│  │  MCP Handler                  │  │
│  │  - Parse JSON-RPC             │  │
│  │  - Route to tools             │  │
│  │  - Create per-request config  │  │
│  └───────────┬───────────────────┘  │
│              │                       │
│  ┌───────────▼───────────────────┐  │
│  │  Trading212 API Client        │  │
│  │  - Use provided API key       │  │
│  │  - Make Trading212 API calls  │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
              │
              │ HTTPS
              │
┌─────────────▼───────────────────────┐
│     Trading212 API                  │
│  https://live.trading212.com/api/v0 │
└─────────────────────────────────────┘
```

### Key Changes to Existing Code

1. **config.rs**
   - Add `new_with_api_key(api_key: String)` method ✓ (already exists!)
   - Make `new()` return `Result` that can fail gracefully

2. **handler.rs**
   - Refactor to accept config as parameter instead of creating internally
   - Remove `config` field from struct, accept as method parameter

3. **New files**
   - `src/bin/remote-server.rs` - HTTP server entry point
   - `src/middleware/auth.rs` - Authentication middleware
   - `src/transport/http.rs` - HTTP transport implementation
   - `Dockerfile` - Container definition
   - `.dockerignore` - Build optimization
   - `docker-compose.yml` - Local testing
   - `.github/workflows/docker-publish.yml` - CI/CD

---

## Progress Summary

### Completed (Phases 1, 2, 5)
- ✅ HTTP server with JSON-RPC 2.0
- ✅ Dual authentication (Bearer + custom header)
- ✅ Docker multi-stage build
- ✅ 49 integration tests, 93.14% coverage
- ✅ Live testing with real API successful
- ✅ Backward compatible (stdio mode unchanged)

### Remaining (Phases 3, 4, 6, 7)
- [ ] GHCR publishing workflow
- [ ] Deployment documentation
- [ ] Observability (metrics, structured logging)
- [ ] Advanced security (rate limiting, validation)
- [ ] Production deployment guides

---

## Next Steps

1. **GHCR Setup (Phase 3)** - Create GitHub Actions workflow for automated Docker publishing
2. **Documentation (Phase 4)** - Add deployment guides (k8s, cloud-run, fly.io, railway)
3. **Production Readiness (Phase 6)** - Add metrics, structured logging, rate limiting
4. **Real-world Testing** - Deploy to production environment and monitor

---

## Notes

- This plan maintains backward compatibility with stdio mode
- Per-user API keys via headers enables multi-tenant deployment
- Docker + GHCR enables easy deployment anywhere
- Health checks enable proper orchestration (k8s, docker-compose)
- Rate limiting per API key prevents abuse
- Non-root user in Docker for security
- Multi-stage build for minimal image size
- GitHub Actions for automated CI/CD

---

## Implementation Decisions Made

1. **Authentication**: ✅ Dual support - Bearer token AND X-Trading212-API-Key header
2. **CORS policy**: ✅ Permissive (CorsLayer::permissive())
3. **Logging**: ✅ Human-readable with tracing-subscriber (JSON for production TBD)
4. **Architecture**: ✅ Simplified - no SSE, direct HTTP POST works better
5. **Feature gating**: ✅ HTTP dependencies behind `http-server` feature

## Open Questions (Future Phases)

1. **Rate limiting**: Per API key, global, or both?
2. **Metrics**: Prometheus format?
3. **Multi-architecture**: ARM64 Docker builds?
4. **Registry**: GHCR only or also Docker Hub?

---

## References

- [MCP Specification](https://modelcontextprotocol.io/)
- [Axum Web Framework](https://docs.rs/axum/)
- [Docker Multi-stage Builds](https://docs.docker.com/build/building/multi-stage/)
- [GitHub Container Registry](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry)
- [Trading212 API Docs](https://t212public-api-docs.redoc.ly/)
