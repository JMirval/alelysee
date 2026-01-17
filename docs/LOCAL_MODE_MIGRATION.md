# Local Mode Migration Guide

This guide helps you migrate your development workflow to use the new local development mode, which eliminates the need for external services (PostgreSQL, SMTP, S3) during development.

## Overview

Local mode provides:
- **SQLite database** instead of PostgreSQL
- **Console email output** instead of SMTP
- **Filesystem storage** instead of S3
- **Pre-seeded mock data** for immediate testing
- **Zero external dependencies** for faster onboarding

## Quick Migration

### For New Developers

1. Clone the repository
2. Copy the example environment file:
   ```bash
   cp env.example .env
   ```

3. Edit `.env` and set:
   ```bash
   APP_MODE=local
   JWT_SECRET=dev-secret-min-32-chars-long-please
   ```

4. Start development:
   ```bash
   make dev
   ```

5. Login with mock credentials:
   - Email: `user1@local.dev`
   - Password: `Password123`

That's it! No PostgreSQL, no SMTP server, no S3 configuration needed.

## Detailed Migration

### From Production-Like Local Setup

If you were previously running PostgreSQL, Redis, or other services locally:

**Before (Production Mode):**
```bash
# .env file
DATABASE_URL=postgres://postgres:postgres@localhost:5432/alelysee
REDIS_URL=redis://localhost:6379
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_USERNAME=test
SMTP_PASSWORD=test
SMTP_FROM=noreply@example.com
AWS_ACCESS_KEY_ID=minioadmin
AWS_SECRET_ACCESS_KEY=minioadmin
AWS_REGION=us-east-1
AWS_S3_BUCKET=alelysee
AWS_S3_ENDPOINT=http://localhost:9000
JWT_SECRET=your-secret-key

# Required services running
docker-compose up -d  # PostgreSQL, Redis, MinIO, MailHog
```

**After (Local Mode):**
```bash
# .env file
APP_MODE=local
JWT_SECRET=your-secret-key

# No services required - just run the app!
make dev
```

### Migrating Data

Local mode uses a fresh SQLite database with pre-seeded mock data. If you need to migrate existing development data:

**Option 1: Start Fresh (Recommended)**
- Use the pre-seeded mock data
- Fastest way to get started
- Consistent test data across team

**Option 2: Export/Import Test Data**
- Export data from PostgreSQL using `pg_dump`
- Convert to SQLite-compatible SQL
- Load into `.dev/local.db`
- Note: This is manual and not officially supported

**Option 3: Use Production Mode Locally**
- Keep `APP_MODE=production` (or omit it)
- Continue using PostgreSQL and other services
- Useful for testing production-like scenarios

## Environment Variables

### Required in All Modes

```bash
JWT_SECRET=your-secret-key-min-32-chars
```

### Local Mode Only

```bash
APP_MODE=local
```

All other environment variables (DATABASE_URL, SMTP_*, AWS_*) are **ignored in local mode**.

### Production Mode

When `APP_MODE=production` or not set, all production environment variables are required:
```bash
DATABASE_URL=postgres://...
REDIS_URL=redis://...
SMTP_HOST=...
SMTP_PORT=...
SMTP_USERNAME=...
SMTP_PASSWORD=...
SMTP_FROM=...
AWS_ACCESS_KEY_ID=...
AWS_SECRET_ACCESS_KEY=...
AWS_REGION=...
AWS_S3_BUCKET=...
JWT_SECRET=...
```

## Mock Data Reference

Local mode pre-seeds the following data:

### Users (All password: `Password123`)
1. `user1@local.dev` - Active user with proposals
2. `user2@local.dev` - Active user with comments
3. `user3@local.dev` - Active user

### Proposals
- 12 total proposals across different states (draft, submitted, approved, rejected)
- Various categories: Education, Environment, Technology, Healthcare
- Mix of video and text content

### Programs
- 3 active programs
- Different eligibility criteria and funding amounts

### Comments
- 25 comments across proposals
- Mix of parent and reply comments

### Resetting Data

To reset local data and get fresh seed data:
```bash
rm -rf .dev/
make dev
```

## Feature Differences

### Email Behavior

**Local Mode:**
- Emails are printed to console/stdout
- Look for "📧 EMAIL (Local Mode - Not Sent)" in logs
- Full email content visible for debugging

**Production Mode:**
- Emails sent via SMTP server
- Must configure SMTP_* environment variables

### Storage Behavior

**Local Mode:**
- Files stored in `.dev/uploads/`
- Organized by content type: `.dev/uploads/proposals/{id}/video.mp4`
- Served at `http://localhost:8080/dev/uploads/...`

**Production Mode:**
- Files uploaded to S3-compatible storage
- Must configure AWS_* environment variables
- Served via CDN/S3 URLs

### Database Behavior

**Local Mode:**
- SQLite database at `.dev/local.db`
- Single-file database (easy to reset/delete)
- Limited concurrent write performance
- Good for single-developer testing

