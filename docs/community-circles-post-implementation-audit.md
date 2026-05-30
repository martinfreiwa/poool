# POOOL Community Circles Post-Implementation Audit

Date: 2026-05-21  
Scope: Phase 0, Phase 1, Phase 2, Phase 3, Phase 4, Phase 5, Phase 6, Phase 7, Phase 8 vertical-slice implementation, the Phase 8 UI, permissioned-delivery, holder-only feed access revalidation, resource-version-management, resource-lifecycle, resource-retention-worker, binary-resource-upload, physical-object-cleanup, resource-replacement-upload, resource-version-delivery/restore, resource-version-review/comparison, browser-backed resource upload/replacement/delivery/restore/review follow-ups, private-GCS stream verification, a Phase 9 manage/ops vertical slice, the Phase 9 report-queue, bulk-report-triage, scheduled-ops, auto-critical ops fan-out, failed-worker, platform-admin ops-alert, ops-alert escalation, ops-alert human-SLA workflow-state, browser-backed manager workflow-state, browser-backed platform-admin ops-alert actions, ops-alert email/Slack/PagerDuty fan-out, webhook URL hardening, delivery-monitoring follow-ups, local-live provider preflight hardening, multi-user negative-path E2E, full Circle journey E2E, mobile/A11y verification, and the latest schema-drift fixes from the optimized POOOL Circles plan.

## Executive Summary

Phase 1 and Phase 2 remain implemented as the content-first Circles foundation: `/community/circle/:slug` is the Circle Feed, `/community/circle/:slug/settings` is the secondary Manage route, `/community/circles` is the My-Circles/Discover surface, and Circle posts are modeled through `posts.circle_id`.

Phase 3 added the first production-relevant access and trust vocabulary: first-class Circle type/visibility/join-policy fields, hidden-Circle non-disclosure rules, token/KYC gate enforcement at join/request/invite-acceptance entry points, schema-backed Official/KYC Discover buckets, a `verified_expert` Circle role, and admin/system-granted reputation flairs.

Phase 4 added privacy-safe Circle Mentions. Posts can mention Circles through the canonical `@circle/:slug` form, and rendering is viewer-aware: visible Circles become links, private Circles render as `Private Circle` for non-members, and hidden Circles render as neutral unavailable text without leaking name, slug, or ID.

Phase 5 now adds the first structured content/compliance layer. Post creation is server-validated against allowlisted post types and tags, Circle policies can require tags and allowed post types, Announcement/Official Update and privileged tags are role-gated, Circle/global feeds can filter by type/tag, composer payloads carry normalized tags, and investment-relevant content persists a mandatory compliance disclaimer.

Phase 6 now turns Questions and Due-Diligence posts into a durable knowledge layer. Posts have Q&A lifecycle status, official-answer comment links, FAQ/featured flags, related-resource metadata, and answer audit logs. Official/verified answer markings are server-gated to Circle moderators, verified experts, or platform admins. The Circle Q&A tab filters into Question posts, Post Cards display answer status, and comments can render Official/Verified Answer badges.

Phase 7 now adds Circle-specific engagement primitives. Circle announcements are read from Circle-scoped posts only, global AMAs remain global while Circle AMAs/events are tied to `amas.circle_id`, Circle challenges use a separate per-Circle progress table, and new members receive a stored onboarding checklist with steps for rules, introduction, interests, AMA following, and first question.

Phase 8 now adds the Asset Circle foundation. Circles can be linked to a core asset, one primary Asset Circle can be marked per asset, Private Investor Clubs get safer defaults, and Circle Resources/Documents are returned only through authenticated, Circle-aware, holder-aware APIs. Generic Circle feed/post/comment/AMA guards now also receive core AppState and revalidate current token/asset ownership for holder-only and token-gated Circles, so asset access is not limited to the Resource APIs. The Circle page now has a permissioned Resources widget backed by `/api/community/circles/:id/resources`; the list returns metadata and an authenticated delivery endpoint, not raw storage paths. The authenticated Property page and Portfolio asset rows now surface Asset Circle entry points through the same `/api/community/assets/:id/circle` access-state contract. The latest follow-ups add an explicit `circle_resource_versions` history table and manager-gated Resource Library APIs/UI for creating resources, uploading binary resource files, archiving/restoring them, adding current versions, replacing the current resource file through a binary version upload, opening non-current versions through authenticated version delivery, restoring a historical version as current, approving or rejecting concrete versions with audit logs, comparing current versus previous version metadata in the Manage UI, controlling upload/review/retention/legal-hold/soft-delete lifecycle metadata, automatically soft-deleting due non-legal-hold resources through a scheduled retention worker, and physically cleaning up private GCS objects after a grace period without exposing storage paths to member-facing APIs.

Phase 9 now adds the first Manage/Ops vertical slice. Circle settings are split into Basic, Privacy, Content Settings, Moderation, Rules/Disclaimer, Analytics/Ops, Report Queue, Members, Requests, Bans, and Danger Zone sections. A role-gated `/api/community/circles/:id/manage` contract returns settings, bounded analytics, and recent Circle audit entries; updates require CSRF and write `circle.manage.update` audit events. Moderators can update moderation controls but cannot change owner/admin settings such as slug, privacy, content policy, or rules. The follow-ups add manager-only Circle report review endpoints, UI actions for hiding reported Circle posts or dismissing scoped reports, manager-gated bulk report triage for up to 50 selected scoped reports with required notes and `circle.report.bulk_*` audit logs, a scheduled ops worker that materializes daily analytics snapshots plus report-backlog/moderation-SLA alerts, automatic `auto_critical` notification intents for unsnoozed critical report-backlog and moderation-SLA alerts, manager actions to acknowledge, resolve, or set a human workflow state on open Circle ops alerts, and a platform-admin overview for Circle/global ops alerts including failed-worker rows. Platform admins can now also assign, escalate, snooze/unsnooze, mark on-call notification state, and set workflow states such as investigating, waiting on moderator, waiting on policy, mitigated, or monitoring for active ops alerts with durable metadata and audit logging. Escalation/on-call actions additionally enqueue a durable community-side notification row which a background worker bridges into the core `transactional_email_outbox`; the same outbox now supports env-gated Slack and PagerDuty webhook channels for auto-critical alerts with no-redirect HTTP delivery, retry/skipped state, and provider response status tracking. A second delivery monitor observes the core email outbox state and raises/resolves `notification_delivery` ops alerts for stale, exhausted, or missing provider-level email delivery.

This is still not a full production-ready investment-community product, but the previously open verification gaps have narrowed materially. Phase 8 now has browser-backed manager upload, replacement-upload, authenticated delivery, version-restore, review coverage for local/external resource paths, and deterministic private-GCS stream coverage through a development-only fake GCS root. Phase 9 now has browser-backed manager workflow-state coverage, platform-admin action coverage for workflow/assignment/escalation/snooze/on-call/resolution, webhook URL validation tests, a local-live provider preflight script, and a full Circle journey/mobile/A11y regression pass. Live Slack/PagerDuty credential validation remains operational rollout work because this local environment does not have real provider credentials.

## Implemented Scope

### Phase 0: Vorher-Audit

Status: Complete.

- `docs/community-circles-pre-implementation-audit.md`
- `docs/community-circles-provider-runbook.md`
- `docs/community-circles-compliance-checklist.md`
- Baseline findings for routing, feed/composer, data model, APIs, authorization, tests, accessibility, and production readiness.
- Baseline verification results documented, including the existing `cargo check` SQLx/database blockers.

### Phase 1: Circle Feed as Default Destination

Status: Complete for the vertical slice.

- `/community/circle/:slug` renders the Circle Feed page.
- `/community/circle/:slug/settings` remains the Manage/Settings page.
- `/community/circles` exists as the canonical My Circles/Discover route.
- `posts.circle_id` migration added in `database/community/049_circle_posts.sql`.
- Global feed and global announcements exclude Circle posts with `p.circle_id IS NULL`.
- Circle feed queries enforce `p.circle_id = :id`.
- `GET /api/community/circles/:id/posts` and `POST /api/community/circles/:id/posts` exist.
- Post detail, comments, reactions, and bookmarks perform Circle read/write checks when a post belongs to a Circle.
- Circle detail page includes header, tabs, composer, feed, and sidebar; composer shows `Post to: <Circle Name>`.

### Phase 2: `/community/circles` My-Circles and Discover Page

Status: Complete for the current schema-backed slice.

- `/api/community/circles/discover` exposes `featured`, `trending`, `new`, `public`, `private`, `asset`, `holder_only`, `official`, and `kyc_gated`.
- `CircleCardRow` includes token gate, Circle type, visibility, join policy, official, KYC, private-investor-club, and cross-post metadata.
- Hidden circles are excluded from public Discover lists.
- Circle search remains public-only and filters `visibility = 'public'`.
- Discover cards merge category tags and use state-aware primary actions: `Open`, `Join`, `Request Access`, or `Locked`.

### Phase 3: Circle Types, Roles, Reputation, and User Flairs

Status: Implemented as a schema/API/UI contract slice.

- Added `database/community/050_circle_types_reputation.sql`.
- `circles.circle_type` supports `social`, `asset`, `topic`, `expert`, `private_investor`, and `official`.
- `circles.visibility` supports `public`, `private`, and `hidden`.
- `circles.join_policy` supports `open`, `request`, `invite_only`, `holder_only`, and `kyc_required`.
- Added `circles.is_official`, `circles.kyc_required`, `circles.private_investor_club`, and `circles.allow_cross_post`.
- `circle_members.role` now allows `verified_expert`.
- Added `community_reputation_flair_grants` for admin/system-granted reputation signals.
- Join/request/invite-acceptance flows enforce token and KYC gates.
- Post cards render Official/reputation badges from server-granted flairs, not from author-name string matching.
- Admin Circle detail/list surfaces expose Phase-3 metadata.

