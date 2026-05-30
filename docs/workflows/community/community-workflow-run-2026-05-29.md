# Community Workflow Run - 2026-05-29

Purpose: Record the live browser pass for the Community AMA and Inner Circle workflows on `http://localhost:8888`.

Run scope:
- Expert AMA user question creation.
- Expert AMA question upvote/like toggle.
- Admin Expert AMA creation.
- Admin AMA status update.
- Admin AMA answer/comment flow.
- Admin AMA featured question toggle.
- Inner Circle creation.
- Inner Circle settings/Danger Zone availability.
- Inner Circle non-owner Danger Zone authorization.
- Inner Circle deletion cleanup.
- Inner Circle deletion with dependent Circle data.
- Community feed pagination query integrity.
- Saved bookmark persistence and removal.
- Post report validation and persistence.
- Post owner edit validation and delete confirmation.
- Poll rendering, voting, vote replacement, expiry, and invalid-option rejection.

## Expert AMA User Flow

Page:
- `/community/circles?tab=ama`

Actions performed:
1. Verified the Expert AMAs tab loaded with `QUESTIONS OPEN`.
2. Submitted a disposable question:
   - `Workflow test question for AMA coverage 2026-05-29T16:03:47.631Z`
3. Reloaded the page.
4. Verified the submitted question persisted after reload.
5. Clicked the question like/upvote control.
6. Verified the count changed from `0` to `1` and `aria-pressed` became `true`.
7. Clicked the same control again.
8. Verified the count changed from `1` to `0` and `aria-pressed` became `false`.

Result:
- Passed.

## Expert AMA Admin Flow

Page:
- `/admin/community/amas.html`

Actions performed:
1. Opened the admin Expert AMAs page as a Super Admin account.
2. Created a disposable AMA:
   - `Workflow Test AMA 2026-05-29T16-06-08-787Z`
   - Expert: `Workflow Expert`
   - Expert title: `Community QA Reviewer`
3. Verified the new AMA appeared in the admin table.
4. Updated the disposable AMA status to `accepting_questions`.
5. Verified the admin table showed `Accepting Questions`.
6. Opened the Questions dialog for the active Cocoa AMA that contained the submitted user question.
7. Answered the submitted user question:
   - `Workflow test expert answer 2026-05-29T16:08:03.972Z`
8. Verified the question changed from `Pending` to `Answered`.
9. Featured the same question.
10. Verified the question changed to `Featured`.
11. Returned to `/community/circles?tab=ama`.
12. Verified the user-facing AMA showed:
    - submitted question
    - `Featured`
    - `Expert Answer`
    - submitted answer text

Result:
- Passed.

Notes:
- The current user-facing AMA page displays the selected/leading AMA detail rather than a broad visible list of all AMAs. Admin-created AMAs can be created and moved to `accepting_questions`, but the public page did not visibly switch to the newly created AMA while another active AMA was already leading the detail view.

## Inner Circle Create/Delete Flow

Page:
- `/community/circles?tab=circle`
- `/community/circle/:slug/settings`

Actions performed:
1. Opened My Circles.
2. Created a disposable Circle:
   - `Workflow Test Circle 2026-05-29T16-09-23-903Z`
3. Reloaded the page.
4. Verified the Circle appeared under My Circles and Discover.
5. Opened the generated settings URL:
   - `/community/circle/workflow-test-circle-2026-05-29t16-09-23-903z/settings`
6. Verified settings loaded with:
   - Circle name
   - owner role
   - Basic Info
   - Privacy
   - Content Settings
   - Danger Zone
   - `Delete circle` button
7. Triggered the `Delete circle` action.

Result:
- Create and settings verification passed.
- Initial delete attempt exposed a workflow blocker: the native browser `confirm()` dialog could not be reliably controlled by the in-app browser automation.
- Fixed by replacing the native confirm with an in-app `ccs-delete-confirm-modal`.
- Retest passed: `Delete circle` opens a visible confirmation dialog, the modal confirmation button deletes the Circle, `/community/circles?tab=circle` no longer shows the Circle, and the previous settings URL shows `Circle not found.`
- Database verification returned no remaining `workflow-test-circle-2026-05-29t16-09-23-903z` row.

## Inner Circle Dependent Data Delete Flow

Page/API:
- `/community/circle/:slug/settings`
- `/api/community/circles/:id`

