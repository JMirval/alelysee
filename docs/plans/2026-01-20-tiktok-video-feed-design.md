# TikTok-Style Video Feed - Design Document

**Date:** 2026-01-20
**Status:** Approved
**Author:** Claude Code with user collaboration

## Overview

This feature adds a full-screen, TikTok-style video browsing experience to Alelysee. Users can discover political proposal and program videos through a personalized feed, interact with votes and bookmarks, and browse videos from specific proposals/programs.

## Goals

1. **Discovery**: Help users discover video content across all proposals and programs through personalized recommendations
2. **Engagement**: Provide intuitive interaction (vote, bookmark, comment) without leaving the video feed
3. **Focus**: Allow users to browse all videos from a single proposal/program
4. **Bookmarking**: Let users save videos to revisit later in their profile
5. **No Replay**: Never show the same video twice unless all videos have been exhausted

## Architecture Overview

### Database Schema

**New Tables:**

```sql
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
```

**Indexes Strategy:**
- `bookmarks_user_idx`: Fast lookup of user's bookmarks chronologically
- `video_views_user_idx`: Efficient check if user has seen a video
- Both tables use unique constraint to prevent duplicates

### API Endpoints

**New Server Functions:**

1. **`list_feed_videos(id_token: String, cursor: Option<String>, limit: i64) -> Result<Vec<Video>, ServerFnError>`**
   - Returns personalized video feed using collaborative filtering
   - Excludes videos user has already viewed
   - Resets view history when all videos exhausted

2. **`list_single_content_videos(target_type: ContentTargetType, target_id: String, cursor: Option<String>, limit: i64) -> Result<Vec<Video>, ServerFnError>`**
   - Returns videos for a specific proposal or program
   - Supports cursor-based pagination
   - Used when browsing videos from a single piece of content

3. **`bookmark_video(id_token: String, video_id: String) -> Result<bool, ServerFnError>`**
   - Toggles bookmark on/off for a video
   - Returns true if bookmarked, false if unbookmarked
   - Uses unique constraint for idempotency

4. **`list_bookmarked_videos(id_token: String, cursor: Option<String>, limit: i64) -> Result<Vec<Video>, ServerFnError>`**
   - Returns user's bookmarked videos
   - Sorted by bookmark creation time (most recent first)
   - Includes full video metadata

5. **`mark_video_viewed(id_token: String, video_id: String) -> Result<(), ServerFnError>`**
   - Records that user has viewed a video
   - Called when video is in viewport for >2 seconds
   - Prevents duplicate views with unique constraint

### Recommendation Algorithm

The feed uses collaborative filtering with three signal sources:

**1. Collaborative Filtering (40% weight):**
```sql
-- Find videos liked by users who liked videos you liked
SELECT DISTINCT v.* FROM videos v
JOIN votes vo ON vo.target_type = 'video' AND vo.target_id = v.id AND vo.value = 1
WHERE vo.user_id IN (
    SELECT DISTINCT vo2.user_id FROM votes vo2
    JOIN votes vo3 ON vo3.target_type = 'video' AND vo3.value = 1 AND vo3.user_id = $1
    WHERE vo2.target_type = 'video' AND vo2.value = 1
        AND vo2.target_id = vo3.target_id
        AND vo2.user_id != $1
)
AND v.id NOT IN (SELECT video_id FROM video_views WHERE user_id = $1)
LIMIT 20
```

**2. Popular Videos (30% weight):**
```sql
-- Videos with highest vote scores in past 7 days
SELECT v.*, COALESCE(SUM(vo.value), 0) as score
FROM videos v
LEFT JOIN votes vo ON vo.target_type = 'video' AND vo.target_id = v.id
WHERE v.created_at > NOW() - INTERVAL '7 days'
    AND v.id NOT IN (SELECT video_id FROM video_views WHERE user_id = $1)
GROUP BY v.id
ORDER BY score DESC
LIMIT 15
```

**3. High Interaction (30% weight):**
```sql
-- Videos with most votes + comments (comments weighted 2x)
SELECT v.*,
    (COUNT(DISTINCT vo.id) + COUNT(DISTINCT c.id) * 2) as interaction_score
FROM videos v
LEFT JOIN votes vo ON vo.target_type = 'video' AND vo.target_id = v.id
LEFT JOIN comments c ON c.target_type = 'video' AND c.target_id = v.id
WHERE v.created_at > NOW() - INTERVAL '7 days'
    AND v.id NOT IN (SELECT video_id FROM video_views WHERE user_id = $1)
GROUP BY v.id
ORDER BY interaction_score DESC
LIMIT 15
```

