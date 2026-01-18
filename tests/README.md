# Integration and E2E Testing

This directory contains integration tests (API-level) and end-to-end tests (browser-based) for Alelysee.

## Quick Start

```bash
# Run all tests
make test-all

# Run only integration tests
make test-integration

# Run only E2E tests
make test-e2e
```

## Test Structure

- **API Integration Tests** (`packages/api/tests/integration/`) - Direct server function tests
- **E2E Tests** (`tests/e2e/tests/`) - Browser automation tests

## Running Tests Locally

### Integration Tests

```bash
# All integration tests
cargo test --package api --test '*'

# Specific test file
cargo test --package api --test auth_tests

# Specific test
cargo test --package api --test auth_tests test_signup_creates_user

# With output
cargo test --package api --test auth_tests -- --nocapture
```

### E2E Tests

```bash
# All E2E tests
cargo test --package e2e --test '*'

# Specific test
cargo test --package e2e --test auth test_signin_page_loads

# With visible browser (non-headless)
HEADLESS=false cargo test --package e2e --test auth
```

## Debugging Failed Tests

### Integration Tests

1. **Check test output**: Use `-- --nocapture` to see print statements
2. **Inspect database**: Failed tests leave `.test-*.db` files for inspection
3. **Enable SQL logging**: Set `RUST_LOG=sqlx=debug` to see queries

### E2E Tests

1. **Check server logs**: Test output includes server stderr
2. **Inspect database**: Failed tests leave `.e2e-test-*.db` files
3. **Run non-headless**: See browser actions with `HEADLESS=false`
4. **Keep server running**: Set `KEEP_SERVER=true` to debug after failure

## Test Isolation

- Each integration test gets a fresh SQLite database (`.test-{uuid}.db`)
- Each E2E test suite gets a fresh server with new database (`.e2e-test-{uuid}.db`)
- All tests use `APP_MODE=local` (no external dependencies)
- Databases are cleaned up automatically on test completion

## Adding New Tests

### Integration Test

1. Create test file in `packages/api/tests/integration/`
2. Import `common::TestContext`
3. Create `TestContext` in each test
4. Call `ctx.set_global()` to make AppState available
5. Test server functions directly

### E2E Test

1. Create test file in `tests/e2e/tests/`
2. Start `TestServer` in each test
3. Launch `Browser` and create `Page`
4. Navigate and interact with UI
5. Assert on page content and behavior

## CI/CD

Tests run automatically in GitHub Actions on:
- Pull requests to main
- Pushes to main/develop

See `.github/workflows/ci-cd.yml` for CI configuration.

## Troubleshooting

### Port conflicts

E2E tests use random ports, but if you see "Address already in use":
- Kill existing server processes: `pkill -f "cargo run"`
- Check for port conflicts: `lsof -i :8080`

### Database locked

If you see "database is locked":
- Close any SQLite connections to test databases
- Delete stale `.test-*.db` files: `rm -f .test-*.db .e2e-test-*.db`

### Browser launch fails

If headless_chrome fails to launch:
- Ensure Chrome/Chromium is installed
- Check system dependencies are installed
- Try non-headless mode for debugging
