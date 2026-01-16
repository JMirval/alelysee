# Email/Password Authentication System Design

**Date:** 2026-01-16
**Status:** Approved
**Author:** Claude (via brainstorming session)

## Overview

Add a complete email/password authentication system to Alelysee alongside the existing OAuth flow. The OAuth backend remains intact but UI is hidden. The new system includes signup, email verification, login, and password reset functionality.

## Goals

- Implement email/password authentication using local JWT tokens
- Maintain consistency with existing OAuth JWT flow
- Provide complete user lifecycle: signup → verify → login → reset password
- Use production-grade security (Argon2id, secure tokens, SMTP via Stalwart)

## Architecture

### Authentication Strategy

**Local JWT Tokens:**
- Generate our own JWT tokens signed with `JWT_SECRET` env var
- Use HS256 algorithm (symmetric signing)
- Store in localStorage like current OAuth flow
- 30-day expiration (configurable)
- Token contains user UUID as subject claim

**Dual Auth Paths:**
1. **OAuth path:** Existing RS256 JWT verification (UI hidden, backend remains)
2. **Email/password path:** New HS256 JWT generation and verification

**Unified token verification:**
- Update `verify_id_token()` to check JWT algorithm header
- Route to RS256 verification (OAuth) or HS256 verification (local)
- Both paths produce same user context

### Database Schema

**Migration: Add columns to `users` table:**
```sql
alter table users
  add column if not exists email text unique,
  add column if not exists password_hash text,
  add column if not exists email_verified boolean not null default false;
```

**New table: `email_verifications`**
```sql
create table email_verifications (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id) on delete cascade,
    token_hash text not null unique,
    expires_at timestamptz not null,
    created_at timestamptz not null default now()
);

create index email_verifications_token_idx on email_verifications(token_hash);
create index email_verifications_user_idx on email_verifications(user_id);
```

**New table: `password_resets`**
```sql
create table password_resets (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id) on delete cascade,
    token_hash text not null unique,
    expires_at timestamptz not null,
    created_at timestamptz not null default now()
);

create index password_resets_token_idx on password_resets(token_hash);
create index password_resets_user_idx on password_resets(user_id);
```

