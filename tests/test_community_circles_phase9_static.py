from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_phase9_schema_adds_manage_moderation_analytics_and_ops_primitives():
    migration = read("database/community/055_circle_manage_ops.sql")
    dedupe_migration = read("database/community/057_circle_ops_alert_dedupe.sql")
    global_alert_migration = read("database/community/058_circle_global_failed_worker_alert.sql")
    workflow_migration = read("database/community/067_circle_ops_alert_workflow_states.sql")

    for token in [
        "ADD COLUMN IF NOT EXISTS rules_text TEXT",
        "ADD COLUMN IF NOT EXISTS investment_disclaimer TEXT",
        "ADD COLUMN IF NOT EXISTS first_post_approval_enabled BOOLEAN NOT NULL DEFAULT FALSE",
        "ADD COLUMN IF NOT EXISTS slow_mode_seconds INTEGER NOT NULL DEFAULT 0",
        "ADD COLUMN IF NOT EXISTS blocked_words TEXT[] NOT NULL DEFAULT '{}'::TEXT[]",
        "ADD COLUMN IF NOT EXISTS investment_risk_keywords TEXT[]",
        "CREATE TABLE IF NOT EXISTS circle_daily_analytics",
        "CREATE TABLE IF NOT EXISTS circle_ops_alerts",
        "'report_backlog'",
        "'moderation_sla'",
    ]:
        assert token in migration

    for token in [
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_circle_ops_alerts_open_unique",
        "ON circle_ops_alerts (circle_id, alert_type)",
        "WHERE status = 'open' AND circle_id IS NOT NULL",
        "Prevents duplicate open Circle ops alerts",
    ]:
        assert token in dedupe_migration

    for token in [
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_circle_ops_alerts_global_open_unique",
        "ON circle_ops_alerts (alert_type)",
        "WHERE status = 'open' AND circle_id IS NULL",
        "failed_worker",
    ]:
        assert token in global_alert_migration

    for token in [
        "ADD COLUMN IF NOT EXISTS workflow_state VARCHAR(40) NOT NULL DEFAULT 'triage'",
        "ADD COLUMN IF NOT EXISTS workflow_note TEXT",
        "ADD COLUMN IF NOT EXISTS workflow_updated_at TIMESTAMPTZ",
        "ADD COLUMN IF NOT EXISTS workflow_updated_by UUID",
        "circle_ops_alerts_workflow_state_allowed",
        "'waiting_on_moderator'",
        "'waiting_on_policy'",
        "idx_circle_ops_alerts_workflow_active",
        "Human SLA workflow state",
    ]:
        assert token in workflow_migration


def test_phase9_backend_exposes_role_gated_manage_and_analytics_contracts():
    routes = read("backend/src/community/routes.rs")

    for token in [
        "async fn ensure_circle_manage_access",
        '"community.manage"',
        "is_circle_manager_role",
        "struct CircleManageSettingsReq",
        "async fn get_circle_manage_summary",
        "async fn update_circle_manage_settings",
        "async fn get_circle_analytics",
        "async fn get_circle_ops_alerts",
        "async fn take_circle_ops_alert_action",
        '"/api/community/circles/:id/manage"',
        '"/api/community/circles/:id/analytics"',
        '"/api/community/circles/:id/ops-alerts"',
        '"/api/community/circles/:id/ops-alerts/:alert_id/action"',
        "require_csrf_header",
        "circle.manage.update",
        "Moderators can update moderation controls",
        "Only the Circle owner or platform admin can change the Circle slug.",
        "content_reports cr",
        "community_audit_logs",
        "report_backlog_status",
        "circle.ops_alert.",
        "workflow_state: Option<String>",
        "normalize_ops_alert_workflow_state",
        '"set_workflow_state"',
        "Circle ops alert action must be acknowledge, resolve, or set_workflow_state.",
        "workflow_state must be triage, investigating, waiting_on_moderator, waiting_on_policy, mitigated, or monitoring.",
    ]:
        assert token in routes


