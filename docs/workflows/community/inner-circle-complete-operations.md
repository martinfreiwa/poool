# Inner Circle Complete Operations

Purpose: Verify the full owner/admin workflow for an Inner Circle/Circle, including creation, settings, content controls, moderation, publishing, member operations, resources, reports, analytics, and deletion.

Primary pages:
- `/community?tab=circle`
- `/community/circle/:slug`
- `/community/circle/:slug/settings`

Prerequisites:
- User is logged in on `http://localhost:8888`.
- Use a disposable test circle for create/edit/delete checks.
- Use a second test user for member, DM, join-request, mention, moderation, and approval checks.
- Use valid test files for image/resource uploads and invalid files for validation checks.
- Record initial values before editing a non-disposable circle and restore them after the pass.

## 1. Circle Creation, Discovery, Membership, and Deletion

Steps:
1. Open `/community?tab=circle`.
2. Verify My Circles, Discover, Featured, Trending, New, Public, Private, Asset, Holder-only, Official, and KYC-gated circle states if present.
3. Create a new test circle with unique name and description.
4. Verify the circle appears in My Circles and can be opened at `/community/circle/:slug`.
5. Verify public join works from a second account.
6. Switch the test circle to private and verify request-to-join flow.
7. Approve and decline join requests.
8. Verify invite, accept invite, decline invite, and invite existing member cases.
9. Leave the circle as a non-owner member.
10. Delete only the disposable test circle from Danger Zone.

Expected Result:
- Circle create/open/join/request/invite/leave/delete flows are stateful, role-gated, and recoverable.
- Deleted test circle no longer appears in My Circles/Discover and direct URL has a clear not-found state.

Create/Delete Verification Detail:
1. Create a disposable Inner Circle with a unique name, description, privacy setting, category, language, and location/topic.
2. Verify the generated slug opens `/community/circle/:slug` and the settings page opens `/community/circle/:slug/settings`.
3. Add at least one disposable post, one image post, one member, one pending join request, one resource, one report, and one announcement/event/challenge if supported.
4. Attempt deletion as a non-owner and verify it is blocked.
5. Attempt deletion as owner, cancel the confirmation, and verify no data changed.
6. Confirm deletion as owner.
7. Verify the user is redirected away from the deleted circle.
8. Verify the deleted circle is absent from My Circles, Discover, leaderboards, Circle search/autocomplete, member profile circle lists, and Circle-scoped feeds.
9. Open the previous direct URLs for detail, settings, resources, reports, members, and announcements/events/challenges.
10. Verify deleted-circle URLs return a clear not-found/removed state and do not expose stale private content.

Expected Result:
- Inner Circle deletion is owner-only, confirmation-gated, and removes or hides all dependent Circle surfaces consistently.
- Stale direct URLs and background refreshes do not revive deleted Circle data.

## 2. Circle Settings and Persistence

Steps:
1. Open `/community/circle/:slug/settings`.
2. Verify hero role, member count, privacy label, and `View feed` link.
3. In `Basic Info`, edit `Name`, `Description`, `Slug`, and `Banner image`.
4. In `Privacy`, toggle `Public`.
5. Save changes.
6. Reload settings and circle detail pages.
7. Verify saved values persisted and the slug redirect/URL behavior is correct.

Expected Result:
- Basic identity, banner, privacy, and slug values persist after reload.
- Save/discard controls correctly track dirty/pristine state.

## 3. Content Settings

Fields:
- `Category`
- `Language`
- `Location / Topic`
- `Required tags`
- `Media uploads`
- `Polls`
- `Link posts`
- `Anonymous posting`

Steps:
1. Set test-safe `Category`, `Language`, `Location / Topic`, and comma-separated `Required tags`.
2. Toggle Media uploads, Polls, Link posts, and Anonymous posting.
3. Save and reload.
4. Create a circle post that includes all required tags.
5. Attempt to create a circle post without required tags.
6. Attempt media upload when Media uploads are off.
7. Attempt poll post when Polls are off.
8. Attempt link post when Link posts are off.
9. Attempt anonymous post when Anonymous posting is off.

Expected Result:
- Content settings persist and are enforced server-side, not only hidden client-side.
- Missing required tags, disabled media, disabled polls, disabled links, and disabled anonymous posts are blocked with clear messages.

## 4. Moderation Settings

Fields:
- `Slow mode seconds`
- `Require approval for first post`
- `Join approval required`
- `Auto-approve verified investors`
- `Allow comments on announcements`
- `Enable new-member onboarding`
- `Blocked words`
- `Investment-risk keywords`

