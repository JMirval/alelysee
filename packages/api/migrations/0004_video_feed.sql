-- Add bookmarks and video views tracking for TikTok-style feed

-- Bookmarks table
create table if not exists bookmarks (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id) on delete cascade,
    video_id uuid not null references videos(id) on delete cascade,
    created_at timestamptz not null default now(),
    unique(user_id, video_id)
);

create index if not exists bookmarks_user_idx on bookmarks(user_id, created_at desc);
create index if not exists bookmarks_video_idx on bookmarks(video_id);

-- Video views tracking (for recommendations and no-replay)
create table if not exists video_views (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id) on delete cascade,
    video_id uuid not null references videos(id) on delete cascade,
    created_at timestamptz not null default now(),
    unique(user_id, video_id)
);

create index if not exists video_views_user_idx on video_views(user_id, created_at desc);
create index if not exists video_views_video_idx on video_views(video_id);