def test_phase9_settings_ui_has_modular_manage_sections_and_save_contract():
    page = read("frontend/platform/community-circle-settings.html")
    js = read("frontend/platform/static/js/community-circle-settings.js")
    css = read("frontend/platform/static/css/community.css")

    for token in [
        "Content Settings",
        "Moderation",
        "Rules & Disclaimer",
        "Analytics & Ops",
        "ccs-input-first-post-approval",
        "ccs-input-risk-keywords",
        "ccs-analytics-grid",
        "ccs-ops-alerts",
        "ccs-audit-log",
    ]:
        assert token in page
    assert "Remove banner" not in page
    assert 'id="ccs-banner-clear"' not in page
    assert "Allowed post types" not in page
    assert 'id="ccs-post-types"' not in page

    for token in [
        "ccs-requests-card",
        "ccs-requests-list",
        "No pending join requests.",
    ]:
        assert token in page

    danger_card = page[
        page.index('id="ccs-danger-card"') : page.index("</section>", page.index('id="ccs-danger-card"'))
    ]
    assert "ccs-card__icon--blue" in danger_card
    assert "ccs-card__icon--danger" not in danger_card
    assert "ccs-btn-brand" in danger_card

    for token in [
        "loadManageSummary",
        "/api/community/circles/' + STATE.circle.id + '/manage",
        "renderAnalytics",
        "loadOpsAlerts",
        "renderOpsAlerts",
        "runOpsAlertAction",
        "promptOpsAlertWorkflowState",
        "data-ccs-alert-action",
        'data-ccs-alert-action="set_workflow_state"',
        "/api/community/circles/' + STATE.circle.id + '/ops-alerts",
        "parseList",
        "required_post_tags",
        "first_post_approval_enabled",
        "investment_risk_keywords",
    ]:
        assert token in js
    assert "readPostTypes" not in js
    assert "setPostTypes" not in js
    assert "allowed_post_types:" not in js
    assert "ccs-banner-clear" not in js

    for token in [
        "Loading join requests...",
        "No pending join requests.",
        "Failed to load join requests.",
    ]:
        assert token in js
    assert "card.hidden = reqs.length === 0" not in js
    assert "navR.hidden = reqs.length === 0" not in js
    assert "if (!STATE.circle || STATE.circle.is_public)" not in js

    for token in [
        ".ccs-check-grid",
        ".ccs-toggle-stack",
        ".ccs-analytics-grid",
        ".ccs-ops-alert-row",
        ".ccs-ops-alert-row__actions",
        ".ccs-audit-log__row",
        ".ccs-resource-form select.ds-input",
        ".ccs-resource-form input[type=\"file\"].ds-input::file-selector-button",
        ".ccs-btn-brand",
    ]:
        assert token in css

    card_head_block = css[
        css.index(".ccs-card__head {") : css.index(".ccs-card__title")
    ]
    assert "border-bottom: 1px solid var(--card-border-color, #E5E7EB);" in card_head_block
    assert "padding-bottom: 16px;" in card_head_block

    checkbox_block = css[
        css.index(".ccs-check-grid input,") : css.index(".ccs-toggle-stack")
    ]
    assert "appearance: none;" in checkbox_block
    assert "background-color: var(--btn-primary-bg, #0000FF);" in checkbox_block
    assert "%2303FF88" in checkbox_block

    danger_block = css[
        css.index(".ccs-card--danger {") : css.index(".ccs-card--danger .ccs-card__title")
    ]
    assert "display: grid;" in danger_block
    assert "grid-template-columns: minmax(0, 1fr) auto;" in danger_block
    assert "align-items: center;" in danger_block

    danger_actions_block = css[
        css.index(".ccs-danger-actions {") : css.index(".ds-btn--danger")
    ]
    assert "justify-content: flex-end;" in danger_actions_block
    assert "@media (max-width: 640px)" in css


