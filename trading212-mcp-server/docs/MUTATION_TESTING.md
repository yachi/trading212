# Mutation Testing Guide

## Overview

This project uses [cargo-mutants](https://mutants.rs/) for mutation testing to verify the effectiveness of our test suite. Mutation testing works by introducing small changes (mutations) to the code and checking if tests catch these changes.

## Installation

```bash
cargo install cargo-mutants
# or via cargo-make
cargo make install-tools
```

## Configuration

Mutation testing is configured in `.cargo/mutants.toml` with:
- 30-second timeout per mutation
- Exclusions for test/bench/example code
- Parallel execution using all available cores
- Detailed report generation

## Running Mutation Tests

### Using cargo-make (recommended)

```bash
# Run full mutation testing
cargo make mutants

# Quick run with 10s timeout
cargo make mutants-quick

# Generate detailed report
cargo make mutants-report

# Test specific file
FILE=src/tools.rs cargo make mutants-file

# Clean mutation outputs
cargo make mutants-clean
```

### Direct cargo-mutants commands

```bash
# Basic run
cargo mutants

# With custom timeout
cargo mutants --timeout 20

# Test specific module
cargo mutants --file src/handler.rs

# Generate HTML report
cargo mutants --output mutants.out
```

## Interpreting Results

### Mutation Types
- **Caught**: Test suite detected the mutation ✅
- **Missed**: Test suite didn't detect the mutation ⚠️
- **Unviable**: Mutation caused compilation failure
- **Timeout**: Tests exceeded time limit

### Example Output
```
Found 150 mutants to test
128 mutants caught
12 mutants missed  
10 unviable mutants

Mutation score: 91.4%
```

### Improving Coverage

For missed mutations:
1. Review the mutation details in `mutants.out/`
2. Add tests for the uncovered scenarios
3. Re-run mutation testing to verify improvements

## CI Integration

Mutation testing can be integrated into CI pipelines:

```yaml
# GitHub Actions example
- name: Run mutation testing
  run: cargo make mutants-quick
  
- name: Upload mutation report
  if: always()
  uses: actions/upload-artifact@v3
  with:
    name: mutation-report
    path: mutants.out/
```

## Best Practices

1. **Regular Testing**: Run mutation testing after significant changes
2. **Target Score**: Aim for >85% mutation coverage
3. **Focus Areas**: Prioritize critical business logic
4. **Time Management**: Use quick mode during development
5. **Report Review**: Analyze missed mutations to improve tests

### Configuration Best Practices

Based on analysis of popular Rust projects using cargo-mutants:

1. **Exclude Non-Testable Code**:
   - Debug/Display implementations
   - Drop implementations (cleanup code)
   - Logging/tracing calls
   - Error construction boilerplate

2. **Performance Optimization**:
   - Use `--all-features` to test all code paths
   - Consider using release profile for faster runs
   - Set appropriate timeouts (10s for quick, 30s for thorough)
   - Use parallel execution (`jobs = 0` for all cores)

3. **Custom Error Values**:
   - Define project-specific error values for mutations
   - This helps catch error handling edge cases

4. **CI Integration**:
   - Quick mode for PRs (10s timeout)
   - Full mode for scheduled/manual runs
   - Upload reports as artifacts
   - Comment results on PRs

## Exclusions

The following are excluded from mutation testing:
- Integration test files (`tests/**`)
- Benchmark code (`benches/**`)
- Example code (`examples/**`)
- Logging/tracing calls
- Debug output (println!, eprintln!)

## Troubleshooting

### Tests timeout frequently
- Increase timeout in `.cargo/mutants.toml`
- Use `--timeout` flag for specific runs

### Too many mutations to test
- Focus on specific files with `--file`
- Use `mutants-quick` for faster feedback

### False positives
- Review exclusion patterns in configuration
- Consider adding skip attributes for specific functions