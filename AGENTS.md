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
8. **Security audit**: Run `cargo audit` to conduct security audit
9. **Dependency hygiene**: Run `cargo machete` to identify unused dependencies
10. **Conventional commits**: Use conventional commit message