def test_phase9_report_queue_is_circle_scoped_and_manager_gated():
    routes = read("backend/src/community/routes.rs")
    page = read("frontend/platform/community-circle-settings.html")
    js = read("frontend/platform/static/js/community-circle-settings.js")
    css = read("frontend/platform/static/css/community.css")

    for token in [
        '"/api/community/circles/:id/reports"',
        '"/api/community/circles/:id/reports/bulk-action"',
        '"/api/community/circles/:id/reports/:report_id/action"',
        "async fn get_circle_report_queue",
        "async fn take_circle_report_action",
        "async fn take_circle_report_bulk_action",
        "struct CircleReportBulkActionReq",
        "report_ids: Vec<Uuid>",
        "ensure_circle_manage_access",
        "content_reports cr",
        "JOIN posts p ON p.id = cr.post_id",
        "WHERE p.circle_id = $1",
        "AND cr.status = 'pending'",
        "LIMIT 50",
        "require_csrf_header",
        '"hide_post" | "dismiss_report"',
        '"hide_posts" | "dismiss_reports"',
        "Circle report actions are limited to hide_post or dismiss_report.",
        "Circle bulk report action must be hide_posts or dismiss_reports.",
        "Bulk report actions are limited to 50 reports.",
        "cr.id = ANY($1)",
        "p.circle_id = $2",
        "FOR UPDATE OF cr, p",
        "WHERE id = ANY($3) AND status = 'pending'",
        "circle.report.bulk_hide_posts",
        "circle.report.bulk_dismiss_reports",
        "'circle_report_bulk_action'",
        "Moderation notes are required.",
        "Report not found for this Circle.",
        "service::action_on_report",
    ]:
        assert token in routes

    for token in [
        "Report Queue",
        "ccs-reports-card",
        "ccs-reports-list",
        "Global bans remain platform-admin only.",
    ]:
        assert token in page
    report_card_header = page[
        page.index('id="ccs-reports-card"') : page.index('<div id="ccs-reports-list"')
    ]
    assert "ccs-card__icon--blue" in report_card_header
    assert "ccs-card__icon--danger" not in report_card_header

    for token in [
        "loadCircleReports",
        "bulkReportToolbar",
        "reportRow",
        "runReportAction",
        "runBulkReportAction",
        "selectedReportIds",
        "/api/community/circles/' + STATE.circle.id + '/reports",
        "/api/community/circles/' + STATE.circle.id + '/reports/bulk-action",
        "data-ccs-report-action",
        "data-ccs-report-bulk-action",
        "data-ccs-report-select",
        "hide_post",
        "dismiss_report",
        "hide_posts",
        "dismiss_reports",
        "circle.report_moderated",
        "circle.reports_bulk_moderated",
        "Moderation notes are required.",
    ]:
        assert token in js

    for token in [
        ".ccs-report-list",
        ".ccs-report-bulk",
        ".ccs-report-bulk__actions",
        ".ccs-report-row",
        ".ccs-report-row__select",
        ".ccs-report-row__body",
        ".ccs-report-row__meta",
        ".ccs-report-row__content",
        ".ccs-report-row__note",
        ".ccs-report-row__actions",
    ]:
        assert token in css