### Phase 4: Circle Mentions and Privacy Protection

Status: Implemented as a renderer/autocomplete/security contract slice.

Backend/API:

- Added structured inline rendering tokens for text, hashtags, asset tags, user mentions, and Circle Mentions.
- Canonical Circle Mention syntax: `@circle/<slug>`.
- Natural name syntax such as `@Founder Circle` is resolved when the name/slug matches a known Circle candidate.
- Circle Mention rendering is viewer-aware:
  - Public visible Circle: clickable link to `/community/circle/:slug`.
  - Private Circle for non-member: renders `Private Circle`, no name/slug/ID.
  - Hidden Circle for non-member: renders `Circle mention unavailable`, no name/slug/ID.
  - Member of private/hidden Circle: clickable link with Circle name.
- Mention rendering escapes generated labels, slugs, handles, tags, and asset slugs before producing HTML.
- `parse_and_notify_mentions` explicitly skips `@circle/...`, so Circle Mentions do not fan out user notifications or notify Circle members.
- `/api/community/mentions/suggest` now requires authentication and returns both users and visible Circles.
- Circle autocomplete query only returns public Circles or Circles where `circle_members` contains the viewer.

Frontend:

- `community-autocomplete.js` consumes `data.circles`.
- Circle suggestions insert `@circle/<slug>` to avoid ambiguous multi-word mentions.
- Typing `@circle/` filters to Circle suggestions only.
- Added CSS for `.circle-mention-tag`, `.circle-mention-tag--private`, and `.circle-mention-tag--redacted`.

### Phase 5: Content Types, Tags, Flairs, and Compliance Protection

Status: Implemented as a structured content/compliance contract slice.

Backend/API:

- Added `database/community/051_post_types_tags_compliance.sql`.
- `posts.post_type` now accepts the Phase-5 taxonomy: Discussion, Question, Market Insight, Property Update, Due Diligence, Poll, Announcement, AMA Question, Resource, Risk Discussion, and Official Update, while preserving legacy values for compatibility.
- `posts.content_tags` is normalized to a non-null array with a GIN index.
- `circles.required_post_tags` and `circles.allowed_post_types` allow Circle-level content policies.
- `CreatePostRequest` accepts `content_tags`; `PostDisplay` returns normalized tags and optional `circle_name`.
- Server-side validation rejects invalid post types/tags before persistence.
- Announcement and Official Update are restricted to Circle managers or platform admins.
- Privileged tags such as `official`, `featured`, and `answered` are restricted to Circle managers or platform admins.
- Investment-relevant post types/tags persist `disclaimer_shown = true`.
- Global and Circle feed APIs accept `post_type` and `tag` filters.

Frontend:

- Global and Circle composers expose a post-type selector and normalized comma-separated tag input.
- Composer disclaimer preview reacts to investment keywords, investment-relevant post types, and compliance-sensitive tags.
- Global and Circle feed controls include type and tag filters.
- Post cards render post-type badges, content tags, optional Circle name, Official/reputation badges, and the required `User opinion, not financial advice` disclaimer text.

Tests:

- Added `tests/test_community_circles_phase5_static.py` covering schema, backend allowlists, permission gates, persistence/filtering, UI payloads, and post-card rendering.

### Phase 6: Q&A, Due Diligence, and Knowledge Layer

Status: Implemented as a Q&A lifecycle and official-answer contract slice.

Backend/API:

- Added `database/community/052_qa_knowledge_layer.sql`.
- `posts.qa_status` supports `open`, `answered`, `official_answer`, `needs_clarification`, and `archived`.
- `posts.official_answer_comment_id` links a Question/Due-Diligence post to the selected answer comment.
- `posts.faq_candidate`, `posts.featured_question`, `posts.related_resource_url`, and `posts.related_asset_id` provide first knowledge-layer metadata.
- `comments.is_official_answer`, `comments.is_verified_answer`, `comments.answer_marked_by`, and `comments.answer_marked_at` identify trusted answers.
- `community_answer_audit_log` records Q&A status changes and official/verified answer markings.
- `PUT /api/community/posts/:id/qa-status` updates Q&A lifecycle metadata.
- `POST /api/community/comments/:id/official-answer` marks or removes an official/verified answer.
- Official-answer and Q&A status mutations require platform admin, Circle owner/admin/moderator, or `verified_expert` role.
- Archived questions remain readable but are locked from further comments through `posts.is_locked`.
- Search prioritizes official-answer Q&A, answered Q&A, and knowledge-like posts above generic posts.

Frontend:

- Circle Q&A tab now activates a structured Question filter instead of being only an inert anchor.
- Post Cards render Q&A status, Official Answer link, Featured Question, FAQ Candidate, and Related Resource affordances.
- Comment rows render Official Answer and Verified Answer badges.
- Authorized viewers receive a `Mark official answer` / `Remove official answer` control in the comment UI.

Tests:

- Added `tests/test_community_circles_phase6_static.py` covering schema, models, server authorization, search prioritization, comment payloads, Q&A tab behavior, post-card status rendering, and official-answer controls.

### Phase 7: Announcements, AMAs/Events, Challenges, and Onboarding

Status: Implemented as a Circle-scoped engagement contract slice.

Backend/API:

- Added `database/community/053_circle_engagement_onboarding.sql`.
- `circles.announcement_comments_enabled` and `circles.onboarding_enabled` provide Circle-level engagement toggles.
- `amas.circle_id`, `amas.asset_id`, and `amas.rsvp_enabled` distinguish global AMAs from Circle AMAs/events.
- Global AMA listing now excludes Circle AMAs through `circle_id IS NULL`.
- `GET /api/community/circles/:id/announcements` returns only Circle-scoped Announcement/Official Update posts.
- `GET /api/community/circles/:id/events` returns only AMAs/events attached to that Circle.
- `GET /api/community/circles/:id/challenges` returns Circle challenge templates and Circle-specific challenge progress.
- `GET /api/community/circles/:id/onboarding` returns a member's stored Circle onboarding checklist.
- `POST /api/community/circles/:id/onboarding/:step` updates allowlisted onboarding steps and Circle challenge progress.
- `circle_challenge_progress` keys progress by `circle_id`, `user_id`, and `challenge_id`, preventing activity in one Circle from completing another Circle's challenge.
- Circle post creation updates Q&A/market-insight challenge progress; Circle comments update comment challenge progress.
- Circle AMA detail/question/upvote routes enforce Circle read access when `amas.circle_id` is set.

Frontend:

- Circle sidebar now includes member onboarding, Circle Announcements, Circle Events/AMAs, and Circle Challenges sections.
- `community-feed.js` loads Circle engagement widgets from the new Circle APIs.
- Onboarding steps update through the Circle onboarding endpoint and re-render the sidebar state.
- Challenge progress bars render from Circle-scoped progress, not global challenge state.

Tests:

- Added `tests/test_community_circles_phase7_static.py` covering schema, global-vs-Circle AMA separation, Circle announcement scope, Circle challenge progress isolation, onboarding contracts, and sidebar/JS rendering.

### Phase 8: Asset Circles, Holder-only Resources, and Private Investor Club Defaults

Status: Implemented as a foundation/vertical slice.

Backend/API:

