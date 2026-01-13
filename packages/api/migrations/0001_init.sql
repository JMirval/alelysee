-- Alelysee: initial schema
-- Requires Postgres extensions for UUID generation
create extension if not exists pgcrypto;

do $$ begin
    create type content_target_type as enum ('proposal', 'program', 'video', 'comment');
exception
    when duplicate_object then null;
end $$;

do $$ begin
    create type activity_action as enum ('created', 'voted_up', 'voted_down', 'commented');
exception
    when duplicate_object then null;
end $$;

create table if not exists users (
    id uuid primary key default gen_random_uuid(),
    auth_subject text not null unique,
    created_at timestamptz not null default now()
);

create table if not exists profiles (
    user_id uuid primary key references users(id) on delete cascade,
    display_name text not null,
    bio text not null default '',
    avatar_url text,
    location text,
    updated_at timestamptz not null default now()
);

create table if not exists proposals (
    id uuid primary key default gen_random_uuid(),
    author_user_id uuid not null references users(id) on delete cascade,
    title text not null,
    summary text not null default '',
    body_markdown text not null default '',
    tags text[] not null default '{}'::text[],
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index if not exists proposals_author_idx on proposals(author_user_id);
create index if not exists proposals_created_idx on proposals(created_at desc);

create table if not exists programs (
    id uuid primary key default gen_random_uuid(),
    author_user_id uuid not null references users(id) on delete cascade,
    title text not null,
    summary text not null default '',
    body_markdown text not null default '',
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index if not exists programs_author_idx on programs(author_user_id);
create index if not exists programs_created_idx on programs(created_at desc);

create table if not exists program_items (
    program_id uuid not null references programs(id) on delete cascade,
    proposal_id uuid not null references proposals(id) on delete cascade,
    position int not null default 0,
    primary key(program_id, proposal_id)
);

create index if not exists program_items_program_pos_idx on program_items(program_id, position);

create table if not exists videos (
    id uuid primary key default gen_random_uuid(),
    owner_user_id uuid not null references users(id) on delete cascade,
    target_type content_target_type not null,
    target_id uuid not null,
    storage_bucket text not null,
    storage_key text not null,
    content_type text not null,
    duration_seconds int,
    created_at timestamptz not null default now()
);

create index if not exists videos_target_idx on videos(target_type, target_id);
create index if not exists videos_owner_idx on videos(owner_user_id);

create table if not exists votes (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id) on delete cascade,
    target_type content_target_type not null,
    target_id uuid not null,
    value smallint not null check (value in (-1, 1)),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique(user_id, target_type, target_id)
);

create index if not exists votes_target_idx on votes(target_type, target_id);

create table if not exists comments (
    id uuid primary key default gen_random_uuid(),
    author_user_id uuid not null references users(id) on delete cascade,
    target_type content_target_type not null,
    target_id uuid not null,
    parent_comment_id uuid references comments(id) on delete cascade,
    body_markdown text not null,
    created_at timestamptz not null default now()
);

create index if not exists comments_target_idx on comments(target_type, target_id, created_at asc);
create index if not exists comments_parent_idx on comments(parent_comment_id);

create table if not exists activity (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id) on delete cascade,
    action activity_action not null,
    target_type content_target_type not null,
    target_id uuid not null,
    created_at timestamptz not null default now()
);

create index if not exists activity_user_created_idx on activity(user_id, created_at desc);