**Algorithm Flow:**
1. Query all three sources in parallel
2. Merge results with weighted shuffling (40/30/30)
3. Filter out already-viewed videos
4. If result is empty (all videos exhausted), delete user's view history and recurse once
5. Apply cursor-based pagination
6. Return batch of videos

**Pagination:**
- Cursor format: `base64(video_id|timestamp)` for deterministic ordering
- Each request returns `limit` videos plus next cursor
- Client tracks cursor for infinite scroll

## UI Components

### Component Hierarchy

```
VideoFeed (container)
├── VideoFeedItem (repeated per video)
│   ├── <video> element
│   ├── VideoMetadata (bottom overlay)
│   └── VideoOverlay (right side controls)
│       ├── UpvoteButton
│       ├── DownvoteButton
│       ├── BookmarkButton
│       └── CommentButton
└── CommentPanel (slide-up drawer)
    └── CommentThread (reused component)
```

### VideoFeed Component

**Responsibilities:**
- Load initial batch of videos (3-5 videos)
- Handle scroll/swipe navigation
- Manage infinite scroll (lazy load more)
- Track current video index
- Sync URL with current video
- Manage filter context (for single-content mode)

**State:**
- `current_video_index: Signal<usize>`
- `videos: Signal<Vec<Video>>`
- `loading_more: Signal<bool>`
- `cursor: Signal<Option<String>>`
- `filter_context: Signal<Option<(ContentTargetType, String)>>`

**Scroll Handling:**
- CSS `scroll-snap-type: y mandatory` on container
- Each video has `scroll-snap-align: start`
- IntersectionObserver detects which video is in viewport
- Updates `current_video_index` and URL

**Swipe Handling:**
- Touch event listeners: `touchstart`, `touchmove`, `touchend`
- Detect horizontal swipe (dx > 50px)
- Programmatic scroll to next/previous video

**Keyboard Handling:**
- Arrow Down: scroll to next video
- Arrow Up: scroll to previous video
- Space: toggle play/pause

**Infinite Scroll:**
- When scrolled to 2nd-to-last video: trigger `load_more()`
- Prevents duplicate loads with `loading_more` flag
- Appends new videos to existing list

### VideoFeedItem Component

**Layout:**
```
┌─────────────────────────┐
│   <video> element       │
│   (fullscreen, cover)   │
│                         │
│  ┌──────────────────┐   │
│  │ Bottom metadata  │   │ ← Title, author, link
│  └──────────────────┘   │
│                      ▲  │
│                      │  │ ← Right side controls
│                      🔺 │    (vote, bookmark, comment)
│                      🔻 │
│                      🔖 │
│                      💬 │
└─────────────────────────┘
```

**Video Element Configuration:**
- `autoplay`: true when in viewport
- `muted`: false (sound on by default)
- `loop`: false (play once)
- `playsInline`: true (for mobile)
- `preload`: "auto" (for current + next 2 videos)
- Object-fit: cover (fill viewport)

**Lifecycle:**
- **On enter viewport:**
  - Call `video.play()`
  - Start 2-second timer
  - After 2 seconds: call `mark_video_viewed()`
  - Update URL to `/videos/:id`
- **On exit viewport:**
  - Call `video.pause()`
  - Cancel view timer if pending

### VideoOverlay Component

**Right-side button stack:**
- Position: `absolute`, `right: 16px`, stacked from `bottom: 120px`
- 60px diameter circular buttons
- 16px gap between buttons
- Semi-transparent background: `rgba(0, 0, 0, 0.4)`

**Buttons:**

1. **Upvote Button:**
   - Icon: arrow up
   - Shows vote score below icon
   - Filled/highlighted if user upvoted
   - On click: call `set_vote()`, optimistically update

2. **Downvote Button:**
   - Icon: arrow down
   - Filled/highlighted if user downvoted
   - On click: call `set_vote()`, optimistically update