def test_phase9_ops_snapshot_worker_materializes_analytics_and_alerts():
    background = read("backend/src/community/background.rs")
    lib = read("backend/src/lib.rs")

    for token in [
        "pub struct CircleOpsSnapshotSummary",
        "pub async fn circle_ops_snapshot_worker",
        "POOOL_CIRCLE_OPS_SNAPSHOT_SECS",
        "pub async fn run_circle_ops_snapshot_once",
        "async fn upsert_circle_daily_analytics",
        "async fn refresh_circle_report_backlog_alerts",
        "async fn refresh_circle_moderation_sla_alerts",
        "async fn enqueue_auto_critical_circle_ops_alert_notifications_once",
        "auto_critical_notifications_enqueued",
        "circle_daily_analytics",
        "ON CONFLICT (circle_id, snapshot_date) DO UPDATE",
        "circle_ops_alerts",
        "circle_ops_alert_notifications",
        "ON CONFLICT (circle_id, alert_type) WHERE status = 'open'",
        "'report_backlog'",
        "'moderation_sla'",
        "POOOL_CIRCLE_REPORT_BACKLOG_WARNING",
        "POOOL_CIRCLE_MODERATION_SLA_HOURS",
        "POOOL_CIRCLE_OPS_AUTO_CRITICAL_COOLDOWN_HOURS",
        "'auto_critical'",
        "alert.status = 'open'",
        "alert.severity = 'critical'",
        "alert.alert_type IN ('report_backlog', 'moderation_sla')",
        "alert.snoozed_until IS NULL OR alert.snoozed_until <= NOW()",
        "existing.created_at >= NOW() - ($1::INT * INTERVAL '1 hour')",
        "status = 'resolved', resolved_at = NOW()",
        "async fn upsert_circle_failed_worker_alert",
        "async fn resolve_circle_failed_worker_alert",
        "'failed_worker'",
        "consecutive_failures >= 3",
    ]:
        assert token in background

    assert "community::background::circle_ops_snapshot_worker" in lib


def test_phase9_platform_admin_circle_ops_alert_overview_is_available():
    routes = read("backend/src/community/routes.rs")
    page = read("frontend/platform/admin/community/circles.html")
    js = read("frontend/platform/static/js/admin-community-circles.js")

    for token in [
        '"/api/admin/community/ops-alerts"',
        '"/api/admin/community/ops-alerts/:id/action"',
        "struct AdminCircleOpsAlertQuery",
        "struct AdminCircleOpsAlertActionReq",
        "async fn admin_list_circle_ops_alerts",
        "async fn admin_take_circle_ops_alert_action",
        "normalize_admin_ops_alert_status",
        "normalize_admin_ops_alert_severity",
        "normalize_admin_ops_alert_type",
        "LEFT JOIN circles c ON c.id = a.circle_id",
        "circle_name",
        "circle_slug",
        "failed_worker_active_count",
        "require_csrf_header",
        "platform.circle_ops_alert.",
        "log_community_admin_action_tx",
        "status must be active, open, acknowledged, resolved, or all",
    ]:
        assert token in routes

    for token in [
        "Circle Ops Alerts",
        "circle-alerts-filter-form",
        "circle-alerts-status-filter",
        "circle-alerts-severity-filter",
        "circle-alerts-type-filter",
        "circle-alerts-open-count",
        "circle-alerts-critical-count",
        "circle-alerts-acknowledged-count",
        "circle-alerts-failed-worker-count",
        "circle-alerts-table-body",
        "circle-alert-action-modal",
        "circle-alert-action-note",
    ]:
        assert token in page

    for token in [
        "loadCircleOpsAlerts",
        "buildAlertsUrl",
        "/api/admin/community/ops-alerts",
        "data-alert-action",
        "openAlertActionModal",
        "confirmAlertAction",
        "getCsrfToken",
        "circle-alerts-status-filter",
        "circle-alerts-severity-filter",
        "circle-alerts-type-filter",
        "Platform-wide",
        "failed_worker_active_count",
    ]:
        assert token in js