Actions performed:
1. Seeded a disposable owner-owned Circle.
2. Added representative Circle-owned/dependent rows:
   - member
   - pending join request
   - ban row
   - announcement-style Circle post with image URL
   - content report against that Circle post
   - resource row
   - resource version rows
   - ops alert
   - ops alert notification
   - Circle-scoped AMA/event
   - event RSVP
   - Circle-scoped challenge
   - challenge progress
   - onboarding progress
   - daily analytics snapshot
3. Opened Circle settings as owner to establish the normal browser session and CSRF token.
4. Called `DELETE /api/community/circles/:id` as the owner.
5. Verified the delete returned success.
6. Counted every seeded dependency table after deletion.
7. Verified all seeded Circle-owned/dependent rows were removed.
8. Verified `/api/community/circles/by-slug/:slug` returned `404`.

Result:
- Passed.

Notes:
- The direct E2E API call must include the browser CSRF token, matching the UI's `X-CSRF-Token` header behavior.
- This now covers the previous follow-up item for deleting Circles with representative dependent posts, image-post data, members, join requests, resources, reports, announcements/events, and challenges.

## Inner Circle Non-Owner Danger Zone Flow

Page:
- `/community/circle/:slug/settings`
- `/api/community/circles/:id`

Actions performed:
1. Seeded a disposable Circle owned by a test owner.
2. Added three non-owner members with elevated and normal roles:
   - `admin`
   - `moderator`
   - `member`
3. Opened Circle settings as each non-owner role.
4. Verified the owner-only Danger Zone card is hidden.
5. Verified the Danger Zone sidebar nav item is hidden.
6. Verified the delete confirmation modal stays hidden.
7. Called `DELETE /api/community/circles/:id` directly as each non-owner role.
8. Verified every direct delete attempt returned `403`.
9. Verified the Circle still existed after every denied delete attempt.
10. Verified the owner could still fetch the Circle by slug after all denied attempts.

Result:
- Initial pass found a UI bug: `#ccs-danger-card` received `hidden=""`, but `.ccs-card--danger { display: grid; }` overrode the browser's default hidden behavior.
- Fixed with the scoped CSS rule `.ccs-page--v2 [hidden] { display: none !important; }`.
- Retest passed for `admin`, `moderator`, and `member` non-owner roles.

## Cleanup

Cleanup performed:
- Removed the disposable AMA question and related upvotes.
- Removed the disposable AMA.
- Verified no remaining `Workflow Test Circle %` rows.

Cleanup result:
- Passed.

## Feed Pagination Query Flow

Page/API:
- `/community?tab=feed`
- `/community/partials/feed/list`

Actions performed:
1. Seeded 25 disposable global feed posts for a fresh test user.
2. Opened `/community?tab=feed`.
3. Waited for the first feed page to render.
4. Scrolled the infinite-scroll sentinel into view.
5. Captured the HTMX page-2 request to `/community/partials/feed/list`.
6. Verified the request returned `200`.
7. Verified `feed_mode=all` appeared exactly once.
8. Verified `sort_by=fresh` appeared exactly once.

Result:
- Initial pass reproduced the documented bug: page-2 requests were sent as `page=2&feed_mode=all&sort_by=fresh&feed_mode=all&sort_by=fresh&post_type=all&tag=all`, causing a `400` duplicate-field deserialization error.
- Fixed by setting `hx-include="this"` on the feed pagination sentinel so the inherited `#feed-filters` form is not submitted on top of the already-built sentinel URL.
- Retest passed.

## Saved Bookmark Flow

Page/API:
- `/community?tab=feed`
- `/community?tab=saved`
- `/api/community/posts/:id/bookmark`
- `/api/community/posts/:id/bookmark/status`

Actions performed:
1. Seeded a disposable feed post.
2. Opened `/community?tab=feed`.
3. Clicked the bookmark button and captured the bookmark API response.
4. Verified the button became bookmarked and `aria-pressed="true"`.
5. Queried `/api/community/posts/:id/bookmark/status`.
6. Verified the status API returned `bookmarked: true`.
7. Opened `/community?tab=saved`.
8. Verified the bookmarked post appeared in the Saved feed.
9. Unbookmarked the post from the Saved feed.
10. Reloaded `/community?tab=saved`.
11. Verified the post no longer appeared in Saved.

Result:
- Passed.

