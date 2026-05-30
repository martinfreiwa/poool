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
- Inner Circle deletion cleanup.

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
- Delete action reached the native browser confirmation step.
- The in-app browser automation stalled on the native `confirm()` dialog before it could reliably send the confirmation keypress.
- The disposable Circle was nevertheless deleted; database verification returned no remaining `Workflow Test Circle %` rows.

Follow-up:
- Replace native `window.confirm()` for Circle deletion with an in-app confirmation modal if this workflow should be fully automatable through browser tests.
- Add an E2E assertion for deleted Circle direct URLs once the confirmation step is test-stable.

## Cleanup

Cleanup performed:
- Removed the disposable AMA question and related upvotes.
- Removed the disposable AMA.
- Verified no remaining `Workflow Test Circle %` rows.

Cleanup result:
- Passed.