def test_phase9_platform_admin_ops_alert_escalation_foundation_is_available():
    migration = read("database/community/060_circle_ops_alert_escalation.sql")
    routes = read("backend/src/community/routes.rs")
    page = read("frontend/platform/admin/community/circles.html")
    js = read("frontend/platform/static/js/admin-community-circles.js")

    for token in [
        "ADD COLUMN IF NOT EXISTS assigned_to_user_id UUID",
        "ADD COLUMN IF NOT EXISTS escalation_level INTEGER NOT NULL DEFAULT 0",
        "CHECK (escalation_level BETWEEN 0 AND 5)",
        "ADD COLUMN IF NOT EXISTS escalated_at TIMESTAMPTZ",
        "ADD COLUMN IF NOT EXISTS snoozed_until TIMESTAMPTZ",
        "ADD COLUMN IF NOT EXISTS escalation_note TEXT",
        "ADD COLUMN IF NOT EXISTS on_call_notified_at TIMESTAMPTZ",
        "idx_circle_ops_alerts_escalation_queue",
        "idx_circle_ops_alerts_assigned_active",
        "idx_circle_ops_alerts_snoozed_active",
        "Platform operator user assigned as current owner",
        "no external fan-out is performed",
    ]:
        assert token in migration

    for token in [
        "assigned_to_user_id: Option<Uuid>",
        "snooze_minutes: Option<i64>",
        '"assign"',
        '"escalate"',
        '"snooze"',
        '"unsnooze"',
        '"mark_on_call_notified"',
        '"set_workflow_state"',
        "assigned_to_user_id is required for assign actions.",
        "snooze_minutes must be between 5 and 10080.",
        "workflow_state: Option<String>",
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND status <> 'deleted')",
        "assigned_to_user_id",
        "escalation_level",
        "escalated_at",
        "snoozed_until",
        "escalation_note",
        "on_call_notified_at",
        "workflow_state",
        "workflow_note",
        "workflow_updated_at",
        "workflow_updated_by",
        "blocked_workflow_count",
        "escalated_active_count",
        "snoozed_active_count",
        "LEAST(escalation_level + 1, 5)",
        "NOW() + ($2::INT * INTERVAL '1 minute')",
        "platform.circle_ops_alert.",
    ]:
        assert token in routes

    for token in [
        "circle-alerts-escalated-count",
        "circle-alerts-snoozed-count",
        "circle-alerts-blocked-workflow-count",
        "circle-alert-action-assignee",
        "circle-alert-action-snooze-minutes",
        "circle-alert-action-workflow-state",
    ]:
        assert token in page

    for token in [
        "mark_on_call_notified",
        "snoozed_active_count",
        "escalated_active_count",
        "blocked_workflow_count",
        "assigned_to_user_id",
        "snooze_minutes",
        "workflow_state",
        "isFutureDate",
        "formatShortId",
        "formatActionPastTense",
        'appendActionButton("assign", "Assign")',
        'appendActionButton("escalate", "Escalate")',
        'appendActionButton("set_workflow_state", "Workflow")',
        "circle-alert-action-assignee-group",
        "circle-alert-action-snooze-group",
        "circle-alert-action-workflow-group",
    ]:
        assert token in js