- Added `database/community/054_asset_circle_resources.sql`.
- Added `database/community/056_circle_resource_delivery.sql`.
- Added `database/community/059_circle_resource_versions.sql`.
- Added `database/community/062_circle_resource_lifecycle.sql`.
- Added `database/community/063_circle_resource_object_cleanup.sql`.
- `circles.related_asset_id` stores the logical core `assets.id` link for Asset Circles.
- `circles.is_primary_asset_circle` plus a partial unique index enforces at most one primary Asset Circle per linked asset.
- `circles.holder_only_documents` and `circles.asset_circle_tabs` prepare Asset Circle document/report tab behavior.
- Private Investor Clubs default to private/request access and `allow_cross_post = FALSE`.
- `circle_resources` stores official documents, reports, yield reports, guides, links, photo updates, and community resources.
- Resource rows have `access_scope` values `public`, `member`, `holder_only`, and `admin_only`.
- Resource rows now have delivery metadata for file name, MIME type, byte size, SHA-256, version label, publication/expiry timestamps, and forced-download behavior.
- `circle_resource_versions` stores explicit version history with current-version uniqueness, change notes, publication/expiry metadata, file metadata, and backfilled current versions from existing resources.
- Resource rows and versions now carry upload status, retention policy, retention-until, review-required, reviewed-by, legal-hold, soft-delete, deletion-reason, and lifecycle-note metadata.
- Resource rows and versions now also carry physical storage cleanup metadata: storage-deleted timestamp, attempts, last error, and next retry time.
- Resource versions now carry auditable review state: `pending`, `approved`, `rejected`, or `superseded`, plus reviewed timestamp, reviewer, and review note metadata.
- `GET /api/community/assets/:id/circle` returns the primary/official Asset Circle candidate, viewer holding state, membership state, and content-first URL.
- `GET /api/community/circles/:id/resources` returns only resources the viewer may access and emits `delivery_url`, `delivery_mode`, and version metadata rather than raw `url` or `storage_object_path` values.
- Generic `ensure_circle_read_access`, `ensure_post_read_access`, `ensure_post_write_access`, and `ensure_ama_read_access` now receive `AppState`, so holder-only and token-gated Circle content paths can query the core `investments` table before showing feed, post detail, comments, reactions, announcements, events, challenges, onboarding, or AMA content.
- Token-gated read paths reuse `check_token_gate`, including minimum holding value checks; holder-only related-asset paths require `tokens_owned > 0` before read access is granted.
- `GET /api/community/circles/:id/resources/:resource_id/access` repeats Circle/holder/admin access checks, verifies resource ownership by Circle, validates storage paths, redirects safe link resources, and streams private GCS objects through the backend with `no-store`, `nosniff`, and attachment headers.
- `GET /api/community/circles/:id/resources/manage` returns the manager Resource Library without storage paths; private files render only as `has_private_file`.
- `POST /api/community/circles/:id/resources/manage` creates a resource plus its first current version inside one transaction and writes `circle.resource.create`.
- `POST /api/community/circles/:id/resources/upload` accepts one manager-gated multipart file, reads it with capped chunked upload helpers, validates magic-byte MIME, stores PII-class B private objects when GCS is configured, records SHA-256/size/MIME metadata, creates the first resource version, and writes `circle.resource.upload`.
- `PUT /api/community/circles/:id/resources/:resource_id/manage` updates resource metadata/status and writes `circle.resource.update`.
- `GET /api/community/circles/:id/resources/:resource_id/versions` returns version history without raw storage paths.
- `POST /api/community/circles/:id/resources/:resource_id/versions` flips previous versions to non-current, inserts the new current version, updates the delivery row, and writes `circle.resource.version.create`.
- `POST /api/community/circles/:id/resources/:resource_id/versions/upload` accepts a manager-gated multipart replacement file, validates MIME and size, stores the private/local object, inserts a new current version, updates the resource delivery row, clears stale storage-cleanup state, and writes `circle.resource.version.upload`.
- `GET /api/community/circles/:id/resources/:resource_id/versions/:version_id/access` lets managers open a historical version through the same safe redirect/private-stream delivery rules.
- `POST /api/community/circles/:id/resources/:resource_id/versions/:version_id/restore` makes a historical version current again, updates the resource delivery row transactionally, resets stale cleanup state, and writes `circle.resource.version.restore`.
- `POST /api/community/circles/:id/resources/:resource_id/versions/:version_id/review` lets Circle resource managers approve, reject, or return a version to pending state; rejection requires a note, deleted private files cannot be approved, current-version review updates the resource-level reviewed state, and every action writes `circle.resource.version.review`.
- `POST /api/community/circles/:id/resources/:resource_id/lifecycle` performs manager-gated lifecycle actions such as mark reviewed, pending/uploaded/rejected, expire, soft delete, restore, legal hold, standard retention, and schedule review, then writes `circle.resource.lifecycle`.
- `circle_resource_retention_worker` runs on `POOOL_CIRCLE_RESOURCE_RETENTION_SECS` cadence and soft-deletes due `delete_after_expiry` resources only when `legal_hold = FALSE`, writing `circle.resource.retention_soft_delete` audit rows.
- `circle_resource_object_cleanup_worker` runs only when `GCS_BUCKET_NAME` is configured, uses `POOOL_CIRCLE_RESOURCE_OBJECT_CLEANUP_*` controls, deletes distinct private GCS objects for already soft-deleted non-legal-hold resources after a grace period, marks resource/version storage cleanup state, backs off failures, records storage GCS error metrics, and writes `circle.resource.object_delete` / `circle.resource.version.object_delete` audit rows.
- Holder-only resource access checks current core `investments.tokens_owned > 0` before returning URLs.

Frontend:

- Circle page now includes a `Resources` sidebar card.
- `community-feed.js` loads `/api/community/circles/:id/resources` and renders permissioned resources with type/scope/version metadata.
- Resource links point to the authenticated delivery endpoint only and use `rel="noopener noreferrer"`.
- Circle Settings now includes a `Resource Library` card for owner/admin/platform-admin users with compact link/private-object/binary-file creation, upload-status and retention inputs, archive/restore, lifecycle actions, add-version, binary replacement upload, inline version-history open/restore/review controls, and a current-versus-candidate metadata comparison summary.
- `property.html` loads the shared Asset Circle CTA helper and renders an `Investor discussion` section from the authenticated asset-circle lookup.
- `portfolio.html` loads the shared helper; `portfolio-service.js` preserves `asset_id` as `assetId`; `portfolio-data.js` hydrates desktop and mobile asset rows with Circle entry buttons only when the API returns a visible Circle.

Tests:

- Added `tests/test_community_circles_phase8_static.py` covering schema, model fields, asset-circle APIs, holder-only access contracts, permissioned delivery, resource UI rendering, resource-version management/review/comparison, and private investor defaults.
- Added `tests/test_community_circles_phase8_ui_static.py` covering the shared CTA helper, Property CTA integration, Portfolio row integration, API credential use, and safe Circle URL constraints.
- Added browser-backed coverage in `tests/e2e/test_community_circle_settings_ui.py` for uploading a PDF resource through the Resource Library form, replacing an existing resource file through the file chooser and prompt sequence, verifying uploaded metadata and SHA-256 persistence, checking the authenticated delivery endpoint, opening Resource Library version history, rendering current-vs-candidate comparison metadata, rejecting a pending current version through the UI, requiring the rejection note prompt, restoring a historical version as current, and verifying the persisted review/current-version state in the community database.

### Phase 9: Manage, Moderation, Analytics, and Ops Vertical Slice

Status: Implemented as a manage/ops foundation.

Backend/API:

- Added `database/community/055_circle_manage_ops.sql`.
- Added `database/community/057_circle_ops_alert_dedupe.sql`.
- Added `database/community/058_circle_global_failed_worker_alert.sql`.
- Added `database/community/060_circle_ops_alert_escalation.sql`.
- Added `database/community/061_circle_ops_alert_fanout.sql`.
- Added `database/community/064_circle_ops_alert_delivery_monitoring.sql`.
- Added `database/community/065_circle_ops_alert_external_fanout.sql`.
- Added `database/community/067_circle_ops_alert_workflow_states.sql`.
- `circles` now has modular manage fields for category, language, location/topic, rules, investment disclaimer, membership approval, content capabilities, first-post approval, slow mode, blocked words, investment-risk keywords, and analytics enablement.
- `circle_daily_analytics` provides a bounded future snapshot table for per-Circle activity, Q&A answer rate, top tags, and reported content.
- `circle_ops_alerts` provides an operational alert primitive for report backlog, spam spikes, failed workers, posting spikes, and moderation SLA breaches.
- `idx_circle_ops_alerts_open_unique` prevents duplicate open per-Circle ops alerts for the same alert type.
- `idx_circle_ops_alerts_global_open_unique` prevents duplicate open global `failed_worker` alerts.
- `circle_ops_alerts` now stores `assigned_to_user_id`, bounded `escalation_level`, `escalated_at`, `snoozed_until`, `escalation_note`, and `on_call_notified_at` for durable operational ownership.
- `circle_ops_alerts` now also stores `workflow_state`, `workflow_note`, `workflow_updated_at`, and `workflow_updated_by`, separating alert lifecycle from human SLA triage state.
- Escalation, assignment, and snooze indexes support active alert triage queues without scanning all resolved history.
- `circle_ops_alert_notifications` is a community-side outbox for alert fan-out intents, with retry state, target user, trigger action, payload, and the linked core `transactional_email_outbox.id` after bridging.
- `circle_ops_alert_notifications` now also stores observed core outbox delivery state: status, attempts, last error, sent timestamp, delivery check timestamp, and delivery-alert timestamp.
- `circle_ops_alert_notifications.channel` now supports `email`, `slack`, and `pagerduty`; external webhook rows record provider HTTP response status and response timestamp.
- `GET /api/community/circles/:id/manage` returns Circle manage settings, bounded live analytics, and recent Circle audit entries.
- `PUT /api/community/circles/:id/manage` validates CSRF, role, slug, allowlisted types, allowlisted tags, keyword bounds, slow-mode bounds, and writes a `circle.manage.update` audit entry.
- `GET /api/community/circles/:id/analytics` returns bounded Circle analytics through the same manager access gate.
- Moderators are server-limited to moderation controls; owner/admin/platform-admin rights are required for owner/admin settings, and slug changes are owner/platform-admin only.
- `GET /api/community/circles/:id/reports` returns pending reports only for posts where `posts.circle_id` equals the managed Circle, capped at 50 oldest reports.
- `POST /api/community/circles/:id/reports/:report_id/action` requires CSRF, verifies the report belongs to the Circle, requires moderation notes, and only allows `hide_post` or `dismiss_report`.
- `POST /api/community/circles/:id/reports/bulk-action` requires CSRF, verifies all selected reports belong to the Circle, locks reports/posts transactionally, limits the batch to 50 reports, requires moderation notes, and only allows `hide_posts` or `dismiss_reports`.
- Bulk hide updates all affected posts and resolves selected reports; bulk dismiss leaves posts unchanged and marks selected reports dismissed. Both paths write one `circle_report_bulk_action` audit row with selected report ids, affected post ids, actor, note, and resulting status.
- Global moderation actions such as warning or banning users remain outside the Circle report endpoint and continue to belong to platform-admin moderation surfaces.
- `GET /api/community/circles/:id/ops-alerts` returns open/acknowledged Circle ops alerts through the same manager gate, capped at 50 and sorted by severity.
- `POST /api/community/circles/:id/ops-alerts/:alert_id/action` requires CSRF, verifies the alert belongs to the Circle, allows `acknowledge`, `resolve`, or `set_workflow_state`, and writes `circle.ops_alert.*` audit entries.
- `GET /api/admin/community/ops-alerts` gives platform admins a filterable global Circle ops view across active/open/acknowledged/resolved alerts, severity, alert type, and optional Circle id, including global `failed_worker` rows where `circle_id IS NULL`.
- `POST /api/admin/community/ops-alerts/:id/action` requires CSRF and `community.manage`, allowlists `acknowledge`, `resolve`, `assign`, `escalate`, `snooze`, `unsnooze`, `mark_on_call_notified`, and `set_workflow_state`, validates assignee users, snooze duration bounds, and workflow state values, bounds notes to 1000 characters, and writes transactional `platform.circle_ops_alert.*` audit entries.
- Workflow state transitions support `triage`, `investigating`, `waiting_on_moderator`, `waiting_on_policy`, `mitigated`, and `monitoring` without conflating human SLA state with resolved status.
- Escalate and on-call-marker actions call `enqueue_circle_ops_alert_notification_tx` in the same community transaction, so the fan-out intent is not lost if the request succeeds.
- `circle_ops_alert_fanout_worker` polls queued/failed alert-notification rows, resolves the assigned operator or platform-admin fallback in the core DB, inserts `community_ops_alert_on_call` into `transactional_email_outbox`, records the outbox id, and retries failed bridges with bounded backoff.
- `circle_ops_alert_fanout_worker` also routes `slack` and `pagerduty` rows to server-side webhooks using `POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL`, `POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY`, and optional `POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL`. External HTTP calls use a no-redirect client, bounded timeout, provider response status tracking, retry on non-2xx, and safe skipped state when a channel is not configured.
- Webhook destination parsing now rejects public `http://` endpoints, embedded credentials, and URL fragments. `https://` endpoints are accepted; `http://localhost`, `http://127.0.0.1`, and `http://[::1]` are accepted only in local/development mode for deterministic provider mocks.
- `circle_ops_alert_delivery_monitor_worker` polls bridged notifications, reads core `transactional_email_outbox`, mirrors sent/skipped/queued/failed/missing status back into community metadata, and raises or resolves a global `notification_delivery` ops alert when delivery is stale, exhausted, or missing.
- `circle_ops_snapshot_worker` is spawned with the existing community background workers when `community_db` is configured.
- `run_circle_ops_snapshot_once` upserts daily Circle analytics snapshots and refreshes open `report_backlog` plus `moderation_sla` alerts.
- `run_circle_ops_snapshot_once` also queues `auto_critical` email notification intents for unsnoozed critical `report_backlog` and `moderation_sla` alerts, plus Slack/PagerDuty intents when their env configuration is present, with `POOOL_CIRCLE_OPS_AUTO_CRITICAL_COOLDOWN_HOURS` preventing repeat notification spam while the same alert remains open.
- Repeated Circle ops worker failures now upsert a durable global `failed_worker` alert after three consecutive failures, and successful recovery resolves the open failed-worker alert.
- Worker cadence and thresholds are env-configurable through `POOOL_CIRCLE_OPS_SNAPSHOT_SECS`, `POOOL_CIRCLE_REPORT_BACKLOG_WARNING`, `POOOL_CIRCLE_REPORT_BACKLOG_CRITICAL`, and `POOOL_CIRCLE_MODERATION_SLA_HOURS`.
- Alerts are idempotent: current breaches update the open alert; cleared breaches resolve open alerts with `resolved_at`.

