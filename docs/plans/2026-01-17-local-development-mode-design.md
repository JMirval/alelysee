# Local Development Mode Design

**Date:** 2026-01-17
**Status:** Approved
**Author:** Claude (via brainstorming session)

## Overview

Add a local development mode that runs the app with mocked services (SQLite, console emails, filesystem storage) while keeping production deployment to Railway unchanged. Mode is controlled by `APP_MODE` environment variable, defaulting to production for safety.

## Goals

- Enable local development without PostgreSQL, SMTP, or S3-compatible storage
- Pre-seed SQLite database with realistic mock data for immediate testing
- Maintain production behavior as default (safe for Railway deployments)
- Support both modes with trait-based abstraction
- No conditional compilation, runtime mode switching only

## Architecture

### Mode Detection

**Enum:**
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Local,
    Production,
}
```

**Detection Logic:**
- Read `APP_MODE` environment variable at startup
- Default to `Production` if unset or invalid value
- Parse: "local" (case-insensitive) → Local, anything else → Production
- Log mode clearly at startup with appropriate log levels

**Safety:**
- Production is the default (prevents accidental mock usage in deployment)
- Only switches to local mode if explicitly set
- Cannot accidentally deploy with mocks to Railway

### Configuration Module

**Location:** `packages/api/src/config.rs`

**Structure:**
```rust
pub struct AppConfig {
    pub mode: AppMode,
    pub database: DatabaseConfig,
    pub email: EmailConfig,
    pub storage: StorageConfig,
    pub jwt_secret: String,
}

pub enum DatabaseConfig {
    PostgreSQL { url: String },
    SQLite { path: String },
}

pub enum EmailConfig {
    SMTP { host: String, port: u16, username: String, password: String, from_email: String, from_name: String },
    Console,
}

pub enum StorageConfig {
    S3 { bucket: String, endpoint: String, region: String, access_key: String, secret_key: String, media_base_url: Option<String> },
    Filesystem { base_path: String, serve_url: String },
}
```

**Validation:**
- Local mode: Only `JWT_SECRET` required from env
- Production mode: All service env vars required (DATABASE_URL, SMTP_*, STORAGE_*)
- `AppConfig::from_env()` validates and returns config or descriptive error
- Clear error messages indicating missing vars and active mode

**Initialization:**
Called at server startup before any services are created.

## Database Abstraction

### Trait Design

**Location:** `packages/api/src/db/mod.rs`

```rust
#[async_trait]
pub trait Database: Send + Sync {
    async fn get_pool(&self) -> &sqlx::AnyPool;
    // Existing query methods can use AnyPool (supports both Postgres and SQLite)
}
```

**Note:** SQLx `AnyPool` supports both PostgreSQL and SQLite with same API.

### PostgreSQL Implementation (Production)

**Location:** `packages/api/src/db/postgres.rs`

- Uses existing `DATABASE_URL` connection
- Creates PostgreSQL connection pool
- Runs migrations on startup (existing behavior)
- Full schema with all tables

### SQLite Implementation (Local)

**Location:** `packages/api/src/db/sqlite.rs`

**Database File:** `.dev/local.db` (gitignored)

**Initialization:**
1. Create `.dev/` directory if missing
2. Connect to SQLite file at `.dev/local.db`
3. Run same migrations as PostgreSQL (SQLx migrations are portable)
4. Check if database is empty (no users)
5. If empty, run seed data script

**Migration Compatibility:**
- SQLx migrations support both PostgreSQL and SQLite
- Use portable SQL (avoid Postgres-specific syntax where possible)
- Test migrations against both databases

### Mock Data Seeding

**Location:** `packages/api/src/db/seed.rs`

**Seed on First Run:**
When `.dev/local.db` doesn't exist or is empty:

**Sample Users (3-5):**
- Email: `user1@local.dev`, `user2@local.dev`, etc.
- Password: `Password123` (meets validation rules)
- All emails pre-verified (`email_verified = true`)
- Varied usernames and profiles

**Sample Proposals (10-15):**
- Realistic French political topics
- Varied creation dates (simulate organic activity)
- Mix of proposal types
- Distributed across different authors

**Sample Programs (2-3):**
- Bundle multiple proposals together
- Realistic French program names
- Varied themes

**Sample Comments (20-30):**
- Distributed across proposals and programs
- Mix of supportive and critical
- Realistic French text
- Varied authors

**Sample Votes:**
- Mix of upvotes and downvotes
- Distributed across content
- Realistic voting patterns

**Sample Activity Feed:**
- Entries for user actions (created proposal, commented, voted)
- Varied timestamps

**Logging:**
On seed completion, log to console:
```
✓ Seeded local database with mock data
  Users: user1@local.dev, user2@local.dev, user3@local.dev
  Password (all): Password123
  Proposals: 12 | Programs: 3 | Comments: 25