3. **Bookmark Button:**
   - Icon: bookmark/heart
   - Filled if bookmarked
   - On click: call `bookmark_video()`, toggle state

4. **Comment Button:**
   - Icon: comment bubble
   - Shows comment count below icon
   - On click: open CommentPanel

**Styling:**
- Animated state transitions (scale on tap)
- Icon size: 28px
- Text size: 12px (for counts)
- Accessibility: proper ARIA labels

### VideoMetadata Component

**Bottom overlay layout:**
- Position: `absolute`, `bottom: 0`, full width
- Gradient background: `linear-gradient(transparent, rgba(0,0,0,0.8))`
- Padding: 24px horizontal, 16px vertical

**Content:**
1. Proposal/Program title (truncated to 2 lines)
2. Author name with avatar (if available)
3. "View full [proposal/program]" link button
4. Timestamp/date (optional)

**Typography:**
- Title: 18px, bold, white
- Author: 14px, regular, rgba(255,255,255,0.8)
- Link: 14px, underlined, accent color

### CommentPanel Component

**Slide-up drawer:**
- Animated from bottom: `translateY(100%)` → `translateY(0)`
- Covers 70% of viewport height
- Semi-transparent backdrop behind panel
- Swipe down or tap backdrop to close

**Layout:**
```
┌─────────────────────────┐
│ ← Comments (23)      ✕  │  ← Header
├─────────────────────────┤
│ Scrollable comment list │
│                         │
│ @user1: Great point...  │
│ @user2: I disagree...   │
│ ...                     │
├─────────────────────────┤
│ [Text input]    [Send]  │  ← Fixed input bar
└─────────────────────────┘
```

**Implementation:**
- Reuses existing `CommentThread` component
- Target: `ContentTargetType::Video`, current video's ID
- Max height: 70vh
- Scrollable content area
- Fixed input bar at bottom

**Behavior:**
- Video continues playing (or pauses) when panel open
- Loads comments on open (lazy loading)
- Supports full CRUD operations on comments

## Routing & Navigation

### Routes

**Primary Routes:**
- `/videos` - Personalized discovery feed (default)
- `/videos/:id` - Opens feed at specific video

**Navigation Flows:**

1. **Discovery Mode:**
   - User navigates to `/videos`
   - Load personalized feed via `list_feed_videos()`
   - No filter context
   - Shows all recommendations

2. **Single-Content Mode:**
   - User clicks video thumbnail on `/proposals/:id` or `/programs/:id`
   - Navigate to `/videos/:video_id`
   - Pass filter context: `(target_type, target_id)`
   - Only show videos from that proposal/program

