-- SQLite: initial schema
PRAGMA foreign_keys = ON;

create table if not exists users (
    id text primary key default (
        lower(hex(randomblob(4))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(6)))
    ),
    auth_subject text not null unique,
    created_at text not null default current_timestamp
);

create table if not exists profiles (
    user_id text primary key references users(id) on delete cascade,
    display_name text not null,
    bio text not null default '',
    avatar_url text,
    location text,
    updated_at text not null default current_timestamp
);

create table if not exists proposals (
    id text primary key default (
        lower(hex(randomblob(4))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(6)))
    ),
    author_user_id text not null references users(id) on delete cascade,
    title text not null,
    summary text not null default '',
    body_markdown text not null default '',
    tags text not null default '[]',
    created_at text not null default current_timestamp,
    updated_at text not null default current_timestamp
);

create index if not exists proposals_author_idx on proposals(author_user_id);
create index if not exists proposals_created_idx on proposals(created_at desc);

create table if not exists programs (
    id text primary key default (
        lower(hex(randomblob(4))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(6)))
    ),
    author_user_id text not null references users(id) on delete cascade,
    title text not null,
    summary text not null default '',
    body_markdown text not null default '',
    created_at text not null default current_timestamp,
    updated_at text not null default current_timestamp
);

create index if not exists programs_author_idx on programs(author_user_id);
create index if not exists programs_created_idx on programs(created_at desc);

create table if not exists program_items (
    program_id text not null references programs(id) on delete cascade,
    proposal_id text not null references proposals(id) on delete cascade,
    position int not null default 0,
    primary key(program_id, proposal_id)
);

create index if not exists program_items_program_pos_idx on program_items(program_id, position);

create table if not exists videos (
    id text primary key default (
        lower(hex(randomblob(4))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(6)))
    ),
    owner_user_id text not null references users(id) on delete cascade,
    target_type text not null,
    target_id text not null,
    storage_bucket text not null,
    storage_key text not null,
    content_type text not null,
    duration_seconds int,
    created_at text not null default current_timestamp
);

create index if not exists videos_target_idx on videos(target_type, target_id);
create index if not exists videos_owner_idx on videos(owner_user_id);

create table if not exists votes (
    id text primary key default (
        lower(hex(randomblob(4))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(6)))
    ),
    user_id text not null references users(id) on delete cascade,
    target_type text not null,
    target_id text not null,
    value smallint not null check (value in (-1, 1)),
    created_at text not null default current_timestamp,
    updated_at text not null default current_timestamp,
    unique(user_id, target_type, target_id)
);

create index if not exists votes_target_idx on votes(target_type, target_id);

create table if not exists comments (
    id text primary key default (
        lower(hex(randomblob(4))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(6)))
    ),
    author_user_id text not null references users(id) on delete cascade,
    target_type text not null,
    target_id text not null,
    parent_comment_id text references comments(id) on delete cascade,
    body_markdown text not null,
    created_at text not null default current_timestamp
);

create index if not exists comments_target_idx on comments(target_type, target_id, created_at asc);
create index if not exists comments_parent_idx on comments(parent_comment_id);

create table if not exists activity (
    id text primary key default (
        lower(hex(randomblob(4))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(2))) || '-' ||
        lower(hex(randomblob(6)))
    ),
    user_id text not null references users(id) on delete cascade,
    action text not null,
    target_type text not null,
    target_id text not null,
    created_at text not null default current_timestamp
);

create index if not exists activity_user_created_idx on activity(user_id, created_at desc);