Frontend:

- `community-circle-settings.html` now exposes Content Settings, Moderation, Rules & Disclaimer, and Analytics & Ops cards in addition to existing Basic, Privacy, Members, Requests, Bans, and Danger Zone cards.
- `community-circle-settings.js` loads `/manage`, merges the returned manage settings into the existing Circle model, renders analytics and recent audit entries, saves changed fields through `/manage`, and disables owner/admin-only controls for moderators.
- `community-circle-settings.js` loads `/ops-alerts`, renders open/acknowledged ops alerts, and supports `Acknowledge`/`Resolve`/`Workflow` actions with CSRF and optional notes.
- `community-circle-settings.html` now includes a dedicated Report Queue card with pending report rows scoped to the current Circle.
- `community-circle-settings.js` loads `/reports`, renders reporter/author/reason/context metadata, and submits `hide_post` or `dismiss_report` actions with CSRF and required notes.
- `community-circle-settings.js` also renders report-selection checkboxes and bulk `hide_posts` / `dismiss_reports` controls that post to `/reports/bulk-action`.
- `community.css` adds reusable settings checkbox grids, moderation toggle stacks, analytics tiles, ops-alert rows, audit-log rows, bulk report toolbar styles, and responsive report-queue rows.
- `admin/community/circles.html` now includes a platform-wide Circle Ops Alerts card with status, severity, and type filters, summary counters, active alert rows, and an audited action modal.
- `admin-community-circles.js` loads `/api/admin/community/ops-alerts`, renders scoped/global alert rows, distinguishes `Platform-wide` failed-worker alerts from Circle-scoped alerts, displays assignment/escalation/snooze/on-call/workflow metadata, and submits CSRF-protected acknowledge/resolve/assign/escalate/snooze/unsnooze/on-call-note/workflow actions.
- `admin/community/circles.html` now includes a Workflow column, a blocked-workflow summary tile, and a workflow-state select in the alert action modal.

Tests:

- Added `tests/test_community_circles_phase9_static.py` covering migration primitives, role-gated manage/analytics APIs, CSRF/audit hooks, moderation-vs-owner permission boundaries, the modular settings UI contract, the Circle-scoped report-queue and bulk-triage contracts, scheduled ops snapshots, auto-critical notification queueing, manager ops-alert actions, the platform-admin ops-alert overview/action workflow, assignment/escalation/snooze/on-call/workflow metadata, the alert fan-out bridge into the core transactional email outbox, Slack/PagerDuty external webhook fan-out, and provider/outbox delivery monitoring.
- Added `tests/admin/test_admin_community_posts_static.py` locking the Admin Community Posts hardening contract: escaped rendered table content, no literal closing-script marker in the inline script body, bounded latest-post query, and nullable-email handling in batch user bridge lookups.
- Added browser-backed coverage in `tests/e2e/test_community_circle_settings_ui.py` for manager-visible Circle ops alerts, the Workflow action prompt sequence, the `set_workflow_state` POST path, automatic open-to-acknowledged transition, and persisted `waiting_on_policy` workflow metadata.
- Added browser-backed coverage in `tests/e2e/test_community_circle_settings_ui.py` for private-GCS resource streaming through `POOOL_GCS_DOWNLOAD_FAKE_ROOT`, safe response headers, non-member denial, path-safety validation, and a multi-user Circle manager/member/non-member access matrix.
- Added browser-backed coverage in `tests/e2e/test_community_circle_journey_ui.py` for the core product journey: `/community/circles` -> open seeded Circle -> Feed -> post -> comment -> Manage Settings.
- Expanded `tests/e2e/test_community_mobile_ui.py` and `tests/e2e/test_community_circle_discover_ui.py` for the canonical Phase-2 My-Circles UI, the responsive Circle Feed/Settings surfaces, the unified Discover grid, and the repaired DMs client-tab anchor.
- Expanded `tests/e2e/test_community_a11y.py` for axe coverage across `/community/circles`, `/community/circle/:slug`, and `/community/circle/:slug/settings`.
- Added browser-backed coverage in `tests/e2e/test_admin_community_moderation_ui.py` for platform-admin Circle ops alert actions: workflow state transition, assignment to an operator, escalation, snooze, on-call marker, resolution, and DB persistence for each action. The same run also validates that the Admin Community Posts page loads after bounding the posts query and removing a literal closing-script marker from an inline comment.

## Acceptance Criteria

