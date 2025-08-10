# Claude Code Project Instructions

## Required Reading

Before working on this project, **always read `AGENTS.md`** for project-specific instructions and code quality requirements.

## Project Overview

This is a Trading212 MCP (Model Context Protocol) server implementation that provides AI assistants access to Trading212 API functionality.

## Development Workflow

1. **Read Documentation**: Review `AGENTS.md` and `trading212-api.md` before starting
2. **Code Quality**: Follow the quality requirements specified in `AGENTS.md`
3. **Testing**: Ensure all changes work with the Trading212 API
4. **Verification**: Run clippy, fmt, and build checks before completing tasks

## Key Files

- `AGENTS.md` - Development instructions and code quality requirements
- `trading212-api.md` - Trading212 API reference and usage guidelines
- `README.md` - User documentation and setup instructions

## API Considerations

- Trading212 has strict rate limits - avoid retry logic
- Always use debug logging for API troubleshooting
- Respect the `pies:read` scope requirement for pie-related functionality