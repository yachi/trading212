# Remote MCP Server & Docker Deployment Plan

## Overview
Transform the Trading212 MCP server from a stdio-based local server into a remote HTTP-based MCP server with Docker containerization and GHCR deployment. Support multi-user authentication via Trading212 API keys in HTTP headers.

---

## Phase 1: Remote MCP Server Architecture

### 1.1 HTTP Server Implementation
**Goal:** Convert stdio-based MCP to HTTP/SSE-based remote MCP

**Tasks:**
- [ ] Add HTTP server dependencies to Cargo.toml
  - `axum` (v0.7) - Modern async web framework
  - `axum-sse` or `tower-http` - SSE support for MCP protocol
  - `hyper` (v1.0) - HTTP server foundation
  - `tower` (v0.5) - Middleware support

- [ ] Create new binary target for remote server
  - `src/bin/remote-server.rs` - HTTP server entry point
  - Keep existing `src/main.rs` for stdio mode

- [ ] Implement HTTP transport layer
  - Create `src/transport/http.rs` module
  - Implement MCP-over-HTTP protocol
  - Support Server-Sent Events (SSE) for streaming responses
  - Handle JSON-RPC 2.0 over HTTP POST requests

**Technical Details:**
```rust
// Example structure
// src/bin/remote-server.rs
#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/mcp/v1", post(handle_mcp_request))
        .route("/health", get(health_check))
        .layer(auth_middleware());

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

### 1.2 Authentication Middleware
**Goal:** Support per-user Trading212 API keys via HTTP headers

**Tasks:**
- [ ] Create authentication middleware module
  - `src/middleware/auth.rs`

- [ ] Implement header-based API key extraction
  - Header name: `X-Trading212-API-Key` or `Authorization: Bearer <key>`
  - Validate API key format (non-empty, reasonable length)

- [ ] Create per-request configuration
  - Extract API key from request headers
  - Create `Trading212Config` instance per request
  - Inject config into request handlers

- [ ] Add security headers
  - CORS configuration (if needed)
  - Rate limiting per API key (optional but recommended)

**Technical Details:**
```rust
// Example middleware
async fn extract_api_key(
    TypedHeader(auth): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
) -> Result<Trading212Config, StatusCode> {
    let api_key = auth.token().to_string();
    Ok(Trading212Config::new_with_api_key(api_key))
}
```

### 1.3 Configuration Refactoring
**Goal:** Support both file-based (stdio mode) and header-based (HTTP mode) API keys

**Tasks:**
- [ ] Modify `config.rs` to support multiple modes
  - Keep existing file-based loading for stdio mode
  - Add method to create config from string (for HTTP mode)

- [ ] Update `Trading212Config::new()` to be optional
  - Return `Result` that's acceptable to fail in HTTP mode

- [ ] Add environment variables for server configuration
  - `MCP_SERVER_MODE` - "stdio" or "http"
  - `MCP_HTTP_PORT` - HTTP server port (default: 3000)
  - `MCP_HTTP_HOST` - Bind address (default: 0.0.0.0)

---

## Phase 2: Docker Containerization

### 2.1 Multi-stage Dockerfile
**Goal:** Create optimized production Docker image

**Tasks:**
- [ ] Create `Dockerfile` in repository root

- [ ] Stage 1: Builder
  - Base: `rust:1.83-slim` or `rust:1.83-alpine`
  - Install build dependencies
  - Copy Cargo.toml and Cargo.lock
  - Build dependencies (cached layer)
  - Copy source code
  - Build release binary

- [ ] Stage 2: Runtime
  - Base: `debian:bookworm-slim` or `alpine:3.20`
  - Install runtime dependencies (ca-certificates, libssl)
  - Copy binary from builder stage
  - Create non-root user
  - Set proper permissions
  - Expose port 3000
  - Health check configuration

**Example Dockerfile:**
```dockerfile
# Stage 1: Builder
FROM rust:1.83-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files
COPY trading212-mcp-server/Cargo.toml trading212-mcp-server/Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --bin remote-server && \
    rm -rf src

# Copy actual source code
COPY trading212-mcp-server/src ./src

