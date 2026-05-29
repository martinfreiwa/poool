# Community Expert AMAs

Purpose: Verify the complete Expert AMA lifecycle, including admin creation, user-facing discovery, question submission inside AMAs, question upvote/like toggles, optional AMA-level likes if the UI exposes them, admin answers, featured questions, status changes, and blocked states.

Prerequisites:
- User is logged in on `http://localhost:8888`.
- Admin account with `community.manage` is available for AMA creation and answering.
- Normal user account is available for questions, likes/upvotes, and authorization checks.
- Second normal user is available to verify counts and cross-user upvote state.
- Use a disposable AMA title such as `Workflow Test AMA YYYY-MM-DD HHMM`.

Pages and endpoints covered:
- `/community?tab=ama`
- `/community/circles?tab=ama`
- `/community/partials/ama`
- `/admin/community/amas.html`
- `/api/community/amas`
- `/api/community/amas/:id`
- `/api/community/amas/:id/questions`
- `/api/community/amas/:id/questions/:qid/upvote`
- `/api/admin/community/amas`
- `/api/admin/community/amas/:id`
- `/api/admin/community/amas/:id/status`
- `/api/admin/community/amas/:id/questions/:qid/answer`
- `/api/admin/community/amas/:id/questions/:qid/feature`

## 1. AMA Tab Loading and Empty States

Steps:
1. Open `/community?tab=ama`.
2. Verify the Expert AMAs tab is active and `/community/partials/ama` loads.
3. Verify AMA list, active AMA detail, expert metadata, schedule/state pill, and question list render.
4. Open `/community/circles?tab=ama` and verify the same AMA surface loads from the Circles entry point.
5. Force or simulate no AMAs and verify the empty state remains usable.
6. Force or simulate an API failure and verify the error state is clear and recoverable.

Expected Result:
- AMA data loads without console errors.
- Empty and failed states do not break tab navigation or the Community shell.

## 2. Admin AMA Creation

Steps:
1. Log in as an admin with `community.manage`.
2. Open `/admin/community/amas.html`.
3. Create a disposable AMA with title, description, expert name, expert title, optional expert avatar URL, optional banner URL, scheduled time, and RSVP setting.
4. Create one global AMA with `circle_id` empty.
5. Create one Circle-scoped AMA when the admin UI exposes Circle selection.
6. Save and verify the created AMA appears in the admin list.
7. Open the AMA detail endpoint/page and verify all saved fields match the submitted values.
8. Open `/community?tab=ama` and verify non-draft global AMAs appear to users.
9. Open the relevant Circle AMA surface and verify Circle-scoped AMAs appear only in the intended Circle context.

Expected Result:
- Admin-created AMAs persist and become visible only in the correct user-facing scope.
- Draft AMAs stay hidden from ordinary users.

Edge Cases:
- Missing title.
- Missing expert name.
- Over-limit title, description, expert name, or expert title.
- Invalid banner/avatar URL.
- Invalid scheduled date.
- Duplicate title.
- Normal user calls admin create endpoint.
- `community.view` admin attempts to create an AMA.

## 3. AMA Status Lifecycle

Statuses to cover:
- `draft`
- `scheduled`
- `accepting_questions`
- `live`
- `closed`
- `archived`

Steps:
1. In admin, move a disposable AMA through each supported status.
2. Reload admin detail after every status change.
3. Reload `/community?tab=ama` after every user-visible status.
4. Verify `started_at` is set when status changes to `live`.
5. Verify `ended_at` is set when status changes to `closed` or `archived`.
6. Verify user-facing status pill, button state, and question form state match the current status.

Expected Result:
- Status changes persist after reload.
- Only `accepting_questions` and `live` allow new user questions.
- `draft` remains admin-only.
- `closed` and `archived` remain readable but block mutation.

Edge Cases:
- Invalid status string.
- Status update for missing AMA.
- Repeating the same status update twice.
- Two admins changing status in parallel.

## 4. Questions Inside AMAs