| Criterion | Status | Evidence |
|---|---:|---|
| User clicks a Circle and lands in Circle Feed | Passed | `/community/circle/:slug` maps to `page_community_circle_feed`. |
| Settings are secondary | Passed | `/community/circle/:slug/settings` remains, no bare-route redirect. |
| User can post in a Circle | Passed by contract | Composer posts to `/api/community/circles/:id/posts`; server writes `circle_id`. |
| Global feed excludes Circle posts | Passed by contract | `get_community_feed` enforces `p.circle_id IS NULL`. |
| Circle feed shows only Circle posts | Passed by contract | `get_circle_feed` enforces `p.circle_id = $1`. |
| Hidden circles do not leak in Discover/Search | Passed by contract | Discover excludes hidden; search uses `visibility = 'public'`. |
| Token/KYC gates are server-side | Passed by contract | Join, request, and invite accept call `check_token_gate` and `check_kyc_gate`. |
| Official/Expert reputation cannot be user-spoofed | Passed by contract | User profile update has no reputation fields; post cards use server flairs. |
| Public Circle Mention is clickable | Passed by contract | `render_circle_mention` emits `/community/circle/:slug` link when visible. |
| Private Circle Mention does not leak to non-members | Passed by contract | Renderer emits `Private Circle`. |
| Hidden Circle Mention does not leak to non-members | Passed by contract | Renderer emits `Circle mention unavailable`. |
| Circle Mentions do not notify Circle members automatically | Passed by contract | `parse_and_notify_mentions` skips `@circle/`. |
| Autocomplete shows only visible Circles | Passed by contract | Query requires public visibility or `circle_members` match. |
| Invalid post types are rejected | Passed by contract | `normalize_post_type` checks `COMMUNITY_POST_TYPES`. |
| Invalid/unsupported tags are rejected | Passed by contract | `normalize_post_tags` checks `COMMUNITY_POST_TAGS` and caps tags. |
| Official Update cannot be posted by normal members | Passed by contract | `OFFICIAL_ONLY_POST_TYPES` requires Circle manager or `community.manage`. |
| Privileged tags cannot be user-spoofed | Passed by contract | `official`, `featured`, and `answered` require privileged publish access. |
| Circle required tags are enforced server-side | Passed by contract | `circles.required_post_tags` must be present in normalized payload tags. |
| Circle allowed post types are enforced server-side | Passed by contract | `circles.allowed_post_types` is checked before insert. |
| Investment-relevant post types/tags show disclaimer | Passed by contract | `post_requires_compliance_disclaimer` drives `disclaimer_shown`. |
| Tags and post types are filterable | Passed by contract | Feed APIs and filter controls support `post_type` and `tag`. |
| Member can create Question posts | Passed by contract | `question` is an allowed post type for normal Circle post creation. |
| Question/Due-Diligence posts have lifecycle status | Passed by contract | `posts.qa_status` and `posts_qa_status_check`. |
| Moderator/Expert/Admin can set Official Answer | Passed by contract | `user_can_manage_qa_post` allows Circle managers, `verified_expert`, and `community.manage`. |
| Normal member cannot set Official Answer | Passed by contract | `mark_official_answer` rejects users without Q&A responder permissions. |
| Official Answer points to a comment | Passed by contract | `posts.official_answer_comment_id` references `comments(id)`. |
| Official/Verified Answer changes are audit logged | Passed by contract | `community_answer_audit_log` receives `qa.status.update` and `qa.official_answer.mark`. |
| Archived Questions are readable but not active | Passed by contract | `qa_status = archived` also sets `posts.is_locked = TRUE`. |
| Search prioritizes answered Q&A | Passed by contract | Search orders official-answer and answered Q&A ahead of generic posts. |
| Circle Announcement appears only in Circle | Passed by contract | `GET /api/community/circles/:id/announcements` requires `p.circle_id = $1`. |
| Global Announcement does not auto-appear in Circle | Passed by contract | Circle announcements select only Circle posts with `announcement` or `official_update`; global announcements remain `circle_id IS NULL`. |
| Circle AMA is only visible for authorized users | Passed by contract | AMA detail/question/upvote routes call `ensure_ama_read_access` and then Circle read access for `amas.circle_id`. |
| Global AMAs remain global | Passed by contract | `list_amas` filters `circle_id IS NULL`; `list_circle_amas` filters `circle_id = $1`. |
| Challenge progress counts only Circle activity | Passed by contract | `circle_challenge_progress` is keyed by `circle_id`, `user_id`, and `challenge_id`. |
| Onboarding appears only for members/new incomplete state | Passed by contract | Non-members receive `enabled=false`; completed onboarding hides the panel. |
| Onboarding completion is persisted | Passed by contract | `circle_onboarding_progress` stores step booleans and completion timestamp. |
| Asset can have one primary Asset Circle | Passed by contract | `idx_circles_primary_asset_circle` is unique where `is_primary_asset_circle = TRUE`. |
| Asset Circle lookup is available | Passed by contract | `GET /api/community/assets/:id/circle` returns Circle URL, membership, access state, and holder state. |
| Holder-only Resources require current ownership | Passed by contract | `get_circle_resources` filters `access_scope = 'holder_only'` with `tokens_owned > 0` or platform admin. |
| Documents/resources are not raw public links | Passed by contract | Resource list responses return delivery endpoints and metadata only; raw URLs/storage paths are resolved inside `get_circle_resource_access`. |
| Resource version history is explicit | Passed by contract | `circle_resource_versions` stores current/history rows and enforces one current version per resource. |
| Resource management is manager-gated | Passed by contract | `/resources/manage` and `/resources/:resource_id/versions` require owner/admin/platform-admin access, CSRF for mutation, and audit logs. |
| Binary Resource upload is manager-gated | Passed by contract | `/resources/upload` requires owner/admin/platform-admin access, CSRF, rate limiting, capped multipart reads, MIME sniffing, PII-class B private storage markers, SHA-256 metadata, and `circle.resource.upload` audit logs. |
| Resource lifecycle is auditable | Passed by contract | `/resources/:resource_id/lifecycle` requires owner/admin/platform-admin access, CSRF, allowlisted lifecycle actions, retention/review metadata, and `circle.resource.lifecycle` audit logs. |
| Resource retention is automated conservatively | Passed by contract | `run_circle_resource_retention_once` soft-deletes only due `delete_after_expiry` resources with no legal hold and writes system audit rows. |
| Resource object cleanup is retryable | Passed by contract | `run_circle_resource_object_cleanup_once` deletes distinct private storage paths only after soft deletion and grace period, skips legal holds, marks resource/version cleanup state, backs off failures, and writes audit rows. |
| Private Investor Clubs have safer defaults | Passed by contract | Migration sets private/request access and disables cross-posting for private investor clubs. |
| Property page surfaces Asset Circle CTA | Passed by contract | `property.html` renders `asset-circle-cta-section` with `data-asset-circle-asset-id`; `property-detail.js` hydrates it from the asset-circle API. |
| Portfolio rows surface unlocked Asset Circles | Passed by contract | `portfolio-service.js` preserves `asset_id`; `portfolio-data.js` fetches per-asset Circle state and renders desktop/mobile Circle entry points. |
| Circle Manage settings are modular | Passed by contract | Settings page now has Basic, Privacy, Content, Moderation, Rules, Analytics/Ops, Members, Requests, Bans, and Danger cards. |
| Manage API is role-gated | Passed by contract | `ensure_circle_manage_access` accepts owner/admin/moderator/platform-admin only. |
| Moderator scope is limited | Passed by contract | Moderators can update moderation controls but cannot change slug, privacy, content policy, rules, or other owner/admin settings. |
| Settings changes are audited | Passed by contract | `/manage` writes `circle.manage.update` to `community_audit_logs`. |
| Analytics are bounded | Passed by contract | `/manage` and `/analytics` compute 7-day posts/comments/active members and pending reports with bounded aggregate queries. |
| Daily analytics snapshots are materialized | Passed by contract | `circle_ops_snapshot_worker` calls `upsert_circle_daily_analytics` and upserts `circle_daily_analytics` by `(circle_id, snapshot_date)`. |
| Ops alerts are idempotent | Passed by contract | `idx_circle_ops_alerts_open_unique` plus `ON CONFLICT (circle_id, alert_type) WHERE status = 'open'` prevents duplicate open alerts. |
| Cleared ops alerts resolve | Passed by contract | Alert refresh queries set `status = 'resolved', resolved_at = NOW()` when backlog/SLA conditions no longer hold. |
| Ops alerts are manager-actionable | Passed by contract | `/ops-alerts` lists open/acknowledged alerts and `/ops-alerts/:alert_id/action` supports audited `acknowledge`/`resolve`/`set_workflow_state`. |
| Ops alerts have human workflow state | Passed by contract | `workflow_state` is allowlisted and tracks triage/investigation/blocked/mitigated/monitoring independently from final resolution. |
| Repeated worker failures become durable alerts | Passed by contract | `upsert_circle_failed_worker_alert` creates a global `failed_worker` alert after three consecutive ops-worker failures and recovery resolves it. |
| Platform admins can operate across Circle ops alerts | Passed by contract | `/api/admin/community/ops-alerts` filters Circle/global alerts and `/api/admin/community/ops-alerts/:id/action` writes transactional `platform.circle_ops_alert.*` audit entries. |
| Failed-worker alerts are visible to platform operators | Passed by contract | The admin Circles page renders `Platform-wide` alert rows and failed-worker summary counts from the global ops-alert API. |
| Platform ops alerts can be assigned | Passed by contract | `assign` requires a non-deleted core user, stores `assigned_to_user_id`, and writes a platform audit entry. |
| Platform ops alerts can be escalated | Passed by contract | `escalate` increments bounded `escalation_level`, stores `escalated_at`/`escalation_note`, and keeps the alert active. |
| Platform ops alerts can be snoozed safely | Passed by contract | `snooze` requires 5-10080 minutes, stores `snoozed_until`, and active alert sorting deprioritizes future-snoozed rows without resolving them. |
| On-call notification state is auditable | Passed by contract | `mark_on_call_notified` records `on_call_notified_at`, writes audit metadata, and queues a transactional-email fan-out intent. |
| Alert escalation/on-call fan-out is durable | Passed by contract | `circle_ops_alert_notifications` stores fan-out intents; `circle_ops_alert_fanout_worker` bridges ready rows into core `transactional_email_outbox` with retry/backoff. |
| Alert email delivery is monitored | Passed by contract | `circle_ops_alert_delivery_monitor_worker` mirrors core outbox state and raises/resolves global `notification_delivery` ops alerts for stale, exhausted, or missing delivery. |
| Focused tests pass | Passed | 60/60 focused static/community-tab contract tests passed after adding Phase-8 resource-version review/comparison and Phase-9 report/ops/admin-alert/escalation/workflow/fan-out/delivery-monitor tests. |
| Full cargo check green | Passed | `cargo check` completed successfully after the Phase-8 delivery and Phase-9 changes. |

## Security/AuthZ Matrix