# Build release binary
RUN cargo build --release --bin remote-server

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 mcp && \
    mkdir -p /app && \
    chown -R mcp:mcp /app

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/remote-server /app/mcp-server

# Switch to non-root user
USER mcp

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

# Run server
CMD ["/app/mcp-server"]
```

### 2.2 Docker Compose (Development)
**Goal:** Easy local testing with Docker

**Tasks:**
- [ ] Create `docker-compose.yml`

- [ ] Configure development environment
  - Volume mount for hot-reload (optional)
  - Environment variable injection
  - Port mapping
  - Logging configuration

**Example docker-compose.yml:**
```yaml
version: '3.8'

services:
  mcp-server:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "3000:3000"
    environment:
      - RUST_LOG=info
      - MCP_SERVER_MODE=http
      - MCP_HTTP_PORT=3000
      - MCP_HTTP_HOST=0.0.0.0
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s
```

### 2.3 .dockerignore
**Goal:** Optimize build context

**Tasks:**
- [ ] Create `.dockerignore` file

**Content:**
```
target/
.git/
.github/
docs/
*.md
!trading212-api.md
.env
.trading212-api-key
.gitignore
docker-compose.yml
```

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

## Phase 5: Testing & Quality Assurance

### 5.1 Integration Tests
**Goal:** Test HTTP server functionality

**Tasks:**
- [ ] Create integration test module
  - `tests/integration/remote_server.rs`

- [ ] Test cases:
  - [ ] Health check endpoint
  - [ ] MCP protocol over HTTP
  - [ ] Authentication success/failure
  - [ ] Invalid API key handling
  - [ ] Missing header handling
  - [ ] Concurrent request handling

### 5.2 Docker Testing
**Goal:** Verify Docker image functionality

**Tasks:**
- [ ] Test Docker build locally
- [ ] Test Docker run with environment variables
- [ ] Verify health check works
- [ ] Test API key injection via env
- [ ] Performance testing (response time, memory usage)

### 5.3 CI/CD Pipeline
**Goal:** Automated testing before deployment

**Tasks:**
- [ ] Update existing CI workflow
  - Add Docker build test
  - Add integration tests for HTTP mode
  - Verify clippy and fmt still pass

- [ ] Add security scanning
  - Trivy for Docker image scanning
  - Cargo audit for dependency vulnerabilities

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

## Timeline Estimate

- **Phase 1:** 2-3 days (HTTP server implementation)
- **Phase 2:** 1 day (Docker containerization)
- **Phase 3:** 1 day (GHCR setup)
- **Phase 4:** 1-2 days (Documentation)
- **Phase 5:** 2 days (Testing)
- **Phase 6:** 2-3 days (Production readiness)
- **Phase 7:** 1 day (Migration support)

**Total:** 10-14 days for full implementation

---

## Success Criteria

- ✅ HTTP server responds to MCP requests
- ✅ API key authentication via headers works
- ✅ Docker image builds successfully
- ✅ Image pushed to GHCR automatically
- ✅ Health checks pass
- ✅ Stdio mode still works (backward compatible)
- ✅ All existing tests pass
- ✅ New integration tests pass
- ✅ Documentation complete
- ✅ Security scan passes

---

## Next Steps

1. **Review this plan** - Ensure all requirements are covered
2. **Set up development environment** - Install dependencies
3. **Start with Phase 1.1** - HTTP server basic structure
4. **Iterate through phases** - Test each phase before moving to next
5. **Get feedback** - Test with real Claude Desktop client

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

## Questions to Consider

1. **Rate limiting strategy**: Per API key? Global? Both?
2. **CORS policy**: Allow all origins or restrict?
3. **Authentication**: Bearer token only or support multiple schemes?
4. **Metrics**: Prometheus format or custom?
5. **Logging**: JSON structured logs or human-readable?
6. **Multi-architecture builds**: ARM64 support needed?
7. **Image registry**: GHCR only or also Docker Hub?

---

## References

- [MCP Specification](https://modelcontextprotocol.io/)
- [Axum Web Framework](https://docs.rs/axum/)
- [Docker Multi-stage Builds](https://docs.docker.com/build/building/multi-stage/)
- [GitHub Container Registry](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry)
- [Trading212 API Docs](https://t212public-api-docs.redoc.ly/)