## Post Report Flow

Page/API:
- `/community`
- `/api/community/posts/:id/report`
- `content_reports`

Actions performed:
1. Seeded a disposable feed post.
2. Opened `/community` and verified the post rendered.
3. Called the report endpoint with an invalid empty `reason`.
4. Verified the server rejected the invalid reason with `400`.
5. Opened the report modal from the post card.
6. Selected `financial_advice`, entered a moderator note, and submitted the report through the UI.
7. Verified the report request returned `200`.
8. Submitted a follow-up report request with an over-limit note.
9. Verified the server capped the persisted note to 500 characters.
10. Verified `content_reports` stored the allowlisted reason and `pending` status.

Result:
- Initial review found the server accepted arbitrary report reasons.
- Fixed by adding a server-side allowlist for `spam`, `harassment`, `financial_advice`, and `inappropriate`.
- Retest passed.

## Post Owner Edit/Delete Flow

Page/API:
- `/community?tab=feed`
- `/api/community/posts/:id`
- `posts`

Actions performed:
1. Seeded a disposable feed post owned by the logged-in test user.
2. Opened `/community?tab=feed`.
3. Opened the post owner action menu.
4. Opened the Edit modal.
5. Submitted whitespace-only content.
6. Verified the UI blocked the edit with `Post content cannot be empty`.
7. Submitted valid replacement content.
8. Verified the `PUT /api/community/posts/:id` response returned `200`.
9. Verified the updated content rendered in the feed.
10. Verified the `posts.content` database value changed to the edited content.
11. Opened the owner action menu again.
12. Opened the Delete modal.
13. Verified the hidden delete post id matched the target post.
14. Confirmed deletion from the modal.
15. Verified the `DELETE /api/community/posts/:id` response returned `200`.
16. Verified the post card was removed from the feed.
17. Verified the `posts` database row was deleted.

Result:
- Initial review found own-post delete still used native `confirm()`, which is not consistent with the Circle delete modal and is weaker for browser automation.
- Fixed by adding `delete-post-modal` and replacing `window.deleteOwnPost` with a modal opener plus `window.submitDeletePost`.
- Retest passed.

## Poll Workflow and Edge Cases

Page/API:
- `/community?tab=feed`
- `/api/community/posts/:id/poll`
- `/api/community/posts/:id/poll/vote`
- `polls`, `poll_options`, `poll_votes`

Actions performed:
1. Seeded a disposable post with a poll containing `Alpha`, `Bravo`, and `Charlie`.
2. Opened `/community?tab=feed`.
3. Verified all seeded poll labels rendered in the poll card.
4. Clicked `Bravo`.
5. Verified the vote request returned `200`.
6. Verified `Bravo` re-rendered with voted state and `aria-pressed="true"`.
7. Clicked `Alpha`, then clicked `Charlie` in the same single-choice poll.
8. Verified the voted state moved from `Alpha` to `Charlie`.
9. Verified only one `poll_votes` row remained for that user and poll.
10. Expired a poll by setting `expires_at` in the past.
11. Verified the UI showed `Poll ended`.
12. Verified all poll option buttons were disabled.
13. Verified a forced UI click did not mark an option as voted.
14. Called the vote endpoint directly for the expired poll.
15. Verified the server rejected the vote with `400`.
16. Seeded a second poll.
17. Submitted an option id from the second poll against the first poll's vote endpoint.
18. Verified the server rejected the mismatched option with `400`.

Result:
- Passed.
- Added coverage for the previously missing single-choice replacement, expired poll, and wrong-poll option edge cases.

## Admin Community Users Workflow Stability

Page/API:
- `/admin/community/users`
- `/admin/community/user-detail.html?id=:user_id`
- `/api/admin/community/users`
- `/api/admin/community/users/:id/ban`

Actions performed:
1. Ran the full Community/Admin Community E2E group.
2. Verified user list rendering, HTML/script sanitization, refresh status, detail link URL, ban dialog focus, Escape close behavior, ban submit, persisted ban state, and audit entry.
3. Observed one broad-run failure in the `View` detail-link step: the test was still coupled to a popup/meta-click style navigation and a row handle immediately after table refresh.
4. Stabilized the workflow by re-querying the refreshed row, asserting the `View` link is visible, reading its `href`, and opening the detail URL explicitly in a new page.
5. Re-ran the Admin Community Users tests.
6. Re-ran the full Community/Admin Community E2E group.

