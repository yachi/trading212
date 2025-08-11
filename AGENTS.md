# Agent Instructions

Before working with Trading212 API, read `trading212-api.md` first.

## Code Quality Requirements

After completing any code-changing tasks:

1. **Run clippy**: Always execute `cargo clippy` to check for lint issues
2. **Run fmt**: Execute `cargo fmt` to ensure consistent code formatting
3. **Address warnings**: Fix any clippy warnings that can be reasonably addressed
4. **Build verification**: Ensure `cargo build --release` succeeds after changes
5. **Test coverage**: Run `cargo llvm-cov --summary-only` to verify test coverage
6. **Code review**: Perform comprehensive code review of all changes
7. **Code tracing**: Read and trace through modified code paths to understand:
   - Data flow and transformations
   - Error propagation paths
   - Integration points and dependencies
   - Business logic correctness
8. **Security audit**: Conduct security audit focusing on:
   - Authentication and authorization
   - Input validation and sanitization
   - Error handling and information disclosure
   - Network security and data handling
   - No hardcoded secrets or sensitive data exposure
9. **Conventional commits**: Use conventional commit message format:
   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation changes
   - `refactor:` for code refactoring
   - `test:` for adding tests
   - `chore:` for maintenance tasks
   - Include scope when relevant (e.g., `feat(pies): add get_pies tool`)

These steps ensure code quality, security, and consistency across the project.