def test_phase9_ops_alert_fanout_foundation_bridges_to_email_outbox():
    migration = read("database/community/061_circle_ops_alert_fanout.sql")
    external_migration = read("database/community/065_circle_ops_alert_external_fanout.sql")
    background = read("backend/src/community/background.rs")
    routes = read("backend/src/community/routes.rs")
    lib = read("backend/src/lib.rs")

    for token in [
        "CREATE TABLE IF NOT EXISTS circle_ops_alert_notifications",
        "REFERENCES circle_ops_alerts(id) ON DELETE CASCADE",
        "CHECK (trigger_action IN ('auto_critical', 'escalate', 'mark_on_call_notified'))",
        "CHECK (status IN ('queued', 'sending', 'enqueued', 'skipped', 'failed'))",
        "enqueued_email_outbox_id UUID",
        "idx_circle_ops_alert_notifications_ready",
        "idx_circle_ops_alert_notifications_active_unique",
        "Worker copies ready rows into core transactional_email_outbox",
    ]:
        assert token in migration

    for token in [
        "DROP CONSTRAINT IF EXISTS circle_ops_alert_notifications_channel_check",
        "CHECK (channel IN ('email', 'slack', 'pagerduty'))",
        "provider_response_status INTEGER",
        "provider_response_at TIMESTAMPTZ",
        "idx_circle_ops_alert_notifications_external_ready",
    ]:
        assert token in external_migration

    for token in [
        "pub struct CircleOpsAlertFanoutSummary",
        "pub async fn enqueue_circle_ops_alert_notification_tx",
        "circle_ops_alert_notifications",
        "ON CONFLICT (",
        "pub async fn circle_ops_alert_fanout_worker",
        "POOOL_CIRCLE_OPS_ALERT_FANOUT_SECS",
        "pub async fn process_circle_ops_alert_fanout_once",
        "FOR UPDATE SKIP LOCKED",
        "bridge_circle_ops_alert_notification",
        "bridge_circle_ops_alert_external_webhook",
        "build_circle_ops_alert_webhook_request",
        "optional_circle_ops_alert_webhook_url",
        "normalize_circle_ops_alert_webhook_url",
        "is_local_fallback_allowed()",
        "Circle ops alert webhook URL must use HTTPS outside local development.",
        "Circle ops alert webhook URL must not contain credentials or fragments.",
        "POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL",
        "POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY",
        "POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL",
        "channel.as_str()",
        '"slack" | "pagerduty"',
        "reqwest::Client::builder()",
        "redirect(reqwest::redirect::Policy::none())",
        "provider_response_status",
        "provider_response_at",
        "slack_sent",
        "pagerduty_sent",
        '"https://events.pagerduty.com/v2/enqueue"',
        "resolve_circle_ops_alert_notification_recipient",
        "transactional_email_outbox",
        "community_ops_alert_on_call",
        "SELECT id, email FROM users WHERE id = $1 AND status <> 'deleted'",
        "admin@poool.app",
        "mark_circle_ops_alert_notification_failed",
        "next_attempt_at = NOW() + ($3::INT * INTERVAL '1 second')",
        "render_circle_ops_alert_email",
        "escape_email_html",
    ]:
        assert token in background

    for token in [
        "fanout_queued",
        'matches!(action.as_str(), "escalate" | "mark_on_call_notified")',
        "enqueue_circle_ops_alert_notification_tx",
        '"trigger_action": action',
        '"assigned_to_user_id": updated_assignee',
    ]:
        assert token in routes

    assert "community::background::circle_ops_alert_fanout_worker" in lib


def test_phase9_ops_alert_delivery_monitor_tracks_core_email_status():
    migration = read("database/community/064_circle_ops_alert_delivery_monitoring.sql")
    background = read("backend/src/community/background.rs")
    routes = read("backend/src/community/routes.rs")
    admin_page = read("frontend/platform/admin/community/circles.html")
    lib = read("backend/src/lib.rs")

    for token in [
        "email_outbox_status TEXT",
        "'missing'",
        "email_outbox_attempts INTEGER",
        "email_outbox_last_error TEXT",
        "email_outbox_sent_at TIMESTAMPTZ",
        "delivery_checked_at TIMESTAMPTZ",
        "delivery_alerted_at TIMESTAMPTZ",
        "idx_circle_ops_alert_notifications_delivery_monitor",
        "idx_circle_ops_alert_notifications_delivery_unhealthy",
        "'notification_delivery'",
    ]:
        assert token in migration

    for token in [
        "CircleOpsAlertDeliveryMonitorSummary",
        "circle_ops_alert_delivery_monitor_worker",
        "POOOL_CIRCLE_OPS_ALERT_DELIVERY_MONITOR_SECS",
        "monitor_circle_ops_alert_delivery_once",
        "POOOL_CIRCLE_OPS_ALERT_DELIVERY_PENDING_MINUTES",
        "POOOL_CIRCLE_OPS_ALERT_DELIVERY_FAILURE_ATTEMPTS",
        "transactional_email_outbox",
        "community_ops_alert_on_call",
        "update_circle_ops_alert_delivery_state",
        "refresh_circle_ops_alert_delivery_health",
        "'notification_delivery'",
        "Circle ops alert email delivery is unhealthy",
        "email_outbox_status = 'missing'",
        "delivery_recovered_at",
    ]:
        assert token in background

    for token in [
        "notification_delivery",
        "alert_type must be report_backlog, spam_spike, failed_worker, posting_spike, moderation_sla, notification_delivery, or all",
    ]:
        assert token in routes

    assert '<option value="notification_delivery">Notification delivery</option>' in admin_page
    assert "community::background::circle_ops_alert_delivery_monitor_worker" in lib