Result:
- Passed.
- The Admin Community Users flow no longer depends on popup timing or a pre-refresh row reference.

## Final Community/Admin Community Broad Run

Page/API:
- `/community`
- `/community/circle/:slug`
- `/community/circle/:slug/settings`
- `/admin/community/*`

Actions performed:
1. Re-ran all static Community/Admin Community contracts after the final UI and test-contract updates.
2. Re-ran the complete browser E2E group for Community, Circles, and Admin Community.
3. Verified Circle Q&A now has a visible `Q&A` feed-view entrypoint on the Circle page.
4. Verified Admin Community Challenges dialog works in a mobile viewport after the sidebar fix.
5. Verified Community accessibility audits no longer depend on flaky `networkidle` readiness.
6. Verified custom Community helper browser contexts block service workers consistently.
7. Verified Admin Community Users ban/unban success remains visible after reloading the user table.

Result:
- Passed.
- Static contracts: `82 passed`.
- Browser E2E group: `105 passed, 1 skipped`.
- Rust and hygiene checks: `cargo check`, `cargo fmt --check`, and `git diff --check` passed.

## Fix Verification

Files changed:
- `frontend/platform/community-circle-settings.html`
- `frontend/platform/community.html`
- `frontend/platform/static/js/community-circle-settings.js`
- `frontend/platform/static/js/community-feed.js`
- `frontend/platform/static/js/admin-community-users.js`
- `frontend/platform/static/css/community.css`
- `frontend/platform/static/css/admin.css`
- `frontend/platform/partials/community_post_list.html`
- `backend/src/community/service.rs`
- `tests/test_community_circles_phase8_static.py`
- `tests/test_community_circles_phase1_static.py`
- `tests/test_community_circles_phase5_static.py`
- `tests/test_community_profile_static.py`
- `tests/admin/test_admin_community_posts_static.py`
- `tests/e2e/test_community_circle_settings_ui.py`
- `tests/e2e/test_community_a11y.py`
- `tests/e2e/test_community_feed_ui.py`
- `tests/e2e/test_community_polls_ui.py`
- `tests/e2e/test_admin_community_users.py`
- `tests/e2e/community_helpers.py`
- `tests/e2e/test_community.py`

Checks:
- Browser retest for Circle delete modal: passed.
- Direct deleted settings URL state: passed.
- Non-owner Danger Zone E2E: passed.
- Dependent data delete E2E: passed.
- `python3 -m pytest tests/test_community_circles_phase8_static.py tests/e2e/test_community_circle_settings_ui.py -q`: 25 passed, 1 skipped.
- Feed pagination duplicate-query E2E: passed.
- Saved bookmark E2E: passed.
- `python3 -m pytest tests/test_community_circles_phase1_static.py tests/e2e/test_community_feed_ui.py -q`: 11 passed.
- `python3 -m pytest tests/e2e/test_community_feed_ui.py -q`: 6 passed.
- Report validation/persistence E2E: passed.
- `python3 -m pytest tests/e2e/test_community.py::test_community_feed_reaction_comment_accessibility tests/e2e/test_community.py::test_community_report_post_validates_reason_and_persists_note tests/e2e/test_community.py::test_community_own_comment_edit_updates_db_and_shows_edited tests/e2e/test_community.py::test_community_comment_reaction_toggle tests/e2e/test_community_feed_ui.py -q`: 10 passed.
- Post owner edit/delete E2E: passed.
- Poll edge-case E2E: passed.
- `python3 -m pytest tests/e2e/test_community_polls_ui.py -q`: 5 passed.
- Admin Community Users E2E: passed.
- Full Community/Admin Community E2E group: `python3 -m pytest tests/e2e/test_community*.py tests/e2e/test_admin_community*.py -q`: 103 passed, 1 skipped.
- Final static Community/Admin contracts: `python3 -m pytest tests/test_community*.py tests/admin/test_admin_community*.py -q`: 82 passed.
- Final full Community/Circles/Admin Community E2E group: `python3 -m pytest tests/e2e/test_community*.py tests/e2e/test_circles.py tests/e2e/test_admin_community*.py -q`: 105 passed, 1 skipped.
- `cargo check`: passed.
- `cargo fmt --check`: passed.
- `git diff --check`: passed.