**Production Mode:**
- PostgreSQL database
- Full ACID compliance
- Better concurrent write performance
- Suitable for production loads

## Troubleshooting

### Database Locked Error

**Symptom:** `database is locked` error in SQLite

**Cause:** SQLite doesn't handle many concurrent writes well

**Solutions:**
1. Reduce concurrent requests (most common during automated tests)
2. Switch to production mode with PostgreSQL locally
3. Add retry logic for database operations

### Emails Not Visible

**Symptom:** Not seeing email content

**Solution:**
- Check stdout/console logs
- Look for "📧 EMAIL (Local Mode - Not Sent)"
- Email content is printed immediately

### Video Uploads Failing

**Symptom:** File upload errors

**Solutions:**
1. Check `.dev/uploads/` directory exists and is writable
2. Check console for detailed error messages
3. Verify disk space available

### Mock Users Can't Login

**Symptom:** Authentication fails for mock users

**Solutions:**
1. Verify using correct credentials: `user1@local.dev` / `Password123`
2. Check startup logs for "Seeded local database" message
3. Reset database: `rm -rf .dev/ && make dev`
4. Check `.dev/local.db` file exists

### App Mode Not Detected

**Symptom:** App still requires production env vars

**Solution:**
- Ensure `.env` has `APP_MODE=local` (lowercase)
- Check for typos in environment variable name
- Restart the application after changing `.env`

## Testing Across Modes

### Testing Local Mode

```bash
# Set up local mode
echo "APP_MODE=local" > .env
echo "JWT_SECRET=test-secret-key-min-32-chars" >> .env

# Run tests
cargo test --workspace

# Manual testing
make dev
# Login at http://localhost:8080 with user1@local.dev / Password123
```

### Testing Production Mode

```bash
# Start required services
docker-compose up -d

# Configure production mode
cp env.example .env
# Edit .env with all production variables
# Remove or comment out APP_MODE=local

# Run tests
cargo test --workspace

# Manual testing
make dev
```

## CI/CD Considerations

### GitHub Actions / CI

Local mode is ideal for CI environments:

```yaml
- name: Test with local mode
  env:
    APP_MODE: local
    JWT_SECRET: ci-test-secret-key-min-32-chars
  run: cargo test --workspace
```

No need to spin up PostgreSQL, Redis, or other services in CI.

### Deployment

Production deployments should **never** use local mode:

```bash
# Railway / production environment
# Do NOT set APP_MODE (defaults to production)
# Ensure all production env vars are configured
```

## Team Workflow

### Recommended Setup

**Individual Developers:**
- Use local mode for daily development
- Fast iteration, no service management
- Consistent mock data

**Integration Testing:**
- Use production mode with docker-compose
- Test against real PostgreSQL
- Verify production-like behavior

**CI/CD:**
- Use local mode for fast tests
- Optional: production mode for integration tests

### Sharing Data

**Local Mode:**
- Share `.dev/` directory is NOT recommended (gitignored)
- Everyone gets same seed data automatically
- Consistent starting point

**Production Mode:**
- Use database dumps for specific test scenarios
- Share docker-compose configs
- Document any custom seed data

## Best Practices

1. **Default to Local Mode**
   - Start all new feature branches in local mode
   - Only switch to production mode when testing external integrations

2. **Reset Data Frequently**
   - Before starting new features: `rm -rf .dev/`
   - After major changes to seed data
   - When data becomes inconsistent

3. **Check Console for Emails**
   - Always review email output in logs
   - Verify email content and formatting
   - Test email templates changes locally

4. **Commit .env.example**
   - Keep example file updated with local mode defaults
   - Document any new environment variables
   - Help new team members onboard faster

5. **Use Production Mode for Final Testing**
   - Before merging to main/production
   - Test with real S3, SMTP, PostgreSQL
   - Verify production configuration

## Migration Checklist

- [ ] Update `.env` with `APP_MODE=local`
- [ ] Remove unused production env vars from `.env` (optional)
- [ ] Stop local PostgreSQL/Redis/MinIO containers (optional)
- [ ] Run `make dev` and verify startup logs show "LOCAL" mode
- [ ] Test login with `user1@local.dev` / `Password123`
- [ ] Verify email output in console
- [ ] Test file uploads to `.dev/uploads/`
- [ ] Update team documentation/README with local mode instructions
- [ ] Share this guide with team members

## Getting Help

**Check Logs First:**
- Startup logs show detected mode and configuration
- Error messages include context and suggestions

**Common Issues:**
- See Troubleshooting section above
- Check `.dev/` directory permissions
- Verify `.env` file is loaded (print statements in config.rs)

**Production Mode Issues:**
- Ensure all required env vars are set
- Check service connectivity (DATABASE_URL, SMTP_HOST, etc.)
- Review Railway logs for deployment issues

## Additional Resources

- [README.md](../README.md) - General development setup
- [env.example](../env.example) - Example environment configuration
- [packages/api/src/config.rs](../packages/api/src/config.rs) - Configuration implementation
- [packages/api/src/db/](../packages/api/src/db/) - Database implementations
