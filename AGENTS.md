# Agent Instructions

Before working with Trading212 API, read `trading212-api.md` first.

## Core Principles

**Reduce Cognitive Load**: Write simple, readable code that's easy to understand and maintain.

## Code Quality Requirements

Code quality is enforced through automated checks:

1. **Pre-commit**: Formatting (`cargo fmt`) is checked on every commit
2. **CI Pipeline**: Clippy, tests, and build verification run automatically on PRs
3. **Address warnings**: Fix any clippy warnings that can be reasonably addressed
4. **Manual checks**: For local development, you can run `cargo clippy`, `cargo test`, and `cargo build --release`
5. **Test coverage**: Run `cargo llvm-cov --summary-only` to verify test coverage
6. **Code review**: Perform comprehensive code review of all changes
7. **Code tracing**: Read and trace through modified code paths to understand:
8. **Security audit**: Run `cargo audit` to conduct security audit
9. **Dependency hygiene**: Run `cargo machete` to identify unused dependencies
10. **Conventional commits**: Use conventional commit message

## Git Hooks

This repo keeps its hooks in `.githooks/` so they are versioned and shared. Git does
not pick that up automatically — `core.hooksPath` is local to each clone, so run this
once after cloning:

```sh
git config core.hooksPath .githooks
```

`.githooks/pre-commit` does two things:

1. **Blocks personal financial data.** This repo is public. Staged lines matching
   `valueGBP`, `pie_id=<n>`, or book-total phrasing are rejected, because portfolio
   position values have been committed here by accident before. Override for a false
   positive with `ALLOW_FINANCIAL=1 git commit ...`.
2. **Runs `cargo fmt --check`.** Clippy, tests, and compilation run in CI instead.

## Code Review Session

### Pre-review Setup:
1. **Run Coverage Analysis**: Execute `cargo llvm-cov` to establish baseline test coverage

### For each file, analyze:
1. **Find Issues**: Identify bugs, potential errors, or problematic patterns
2. **Suggest Improvements**: Recommend better approaches or optimizations
3. **Find Over-engineered Code**: Identify unnecessary complexity that can be simplified
4. **Preserve Test Coverage**: Ensure any code deletion won't reduce test coverage
