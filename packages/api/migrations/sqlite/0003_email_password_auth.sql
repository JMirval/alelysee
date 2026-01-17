-- Add email/password auth columns to users table
alter table users
  add column email text;
alter table users
  add column password_hash text;
alter table users
  add column email_verified integer not null default 0;

create unique index if not exists users_email_unique_idx on users(email);

-- Email verification tokens
create table if not exists email_verifications (
    id text primary key default (
        lower(hex(randomblob(4))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(6)))
    ),
    user_id text not null references users(id) on delete cascade,
    token_hash text not null unique,
    expires_at text not null,
    created_at text not null default current_timestamp
);

create index if not exists email_verifications_token_idx on email_verifications(token_hash);
create index if not exists email_verifications_user_idx on email_verifications(user_id);

-- Password reset tokens
create table if not exists password_resets (
    id text primary key default (
        lower(hex(randomblob(4))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(6)))
    ),
    user_id text not null references users(id) on delete cascade,
    token_hash text not null unique,
    expires_at text not null,
    created_at text not null default current_timestamp
);

create index if not exists password_resets_token_idx on password_resets(token_hash);
create index if not exists password_resets_user_idx on password_resets(user_id);