Steps:
1. Set `Slow mode seconds` to a small test value.
2. Post once, then immediately try to post again as the same user.
3. Enable `Require approval for first post` and post from a new test member.
4. Verify the first post goes into approval/pending state or is blocked until approval according to implementation.
5. Enable `Join approval required` and verify join requests require owner/admin approval.
6. Enable `Auto-approve verified investors` and verify a verified investor test account is auto-approved while an unverified account is not.
7. Toggle `Allow comments on announcements` and verify announcement comments follow the setting.
8. Enable `Enable new-member onboarding` and verify onboarding checklist appears for a newly joined member.
9. Add blocked words and attempt posts/comments containing them.
10. Add investment-risk keywords such as `guaranteed return` and `risk-free`; verify warnings, disclaimer, moderation flag, or rejection behavior.
11. Save, reload, and verify all moderation values persist.

Expected Result:
- Slow mode, approval, onboarding, blocked-word, and investment-risk controls are enforced consistently in feed, circle feed, comments, announcements, and direct API calls.

## 5. Circle Publishing: Posts, Images, Mentions, Comments, Likes, Saves, Reports

Steps:
1. Create a circle post with text only.
2. Create a circle post with image upload.
3. Create a circle post with a poll.
4. Create a circle post with member mention.
5. Create a circle post with hashtag and required tags.
6. Like/react to a circle post.
7. Comment on a circle post.
8. Reply to another member's comment if threaded replies are supported.
9. Bookmark/save a circle post.
10. Report a circle post and a circle comment.
11. Edit your own circle post and comment.
12. Delete your own circle post and comment.
13. Attempt to edit/delete another member's post/comment.

Expected Result:
- Circle feed actions behave like main Community feed actions while respecting Circle-specific settings and membership permissions.

## 6. Rules and Disclaimers

Steps:
1. Open `Rules & Disclaimer`.
2. Edit `Circle rules` with a unique test rule set.
3. Edit `Investment disclaimer` with a unique test disclaimer.
4. Save and reload settings.
5. Open the circle detail/feed page and verify rules/disclaimer appear wherever the product exposes them.
6. Attempt max-length and over-limit values for both fields.
7. Attempt HTML/script text and verify it renders as text or is safely sanitized.

Expected Result:
- Rules and disclaimer persist after reload.
- Over-limit and unsafe content is blocked or safely rendered.
- Disclaimer appears in relevant investment-risk or circle onboarding contexts where implemented.

## 7. Analytics and Operations

Steps:
1. Open `Analytics & Ops`.
2. Verify `Posts 7d`, `Comments 7d`, `Active members 7d`, and `Pending reports` load.
3. Create a disposable circle post and comment, then reload analytics when possible.
4. Verify counters are non-negative and bounded.
5. Verify `Ops Alerts` list renders empty, loading, populated, and failed states.
6. For a disposable alert, test `acknowledge`, `resolve`, and `set_workflow_state`.
7. For workflow state, test allowed states: `triage`, `investigating`, `waiting_on_moderator`, `waiting_on_policy`, `mitigated`, and `monitoring`.
8. Verify invalid workflow state is rejected.
9. Verify audit log entries appear after settings save, report action, member action, resource action, and ops alert action where exposed.

Expected Result:
- Analytics values load without exposing hidden/private data to unauthorized roles.
- Ops alert actions require owner/admin role, optional/required notes behave correctly, and state changes persist.
- Audit log reflects operational actions where the UI exposes it.

## 8. Report Queue

Steps:
1. Create a test report by reporting a disposable circle post/comment from a second account.
2. Open `Report Queue` as circle owner/admin.
3. Verify the report appears with reporter/target/reason/status metadata.
4. Take an individual report action such as `hide_post` or `dismiss`.
5. Verify moderation notes are required.
6. Verify the report list refreshes and analytics pending-report count updates.
7. Create multiple disposable reports.
8. Select multiple reports and run a bulk action.
9. Verify bulk action with no selected reports is blocked.
10. Verify a non-admin/non-owner cannot view or act on the queue.

Expected Result:
- Report queue accurately reflects circle-scoped reports.
- Individual and bulk actions require notes, update state, refresh counters, and are permission-gated.

## 9. Resource Library

Fields:
- `Title`
- `External URL`
- `Upload file`
- `Private object path`
- `Type`
- `Access`
- `Version`
- `Upload status`
- `Retention policy`
- `Retention until`
- `Review required`
- `Lifecycle notes`
- `Official`

Steps:
1. Open `Resource Library`.
2. Create an external URL resource with title, type, access, version, retention policy, lifecycle notes, and official flag.
3. Create a file-backed resource using a valid PDF/image/doc/zip test file.
4. Verify resource appears in the management list.
5. Open the resource link or delivery URL.
6. Toggle `Archive` and `Restore`.
7. Run lifecycle actions: `Reviewed`, `Legal hold`, `Expire`, and `Soft delete`.
8. Add an external URL version.
9. Upload a replacement file version.
10. Open `Versions`.
11. Approve a pending version.
12. Reject a version with a required rejection note.
13. Attempt reject without note and verify it is blocked.
14. Restore an older version as current.
15. Verify access scopes: `Members`, `Holders only`, `Admins only`, and `Public`.

Expected Result:
- Resources can be created, versioned, reviewed, archived/restored, and soft-deleted.
- Access scope and lifecycle metadata persist and are enforced.
- Invalid resource inputs fail clearly.