Steps:
1. Set the disposable AMA to `accepting_questions`.
2. Log in as a normal user and open `/community?tab=ama`.
3. Submit a valid question inside the AMA.
4. Verify the question appears in the question list after submit.
5. Reload the page and verify the question persists.
6. Submit up to the per-user limit for the same AMA.
7. Attempt to exceed the per-user question limit.
8. Verify question ordering, count, author attribution where exposed, and timestamp formatting.
9. Repeat from a second user and verify both users' questions appear.

Expected Result:
- Valid questions persist and appear in the AMA detail.
- The server enforces the per-user question limit.
- Question count and visible list stay consistent after reload.

Edge Cases:
- Empty question.
- Whitespace-only question.
- Very long question.
- HTML/script text.
- Rapid double-submit.
- Submit while logged out.
- Submit to a missing AMA.
- Submit while AMA changes from `accepting_questions` to `closed` in another tab.
- Submit when `max_questions` has been reached.

## 5. AMA Likes and Question Upvotes

Steps:
1. Check whether the AMA card/detail itself exposes a like control.
2. If an AMA-level like control exists, click it, reload, verify active state/count persists, click again, and verify it toggles off.
3. If no AMA-level like control exists, record that AMA liking is not part of the current UI/API contract and continue with question upvotes.
4. Use a question created in the disposable AMA.
5. As the question author, click the question upvote/like control if the UI allows it.
6. As a second user, upvote the same question.
7. Verify count increments and user state changes to active.
8. Click the control again and verify the upvote is removed.
9. Reload the page and verify count and current-user upvote state persist.
10. Verify featured and high-upvote questions sort above normal questions according to the UI contract.

Expected Result:
- Any visible AMA-level like control is backed by persistent state or removed from the UI.
- Upvote acts as a toggle, not an unbounded counter.
- Counts are user-specific and persist after reload.
- Rapid repeated clicks do not create duplicate upvotes.

Edge Cases:
- Upvote missing question.
- Upvote question from another AMA with stale detail open.
- Upvote while logged out.
- Two tabs toggle the same question.
- API succeeds but UI refresh fails.

## 6. AMA Comments, Answers, and Featured Questions

Current contract:
- User participation inside AMAs is represented by questions.
- Admin/expert replies are represented by answers to those questions.
- If the UI adds a separate free-form comment field, verify it has a real endpoint and persistence; otherwise treat Questions/Answers as the AMA comment flow.

Steps:
1. Create a user question in an `accepting_questions` or `live` AMA.
2. Log in as admin and open `/admin/community/amas.html`.
3. Open the disposable AMA detail.
4. Answer the user question with a unique test answer.
5. Save and verify the answer appears in admin detail.
6. Reload `/community?tab=ama` as the question author.
7. Verify the answer/comment appears under the correct question.
8. Verify the question author receives any exposed answered-question notification or XP state.
9. Feature the question from admin.
10. Verify featured state appears in admin and user-facing lists.
11. Unfeature the same question and verify state returns to normal.

Expected Result:
- Admin answers persist and are displayed to users as the expert response/comment.
- Featured questions are highlighted or sorted consistently.
- Answering and feature toggles are admin-only.

Edge Cases:
- Empty answer.
- Over-limit answer.
- Answer question from wrong AMA.
- Answer missing question.
- Answer already answered question.
- Feature/unfeature missing question.
- Normal user attempts admin answer or feature endpoint.
- Answer contains HTML/script text.

## 7. Closed, Archived, and Unauthorized States

Steps:
1. Set AMA to `closed`.
2. Verify users can read existing questions and answers.
3. Verify users cannot submit a new question.
4. Verify users cannot mutate closed-state controls if the backend disallows them.
5. Set AMA to `archived`.
6. Verify archived AMA is readable according to product rules and does not show active participation controls.
7. Log out and verify the logged-out view blocks all write actions.
8. Log in as a normal user and directly call admin endpoints; verify authorization errors.

Expected Result:
- Closed and archived AMAs are not accidentally interactive.
- Unauthorized writes are blocked server-side and not only hidden in the UI.

## 8. Cleanup

Steps:
1. Move disposable AMA to `archived` or delete it if delete is supported by the admin surface.
2. Verify it no longer appears as an active user-facing AMA.
3. Remove or clearly label any test questions and answers if deletion is supported.

Expected Result:
- Test data does not pollute active Community AMA surfaces.