```

**Idempotency:**
- Seed function checks if data exists before inserting
- Safe to call multiple times (won't duplicate)
- Can delete `.dev/local.db` to reset and re-seed

## Email Abstraction

### Trait Design

**Location:** `packages/api/src/email.rs`

```rust
#[async_trait]
pub trait EmailService: Send + Sync {
    async fn send_email(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<(), anyhow::Error>;
}
```

### SMTP Implementation (Production)

**Location:** `packages/api/src/email.rs` (refactor existing code)

- Uses existing `lettre` code
- Requires SMTP_* env vars
- Actual email delivery via Stalwart or configured SMTP server
- Keep existing error handling

### Console Implementation (Local)

**Location:** `packages/api/src/email.rs`

**Behavior:**
Pretty-print email to stdout instead of sending.

**Format:**
```
📧 EMAIL (Local Mode - Not Sent)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
To: user@example.com
Subject: Verify your email address
────────────────────────────────
HTML:
<html><body>...</body></html>
────────────────────────────────
Text:
Please verify your email by clicking...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**Error Handling:**
- Console implementation never fails (always returns Ok)
- Useful for local development (no SMTP configuration needed)

**Helper Functions:**
- Keep existing `send_verification_email()` and `send_password_reset_email()`
- They call the trait method internally
- Work transparently with both implementations

## Object Storage Abstraction

### Trait Design

**Location:** `packages/api/src/storage/mod.rs`

```rust
#[async_trait]
pub trait StorageService: Send + Sync {
    async fn upload(&self, key: &str, data: Vec<u8>) -> Result<(), anyhow::Error>;
    async fn get_url(&self, key: &str) -> Result<String, anyhow::Error>;
    async fn delete(&self, key: &str) -> Result<(), anyhow::Error>;
}
```

### S3 Implementation (Production)

**Location:** `packages/api/src/storage/s3.rs`

- Uses existing S3-compatible storage logic
- Requires STORAGE_* env vars
- Generates pre-signed URLs for uploads
- Returns CDN URLs via MEDIA_BASE_URL for playback
- Keep existing error handling

### Filesystem Implementation (Local)

**Location:** `packages/api/src/storage/filesystem.rs`

**Storage Location:** `.dev/uploads/` (gitignored)

**Directory Structure:**
```
.dev/uploads/
  proposals/
    {proposal_id}/video.mp4
  programs/
    {program_id}/video.mp4
```

**Upload:**
- Auto-creates directory structure if missing
- Writes file to `.dev/uploads/{type}/{id}/{filename}`
- Returns Ok on success

**Get URL:**
- Returns local URL: `http://localhost:8080/dev/uploads/{type}/{id}/{filename}`
- Works with local static file serving

**Delete:**
- Removes file from filesystem
- Ignores errors if file doesn't exist

**Static File Serving:**
Add route in `packages/web/src/main.rs` (local mode only):
- Mount `.dev/uploads/` at `/dev/uploads` path
- Only available when `APP_MODE=local`
- Use Dioxus static file serving or `tower-http::services::ServeDir`

**Optional Mock Video:**
Include small sample video file in `.dev/uploads/` for demo purposes (can be committed to git).

## Service Initialization and Dependency Injection

### Application State

**Location:** `packages/api/src/state.rs`

```rust
pub struct AppState {
    pub db: Arc<dyn Database + Send + Sync>,
    pub email: Arc<dyn EmailService + Send + Sync>,
    pub storage: Arc<dyn StorageService + Send + Sync>,
    pub config: AppConfig,
}

impl AppState {
    pub async fn from_config(config: AppConfig) -> Result<Self, anyhow::Error> {
        let db = match &config.database {
            DatabaseConfig::PostgreSQL { url } => {
                Arc::new(PostgresDatabase::connect(url).await?) as Arc<dyn Database + Send + Sync>
            }
            DatabaseConfig::SQLite { path } => {
                let sqlite = SqliteDatabase::connect(path).await?;
                sqlite.run_migrations().await?;
                sqlite.seed_if_empty().await?;
                Arc::new(sqlite) as Arc<dyn Database + Send + Sync>
            }
        };

        let email = match &config.email {
            EmailConfig::SMTP { .. } => {
                Arc::new(SmtpEmailService::new(&config)) as Arc<dyn EmailService + Send + Sync>
            }
            EmailConfig::Console => {
                Arc::new(ConsoleEmailService) as Arc<dyn EmailService + Send + Sync>
            }
        };

        let storage = match &config.storage {
            StorageConfig::S3 { .. } => {
                Arc::new(S3StorageService::new(&config)) as Arc<dyn StorageService + Send + Sync>
            }
            StorageConfig::Filesystem { base_path, serve_url } => {
                Arc::new(FilesystemStorageService::new(base_path, serve_url)) as Arc<dyn StorageService + Send + Sync>
            }
        };

        Ok(Self { db, email, storage, config })
    }
}
```

### Initialization Flow

**Location:** `packages/web/src/main.rs` (and desktop/mobile)

**Startup Sequence:**
1. Load `AppConfig::from_env()` - detects mode, validates env vars
2. Log mode and configuration summary
3. Initialize `AppState::from_config(config).await`
   - Creates database connection (Postgres or SQLite)
   - Runs migrations
   - Seeds SQLite if empty
   - Creates email service (SMTP or Console)
   - Creates storage service (S3 or Filesystem)
4. Provide `AppState` to Dioxus server context
5. Start server

**Error Handling:**
- Startup failures (missing env vars, connection errors) exit with clear error message
- Log mode and which services are active
- Fail fast in production if services unavailable
- More forgiving in local mode (auto-creates directories, etc.)

### Server Function Access

**Example:**
```rust
#[server]
pub async fn signup(email: String, password: String) -> Result<(), ServerFnError> {
    let state = expect_context::<AppState>();

    // Use state.db for database queries
    // Use state.email.send_email() for sending emails

    // ... existing logic
}
```

**Migration:**
Update existing server functions to:
1. Get `AppState` from context
2. Use `state.db`, `state.email`, `state.storage` instead of direct connections
3. Keep existing business logic unchanged

## Testing Strategy

### Unit Tests

**Database Tests:**
- Use SQLite in-memory (`:memory:`) for fast tests
- No env vars needed
- Can explicitly create local mode services

**Email Tests:**
- Use Console implementation (always succeeds)
- Verify correct parameters passed to trait method

**Storage Tests:**
- Use Filesystem implementation with temp directory
- Verify files written correctly

### Integration Tests

**Mode Detection:**
- Test `APP_MODE=local` → Local
- Test `APP_MODE=production` → Production
- Test unset → Production (default)
- Test invalid value → Production (default)

**Configuration:**
- Test local mode only requires JWT_SECRET
- Test production mode requires all service env vars
- Test validation error messages

### Existing Tests

- Continue to work without changes
- Can be enhanced with trait mocks where needed
- Add new tests for mode detection and initialization

## Migration Path

### Backwards Compatibility

**Production Deployments:**
- Existing Railway deployments continue working (default to production)
- No changes to Railway configuration needed
- All existing env vars still work

**Local Development:**
- Developers add `APP_MODE=local` to `.env` file
- Optionally keep DATABASE_URL in local mode (escape hatch to use Postgres)
- If DATABASE_URL exists in local mode, prefer it over SQLite

**Escape Hatches:**
- Can force production mode locally by omitting `APP_MODE` or setting to "production"
- Can use Postgres in local mode by setting DATABASE_URL
- Can mix modes (e.g., local DB with production SMTP for testing emails)

### Refactoring Strategy

**Phase 1: Add Abstractions**
1. Add config module with mode detection
2. Add database trait and implementations
3. Add email trait and implementations
4. Add storage trait and implementations
5. Add AppState and initialization

**Phase 2: Update Server Functions**
1. Update server functions to use AppState context
2. Test each service independently
3. Verify production mode still works

**Phase 3: Add Seeding**
1. Implement seed data generation
2. Test local mode end-to-end
3. Document mock credentials

**Phase 4: Documentation**
1. Update README with local development section
2. Update env.example with APP_MODE comment
3. Add troubleshooting guide

### .gitignore Updates

Add to `.gitignore`:
```
# Local development mode
.dev/
*.db
*.db-shm
*.db-wal
```

## Environment Variables

### Local Mode (.env)

Required:
```
APP_MODE=local
JWT_SECRET=dev-secret-min-32-chars-for-local-testing
```

Optional (escape hatches):
```
DATABASE_URL=postgres://...  # Override SQLite with Postgres
```

### Production Mode (Railway)

No changes needed. Existing env vars:
```
# APP_MODE not set (defaults to production)
DATABASE_URL=postgres://...
JWT_SECRET=production-secret
SMTP_HOST=...
SMTP_PORT=587
SMTP_USERNAME=...
SMTP_PASSWORD=...
SMTP_FROM_EMAIL=...
APP_BASE_URL=...
STORAGE_BUCKET=...
STORAGE_ENDPOINT=...
STORAGE_REGION=...
STORAGE_ACCESS_KEY=...
STORAGE_SECRET_KEY=...
MEDIA_BASE_URL=...
```

## Edge Cases and Error Handling

### Corrupted SQLite Database

**Issue:** `.dev/local.db` becomes corrupted
**Solution:** Delete file, restart app (auto re-seeds)
**Log:** "SQLite database error, delete .dev/local.db to reset"

### Missing Uploads Directory

**Issue:** `.dev/uploads/` deleted while app running
**Solution:** Auto-recreate on next upload
**Error:** Return 404 if trying to access non-existent file

### Mode Mismatch

**Issue:** Switch from local to production or vice versa
**Solution:** Log clear warning about different databases/services
**Example:** "Switching from local SQLite to production PostgreSQL - data will differ"

### Startup Failures

**Production Mode:**
- Missing env vars: Exit with error listing missing vars
- Service connection failure: Exit with error and connection details
- Migration failure: Exit with migration error

**Local Mode:**
- Missing JWT_SECRET: Exit with error (only required var)
- SQLite creation failure: Exit with filesystem error
- Seed failure: Log warning, continue (can retry)

### Runtime Failures

**Database:**
- Query failures return ServerFnError as before
- Connection pool exhaustion handled by SQLx

**Email:**
- SMTP failures return error (production)
- Console never fails (local)

**Storage:**
- S3 failures return error (production)
- Filesystem failures return error (local, but auto-creates dirs)

## Logging and Observability

### Startup Logs

**Local Mode:**
```
INFO  [api::config] App Mode: LOCAL
INFO  [api::db] Using SQLite database: .dev/local.db
INFO  [api::db] Running migrations...
INFO  [api::db] ✓ Seeded local database with mock data
      Users: user1@local.dev, user2@local.dev, user3@local.dev
      Password (all): Password123
      Proposals: 12 | Programs: 3 | Comments: 25
INFO  [api::email] Email service: Console (not sending)
INFO  [api::storage] Storage service: Filesystem (.dev/uploads/)
INFO  [web] Server listening on http://localhost:8080
```

**Production Mode:**
```
INFO  [api::config] App Mode: PRODUCTION
INFO  [api::db] Using PostgreSQL database
INFO  [api::db] Running migrations...
INFO  [api::email] Email service: SMTP (stalwart.railway.internal)
INFO  [api::storage] Storage service: S3 (your-bucket)
INFO  [web] Server listening on 0.0.0.0:8080
```

### Runtime Logs

**Local Mode Email:**
```
📧 EMAIL (Local Mode - Not Sent)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
To: newuser@example.com
Subject: Verify your email address
...
```

**Local Mode Upload:**
```
DEBUG [api::storage] Uploaded to .dev/uploads/proposals/uuid/video.mp4
DEBUG [api::storage] Serving at http://localhost:8080/dev/uploads/proposals/uuid/video.mp4
```

## Documentation Updates

### README Changes

Add section: "Local Development"

```markdown
## Local Development

### Quick Start (No External Services)

1. Set up local mode:
   ```bash
   cp env.example .env
   # Edit .env and set:
   # APP_MODE=local
   # JWT_SECRET=dev-secret-min-32-chars
   ```

2. Run the app:
   ```bash
   make dev
   ```

3. Login with mock user:
   - Email: `user1@local.dev`
   - Password: `Password123`

### Local Mode Features

- **SQLite database** - Data stored in `.dev/local.db`
- **Pre-seeded mock data** - 3 users, 12 proposals, 3 programs, 25 comments
- **Console emails** - Email content printed to stdout (not sent)
- **Filesystem uploads** - Videos stored in `.dev/uploads/`
- **No external services required** - No PostgreSQL, SMTP, or S3 needed

### Resetting Local Data

```bash
rm -rf .dev/
make dev  # Restarts with fresh seed data
```

### Production Mode Locally

To test production mode locally (requires PostgreSQL, SMTP, S3):
```bash
# Remove APP_MODE=local from .env or set APP_MODE=production
# Ensure all production env vars are set
make dev
```
```

### env.example Updates

```bash
## App Mode (optional, defaults to production)
# Set to "local" for development without external services
# APP_MODE=local

## Required in all modes
JWT_SECRET=your-secret-key-min-32-chars

## Required in production mode only
DATABASE_URL=postgres://postgres:postgres@localhost:5432/alelysee
SMTP_HOST=your-smtp-host
SMTP_PORT=587
# ... rest of production vars
```

### Troubleshooting Guide

Add to README:

```markdown
### Local Mode Troubleshooting

**Issue: Database locked**
- SQLite can't handle many concurrent writes
- Solution: Use PostgreSQL locally or reduce concurrent requests

**Issue: Emails not visible**
- Check stdout/console for email output
- Should see "📧 EMAIL (Local Mode - Not Sent)"

**Issue: Video uploads not working**
- Check `.dev/uploads/` directory exists and is writable
- Check console for error messages

**Issue: Mock users can't login**
- Verify `user1@local.dev` / `Password123`
- Check database was seeded (see startup logs)
- Delete `.dev/local.db` and restart to re-seed
```

## Future Enhancements

- Add `make seed` command to re-seed without restarting
- Add configurable seed data via JSON/TOML files
- Add mock OAuth provider for local testing
- Add GraphQL playground in local mode
- Add request logging/debugging in local mode
- Support hot-reload of seed data
- Add seed data for edge cases (empty profiles, no votes, etc.)

## Questions for Future Consideration

1. Should local mode support multiple SQLite databases (one per feature branch)?
2. Add a web UI for browsing local emails (like MailHog)?
3. Support switching modes without restart (reload config endpoint)?
4. Add telemetry/metrics collection in local mode for debugging?