Edge Cases:
- Empty title.
- Resource with neither URL nor file.
- Invalid URL.
- Unsupported file type, empty file, oversized file, and corrupted file.
- Duplicate version label.
- Retention date in the past.
- Review-required date in the past.
- Restore stale version after newer approved version exists.
- Public resource opened by logged-out user if intended.

## 10. Members

Steps:
1. Open `Members`.
2. Verify member rows include display name/user identity and role badge.
3. Promote a disposable member to moderator.
4. Demote the same member.
5. Kick a disposable member.
6. Ban a disposable member with and without optional reason.
7. Verify member list refreshes after each action.
8. Verify role/member-count changes propagate to circle detail and My Circles where applicable.
9. Verify non-owner/non-admin users see read-only member state.

Expected Result:
- Promote, demote, kick, and ban actions are role-gated, persisted, and reflected in list state.

Edge Cases:
- Promote owner.
- Demote owner.
- Kick owner.
- Ban owner.
- Kick/ban already removed user.
- Ban already banned user.
- User opens settings in another tab during role change.

## 11. Join Requests

Steps:
1. Enable private circle or `Join approval required`.
2. From a second account, request to join.
3. Verify request appears in `Join Requests`.
4. Approve the request and verify the user becomes a member.
5. Create another request and decline it.
6. Verify declined user does not become a member.
7. Verify the requests list empty state.
8. Verify request count/badges update where exposed.

Expected Result:
- Join requests appear only where appropriate.
- Approve/decline actions persist, update membership, and refresh list state.

Edge Cases:
- Approve request already cancelled by requester.
- Decline request already approved.
- Duplicate join request.
- Banned user requests to join.
- Verified investor auto-approval enabled versus disabled.

## 12. Banned Users

Steps:
1. Ban a disposable test member from `Members`.
2. Open `Banned Users`.
3. Verify banned user appears with reason and metadata where exposed.
4. Attempt to rejoin/request/invite/post/comment as the banned user.
5. Unban the user.
6. Verify the user can request/join again according to circle privacy settings.

Expected Result:
- Banned users cannot participate in the circle until unbanned.
- Unban removes the restriction without granting unintended membership.

Edge Cases:
- Ban non-member.
- Ban owner.
- Ban moderator/admin.
- Unban user who is not banned.
- Banned user has pending request or invite.

## 13. Danger Zone

Steps:
1. Verify `Danger Zone` is hidden or disabled for non-owner roles.
2. As owner, verify `Transfer ownership...` is visible.
3. Transfer ownership only on a disposable test circle.
4. Verify old owner loses owner-only controls and new owner gains them.
5. Verify `Delete circle` requires confirmation.
6. Delete only a disposable test circle.
7. Verify redirect to `/community/circles`.
8. Verify deleted circle no longer appears in My Circles/Discover and direct URL is not available.

Expected Result:
- Transfer and delete are owner-only and destructive actions are confirmation-gated.
- Deleted circle data is removed or hidden consistently across UI surfaces.

Edge Cases:
- Transfer to self.
- Transfer to non-member.
- Transfer to banned member.
- Delete circle with members, resources, reports, join requests, bans, and posts.
- Delete cancelled at confirmation.
- Delete request fails after optimistic UI change.

## 14. Announcements, Events, and Challenges

Steps:
1. Verify circle announcements list renders on the circle page.
2. Create or manage announcements from the available admin/owner surface if exposed.
3. Verify `Allow comments on announcements` changes comment behavior.
4. Verify circle events list renders.
5. Create or manage events from the available admin/owner surface if exposed.
6. Verify circle challenges list renders.
7. Complete or submit a circle challenge if exposed.

Expected Result:
- Announcements, events, and challenges render and respect role/access settings.

## 15. Member DMs and Cross-Member Replies

Steps:
1. Send a DM to another circle member.
2. Reply to another member's DM.
3. Verify DMs respect recipient privacy settings.
4. Verify blocked/muted/banned users cannot start or continue disallowed conversations.

Expected Result:
- Member messaging works for allowed users and is blocked clearly for disallowed states.

Edge Cases:
- Duplicate circle name/slug, invalid slug, empty values, and over-limit values.
- Slow mode `0`, `1`, maximum `86400`, negative, decimal, and non-number.
- Required tags with duplicates, uppercase, invalid taxonomy, emoji, HTML/script, and empty entries.
- Blocked/risk keyword in post, edited post, comment, poll option, image caption, and announcement.
- Mention nonexistent, blocked, banned, or non-member users.
- Report already reported content.
- Save/unsave rapidly.
- Announcement comments disabled after comments already exist.
- Resource with neither URL nor file, invalid URL, unsupported file type, empty file, oversized file, reject without note, stale restore.
- Promote/kick/ban owner, already removed member, and non-member.
- Non-admin opens reports/resources/analytics/ops endpoints.
- Bulk report action with no selected reports.
- Ops alert invalid state.
- Danger Zone visible to non-owner.
