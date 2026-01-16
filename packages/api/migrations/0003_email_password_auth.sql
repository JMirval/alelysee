-- Add email/password auth columns to users table
alter table users
  add column if not exists email text unique,
  add column if not exists password_hash text,
  add column if not exists email_verified boolean not null default false;

-- Email verification tokens
create table if not exists email_verifications (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id) on delete cascade,
    token_hash text not null unique,
    expires_at timestamptz not null,
    created_at timestamptz not null default now()
);

create index if not exists email_verifications_token_idx on email_verifications(token_hash);
create index if not exists email_verifications_user_idx on email_verifications(user_id);

-- Password reset tokens
create table if not exists password_resets (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id) on delete cascade,
    token_hash text not null unique,
    expires_at timestamptz not null,
    created_at timestamptz not null default now()
);

create index if not exists password_resets_token_idx on password_resets(token_hash);
create index if not exists password_resets_user_idx on password_resets(user_id);
