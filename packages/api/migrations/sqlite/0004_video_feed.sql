-- Add bookmarks and video views tracking for TikTok-style feed (SQLite version)

-- Bookmarks table
create table if not exists bookmarks (
    id text primary key default (lower(hex(randomblob(16)))),
    user_id text not null references users(id) on delete cascade,
    video_id text not null references videos(id) on delete cascade,
    created_at text not null default (datetime('now')),
    unique(user_id, video_id)
);

create index if not exists bookmarks_user_idx on bookmarks(user_id, created_at desc);
create index if not exists bookmarks_video_idx on bookmarks(video_id);

-- Video views tracking (for recommendations and no-replay)
create table if not exists video_views (
    id text primary key default (lower(hex(randomblob(16)))),
    user_id text not null references users(id) on delete cascade,
    video_id text not null references videos(id) on delete cascade,
    created_at text not null default (datetime('now')),
    unique(user_id, video_id)
);

create index if not exists video_views_user_idx on video_views(user_id, created_at desc);
create index if not exists video_views_video_idx on video_views(video_id);