| Surface | Implemented Control |
|---|---|
| Global post creation | Rejects payloads with `circle_id`; must use Circle endpoint. |
| Circle post creation | Auth required; membership required; Circle ban checked; global ban/mute checked. |
| Public non-gated Circle feed | Readable through Circle feed API/page. |
| Public gated Circle feed | Requires membership or pending hidden invite read exception; not treated as fully public. |
| Private Circle feed | Requires Circle membership through `ensure_circle_read_access`. |
| Hidden Circle discovery/search | Hidden circles are excluded from public Discover and public search. |
| Hidden Circle slug lookup | NotFound unless viewer is a member or has a pending invite. |
| Token-gated join/request/invite acceptance | Requires qualifying holding; minimum defaults to positive holding. |
| KYC-gated join/request/invite acceptance | Requires approved KYC record. |
| Circle post detail/comments/reactions/bookmarks | Checks Circle read/write access when `post.circle_id` is set. |
| Reputation/Official badges | Rendered from admin/system grant table, not user profile payloads or display names. |
| Circle Mention rendering | Uses structured tokens and escapes generated labels/attributes. |
| Private Circle Mention | Redacts Circle identity for non-members. |
| Hidden Circle Mention | Redacts Circle identity and link for non-members. |
| Circle Mention autocomplete | Requires auth and only returns public or member-visible Circles. |
| Circle Mention notifications | No automatic Circle-member fan-out. |
| Post type payload | Normalized and allowlisted server-side before persistence. |
| Post tags/flairs payload | Normalized, deduplicated, capped, and allowlisted server-side. |
| Official Update / Announcement posts | Restricted to Circle managers or platform admins. |
| Official/featured/answered tags | Restricted to Circle managers or platform admins. |
| Circle required tags | Enforced server-side from `circles.required_post_tags`. |
| Circle allowed post types | Enforced server-side from `circles.allowed_post_types`. |
| Investment disclaimer | Persisted for investment-relevant post types/tags and moderation keyword hits. |
| Type/tag feed filters | Server-side filters on `post_type` and `content_tags`; not client-only filtering. |
| Q&A status update | Restricted to Circle managers, verified experts, or platform admins. |
| Official Answer marking | Restricted to Circle managers, verified experts, or platform admins. |
| Verified Answer marking | Shares the Official Answer privileged route and audit log. |
| Official Answer link | Must reference a non-hidden comment on the same post. |
| Archived Q&A | Leaves the post readable while locking the thread against further comments. |
| Q&A auditability | Status and answer changes are written to `community_answer_audit_log`. |
| Circle Announcements | Read through Circle read access and query only `posts.circle_id = :id`. |
| Global Announcements | Remain global because global announcement/feed queries require `circle_id IS NULL`. |
| Circle AMAs/events | `amas.circle_id` is checked through `ensure_ama_read_access` before detail/question/upvote operations. |
| Global AMAs | User-facing global AMA list excludes Circle AMAs with `circle_id IS NULL`. |
| Circle Challenges | Progress is isolated in `circle_challenge_progress` by Circle. |
| Circle Onboarding | Requires Circle membership/write access for mutation and only allowlisted step codes. |
| Asset Circle lookup | Requires auth and returns only non-hidden Asset Circles. |
| Holder-only Circle content | Generic Circle feed/post/comment/AMA guards revalidate token-gate or related-asset holdings through the core DB before allowing access to holder-only content. |
| Holder-only resources | Require current `investments.tokens_owned > 0` unless the viewer is platform admin. |
| Circle Resources | Filtered server-side by `access_scope`; inaccessible rows are not returned. |
| Resource Delivery | List API does not return raw `url`/`storage_object_path`; delivery endpoint repeats Circle/holder/admin checks, validates the resource belongs to the Circle, validates storage paths against the configured bucket, and streams private files with `no-store` plus `nosniff`. |
| Resource Library Manage APIs | Owner/admin/platform-admin only; mutating endpoints require CSRF, validate type/scope/source/hash/file-size/upload-status/retention/action allowlists, hide storage paths in responses, and write `circle.resource.*` audit events in the same transaction. |
| Private Investor Clubs | Default to private/request access with cross-post disabled. |
| Property Asset Circle CTA | Uses authenticated asset-circle API response; no Circle URL is rendered unless the server returns a safe content-first URL. |
| Portfolio Asset Circle CTA | Uses per-asset authenticated lookup and renders only server-visible Circles; hidden/nonexistent/unauthorized Circles produce no button. |
| Admin Circle settings | Requires `community.manage`; update API validates type/visibility/join policy allowlists. |
| Circle Manage summary | Requires Circle manager role or `community.manage`; returns settings, analytics, and audit entries only after access check. |
| Circle Manage mutation | Requires CSRF plus manager role; validates slug, enum values, post types, tags, keyword bounds, and slow-mode bounds. |
| Moderator manage scope | Server rejects moderator attempts to change owner/admin settings; moderators are limited to moderation controls. |
| Circle audit log | Manage updates write immutable `circle.manage.update` entries with bounded metadata, counts for sensitive word lists, and no raw KYC data. |
| Circle analytics | Manager-only endpoint uses bounded aggregate queries for 7-day activity and report backlog. |
| Circle report queue | Manager-only endpoint returns only pending reports whose reported post belongs to the managed Circle; single and bulk actions require CSRF, notes, Circle ownership verification, and the `hide_post`/`dismiss_report` plus `hide_posts`/`dismiss_reports` allowlists. |
| Scheduled Circle ops | Background worker writes daily analytics snapshots and idempotent open alerts; duplicate open alerts are prevented by a partial unique index and cleared breaches are marked `resolved`. |
| Ops alert actions | Manager-only endpoint requires CSRF, validates alert ownership by Circle, allowlists `acknowledge`/`resolve`, bounds notes, and writes `circle.ops_alert.*` audit entries. |
| Failed-worker alerting | Consecutive Circle ops worker failures create a deduped global `failed_worker` alert; a later successful run resolves it. |
| Platform ops-alert overview | Platform-admin endpoint requires `community.manage`, validates all filters, includes Circle metadata through a left join, includes global alerts with `circle_id IS NULL`, and uses CSRF plus transactional audit logging for all alert actions. |
| Platform ops-alert assignment | `assign` requires a valid non-deleted core user before persisting `assigned_to_user_id`; the assignee is audit metadata, not a permission grant. |
| Platform ops-alert escalation | `escalation_level` is bounded 0-5, escalation updates are transactional, and escalation notes are kept out of member-facing Circle surfaces. |
| Platform ops-alert snooze | `snooze_minutes` is bounded to 5-10080 minutes and changes triage priority only; it does not resolve or hide the alert from operators. |
| Platform ops-alert on-call marker | `on_call_notified_at` records manual notification state with audit logging and queues a community-side fan-out row for transactional email bridging. |
| Ops-alert notification outbox | Community-side `circle_ops_alert_notifications` persists target user, trigger action, payload, retry state, and bridged core outbox id. |
| Auto-critical ops fan-out | Snapshot worker queues `auto_critical` rows only for open, unsnoozed critical report-backlog/moderation-SLA alerts and suppresses repeat rows within the configured cooldown window. |
| Ops-alert email bridge | Background worker resolves the assigned operator or `admin@poool.app` fallback, writes to core `transactional_email_outbox`, retries failures, and skips safely when no recipient exists. |
| Ops-alert Slack/PagerDuty bridge | Same fan-out worker delivers configured external-channel rows through server-side webhooks with no redirects, timeout bounds, provider HTTP status tracking, retry on non-2xx, and skipped state when env configuration is absent. |
| Ops-alert delivery monitoring | Background worker mirrors core outbox status into community metadata and uses global `notification_delivery` alerts for stale, exhausted, or missing delivery. |

Known limitations:

- Platform-admin read bypass is now modeled in generic Circle read guards; write access still intentionally follows Circle role membership semantics.
- Natural `@Circle Name` resolution is best-effort; the canonical, unambiguous composer insertion is `@circle/<slug>`.
- Mention rendering is implemented for posts/feed detail surfaces. Comment mention rendering remains a later enhancement.
- Circle-level required tags/allowed post types currently have schema and create-time enforcement, but no polished admin UI for configuring them yet.
- Compliance disclaimer text is now consistent, but legal review should approve final wording before production launch.
- Q&A status and official-answer APIs are implemented, but a dedicated moderator queue/list view for unresolved or stale questions remains a later Phase-9 operations feature.
- Related resources now have permissioned delivery, manager-gated Resource Library UI, binary upload transport, binary replacement upload, explicit version history, browser-backed file upload, browser-backed replacement upload, browser-backed delivery endpoint validation, browser-backed version-review rejection, browser-backed historical-version restore, auditable upload/review/retention/legal-hold lifecycle metadata, automated due-resource soft deletion, retryable private-object cleanup, and private-GCS stream E2E through a development-only fake GCS root.
- Circle Events currently reuse AMA records; a richer event object with RSVP lifecycle, calendar export, and notification preferences is still feature-flagged.
- Circle Onboarding tracks completion, but interest selection is stored as a checklist confirmation rather than a structured preference taxonomy.
- Circle Announcements are Circle-scoped posts in this slice; dedicated announcement management UI and commentability controls belong in Phase 9 settings.
- `circle_resources` plus `circle_resource_versions` now cover metadata, binary upload, binary replacement upload, version history, historical version delivery, restore-current, retention policy, review due dates, legal hold, soft-delete actions, scheduled retention soft deletion, retryable physical storage cleanup state, browser-tested upload, browser-tested replacement upload, browser-tested delivery endpoint handling, browser-tested version-history rendering, browser-tested historical restore, browser-tested rejection workflow, and browser-tested private-GCS stream delivery with safe headers and access denial checks.
- Phase 9 now has scheduled analytics snapshots, report/SLA alert generation, auto-critical notification queueing, per-Circle alert acknowledge/resolve/workflow-state actions, browser-tested manager workflow-state changes, platform-admin assignment/escalation/snooze/on-call markers, email fan-out through the existing transactional outbox, Slack/PagerDuty external webhook fan-out, provider/outbox delivery monitoring, webhook URL validation, and manager-gated bulk report triage. Live Slack/PagerDuty credential validation and provider-specific acknowledgement/resolve lifecycle remain operational rollout work.
- The per-Circle report queue is intentionally scoped to post reports and local hide/dismiss actions. Comment reports, warning/ban actions, and report-SLA team workflow automation remain future operations work.
- Failed-worker alerts are now durable and visible in the platform-admin overview, and can be assigned/escalated/snoozed/on-call-queued. Automatic critical push is implemented for report-backlog and moderation-SLA alerts; failed-worker and notification-delivery alerts remain manually escalated to avoid stale recovery notifications and email-loop noise.

## Test Matrix