**Key design decisions:**
- `email` nullable for backwards compatibility with OAuth users
- `auth_subject` for email/password users = user UUID
- `password_hash` nullable (OAuth users don't have passwords)
- Token hashes stored (not raw tokens) using SHA-256

## API Design

### Server Functions

**Location:** `packages/api/src/auth.rs`

#### 1. `signup(email: String, password: String) -> Result<(), ServerFnError>`

**Flow:**
1. Validate email format (basic regex)
2. Validate password strength (min 8 chars, uppercase, lowercase, number)
3. Check email doesn't already exist
4. Hash password with Argon2id (default params)
5. Create user: `auth_subject = user.id.to_string()`
6. Generate verification token (32 random bytes → 64 hex chars)
7. Hash token with SHA-256, store in `email_verifications` (expires in 24h)
8. Send verification email via SMTP
9. Return success

**Error cases:**
- Email already registered: "Email already registered"
- Weak password: "Password must be at least 8 characters with uppercase, lowercase, and number"
- SMTP failure: Log error, return generic "Failed to send verification email"

#### 2. `verify_email(token: String) -> Result<(), ServerFnError>`

**Flow:**
1. Hash provided token with SHA-256
2. Look up in `email_verifications` by `token_hash`
3. Check expiration (reject if expired)
4. Set `users.email_verified = true` for associated user
5. Delete verification record
6. Return success

**Error cases:**
- Token not found or expired: "Verification link is invalid or has expired"
- Already verified: Return success (idempotent)

#### 3. `signin(email: String, password: String) -> Result<String, ServerFnError>`

**Flow:**
1. Look up user by email
2. Verify user has `password_hash` (not OAuth-only user)
3. Verify password using Argon2id
4. Check `email_verified = true`
5. Generate local JWT with `sub = user.id`, `iss = "alelysee"`, `exp = now + 30 days`
6. Sign with `JWT_SECRET` using HS256
7. Return JWT token

**Error cases:**
- Email not found or wrong password: "Invalid email or password" (generic)
- Email not verified: "Please verify your email before signing in"
- OAuth user trying password login: "This account uses OAuth. Please sign in with your provider."

#### 4. `request_password_reset(email: String) -> Result<(), ServerFnError>`

**Flow:**
1. Look up user by email
2. If not found: return success anyway (security: don't reveal email existence)
3. If found and has `password_hash`:
   - Generate reset token (32 random bytes → 64 hex chars)
   - Hash token with SHA-256, store in `password_resets` (expires in 1h)
   - Send reset email via SMTP
4. Return success (always)

**Error cases:**
- SMTP failure: Log error, return generic success (don't reveal)

#### 5. `reset_password(token: String, new_password: String) -> Result<(), ServerFnError>`

**Flow:**
1. Hash provided token with SHA-256
2. Look up in `password_resets` by `token_hash`
3. Check expiration
4. Validate new password strength
5. Hash new password with Argon2id
6. Update `users.password_hash`
7. Delete reset record
8. Return success

**Error cases:**
- Token not found or expired: "Reset link is invalid or has expired"
- Weak password: "Password must be at least 8 characters with uppercase, lowercase, and number"

### Helper Functions

**`generate_local_jwt(user_id: Uuid) -> Result<String, anyhow::Error>`**
- Create claims: `sub`, `iss`, `exp`, `iat`
- Sign with HS256 using `JWT_SECRET`
- Return encoded token

**`verify_local_jwt(token: &str) -> Result<Uuid, anyhow::Error>`**
- Decode header, check algorithm is HS256
- Validate signature with `JWT_SECRET`
- Verify expiration
- Return user UUID from `sub` claim

**Update `verify_id_token(id_token: &str) -> Result<String, anyhow::Error>`**
- Decode header
- If algorithm is RS256: use existing OAuth verification
- If algorithm is HS256: use new local verification
- Return auth subject (user UUID)

## Email System

### SMTP Configuration

**New env vars:**
```
SMTP_HOST=stalwart.railway.internal
SMTP_PORT=587
SMTP_USERNAME=your-username
SMTP_PASSWORD=your-password
SMTP_SECURE_MODE=starttls
SMTP_FROM_EMAIL=noreply@alelysee.com
SMTP_FROM_NAME=Alelysee
APP_BASE_URL=http://localhost:8080
```

### Email Module

**Location:** `packages/api/src/email.rs`

**Dependencies:** `lettre` crate with SMTP transport

**Function: `send_email(to: &str, subject: &str, html: &str, text: &str) -> Result<(), anyhow::Error>`**
- Build SMTP client from env vars
- Create multipart message (HTML + plain text)
- Set from address and name
- Send via Stalwart
- Return result

**Function: `send_verification_email(to: &str, token: &str) -> Result<(), anyhow::Error>`**
- Generate verification URL: `{APP_BASE_URL}/auth/verify?token={token}`
- HTML template: branded email with clear CTA button
- Text template: plain text with link
- Subject: "Verify your email address"
- Call `send_email()`

**Function: `send_password_reset_email(to: &str, token: &str) -> Result<(), anyhow::Error>`**
- Generate reset URL: `{APP_BASE_URL}/auth/reset-password/confirm?token={token}`
- HTML template: branded email with clear CTA button, mention 1h expiry
- Text template: plain text with link
- Subject: "Reset your password"
- Call `send_email()`

### Token Generation

**Function: `generate_token() -> String`**
- Use `rand::thread_rng()` with `rand::Rng::gen::<[u8; 32]>()`
- Encode as hex string (64 characters)
- Cryptographically secure

**Function: `hash_token(token: &str) -> String`**
- Use SHA-256 (via `sha2` crate)
- Return hex-encoded hash
- Fast, one-way, sufficient for random tokens

## UI Components

### New Components in `packages/ui/src/auth/mod.rs`

#### 1. `SignUpForm` Component

**Rendered at:** `/auth/signup`

**State:**
- `email: String`
- `password: String`
- `confirm_password: String`
- `error: Option<String>`
- `success: bool`

**Validation:**
- Email format check (client-side)
- Password strength indicator (visual feedback)
- Passwords match check

**Flow:**
1. User fills form
2. Submit calls `signup()` server function
3. On success: show message "Check your email to verify your account"
4. On error: display error inline

**Links:**
- "Already have an account? Sign in"

#### 2. `SignInForm` Component (replaces current OAuth-only)

**Rendered at:** `/auth/signin`

**State:**
- `email: String`
- `password: String`
- `error: Option<String>`

**Flow:**
1. User fills form
2. Submit calls `signin()` server function
3. On success:
   - Store JWT in localStorage (`alelysee_id_token`)
   - Update global `id_token` signal
   - Redirect to `/me`
4. On error: display error inline

**Links:**
- "Forgot password?"
- "Need an account? Sign up"

**OAuth section:**
- Commented out with note: "OAuth temporarily disabled"
- Code remains for future re-enabling

#### 3. `VerifyEmailPage` Component

**Rendered at:** `/auth/verify?token=xxx`

**Flow:**
1. Extract token from URL query params
2. On mount: call `verify_email(token)` server function
3. Show loading state
4. On success: "Email verified! You can now sign in"
5. On error: "Verification failed. This link may be expired."

**Links:**
- "Go to sign in" (always visible)

#### 4. `RequestPasswordResetForm` Component

**Rendered at:** `/auth/reset-password`

**State:**
- `email: String`
- `submitted: bool`

**Flow:**
1. User enters email
2. Submit calls `request_password_reset()` server function
3. Always show success message (security)
4. Message: "If that email is registered, you'll receive reset instructions"

**Links:**
- "Back to sign in"

#### 5. `ResetPasswordForm` Component

**Rendered at:** `/auth/reset-password/confirm?token=xxx`

**State:**
- `new_password: String`
- `confirm_password: String`
- `error: Option<String>`

**Validation:**
- Password strength indicator
- Passwords match check

**Flow:**
1. Extract token from URL
2. User enters new password
3. Submit calls `reset_password(token, password)` server function
4. On success: redirect to `/auth/signin` with success message
5. On error: display error inline

### Route Updates

**Location:** `packages/web/src/main.rs`, `packages/desktop/src/main.rs`, `packages/mobile/src/main.rs`

**Add routes:**
- `/auth/signup` → `SignUpForm`
- `/auth/verify` → `VerifyEmailPage`
- `/auth/reset-password` → `RequestPasswordResetForm`
- `/auth/reset-password/confirm` → `ResetPasswordForm`

**Update route:**
- `/auth/signin` → `SignInForm` (email/password, OAuth hidden)

## Security Considerations

### Password Security

- **Hashing:** Argon2id with default recommended parameters
- **Requirements:** Min 8 chars, at least one uppercase, one lowercase, one number
- **Storage:** Never log or display password_hash

### Token Security

- **Generation:** Cryptographically secure random (32 bytes)
- **Storage:** Hashed with SHA-256 in database
- **Expiry:** Email verification 24h, password reset 1h
- **Single-use:** Deleted after successful use

### Attack Prevention

- **Timing attacks:** Argon2 uses constant-time comparison
- **Enumeration:** Password reset doesn't reveal if email exists
- **Brute force:** Consider rate limiting (future enhancement)
- **Session fixation:** Generate new JWT on each login

### Error Messages

- Generic errors for authentication failures: "Invalid email or password"
- Don't reveal whether email exists during password reset
- Clear expiration messages for tokens

### Edge Cases

- Multiple verification requests: Invalidate old tokens when creating new ones
- Multiple password reset requests: Allow multiple valid tokens (user may request multiple times)
- Already verified email: Return success (idempotent)
- OAuth user tries password reset: Show message to use OAuth provider
- User signs up with existing OAuth email: "Email already registered"

## Implementation Dependencies

### New Rust Crates

**`packages/api/Cargo.toml`:**
```toml
argon2 = "0.5"
lettre = "0.11"
sha2 = "0.10"
rand = "0.8"
hex = "0.4"
```

### Environment Variables

**Add to `env.example` and deployment:**
```
JWT_SECRET=your-secret-key-min-32-chars
SMTP_HOST=stalwart.railway.internal
SMTP_PORT=587
SMTP_USERNAME=your-username
SMTP_PASSWORD=your-password
SMTP_SECURE_MODE=starttls
SMTP_FROM_EMAIL=noreply@alelysee.com
SMTP_FROM_NAME=Alelysee
APP_BASE_URL=http://localhost:8080
```

## Testing Strategy

### Manual Testing Checklist

1. Signup flow: valid email/password → receive email → verify → login
2. Weak password rejection during signup
3. Duplicate email rejection during signup
4. Login with unverified email (should reject)
5. Login with wrong password (generic error)
6. Password reset flow: request → receive email → reset → login with new password
7. Expired verification token handling
8. Expired reset token handling
9. OAuth flow still works for existing users (even with UI hidden)

### Database Testing

1. Verify password_hash is Argon2id format
2. Verify tokens are hashed in database
3. Verify token cleanup after use
4. Verify email uniqueness constraint

## Rollout Plan

### Phase 1: Database & Backend
1. Create migration file
2. Run migration on dev database
3. Implement auth functions in `packages/api/src/auth.rs`
4. Implement email module in `packages/api/src/email.rs`
5. Update `verify_id_token()` for dual algorithm support
6. Test with Postman/curl

### Phase 2: UI Components
1. Create new auth components in `packages/ui/src/auth/mod.rs`
2. Update routes in all platforms (web/desktop/mobile)
3. Hide OAuth UI in `SignInForm` (comment out, add note)
4. Add CSS styling for new forms
5. Test signup → verify → login flow

### Phase 3: Email Integration
1. Set up Stalwart on Railway
2. Configure SMTP env vars
3. Test verification emails
4. Test password reset emails
5. Test error handling (SMTP failures)

### Phase 4: Production Deploy
1. Generate production `JWT_SECRET` (cryptographically random, 32+ chars)
2. Configure production SMTP credentials
3. Set production `APP_BASE_URL`
4. Deploy to Railway
5. Monitor logs for errors
6. Test complete flow in production

## Future Enhancements

- Rate limiting on auth endpoints (prevent brute force)
- Password strength meter on client side
- Remember me functionality (longer JWT expiry)
- Two-factor authentication (TOTP)
- Account lockout after failed attempts
- OAuth re-enabling and integration with email/password accounts
- Email change flow with verification
- Account deletion

## Questions for Future Consideration

1. Should we merge OAuth and email/password accounts by email?
2. Rate limiting strategy: in-app vs Railway/reverse proxy?
3. Session management: add ability to revoke tokens?
4. Admin panel for user management?
