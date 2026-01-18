# Integration Testing System Design

**Date:** 2026-01-18
**Status:** Approved
**Author:** Claude (via brainstorming session)

## Overview

Add comprehensive integration testing to Alelysee with two complementary layers: fast API-level tests for business logic and E2E browser tests for user workflows. All tests use local mode (SQLite) for zero external dependencies.

## Goals

- Test all critical features: auth, proposals, programs, voting, comments, profiles
- Fast feedback loop for developers (API tests in milliseconds, E2E in minutes)
- CI-ready with no external service dependencies
- Easy debugging with screenshots, logs, and preserved test databases
- Prevent regressions as features are added

## Architecture

### Two-Layer Testing Approach

**API Tests** (`packages/api/tests/integration/`)
- Direct Rust test functions calling server functions
- Each test creates isolated SQLite database (`.test-{uuid}.db`)
- Sets `APP_MODE=local` via environment
- Fast execution (no browser overhead)
- Tests business logic, data validation, database operations

**E2E Tests** (`tests/e2e/` at workspace root)
- Playwright driving real browser (Chromium headless)
- Tests run against actual running server
- Server uses fresh SQLite DB per test suite
- Tests user workflows, UI interactions, full request/response cycles

**Shared Test Utilities** (`packages/api/src/test_utils.rs`)
- `TestContext::new()` - creates fresh DB, returns pool + cleanup handle
- `seed_test_user()` - creates authenticated user for tests
- `cleanup_test_db()` - removes SQLite file on drop

**Benefits:**
- API tests catch logic bugs fast (runs in milliseconds)
- E2E tests catch integration issues (browser rendering, JS, routes)
- Both use local mode - no PostgreSQL/SMTP/S3 needed
- CI runs both layers in ~2-3 minutes total

## API Testing Design

### Test Structure

```
packages/api/tests/integration/
├── auth_tests.rs      # signup, signin, verify, password reset
├── profile_tests.rs   # create, edit, view profiles
├── proposal_tests.rs  # CRUD proposals, voting
├── program_tests.rs   # CRUD programs, proposal bundling
├── comment_tests.rs   # create, list, delete comments
└── common/
    └── mod.rs         # shared test utilities
```

### Test Utilities

**Location:** `packages/api/src/test_utils.rs`

```rust
pub struct TestContext {
    pub pool: Pool<Any>,
    pub state: Arc<AppState>,
    db_path: PathBuf,
}

impl TestContext {
    pub async fn new() -> Self {
        let uuid = Uuid::new_v4();
        let db_path = PathBuf::from(format!(".test-{}.db", uuid));

        // Create SQLite database
        let database = SqliteDatabase::new(db_path.to_string_lossy().to_string()).await.unwrap();
        database.run_migrations().await.unwrap();

        // Seed if empty (creates test users)
        if let SqliteDatabase { pool, .. } = &database {
            seed_if_empty(pool).await.unwrap();
        }

        let pool = database.pool().await.clone();

        // Create minimal AppState for tests
        let state = Arc::new(AppState {
            db: Arc::new(database),
            email: Arc::new(ConsoleEmailService),
            storage: Arc::new(FilesystemStorage::new(".test-uploads".into())),
            config: AppConfig {
                mode: AppMode::Local,
                // ... minimal config
            },
        });

        Self { pool, state, db_path }
    }

    pub async fn seed_user(&self, email: &str, password: &str) -> User {
        // Create and return authenticated user
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        std::fs::remove_file(&self.db_path).ok();
        std::fs::remove_dir_all(".test-uploads").ok();
    }
}
```

### Example Tests

**Auth Flow:**
```rust
#[tokio::test]
async fn test_signup_and_signin() {
    let ctx = TestContext::new().await;

    // Signup
    signup("test@example.com".to_string(), "Password123".to_string())
        .await
        .expect("signup should succeed");

    // Should be able to signin
    let token = signin("test@example.com".to_string(), "Password123".to_string())
        .await
        .expect("signin should succeed");

    assert!(!token.is_empty());
}

#[tokio::test]
async fn test_signup_rejects_weak_password() {
    let ctx = TestContext::new().await;

    let result = signup("test@example.com".to_string(), "weak".to_string()).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Password must be"));
}
```