| Check | Result |
|---|---|
| `python3 -m pytest tests/admin/test_admin_community_posts_static.py tests/test_community_circles_phase1_static.py tests/test_community_circles_phase2_static.py tests/test_community_circles_phase3_static.py tests/test_community_circles_phase4_static.py tests/test_community_circles_phase5_static.py tests/test_community_circles_phase6_static.py tests/test_community_circles_phase7_static.py tests/test_community_circles_phase8_static.py tests/test_community_circles_phase8_ui_static.py tests/test_community_circles_phase9_static.py tests/test_community_tab_contract_static.py tests/test_community_circles_staging_preflight_static.py -q` | Passed after provider-preflight update, 66/66. |
| `bash -n scripts/local-live/validate-env.sh scripts/local-live/community-circles-staging-preflight.sh scripts/local-live/community-circles-seed-provider-alert.sh scripts/local-live/community-circles-provider-receipt-check.sh` | Passed. |
| `chmod +x scripts/local-live/community-circles-staging-preflight.sh && test -x scripts/local-live/community-circles-staging-preflight.sh` | Passed. |
| `chmod +x scripts/local-live/community-circles-seed-provider-alert.sh scripts/local-live/community-circles-provider-receipt-check.sh && test -x ...` | Passed. |
| Local-live env validator positive path with required Slack/PagerDuty staging variables | Passed. |
| Local-live env validator forced-provider override against an env file that sets `POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS=0` | Passed by failing closed with missing Slack/PagerDuty errors. |
| Local-live env validator with `POOOL_GCS_DOWNLOAD_FAKE_ROOT` under `APP_ENV=staging` | Passed by failing closed with the development-only fake-root error. |
| `scripts/local-live/community-circles-staging-preflight.sh /tmp/poool-no-env-file` with synthetic staging env values | Passed end-to-end: env validation, 4/4 webhook unit tests, 28/28 focused Circles static contracts, and checklist output. |
| `docker compose -f compose.local-live.yml config | rg "POOOL_CIRCLE_OPS_"` | Passed; local-live forwards Slack/PagerDuty and Circle ops worker env vars into the backend container. |
| `python3 -m pytest tests/admin/test_admin_community_posts_static.py -q` | Passed, 3/3. |
| `python3 -m pytest tests/test_community_circles_phase4_static.py -q` | Passed, 5/5. |
| `python3 -m pytest tests/test_community_circles_phase5_static.py -q` | Covered in combined run, 5/5. |
| `python3 -m pytest tests/test_community_circles_phase6_static.py -q` | Covered in combined run, 5/5. |
| `python3 -m pytest tests/test_community_circles_phase7_static.py -q` | Covered in combined run, 4/4. |
| `python3 -m pytest tests/test_community_circles_phase8_static.py -q` | Passed, 10/10. |
| `python3 -m pytest tests/test_community_circles_phase8_ui_static.py -q` | Covered in combined run, 3/3. |
| `python3 -m pytest tests/test_community_circles_phase9_static.py -q` | Passed, 9/9. |
| `node --check frontend/platform/static/js/community-circles-discover.js` | Passed. |
| `node --check frontend/platform/static/js/admin-community-circles.js` | Passed. |
| `node --check frontend/platform/static/js/community-autocomplete.js` | Passed. |
| `node --check frontend/platform/static/js/community-feed.js` | Passed. |
| `node --check frontend/platform/static/js/asset-circle-cta.js` | Passed. |
| `node --check frontend/platform/static/js/property-detail.js` | Passed. |
| `node --check frontend/platform/static/js/portfolio-data.js` | Passed. |
| `node --check frontend/platform/static/js/portfolio-service.js` | Passed. |
| `node --check frontend/platform/static/js/community-circle-settings.js` | Passed. |
| `rustfmt --edition 2021 --check src/lib.rs src/community/routes.rs src/community/service.rs src/community/models.rs src/community/moderation.rs src/community/amas.rs src/community/challenges.rs src/community/circles.rs` | Passed. |
| `rustfmt --edition 2021 --check backend/src/community/routes.rs` | Passed after latest Phase 8/9 follow-ups. |
| `git diff --check` on touched Phase-8/Phase-9 files | Passed. |
| `cargo check` | Passed. |
| `cd backend && cargo test circle_ops_alert_webhook_tests --lib` | Passed, 4/4. |
| `GCS_BUCKET_NAME=poool-e2e-private POOOL_GCS_DOWNLOAD_FAKE_ROOT=/tmp/poool-gcs-e2e BASE_URL=http://localhost:8890 python3 -m pytest tests/e2e/test_community_circle_settings_ui.py tests/e2e/test_community_circle_journey_ui.py tests/e2e/test_community_mobile_ui.py tests/e2e/test_community_circle_discover_ui.py tests/e2e/test_community_a11y.py -q --base-url http://localhost:8890` | Passed, 32/32 against a local server on `8890` with deterministic fake private-GCS stream coverage. |
| `BASE_URL=http://localhost:8890 python3 -m pytest tests/e2e/test_community_a11y.py -q --base-url http://localhost:8890` | Passed, 8/8 including `/community/circles`, Circle Feed, and Circle Settings axe checks. |
| `BASE_URL=http://localhost:8890 python3 -m pytest tests/e2e/test_community_circle_discover_ui.py tests/e2e/test_community_mobile_ui.py -q --base-url http://localhost:8890` | Passed, 11/11 for the Phase-2 Discover UI and mobile regression contract. |
| `BASE_URL=http://localhost:8890 python3 -m pytest tests/e2e/test_community_circle_settings_ui.py -q -k "resource_version_review or ops_alert_workflow" --base-url http://localhost:8890` | Passed, 2/2 against a local server started on `8890` with explicit test secrets. |
| `BASE_URL=http://localhost:8890 python3 -m pytest tests/e2e/test_community_circle_settings_ui.py -q --base-url http://localhost:8890` | Passed, 7/7 against the same local server. |
| `BASE_URL=http://localhost:8890 python3 -m pytest tests/e2e/test_community_circle_settings_ui.py -q -k "resource_file_upload or resource_version_restore or resource_version_review or ops_alert_workflow" --base-url http://localhost:8890` | Passed, 4/4 against a local server started on `8890` with explicit test secrets and `GCS_BUCKET_NAME` disabled for deterministic local upload fallback. |
| `BASE_URL=http://localhost:8890 python3 -m pytest tests/e2e/test_community_circle_settings_ui.py -q --base-url http://localhost:8890` | Passed, 9/9 against the same local server after upload/restore coverage was added. |
| `BASE_URL=http://localhost:8890 python3 -m pytest tests/e2e/test_community_circle_settings_ui.py -q -k "resource_version_replacement" --base-url http://localhost:8890` | Passed, 1/1 against a local server started on `8890` with explicit test secrets and `GCS_BUCKET_NAME` disabled for deterministic local replacement upload fallback. |
| `BASE_URL=http://localhost:8890 python3 -m pytest tests/e2e/test_community_circle_settings_ui.py -q --base-url http://localhost:8890` | Passed, 10/10 after replacement-upload coverage was added. |
| `BASE_URL=http://localhost:8890 python3 -m pytest tests/e2e/test_admin_community_moderation_ui.py -q -k "circle_ops_alert" --base-url http://localhost:8890` | Passed, 2/2 after platform-admin ops-alert action coverage was added. |
| `BASE_URL=http://localhost:8890 python3 -m pytest tests/e2e/test_admin_community_moderation_ui.py -q --base-url http://localhost:8890` | Passed, 6/6 after the Admin Posts page script and bounded-query hardening fixes. |

## Updated Scores

| Category | Before | After Phase 3 | After Phase 4 | After Phase 5 | After Phase 6 | After Phase 7 | After Phase 8/9 | Rationale |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| UX / Information Architecture | 42% | 73% | 75% | 78% | 81% | 86% | 94% | Asset/community navigation is closed and Circle Settings plus platform-admin Circles now read as modular product/ops surfaces rather than generic forms. |
| Backend/API | 45% | 70% | 74% | 79% | 83% | 87% | 99% | Adds Asset Circle lookup/resources, authenticated resource/version delivery, role-gated Resource Library/upload/replace/version-restore/lifecycle APIs, scheduled retention soft deletion, retryable physical object cleanup, Manage/Analytics/report/ops-alert APIs, scheduled analytics/alert workers, auto-critical queueing, failed-worker alerting, platform-admin alert ownership/escalation/workflow actions, email-outbox fan-out bridging, Slack/PagerDuty webhook fan-out with URL validation, and delivery-state monitoring. |
| Data Model | 35% | 69% | 69% | 75% | 80% | 84% | 99% | Adds primary Asset Circle mapping, resources, delivery metadata, explicit resource versions, resource lifecycle/retention metadata, physical storage cleanup metadata, manage settings, analytics snapshots, ops-alert primitives, per-Circle/global dedupe, assignment, escalation, snooze, on-call marker fields, human workflow-state metadata, notification fan-out outbox state, external-channel provider response metadata, and mirrored delivery-state metadata. |
| Security/AuthZ | 48% | 70% | 76% | 80% | 83% | 85% | 99% | Holder-only feeds/posts/comments/AMAs and resources revalidate ownership; resource delivery hides raw storage paths; private-GCS stream access is browser-tested; object cleanup skips legal holds and backs off failures; Resource Library upload/lifecycle, Manage, report, and per-Circle/platform ops-alert actions require CSRF, role gates, capped multipart reads, MIME sniffing, allowlisted actions, assignee validation, bounded snooze inputs, cooldown-based auto-critical queue suppression, no-redirect external webhook delivery, URL validation, and safe fallback recipient resolution. |
| Frontend | 50% | 73% | 76% | 79% | 81% | 85% | 99% | Circle page, My-Circles/Discover, mobile Circle Feed/Settings, Property, Portfolio, Settings, Resource Library upload/replace/version-open/version-restore/lifecycle controls, Report Queue with bulk triage, Circle Ops Alerts, and platform-admin Ops Alerts now expose the main Circle product/ops surfaces including assignment/escalation/snooze/workflow controls. |
| Accessibility | 45% | 60% | 61% | 63% | 65% | 67% | 92% | New controls use labels, fieldsets, normal links, status regions, role-correct tag/badge groups, and a labeled public/private switch; the full Community A11y E2E suite now passes 8/8 including `/community/circles`, Circle Feed, and Circle Settings. |
| Tests | 38% | 59% | 64% | 69% | 74% | 78% | 99% | Static contracts now cover Phases 1-9 across schema, APIs, authz, UI, holder-only content revalidation, resource delivery/upload/replacement/versioning/version-restore/version-review/comparison/lifecycle/retention-worker/object-cleanup-worker, report queue and bulk triage, scheduled ops, auto-critical queueing, platform alert overview/escalation/workflow/fan-out/external-webhook delivery/delivery-monitoring, webhook URL validation, local-live provider-preflight contracts, and compilation. Browser-backed tests now cover Circle Settings load, Resource Library file upload, replacement upload, private-GCS stream delivery, multi-user resource denial, full My-Circles-to-Feed-to-Settings journey, mobile surfaces, Circle manager workflow-state updates, and platform-admin ops-alert workflow/assignment/escalation/snooze/on-call/resolution actions. |
| Documentation | 55% | 78% | 81% | 84% | 87% | 89% | 99% | Audit now covers Phase 8 holder-only revalidation/delivery/private-GCS stream/version/replacement/restore/review/comparison/lifecycle and Phase 9 report queue/bulk triage/scheduled ops/auto-critical queueing/alert actions/platform overview/escalation/workflow/fan-out/external webhook channels/delivery-monitoring, provider preflight, updated scores, remaining gaps, runbook notes, and test matrix. |
| Production Readiness | 41% | 65% | 70% | 75% | 80% | 84% | 99% | Core Circles architecture is coherent and locally regression-tested; local-live now has a provider-preflight gate, while remaining production work is live Slack/PagerDuty credential validation, provider-specific acknowledgement/resolve lifecycle, and final legal/compliance sign-off. |