3. **Bookmarks:**
   - User navigates to `/me` (profile)
   - Clicks "Bookmarks" tab
   - Shows grid of bookmarked videos
   - Click thumbnail → navigate to `/videos/:id` (filtered to that video's content)

**URL Synchronization:**
- As user scrolls, URL updates to current video ID
- Enables sharing direct links to videos
- Browser back button returns to previous page, not previous video

## Profile Integration

### Bookmarks Section

**Location:** `/me` (profile page), new "Bookmarks" tab

**Layout:**
```
┌─────────────────────────┐
│ Profile Header          │
│ (existing: avatar, bio) │
├─────────────────────────┤
│ Tabs:                   │
│ [Activity] [Bookmarks]  │  ← New tab
├─────────────────────────┤
│ Bookmarked Videos (12)  │
│                         │
│ ┌──┐ ┌──┐ ┌──┐         │
│ │▶️│ │▶️│ │▶️│         │  ← Grid layout
│ └──┘ └──┘ └──┘         │
│ ┌──┐ ┌──┐ ┌──┐         │
│ │▶️│ │▶️│ │▶️│         │
│ └──┘ └──┘ └──┘         │
└─────────────────────────┘
```

**Grid Specifications:**
- Desktop: 3 columns
- Tablet: 2 columns
- Mobile: 1 column
- Gap: 16px
- Responsive grid with `grid-template-columns: repeat(auto-fill, minmax(200px, 1fr))`

**Video Thumbnail Card:**
- Video preview (first frame or placeholder)
- Play button overlay
- Duration badge (from `duration_seconds`)
- Vote score badge
- Click → navigate to `/videos/:id`

**Remove Functionality:**
- Long-press or hover shows "Remove" button
- Calls `bookmark_video()` to toggle off
- Optimistically removes from grid
- Shows confirmation toast

**Empty State:**
- Message: "You haven't bookmarked any videos yet"
- Suggestion: "Discover videos to save your favorites"
- Button: "Explore Videos" → `/videos`

**Data Loading:**
- Call `list_bookmarked_videos()` on mount
- Infinite scroll: load more at bottom
- Loading skeleton during fetch
- Error state with retry button

## Testing Strategy

### Unit Tests

**API Functions (`packages/api/src/video_feed_tests.rs`):**

1. `test_list_feed_videos_collaborative_filtering`
   - Create users A, B, C with videos
   - User A upvotes video 1
   - User B upvotes video 1 and video 2
   - User A's feed should include video 2 (collaborative)

2. `test_list_feed_videos_excludes_viewed`
   - Create videos 1, 2, 3
   - Mark videos 1, 2 as viewed
   - Feed should only return video 3

3. `test_list_feed_videos_reset_when_exhausted`
   - Create 3 videos
   - Mark all as viewed
   - Feed should reset views and return all 3

4. `test_bookmark_video_toggle`
   - Bookmark video → verify in DB
   - Bookmark again → verify removed
   - Check idempotency

5. `test_mark_video_viewed_prevents_duplicates`
   - Mark video as viewed twice
   - Verify only one row in DB

6. `test_list_single_content_videos_filters_by_target`
   - Create proposal with 3 videos
   - Create program with 2 videos
   - Query proposal videos → verify only 3 returned

7. `test_feed_weights_sources_correctly`
   - Create diverse video set
   - Verify collaborative videos ~40% of results
   - Verify popular videos ~30% of results
   - Verify interactive videos ~30% of results

### Integration Tests

**Located in `packages/api/src/domain_tests.rs`:**

1. `test_video_feed_workflow`
   - Setup: 3 users, 10 videos
   - User A upvotes videos 1, 2
   - User B upvotes videos 1, 3
   - User C upvotes videos 2, 4
   - User A's feed should include videos 3, 4 (collaborative)
   - User A views all videos
   - Verify views tracked
   - Verify feed resets when exhausted

2. `test_bookmark_workflow`
   - User creates account
   - Bookmark 3 videos
   - Verify in `list_bookmarked_videos`
   - Unbookmark 1 video
   - Verify updated list
   - Check sort order (most recent first)

3. `test_single_content_feed`
   - Create proposal with 5 videos
   - Create program with 3 videos
   - Query proposal videos with pagination
   - Verify only proposal videos
   - Verify cursor pagination works
   - Verify limit respected

4. `test_feed_personalization_cold_start`
   - New user with no votes
   - Feed should show popular/interactive videos
   - Verify no errors with empty vote history

5. `test_concurrent_bookmarks`
   - Simulate 2 concurrent bookmark requests
   - Verify unique constraint handles race condition
   - Verify final state is consistent

### E2E Tests

**Using Playwright (`tests/e2e/video_feed_test.rs` or JS):**

1. `test_video_feed_navigation`
   ```
   - Navigate to /videos
   - Wait for video to load and autoplay
   - Verify video is playing (not paused)
   - Verify sound is on (unmuted)
   - Scroll down
   - Verify next video starts playing
   - Verify previous video stops
   - Verify URL updated to /videos/:new_id
   - Verify back button returns to previous page
   ```

2. `test_video_interactions`
   ```
   - Sign in
   - Navigate to /videos
   - Click upvote button
   - Verify button filled/highlighted
   - Verify score incremented
   - Click bookmark button
   - Verify button filled
   - Navigate to /me
   - Click Bookmarks tab
   - Verify video appears in grid
   - Click video thumbnail
   - Verify navigates to /videos/:id
   ```

3. `test_comment_panel`
   ```
   - Navigate to /videos
   - Click comment button
   - Verify panel slides up
   - Verify comments loaded
   - Type comment and submit
   - Verify comment appears
   - Close panel (tap backdrop)
   - Verify panel slides down
   - Reopen panel
   - Verify comment persists
   ```

4. `test_single_content_mode`
   ```
   - Navigate to /proposals/:id
   - Wait for page load
   - Click video thumbnail
   - Verify navigates to /videos/:video_id
   - Scroll through videos
   - Verify only videos from that proposal shown
   - Verify can't scroll to videos from other proposals
   ```

5. `test_swipe_navigation`
   ```
   - Navigate to /videos (on mobile viewport)
   - Simulate touch swipe right
   - Verify scrolls to next video
   - Simulate touch swipe left
   - Verify scrolls to previous video
   ```

6. `test_keyboard_navigation`
   ```
   - Navigate to /videos
   - Press Arrow Down
   - Verify scrolls to next video
   - Press Arrow Up
   - Verify scrolls to previous video
   - Press Space
   - Verify video pauses/plays
   ```

7. `test_infinite_scroll`
   ```
   - Navigate to /videos
   - Scroll through 5 videos (initial batch)
   - Verify loading indicator appears
   - Verify more videos loaded
   - Continue scrolling
   - Verify seamless pagination
   ```

8. `test_view_tracking`
   ```
   - Navigate to /videos
   - Watch video for >2 seconds
   - Scroll to next video immediately (<2 seconds)
   - Scroll back to first video
   - Verify first video marked as viewed (doesn't reappear in feed)
   - Verify second video not marked as viewed (reappears)
   ```

### Database Migration Tests

1. **PostgreSQL Migration:**
   - Run migration on test database
   - Verify tables created
   - Verify indexes created
   - Insert test data
   - Verify constraints work (unique, foreign keys)

2. **SQLite Migration:**
   - Same tests as PostgreSQL
   - Verify SQLite-specific syntax works
   - Test local dev mode compatibility

3. **Migration Rollback:**
   - Run migration
   - Run rollback (if applicable)
   - Verify clean state

## Implementation Phases

### Phase 1: Database & API Foundation
**Goal:** Set up data layer and core API endpoints

- [ ] Create migration file for `bookmarks` and `video_views` tables
- [ ] Run migration on dev database (PostgreSQL and SQLite)
- [ ] Implement `mark_video_viewed()` server function
- [ ] Implement `bookmark_video()` server function (toggle)
- [ ] Implement `list_bookmarked_videos()` server function
- [ ] Write unit tests for new API functions
- [ ] Test on local dev mode

**Success Criteria:**
- Migrations run cleanly on both PostgreSQL and SQLite
- All unit tests pass
- API endpoints callable from client

### Phase 2: Feed Algorithm
**Goal:** Implement personalized recommendation system

- [ ] Implement `list_feed_videos()` with collaborative filtering query
- [ ] Add popular videos query (weighted 30%)
- [ ] Add high-interaction videos query (weighted 30%)
- [ ] Implement weighted shuffling algorithm
- [ ] Add view history exclusion logic
- [ ] Implement reset behavior when videos exhausted
- [ ] Add cursor-based pagination
- [ ] Implement `list_single_content_videos()` with filtering
- [ ] Write integration tests for algorithm
- [ ] Performance test with 1000+ videos

**Success Criteria:**
- Feed returns diverse, personalized videos
- No repeated videos unless exhausted
- Pagination works smoothly
- Query performance <200ms

### Phase 3: Core UI Components
**Goal:** Build video feed container with navigation

- [ ] Create `VideoFeed` component in `packages/ui/src/video_feed.rs`
- [ ] Implement scroll-snap CSS for vertical scrolling
- [ ] Add IntersectionObserver for viewport detection
- [ ] Implement video play/pause lifecycle
- [ ] Add scroll event handling
- [ ] Implement touch swipe gesture detection
- [ ] Add keyboard navigation (arrow keys)
- [ ] Implement URL synchronization (update on scroll)
- [ ] Add infinite scroll (lazy load more videos)
- [ ] Implement loading states and skeletons

**Success Criteria:**
- Videos snap to viewport on scroll
- Smooth navigation with scroll/swipe/keyboard
- URL updates as user scrolls
- Infinite scroll loads more videos seamlessly

### Phase 4: Video Controls & Overlay
**Goal:** Add interactive buttons and metadata display

- [ ] Create `VideoFeedItem` component
- [ ] Implement fullscreen video element with autoplay
- [ ] Create `VideoOverlay` component (right-side buttons)
- [ ] Add upvote button with optimistic updates
- [ ] Add downvote button with optimistic updates
- [ ] Add bookmark button with toggle behavior
- [ ] Add comment button (opens panel)
- [ ] Create `VideoMetadata` component (bottom overlay)
- [ ] Display proposal/program title, author, link
- [ ] Wire up vote API calls
- [ ] Wire up bookmark API calls
- [ ] Add error handling and retry logic

**Success Criteria:**
- All buttons functional and responsive
- Optimistic updates feel instant
- Error states handled gracefully
- Metadata displays correctly

### Phase 5: Comment Panel
**Goal:** Enable commenting from video feed

- [ ] Create `CommentPanel` component
- [ ] Implement slide-up animation (bottom drawer)
- [ ] Integrate existing `CommentThread` component
- [ ] Add open/close logic (swipe/tap backdrop)
- [ ] Wire up to comment button in overlay
- [ ] Handle video pause/play when panel open
- [ ] Add lazy loading (load comments on open)
- [ ] Test comment CRUD operations
- [ ] Add loading and error states

**Success Criteria:**
- Panel slides up smoothly
- Comments load and display correctly
- Can post new comments
- Closing panel resumes video

### Phase 6: Profile Integration
**Goal:** Add bookmarks section to profile page

- [ ] Update `packages/ui/src/profile.rs`
- [ ] Add "Bookmarks" tab to profile UI
- [ ] Create `BookmarksSection` component
- [ ] Implement grid layout (responsive columns)
- [ ] Create video thumbnail card component
- [ ] Add click handler (navigate to `/videos/:id`)
- [ ] Implement remove bookmark functionality
- [ ] Add infinite scroll for bookmarks list
- [ ] Create empty state UI
- [ ] Add loading and error states

**Success Criteria:**
- Bookmarks tab appears in profile
- Grid layout responsive across devices
- Can view and remove bookmarks
- Navigation to video feed works

### Phase 7: Routing & Navigation
**Goal:** Integrate video feed into app routing

- [ ] Add `/videos` route to router
- [ ] Add `/videos/:id` route with param parsing
- [ ] Update proposal detail page: add click handler to video thumbnails
- [ ] Update program detail page: add click handler to video thumbnails
- [ ] Implement navigation with filter context
- [ ] Add route guards (auth requirements)
- [ ] Handle 404 for invalid video IDs
- [ ] Test browser back button behavior
- [ ] Add navigation to main app menu/header

**Success Criteria:**
- All routes functional
- Clicking thumbnails opens video feed
- Single-content mode filters correctly
- Back button navigation works

### Phase 8: E2E Testing
**Goal:** Comprehensive end-to-end test coverage

- [ ] Set up E2E test framework (if not exists)
- [ ] Write `test_video_feed_navigation`
- [ ] Write `test_video_interactions` (vote, bookmark)
- [ ] Write `test_comment_panel`
- [ ] Write `test_single_content_mode`
- [ ] Write `test_swipe_navigation` (mobile)
- [ ] Write `test_keyboard_navigation` (desktop)
- [ ] Write `test_infinite_scroll`
- [ ] Write `test_view_tracking`
- [ ] Test on multiple browsers (Chrome, Firefox, Safari)
- [ ] Test on mobile devices (iOS, Android)
- [ ] Test on desktop app
- [ ] CI/CD integration

**Success Criteria:**
- All E2E tests pass
- Cross-browser compatibility verified
- Mobile and desktop apps work correctly
- Tests run in CI pipeline

### Phase 9: Polish & Performance
**Goal:** Optimize performance and UX

- [ ] Add video preloading strategy (current + next 2)
- [ ] Optimize recommendation query performance
- [ ] Add error boundaries for resilience
- [ ] Implement retry logic for failed requests
- [ ] Add analytics tracking (view time, interactions)
- [ ] Accessibility audit (keyboard nav, screen readers)
- [ ] Add ARIA labels to all interactive elements
- [ ] Test with slow network (3G simulation)
- [ ] Add loading skeletons for better perceived performance
- [ ] Performance profiling (identify bottlenecks)
- [ ] Memory leak testing (long scrolling sessions)
- [ ] Final UX review and adjustments

**Success Criteria:**
- Smooth 60fps scrolling
- Fast initial load (<2s)
- No memory leaks during extended use
- WCAG AA accessibility compliance
- Works well on slow connections

## Technical Considerations

### Video Thumbnails (MVP Simplification)

For MVP, we won't generate server-side video thumbnails. Instead:
- Use video element with `preload="metadata"` to show first frame
- Or use generic placeholder with play icon
- Future enhancement: server-side thumbnail extraction using FFmpeg

### View History Cleanup

The `video_views` table will grow continuously. For MVP, we accept this. Future optimizations:
- Periodic cleanup job (delete views >90 days old)
- Add `reset_view_history` endpoint for users
- Consider moving to time-based windowing instead of permanent storage

### Recommendation Algorithm Tuning

The 40/30/30 weights are initial estimates. Future improvements:
- A/B testing framework to optimize weights
- Machine learning model for better personalization
- Category/tag-based filtering (requires tagging system)
- Time-of-day and trending signals

### Performance Optimization

**Database Queries:**
- Recommendation queries are complex and may be slow
- Consider materialized views for popular/interactive scores
- Add query result caching (Redis) for common requests
- Monitor query performance in production

**Video Preloading:**
- Preload current + next 2 videos
- Cancel preload for videos scrolled past
- Adaptive preloading based on network speed

**State Management:**
- Consider using Dioxus state management patterns
- Avoid unnecessary re-renders
- Memoize expensive computations

### Mobile Considerations

**Gestures:**
- Touch swipe should feel native
- Prevent browser pull-to-refresh conflicts
- Support pinch-to-zoom on video (optional)

**Performance:**
- Video decoding is CPU-intensive on mobile
- Limit preloading on slower devices
- Consider video quality adaptation (HLS/DASH)

**Battery:**
- Autoplay with sound drains battery faster
- Consider pausing when app backgrounded
- Monitor and optimize power consumption

### Accessibility

**Keyboard Navigation:**
- All interactions accessible via keyboard
- Focus indicators visible and clear
- Logical tab order

**Screen Readers:**
- Proper ARIA labels on all controls
- Announce video title and metadata
- Announce state changes (voted, bookmarked)

**Captions:**
- Future: support for video captions/subtitles
- Critical for accessibility compliance

## Future Enhancements

### Short-Term (Next 3-6 months)
- Video thumbnail generation (server-side FFmpeg)
- View history cleanup strategy
- Search and filter in video feed
- Share video feature (copy link, social media)
- Playlist creation (group bookmarks)

### Medium-Term (6-12 months)
- Advanced recommendation ML model
- Video upload quality optimization (compression)
- Live streaming support
- Video analytics dashboard (creators)
- Push notifications for new videos

### Long-Term (12+ months)
- Video editing tools (trim, add captions)
- Duet/reaction videos
- Video challenges and campaigns
- Content moderation tools
- Multi-language support for videos

## Success Metrics

**Engagement:**
- Average watch time per session
- Videos watched per user per session
- Vote rate (% of videos voted on)
- Bookmark rate (% of videos bookmarked)
- Comment rate (% of videos commented on)

**Discovery:**
- % of users discovering new proposals/programs via video feed
- Click-through rate to full proposal/program pages
- Repeat view rate (users returning to video feed)

**Technical:**
- Feed load time (p50, p95, p99)
- Video start time (time to first frame)
- Error rate (API failures, video playback errors)
- Infinite scroll smoothness (no stutters)

**Target Metrics (MVP):**
- Average 5+ videos watched per session
- <2s initial feed load time
- >80% video start success rate
- <5% API error rate

## Open Questions & Decisions

### Resolved:
- ✅ Feed shows personalized recommendations (not all videos)
- ✅ Algorithm uses collaborative filtering + popular + interactive
- ✅ Videos autoplay with sound on
- ✅ Bookmarks appear in profile page
- ✅ Right-side overlay for controls (TikTok-style)
- ✅ Both scroll and swipe navigation supported
- ✅ Comments accessible via slide-up panel
- ✅ Routes: `/videos` and `/videos/:id`
- ✅ Load 3-5 videos initially, lazy load more

### For Future Discussion:
- Video thumbnail generation approach
- View history retention policy
- A/B testing framework for algorithm tuning
- Video quality/compression settings
- Mobile app-specific optimizations
- Content moderation requirements

---

**Next Steps:**
1. Review and approve this design
2. Set up git worktree for isolated development
3. Create detailed implementation plan
4. Begin Phase 1 implementation