**Proposal CRUD:**
```rust
#[tokio::test]
async fn test_create_and_vote_proposal() {
    let ctx = TestContext::new().await;
    let user = ctx.seed_user("author@test.com", "Password123").await;

    // Create proposal
    let proposal_id = create_proposal(
        "Test Proposal".to_string(),
        "Description".to_string(),
        user.auth_token.clone(),
    ).await.unwrap();

    // Upvote
    upvote_proposal(proposal_id.clone(), user.auth_token.clone())
        .await
        .unwrap();

    // Verify vote count
    let proposal = get_proposal(proposal_id).await.unwrap();
    assert_eq!(proposal.upvotes, 1);
}
```

### Test Coverage

**Auth Tests:**
- ✅ Successful signup → verify email → signin flow
- ✅ Weak password rejection
- ✅ Duplicate email prevention
- ✅ Signin with unverified email blocked
- ✅ Password reset flow (request → reset with token)
- ✅ Expired verification/reset tokens rejected
- ✅ Invalid JWT tokens rejected

**Proposal/Program Tests:**
- ✅ CRUD operations (create, read, update, delete)
- ✅ Only author can edit/delete
- ✅ Voting (upvote, downvote, can't vote twice)
- ✅ Comment threads (create, list, delete own comments)
- ✅ Programs bundle proposals correctly
- ✅ Pagination for lists

**Profile Tests:**
- ✅ Create and update profile
- ✅ View own and others' profiles
- ✅ Activity feed shows user actions

## E2E Testing Design

### Test Structure

```
tests/e2e/
├── auth.spec.rs           # signup, signin, signout flows
├── proposals.spec.rs      # create, edit, vote, comment on proposals
├── programs.spec.rs       # create programs, bundle proposals
├── profile.spec.rs        # view and edit profile
├── navigation.spec.rs     # routing, navbar, page loads
└── helpers/
    └── mod.rs             # browser helpers, page objects
```

### Test Server Setup

Each E2E test file starts its own server:

```rust
pub struct TestServer {
    url: String,
    process: Child,
    db_path: PathBuf,
}

impl TestServer {
    pub async fn start() -> Self {
        let port = get_random_port();
        let uuid = Uuid::new_v4();
        let db_path = PathBuf::from(format!(".e2e-test-{}.db", uuid));

        // Set environment variables
        std::env::set_var("APP_MODE", "local");
        std::env::set_var("PORT", port.to_string());
        std::env::set_var("JWT_SECRET", "test-secret-key-min-32-chars-long");

        // Start server process
        let process = Command::new("cargo")
            .args(&["run", "--package", "web", "--features", "server"])
            .spawn()
            .expect("Failed to start server");

        // Wait for server to be ready
        wait_for_server(&format!("http://localhost:{}", port)).await;

        Self {
            url: format!("http://localhost:{}", port),
            process,
            db_path,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.process.kill().ok();
        std::fs::remove_file(&self.db_path).ok();
    }
}
```

### Playwright Integration

**Dependencies** (`tests/e2e/Cargo.toml`):
```toml
[package]
name = "e2e"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
playwright = "0.0.20"  # or use headless_chrome if playwright bindings immature
serde_json = "1.0"
```

**Example E2E Test:**
```rust
use playwright::Playwright;

#[tokio::test]
async fn test_full_signup_flow() {
    let server = TestServer::start().await;
    let playwright = Playwright::initialize().await.unwrap();
    let browser = playwright.chromium().launcher().headless(true).launch().await.unwrap();
    let page = browser.new_page().await.unwrap();

    // Navigate to signup
    page.goto(&format!("{}/auth/signup", server.url())).await.unwrap();

    // Fill form
    page.fill("input[name=email]", "user@test.com").await.unwrap();
    page.fill("input[name=password]", "Password123").await.unwrap();
    page.fill("input[name=confirm_password]", "Password123").await.unwrap();
    page.click("button[type=submit]").await.unwrap();

    // Check success message
    let text = page.text_content(".success-message").await.unwrap();
    assert!(text.unwrap().contains("Check your email"));

    browser.close().await.unwrap();
}
```

### Helper Utilities

**Location:** `tests/e2e/helpers/mod.rs`

```rust
pub async fn login_as(page: &Page, email: &str, password: &str) {
    page.goto(&format!("{}/auth/signin", BASE_URL)).await.unwrap();
    page.fill("input[name=email]", email).await.unwrap();
    page.fill("input[name=password]", password).await.unwrap();
    page.click("button[type=submit]").await.unwrap();
    page.wait_for_url("**/me").await.unwrap();
}

pub async fn create_proposal(page: &Page, title: &str, body: &str) -> String {
    page.goto(&format!("{}/proposals/new", BASE_URL)).await.unwrap();
    page.fill("input[name=title]", title).await.unwrap();
    page.fill("textarea[name=body]", body).await.unwrap();
    page.click("button[type=submit]").await.unwrap();

    // Extract proposal ID from URL
    page.url().await.unwrap().split('/').last().unwrap().to_string()
}
```

### E2E Critical Paths

- ✅ Complete user journey: signup → verify → signin → create proposal → vote → comment
- ✅ Navigation: all routes load without errors
- ✅ Responsive layout (desktop viewport)
- ✅ Form validation errors display correctly
- ✅ 404 page for invalid routes
- ✅ Signout clears session

## CI/CD Integration

### Makefile Commands

**Location:** `Makefile` (add these targets)

```makefile
test-integration: ## Run API integration tests
	cargo test --package api --test '*' -- --test-threads=1

test-e2e: ## Run E2E browser tests
	cargo test --package e2e --test '*' -- --test-threads=1

test-all: test test-integration test-e2e ## Run all tests (unit + integration + E2E)
```

### GitHub Actions Updates

**Location:** `.github/workflows/ci-cd.yml`

**Update the test job:**
```yaml
jobs:
  test:
    name: Test
    runs-on: ubuntu-latest

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: ${{ env.RUST_VERSION }}

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Install Playwright dependencies
        run: |
          npx playwright install --with-deps chromium

      - name: Run unit tests
        run: make test-ci

      - name: Run integration tests
        run: make test-integration
        env:
          APP_MODE: local
          JWT_SECRET: test-secret-key-for-ci-only-min-32-chars

      - name: Run E2E tests
        run: make test-e2e
        env:
          APP_MODE: local
          JWT_SECRET: test-secret-key-for-ci-only-min-32-chars

      - name: Upload test artifacts on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: test-artifacts
          path: |
            tests/e2e/screenshots/
            .test-*.db
            .e2e-test-*.db
```

### CI Performance

- **Unit tests:** ~30 seconds
- **Integration tests:** ~1 minute
- **E2E tests:** ~2-3 minutes
- **Total CI time:** ~4-5 minutes

**Optimizations:**
- Playwright browser cached by CI
- Rust dependencies cached by `rust-cache`
- Tests run sequentially (`--test-threads=1`) to avoid port conflicts
- Database files cleaned up automatically

## Error Handling & Developer Experience

### Test Failure Debugging

**API Tests:**
- Use `RUST_LOG=debug` to see SQL queries and logic flow
- Failed assertions show actual vs expected values
- Database state preserved on failure (don't delete `.test-*.db` files immediately)
- Clear assertion messages: `assert_eq!(actual, expected, "Expected user to be verified after token verification")`

**E2E Tests:**
- Screenshot on failure saved to `tests/e2e/screenshots/{test_name}.png`
- Browser console logs captured and printed on failure
- Video recording of failed tests (optional, via Playwright config)
- Server logs included in test output
- Test database preserved for inspection

### Local Development

**Run specific test:**
```bash
# Single API test
cargo test --package api --test auth_tests test_signup

# Single E2E test
cargo test --package e2e --test auth test_signup_flow
```

**Debug mode:**
```bash
# E2E with visible browser (non-headless)
HEADLESS=false cargo test --package e2e --test auth

# Keep server running after E2E failure
KEEP_SERVER=true cargo test --package e2e

# Verbose test output
cargo test -- --nocapture --test-threads=1
```

**Clean test artifacts:**
```bash
# Remove all test databases
rm -f .test-*.db .e2e-test-*.db

# Remove screenshots
rm -rf tests/e2e/screenshots/
```

### Error Scenarios Tested

- ✅ Invalid form input (client-side validation)
- ✅ Server errors (500 responses)
- ✅ Authentication failures (401 responses)
- ✅ Authorization failures (403 responses)
- ✅ Not found errors (404 responses)
- ✅ Database constraint violations
- ✅ Concurrent operations (race conditions)
- ✅ Token expiration

### Documentation

**Create:**
- `tests/README.md` - how to run tests, troubleshoot failures
- `tests/CONTRIBUTING.md` - how to add new tests, testing conventions

**Content:**
- Quick start: running all tests
- Running specific test suites
- Debugging failed tests
- Writing new tests (patterns to follow)
- CI/CD integration details

## Test Organization Principles

### Independent Tests
- Each test is self-contained (no shared state)
- Fresh database per test (API) or per suite (E2E)
- Tests can run in any order
- No dependencies between tests

### Descriptive Names
```rust
// Good
test_signup_rejects_weak_password()
test_user_cannot_edit_others_proposal()
test_upvote_increments_vote_count()

// Bad
test_auth()
test_proposals()
test_voting()
```

### DRY with Helpers
- Common setup in helper functions
- Reusable assertions
- Page objects for E2E (encapsulate selectors)

### Fast Feedback
- API tests run in parallel (fast)
- E2E tests run sequentially (avoid conflicts, still fast enough)
- Fail fast on first error (optional flag)

## Implementation Dependencies

### New Rust Crates

**`packages/api/Cargo.toml`** (dev-dependencies):
```toml
[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
```

**`tests/e2e/Cargo.toml`** (new package):
```toml
[package]
name = "e2e"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
playwright = "0.0.20"  # or headless_chrome
serde_json = "1.0"
```

**Workspace `Cargo.toml`:**
```toml
[workspace]
members = [
    "packages/ui",
    "packages/web",
    "packages/desktop",
    "packages/mobile",
    "packages/api",
    "tests/e2e",  # Add E2E package
]
```

### Environment Variables

**For CI only:**
```
APP_MODE=local
JWT_SECRET=test-secret-key-for-ci-only-min-32-chars
```

## Rollout Plan

### Phase 1: Test Infrastructure
1. Create `packages/api/src/test_utils.rs` with `TestContext`
2. Add test utilities to `packages/api/lib.rs` (gated by `#[cfg(test)]`)
3. Set up E2E package structure (`tests/e2e/`)
4. Add Makefile targets for test commands
5. Test infrastructure locally

### Phase 2: API Tests
1. Write auth integration tests
2. Write proposal integration tests
3. Write program integration tests
4. Write comment integration tests
5. Write profile integration tests
6. Verify all API tests pass locally

### Phase 3: E2E Tests
1. Create `TestServer` helper
2. Set up Playwright configuration
3. Write auth E2E tests
4. Write proposal E2E tests
5. Write navigation E2E tests
6. Verify all E2E tests pass locally

### Phase 4: CI Integration
1. Update `.github/workflows/ci-cd.yml`
2. Add Playwright installation step
3. Run integration and E2E tests in CI
4. Test full CI pipeline on feature branch
5. Merge to main

### Phase 5: Documentation
1. Write `tests/README.md`
2. Write `tests/CONTRIBUTING.md`
3. Update root `README.md` with testing section
4. Add troubleshooting guide

## Future Enhancements

- Video recording for all E2E tests (not just failures)
- Performance benchmarks (track response times)
- Mobile viewport testing (responsive design)
- Accessibility testing (ARIA, keyboard navigation)
- Visual regression testing (screenshot comparison)
- Load testing (concurrent users)
- Mutation testing (verify test quality)
- Test coverage reporting (codecov integration)

## Questions for Future Consideration

1. Should we add contract testing for API endpoints?
2. Parallel E2E execution with multiple server ports?
3. Flaky test detection and automatic retries?
4. Test data generators for more realistic scenarios?