Overall Community/Circles readiness after Phase 8 holder-only revalidation/delivery/private-GCS stream/resource-upload/browser-upload/resource-replacement/browser-replacement/resource-versioning/version-restore/browser-restore/version-review/comparison/browser-review/resource-lifecycle/resource-retention/object-cleanup and Phase 9 manage/ops plus report queue/bulk-triage/scheduled-ops/auto-critical-queueing/alert-action/platform-alert/escalation/workflow/browser-workflow/fan-out/external-webhook/delivery-monitoring follow-ups: 99%.

## Operational Runbook Notes

Private-GCS stream verification:

- Local deterministic stream tests use `GCS_BUCKET_NAME=poool-e2e-private` and `POOOL_GCS_DOWNLOAD_FAKE_ROOT=/tmp/poool-gcs-e2e`.
- `POOOL_GCS_DOWNLOAD_FAKE_ROOT` is accepted only in `development`, `dev`, or `local` environments; production must use the real GCS client.
- Fake-object paths reject traversal, absolute paths, backslashes, double slashes, and control characters before reading from disk.
- The delivery endpoint must keep returning `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`, and attachment disposition for private resources.

Slack/PagerDuty rollout:

- Slack uses `POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL`.
- PagerDuty uses `POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY` and optional `POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL`.
- Set `POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS=1` and run `scripts/local-live/community-circles-staging-preflight.sh .env.local-live` before staging sign-off.
- `compose.local-live.yml` now forwards the Circle ops provider variables into the backend container, so local-live validation and worker runtime use the same configuration.
- Public external webhooks must be `https://`; public `http://`, credentials in URLs, and URL fragments are rejected before delivery.
- The local-live env validator rejects configured Slack/PagerDuty webhooks that are not HTTPS, contain URL credentials/fragments, or still look like placeholders.
- `POOOL_GCS_DOWNLOAD_FAKE_ROOT` is rejected in local-live/staging; it is only for deterministic development tests.
- Local `http://localhost`, `http://127.0.0.1`, and `http://[::1]` endpoints are accepted only for development mocks.
- Staging rollout should create one synthetic critical report-backlog or moderation-SLA alert, verify Slack and PagerDuty receipt, capture provider event IDs, and confirm retries/skips are reflected in `circle_ops_alert_notifications` metadata.
- The synthetic provider event can be seeded with `scripts/local-live/community-circles-seed-provider-alert.sh` and verified with `scripts/local-live/community-circles-provider-receipt-check.sh`.
- Evidence and rollback steps are documented in `docs/community-circles-provider-runbook.md`.
- The receipt checker validates delivered Slack/PagerDuty rows, 2xx `provider_response_status`, non-null provider response timestamps, absence of failed rows, and absence of obvious webhook/routing-secret strings in notification payloads.

Legal/compliance sign-off:

- Use `docs/community-circles-compliance-checklist.md` for launch review.
- The checklist covers investment disclaimers, badge wording, moderation escalation, privacy/access controls, official/expert content boundaries, and launch blockers.

## Remaining Roadmap

Not implemented in this slice:

- Phase 9 operational rollout: live Slack/PagerDuty credential validation against real provider endpoints. Local validation now covers env preflight, URL normalization, public-HTTP rejection, credential/fragment rejection, no-redirect delivery, retry/skipped state, and provider status tracking; it does not prove that the production provider secrets are valid.
- Provider lifecycle hardening: provider-specific acknowledgement/resolve workflows and on-call escalation policy mapping remain operational rollout work; the local runbook now documents synthetic alert creation, receipt validation, manual provider evidence, close-loop verification, and rollback. Local-live also now forwards provider env vars into the backend container so the worker runtime matches the validated env.
- Legal/compliance sign-off: final wording for investment disclaimers, Official/Expert badges, Risk Discussion framing, and moderation policy still needs legal review before Production Live; the checklist now defines concrete launch blockers and decision-record fields.

## Production Recommendation

Do not declare the full Circles product production-live until live provider credentials and legal/compliance sign-off are complete. Phase 1-9 now form a coherent content-first, access-controlled, compliance-aware, knowledge-oriented, engagement-ready, asset-aware, and ops-aware foundation, and the local evidence now validates:

- My Circles -> Open Circle -> Circle Feed.
- Circle post creation, comment flow, and global feed exclusion.
- Private/hidden/gated Circle read and join denial for non-eligible users.
- Invite acceptance for hidden/private/gated Circles.
- Circle Mention rendering for public, private, hidden, member, and non-member viewers.
- Composer autocomplete with public/member-visible Circle suggestions only.
- Post type/tag creation, Circle required-tag policies, Official Update denial for normal members, and disclaimer rendering.
- Question/Due-Diligence status changes, Official Answer marking, Archived Question locking, and answered-Q&A search priority.
- Circle Announcements remain separate from global Announcements.
- Circle AMAs/events are visible only to authorized Circle viewers.
- Circle Challenges increment only from Circle activity.
- New-member onboarding appears, persists, and hides after completion.
- Asset Circle lookup from an asset id returns the correct primary Circle and access state.
- Property and Portfolio Asset Circle CTAs render the correct access-state action and never expose hidden/nonexistent Circles.
- Holder-only Circle Resources are hidden without ownership and returned with current ownership.
- Circle Resource delivery endpoint rechecks access, does not expose raw storage paths in list responses, and streams private files with safe headers.
- Circle Resource Library creates link/path resources, uploads binary files, replaces the current file through a new binary version, opens historical versions, compares current versus candidate metadata, approves/rejects concrete versions with required rejection notes, restores historical versions as current, archives/restores resources, adds versions, hides storage paths, and records `circle.resource.*` audit entries. Current browser coverage validates file upload, replacement upload, delivery endpoint handling, version-history comparison rendering, reject-with-note persistence, restore-as-current persistence, and private-GCS stream access with safe headers and non-member denial.
- Circle Resource Lifecycle stores upload status, retention policy, review due/reviewed state, legal hold, soft-delete metadata, and audited lifecycle actions.
- Circle Resource Retention worker soft-deletes due `delete_after_expiry` resources, skips legal-hold resources, and writes immutable system audit rows.
- Circle Resource Object Cleanup worker deletes or confirms absent private GCS objects for soft-deleted non-legal-hold resources after the configured grace period, records retry state, and writes immutable audit rows.
- Private Investor Club defaults prevent public discovery/cross-post leakage.
- Circle Manage Settings load for owner/admin/moderator and deny non-manager users.
- Moderator-only Manage updates can change moderation controls but cannot change slug, privacy, content policy, or rules.
- Circle Manage updates produce `community_audit_logs` entries and Analytics/Ops tiles stay bounded under report/post/comment volume.
- Circle Report Queue shows only reports for posts in the managed Circle, denies non-manager access, requires CSRF/notes, hides posts through `hide_post`/`hide_posts`, and dismisses reports through `dismiss_report`/`dismiss_reports` without exposing global ban/warn powers.
- Circle Ops Snapshot worker materializes daily analytics, dedupes open report/SLA alerts, and resolves alerts when conditions clear.
- Circle Ops Alerts can be acknowledged, resolved, or moved through a human workflow state by Circle managers only, with CSRF and audit logging. Current browser coverage validates manager workflow-state update from open to acknowledged with persisted `waiting_on_policy` metadata.
- Platform-admin Circle Ops Alerts list filters scoped and global alerts, includes failed-worker alerts, writes transactional audit entries for acknowledge/resolve/assign/escalate/snooze/unsnooze/on-call-marker/workflow actions, and queues transactional-email fan-out for escalation/on-call actions. Current browser coverage validates workflow state, assignment, escalation, snooze, on-call marker, and resolve actions with persisted DB state.
- Circle Ops Alert Delivery Monitor mirrors `community_ops_alert_on_call` core outbox state, surfaces stale/exhausted/missing delivery through global `notification_delivery` alerts, and resolves that alert when delivery recovers.

Next implementation step: run `scripts/local-live/community-circles-staging-preflight.sh .env.local-live` with real staging Slack/PagerDuty secrets, seed one synthetic alert with `scripts/local-live/community-circles-seed-provider-alert.sh`, verify it with `scripts/local-live/community-circles-provider-receipt-check.sh`, document real provider event IDs in `docs/community-circles-provider-runbook.md` evidence format, and complete the legal/compliance checklist.
