use crate::auth::routes::AppState;

/// RBAC and invitation management for admin users.
pub mod access;
pub mod extractors;

/// Module
pub mod approvals;
/// Module
pub mod assets;
/// Module
pub mod audit;
/// Blockchain administration — tokenization, treasury, emergency controls.
#[allow(missing_docs)]
pub mod blockchain;
/// Compose all admin-domain routes into a single mountable [`Router`].
///
/// Covers:
/// - HTML pages under `/admin/*`
/// - All JSON API endpoints under `/api/admin/*`
///
/// Module
pub mod dashboard;
/// Module
pub mod deposits;
/// Module
pub mod developer_projects;
/// Module
pub mod emails;
/// Module
pub mod kyc;
/// Module
pub mod legal;
/// Admin marketplace section — trading oversight, orderbooks, trades, kill-switch.
pub mod marketplace;
/// Module
pub mod notifications;
/// Primary Offering Escrow Management module
pub mod primary_escrow;

/// Module
pub mod orders;
/// Module
pub mod pages;
/// Module
pub mod reports;
/// Module
pub mod rewards;
/// Module
pub mod settings;
/// Module
pub mod storage;
/// Module
pub mod submissions;
/// Module
pub mod support;
/// Module
pub mod system;
/// Module
pub mod treasury;
/// Module
pub mod users;
/// Module
pub mod withdrawals;

pub use approvals::*;
pub use assets::*;
pub use audit::*;
pub use blockchain::*;
pub use dashboard::*;
pub use deposits::*;
pub use developer_projects::*;
pub use emails::*;
pub use kyc::*;
pub use legal::*;
pub use marketplace::*;
pub use notifications::*;
pub use orders::*;
pub use pages::*;
pub use reports::*;
pub use rewards::*;
pub use settings::*;
pub use storage::*;
pub use submissions::*;
pub use support::*;
pub use system::*;
pub use treasury::*;
pub use users::*;
pub use withdrawals::*;

/// Router function for admin API endpoints.
pub fn router() -> axum::Router<AppState> {
    use axum::routing::{delete, get, patch, post, put};
    #[allow(unused_mut)]
    let mut r = axum::Router::new()
        // ── HTML Pages ──────────────────────────────────────────
        .route("/admin/", get(page_admin_dashboard))
        .route("/admin/index.html", get(page_admin_dashboard))
        .route("/admin/blog.html", get(page_admin_blog))
        .route("/admin/blog", get(page_admin_blog))
        .route("/admin/blog-persona.html", get(page_admin_blog_persona))
        .route("/admin/blog-persona", get(page_admin_blog_persona))
        .route("/admin/blog-strategy.html", get(page_admin_blog_strategy))
        .route("/admin/blog-strategy", get(page_admin_blog_strategy))
        .route("/admin/blog-editor.html", get(page_admin_blog_editor))
        .route("/admin/blog-editor", get(page_admin_blog_editor))
        .route("/admin/users.html", get(page_admin_generic))
        .route("/admin/users", get(page_admin_generic))
        .route("/admin/user-details.html", get(page_admin_generic))
        .route("/admin/user-details", get(page_admin_generic))
        .route("/admin/kyc.html", get(page_admin_generic))
        .route("/admin/kyc", get(page_admin_generic))
        .route("/admin/support.html", get(page_admin_generic))
        .route("/admin/support", get(page_admin_generic))
        .route("/admin/developer-submissions.html", get(page_admin_generic))
        .route("/admin/developer-submissions", get(page_admin_generic))
        .route("/admin/asset-change-requests.html", get(page_admin_generic))
        .route("/admin/asset-change-requests", get(page_admin_generic))
        .route("/admin/assets.html", get(page_admin_generic))
        .route("/admin/assets", get(page_admin_generic))
        .route("/admin/orders.html", get(page_admin_generic))
        .route("/admin/orders", get(page_admin_generic))
        .route("/admin/deposits.html", get(page_admin_deposits))
        .route("/admin/deposits", get(page_admin_deposits))
        .route("/admin/treasury.html", get(page_admin_generic))
        .route("/admin/treasury", get(page_admin_generic))
        .route("/admin/rewards.html", get(page_admin_generic))
        .route("/admin/rewards", get(page_admin_generic))
        .route("/admin/audit-logs.html", get(page_admin_audit_logs))
        .route("/admin/audit-logs", get(page_admin_audit_logs))
        .route("/admin/reports.html", get(page_admin_generic))
        .route("/admin/reports", get(page_admin_generic))
        .route("/admin/notifications.html", get(page_admin_generic))
        .route("/admin/notifications", get(page_admin_generic))
        .route("/admin/settings.html", get(page_admin_generic))
        .route("/admin/settings", get(page_admin_generic))
        .route("/admin/system.html", get(page_admin_generic))
        .route("/admin/system", get(page_admin_generic))
        .route("/admin/storage.html", get(page_admin_generic))
        .route("/admin/storage", get(page_admin_generic))
        .route("/admin/templates/icons.html", get(page_admin_generic))
        .route("/admin/templates/icons", get(page_admin_generic))
        .route(
            "/admin/developer-submission-review.html",
            get(page_admin_generic),
        )
        .route(
            "/admin/developer-submission-review",
            get(page_admin_generic),
        )
        .route("/admin/asset-details.html", get(page_admin_generic))
        .route("/admin/asset-details", get(page_admin_generic))
        .route("/admin/dividends.html", get(page_admin_generic))
        .route("/admin/dividends", get(page_admin_generic))
        .route("/admin/admins.html", get(page_admin_generic))
        .route("/admin/admins", get(page_admin_generic))
        .route("/admin/email-marketing.html", get(page_admin_generic))
        .route("/admin/email-marketing", get(page_admin_generic))
        .route("/admin/roles.html", get(page_admin_generic))
        .route("/admin/roles", get(page_admin_generic))
        .route("/admin/support-ticket.html", get(page_admin_generic))
        .route("/admin/support-ticket", get(page_admin_generic))
        .route("/admin/approvals.html", get(page_admin_generic))
        .route("/admin/approvals", get(page_admin_generic))
        // ── Admin Community Pages ─────────────────────────────────
        .route("/admin/community/", get(page_admin_generic))
        .route("/admin/community/index.html", get(page_admin_generic))
        .route("/admin/community/amas.html", get(page_admin_community_amas))
        .route("/admin/community/amas", get(page_admin_community_amas))
        .route(
            "/admin/community/announcements.html",
            get(page_admin_community_announcements),
        )
        .route(
            "/admin/community/announcements",
            get(page_admin_community_announcements),
        )
        .route("/admin/community/badges.html", get(page_admin_generic))
        .route("/admin/community/badges", get(page_admin_generic))
        .route(
            "/admin/community/challenges.html",
            get(page_admin_community_challenges),
        )
        .route(
            "/admin/community/challenges",
            get(page_admin_community_challenges),
        )
        .route(
            "/admin/community/circle-detail.html",
            get(page_admin_generic),
        )
        .route("/admin/community/circle-detail", get(page_admin_generic))
        .route(
            "/admin/community/circles.html",
            get(page_admin_community_circles),
        )
        .route(
            "/admin/community/circles",
            get(page_admin_community_circles),
        )
        .route("/admin/community/comments.html", get(page_admin_generic))
        .route("/admin/community/comments", get(page_admin_generic))
        .route("/admin/community/leaderboard.html", get(page_admin_generic))
        .route("/admin/community/leaderboard", get(page_admin_generic))
        .route("/admin/community/post-detail.html", get(page_admin_generic))
        .route("/admin/community/post-detail", get(page_admin_generic))
        .route("/admin/community/posts.html", get(page_admin_generic))
        .route("/admin/community/posts", get(page_admin_generic))
        .route("/admin/community/reports.html", get(page_admin_generic))
        .route("/admin/community/reports", get(page_admin_generic))
        .route("/admin/community/user-detail.html", get(page_admin_generic))
        .route("/admin/community/user-detail", get(page_admin_generic))
        .route("/admin/community/users.html", get(page_admin_generic))
        .route("/admin/community/users", get(page_admin_generic))
        // ── Admin Affiliate Pages ─────────────────────────────────
        .route(
            "/admin/affiliate-applications.html",
            get(page_admin_generic),
        )
        .route("/admin/affiliate-applications", get(page_admin_generic))
        .route("/admin/affiliate-finance.html", get(page_admin_generic))
        .route("/admin/affiliate-finance", get(page_admin_generic))
        .route("/admin/admin-affiliate-fraud.html", get(page_admin_generic))
        .route("/admin/admin-affiliate-fraud", get(page_admin_generic))
        .route("/admin/affiliate-fraud.html", get(page_admin_generic))
        .route("/admin/affiliate-fraud", get(page_admin_generic))
        .route(
            "/admin/affiliate-compliance.html",
            get(page_admin_affiliate_compliance_redirect),
        )
        .route(
            "/admin/affiliate-compliance",
            get(page_admin_affiliate_compliance_redirect),
        )
        // ── Admin Blockchain Pages ───────────────────────────────
        .route("/admin/asset-tokenize.html", get(page_admin_generic))
        .route("/admin/asset-tokenize", get(page_admin_generic))
        .route("/admin/blockchain-treasury.html", get(page_admin_generic))
        .route("/admin/blockchain-treasury", get(page_admin_generic))
        .route("/admin/pending-settlements.html", get(page_admin_generic))
        .route("/admin/pending-settlements", get(page_admin_generic))
        .route("/admin/blockchain-contracts.html", get(page_admin_generic))
        .route("/admin/blockchain-contracts", get(page_admin_generic))
        .route(
            "/admin/blockchain-contract-detail.html",
            get(page_admin_generic),
        )
        .route("/admin/blockchain-contract-detail", get(page_admin_generic))
        .route("/admin/blockchain-sync.html", get(page_admin_generic))
        .route("/admin/blockchain-sync", get(page_admin_generic))
        // ── Admin Marketplace Pages ──────────────────────────────
        .route("/admin/marketplace/", get(page_admin_generic))
        .route("/admin/marketplace/index.html", get(page_admin_generic))
        .route("/admin/marketplace/orderbook.html", get(page_admin_generic))
        .route("/admin/marketplace/orderbook", get(page_admin_generic))
        .route("/admin/marketplace/trades.html", get(page_admin_generic))
        .route("/admin/marketplace/trades", get(page_admin_generic))
        .route("/admin/marketplace/orders.html", get(page_admin_generic))
        .route("/admin/marketplace/orders", get(page_admin_generic))
        .route("/admin/marketplace/approvals.html", get(page_admin_generic))
        .route("/admin/marketplace/approvals", get(page_admin_generic))
        .route(
            "/admin/marketplace/primary-escrow.html",
            get(page_admin_generic),
        )
        .route("/admin/marketplace/primary-escrow", get(page_admin_generic))
        .route(
            "/admin/marketplace/reconciliation.html",
            get(page_admin_generic),
        )
        .route("/admin/marketplace/reconciliation", get(page_admin_generic))
        .route("/admin/marketplace/fees.html", get(page_admin_generic))
        .route("/admin/marketplace/fees", get(page_admin_generic))
        .route("/admin/marketplace/settings.html", get(page_admin_generic))
        .route("/admin/marketplace/settings", get(page_admin_generic))
        .route("/admin/marketplace/p2p.html", get(page_admin_generic))
        .route("/admin/marketplace/p2p", get(page_admin_generic))
        .route("/admin/marketplace/analytics.html", get(page_admin_generic))
        .route("/admin/marketplace/analytics", get(page_admin_generic))
        .route("/admin/marketplace/alerts.html", get(page_admin_generic))
        .route("/admin/marketplace/alerts", get(page_admin_generic))
        .route(
            "/admin/marketplace/compliance.html",
            get(page_admin_marketplace_compliance),
        )
        .route(
            "/admin/marketplace/compliance",
            get(page_admin_marketplace_compliance),
        )
        // ── JSON API ─────────────────────────────────────────────
        .route("/api/admin/stats/overview", get(api_admin_stats_overview))
        .route("/api/admin/search", get(api_admin_search))
        // Access / RBAC
        .route("/api/admin/admins", get(access::api_admin_list))
        .route(
            "/api/admin/roles",
            get(access::api_roles_list).post(access::api_roles_create),
        )
        .route(
            "/api/admin/roles/permissions",
            post(access::api_roles_update_permissions),
        )
        .route("/api/admin/permissions", get(access::api_permissions_list))
        .route(
            "/api/admin/blog/overview",
            get(crate::blog::routes::admin_blog_overview),
        )
        .route(
            "/api/admin/blog/articles",
            get(crate::blog::routes::admin_blog_articles)
                .post(crate::blog::routes::admin_blog_create_article),
        )
        .route(
            "/api/admin/blog/articles/:id",
            get(crate::blog::routes::admin_blog_get_article)
                .put(crate::blog::routes::admin_blog_update_article),
        )
        .route(
            "/api/admin/blog/articles/:id/publish",
            post(crate::blog::routes::admin_blog_publish_article),
        )
        .route(
            "/api/admin/blog/articles/:id/unpublish",
            post(crate::blog::routes::admin_blog_unpublish_article),
        )
        .route(
            "/api/admin/blog/articles/:id/archive",
            post(crate::blog::routes::admin_blog_archive_article),
        )
        .route(
            "/api/admin/blog/articles/:id/restore",
            post(crate::blog::routes::admin_blog_restore_article),
        )
        .route(
            "/api/admin/blog/assets",
            post(crate::blog::routes::admin_blog_upload_asset),
        )
        .route(
            "/api/admin/blog/authors",
            get(crate::blog::routes::admin_blog_list_authors)
                .post(crate::blog::routes::admin_blog_save_author),
        )
        .route(
            "/api/admin/blog/categories",
            get(crate::blog::routes::admin_blog_list_categories)
                .post(crate::blog::routes::admin_blog_save_category),
        )
        .route(
            "/api/admin/blog/import/db-to-sanity/dry-run",
            post(crate::blog::routes::admin_blog_import_dry_run),
        )
        .route(
            "/api/admin/blog/import/db-to-sanity",
            post(crate::blog::routes::admin_blog_import_run),
        )
        .route("/api/admin/admins/invite", post(access::api_admin_invite))
        .route(
            "/api/admin/admins/invitations",
            get(access::api_admin_invitations_list),
        )
        .route(
            "/api/admin/admins/invitations/:id",
            delete(access::api_admin_invitation_revoke),
        )
        .route(
            "/api/admin/admins/invitations/:id/resend",
            post(access::api_admin_invitation_resend),
        )
        // Users
        .route("/api/admin/users", get(api_admin_users))
        .route("/api/admin/users/:user_id", get(api_admin_user_detail))
        .route(
            "/api/admin/users/:user_id/profile",
            post(api_admin_user_update_profile),
        )
        .route(
            "/api/admin/users/:user_id/balance",
            post(api_admin_user_update_balance),
        )
        .route(
            "/api/admin/users/:user_id/status",
            post(api_admin_user_update_status),
        )
        .route(
            "/api/admin/users/:user_id/sessions",
            delete(api_admin_user_revoke_sessions),
        )
        .route(
            "/api/admin/users/:user_id/force-password-reset",
            post(api_admin_user_force_password_reset),
        )
        .route(
            "/api/admin/users/:user_id/roles",
            post(api_admin_user_update_roles),
        )
        .route(
            "/api/admin/users/:user_id/investment-limit",
            post(api_admin_user_set_investment_limit),
        )
        // Deposits
        .route("/api/admin/deposits", get(api_admin_deposits))
        .route(
            "/api/admin/deposits/:tx_id/confirm",
            post(api_admin_deposit_confirm),
        )
        .route(
            "/api/admin/deposits/:tx_id/cancel",
            post(api_admin_deposit_cancel),
        )
        .route(
            "/api/admin/deposits/:tx_id/extend",
            post(api_admin_deposit_extend_expiry),
        )
        // Withdrawals
        .route("/api/admin/withdrawals", get(api_admin_withdrawals))
        .route(
            "/api/admin/withdrawals/:req_id/approve",
            post(api_admin_withdrawal_approve),
        )
        .route(
            "/api/admin/withdrawals/:req_id/reject",
            post(api_admin_withdrawal_reject),
        )
        // KYC
        .route("/api/admin/kyc", get(api_admin_kyc_records))
        .route(
            "/api/admin/kyc/:kyc_id/documents",
            get(api_admin_kyc_documents),
        )
        .route(
            "/api/admin/kyc/:kyc_id/approve",
            post(api_admin_kyc_approve),
        )
        .route("/api/admin/kyc/:kyc_id/reject", post(api_admin_kyc_reject))
        // Submissions
        .route("/api/admin/submissions", get(api_admin_submissions))
        .route(
            "/api/admin/submissions/:asset_id/approve",
            post(api_admin_submission_approve),
        )
        .route(
            "/api/admin/submissions/:asset_id/reject",
            post(api_admin_submission_reject),
        )
        .route(
            "/api/admin/submissions/:asset_id/detail",
            get(api_admin_submission_detail),
        )
        // Developer projects
        .route(
            "/api/admin/developer-projects",
            get(api_admin_developer_projects),
        )
        .route(
            "/api/admin/developer-projects/:id",
            get(api_admin_developer_project_detail),
        )
        .route(
            "/api/admin/developer-projects/:id/review",
            post(api_admin_developer_project_review),
        )
        .route(
            "/api/admin/developer-projects/:id/notes",
            get(api_admin_project_notes_list).post(api_admin_project_notes_create),
        )
        .route(
            "/api/admin/developer-projects/:id/checklist",
            get(api_admin_project_checklist_get).put(api_admin_project_checklist_save),
        )
        // Orders (read only; approve/reject are in payments::router())
        .route("/api/admin/orders", get(api_admin_orders))
        .route("/api/admin/orders/:id", get(api_admin_order_detail))
        // Investments
        .route("/api/admin/investments", get(api_admin_investments))
        // Assets
        .route("/api/admin/assets", get(api_admin_assets))
        .route(
            "/api/admin/assets/:asset_id/toggle-featured",
            post(api_admin_toggle_featured),
        )
        .route(
            "/api/admin/assets/:asset_id/publication",
            patch(api_admin_asset_publication),
        )
        .route(
            "/api/admin/assets/:asset_id/funding-status",
            patch(api_admin_asset_funding_status),
        )
        .route(
            "/api/admin/assets/:asset_id/detail",
            get(api_admin_asset_detail),
        )
        .route(
            "/api/admin/assets/:asset_id/images",
            post(api_admin_asset_image_upload),
        )
        .route(
            "/api/admin/assets/:asset_id/images/:image_id",
            delete(api_admin_asset_image_delete),
        )
        .route(
            "/api/admin/assets/:asset_id/images/reorder",
            put(api_admin_asset_images_reorder),
        )
        // Treasury & Rewards
        .route("/api/admin/treasury", get(api_admin_treasury))
        .route("/api/admin/rewards", get(api_admin_rewards))
        .route(
            "/api/admin/rewards/balances/:user_id/adjust",
            post(api_admin_rewards_balance_adjust),
        )
        .route("/api/admin/rewards/tiers", post(api_admin_tier_create))
        .route(
            "/api/admin/rewards/tiers/:tier_name",
            patch(api_admin_tier_update),
        )
        .route(
            "/api/admin/rewards/referrals/:ref_id",
            patch(api_admin_referral_update),
        )
        // Affiliates Onboarding & Payouts
        .route(
            "/api/admin/rewards/affiliates/pending",
            get(api_admin_affiliates_pending),
        )
        .route(
            "/api/admin/rewards/affiliates/fraud-scan",
            get(crate::admin::rewards::api_admin_affiliate_fraud_scan),
        )
        .route(
            "/api/admin/rewards/affiliates/:id/approve",
            post(api_admin_affiliate_approve),
        )
        .route(
            "/api/admin/rewards/affiliates/:id/reject",
            post(api_admin_affiliate_reject),
        )
        .route(
            "/api/admin/rewards/affiliates/:id/suspend",
            post(api_admin_affiliate_suspend),
        )
        .route(
            "/api/admin/rewards/affiliates/payouts/pending",
            get(api_admin_affiliate_payouts_pending),
        )
        .route(
            "/api/admin/rewards/affiliates/:id/payout",
            post(api_admin_affiliate_batch_payout),
        )
        .route(
            "/api/admin/rewards/affiliates/:id/clawback",
            post(api_admin_affiliate_clawback),
        )
        // GAP-11: Materials review board
        .route(
            "/api/admin/rewards/affiliates/materials",
            get(api_admin_affiliate_materials_list),
        )
        .route(
            "/api/admin/rewards/affiliates/materials/:id/review",
            post(api_admin_affiliate_material_review),
        )
        // Change Requests
        .route(
            "/api/admin/change-requests",
            get(crate::developer::change_requests::admin_list),
        )
        .route(
            "/api/admin/change-requests/:id",
            get(crate::developer::change_requests::admin_detail),
        )
        .route(
            "/api/admin/change-requests/:id/approve",
            post(crate::developer::change_requests::admin_approve),
        )
        .route(
            "/api/admin/change-requests/:id/reject",
            post(crate::developer::change_requests::admin_reject),
        )
        // Support
        .route("/api/admin/support", get(api_admin_support_tickets))
        .route("/api/admin/support/bulk", patch(api_admin_support_bulk))
        .route(
            "/api/admin/support/:ticket_id",
            get(api_admin_support_ticket_detail).patch(api_admin_support_update),
        )
        .route(
            "/api/admin/support/:ticket_id/messages",
            post(api_admin_support_ticket_reply),
        )
        // Audit & Notifications
        .route("/api/admin/audit-logs", get(api_admin_audit_logs))
        .route("/api/admin/notifications", get(api_admin_notifications))
        .route(
            "/api/admin/notifications/broadcast",
            post(api_admin_notification_broadcast),
        )
        // System
        .route("/api/admin/system", get(api_admin_system))
        .route("/api/admin/system/jobs", get(api_admin_system_jobs))
        .route(
            "/api/admin/system/jobs/:id",
            delete(api_admin_system_job_cancel),
        )
        .route(
            "/api/admin/system/jobs/:id/retry",
            post(api_admin_system_job_retry),
        )
        .route(
            "/api/admin/system/webhooks",
            get(api_admin_system_webhooks),
        )
        .route(
            "/api/admin/system/webhooks/:id/replay",
            post(api_admin_system_webhook_replay),
        )
        .route(
            "/api/admin/system/sessions",
            get(api_admin_system_sessions),
        )
        .route(
            "/api/admin/system/sessions/bulk-revoke",
            post(api_admin_system_sessions_bulk_revoke),
        )
        .route(
            "/api/admin/system/sessions/:id",
            delete(api_admin_system_session_revoke),
        )
        .route(
            "/api/admin/system/password-resets",
            get(api_admin_system_password_resets),
        )
        // Dividends (legacy)
        .route(
            "/api/admin/dividends/calculate",
            post(api_admin_dividends_calculate),
        )
        .route(
            "/api/admin/dividends/process",
            post(api_admin_dividends_process),
        )
        // Dividend Distributions (Phase 9 — full lifecycle)
        .route(
            "/api/admin/dividends/distributions",
            get(api_admin_dividends_list).post(api_admin_dividends_create_distribution),
        )
        .route(
            "/api/admin/dividends/distributions/:dist_id",
            get(api_admin_dividends_distribution_detail),
        )
        .route(
            "/api/admin/dividends/distributions/:dist_id/approve",
            post(api_admin_dividends_approve_distribution),
        )
        .route(
            "/api/admin/dividends/distributions/:dist_id/execute",
            post(api_admin_dividends_execute_distribution),
        )
        .route(
            "/api/admin/dividends/distributions/:dist_id/cancel",
            post(api_admin_dividends_cancel_distribution),
        )
        // Emails
        .route("/api/admin/emails", get(api_admin_emails))
        .route("/api/admin/emails/templates", post(api_admin_emails_create))
        .route(
            "/api/admin/emails/templates/:id",
            put(api_admin_emails_update),
        )
        .route(
            "/api/admin/emails/campaigns",
            post(api_admin_emails_campaign),
        )
        // Settings
        .route(
            "/api/admin/settings",
            get(api_admin_get_settings).post(api_admin_update_settings),
        )
        .route("/api/admin/settings/admins", post(api_admin_add_admin))
        .route(
            "/api/admin/settings/admins/:user_id",
            delete(api_admin_remove_admin).patch(api_admin_update_admin_role),
        )
        .route("/api/admin/settings/roles", get(api_admin_list_roles))
        .route(
            "/api/admin/settings/maintenance",
            post(api_admin_toggle_maintenance),
        )
        // Maintenance
        .route(
            "/api/admin/maintenance/clear-cache",
            post(api_admin_clear_cache),
        )
        .route(
            "/api/admin/maintenance/rotate-logs",
            post(api_admin_rotate_logs),
        )
        // Debug & Reports (debug builds only — seeding must never ship to prod)
        // Tax & Fiscal
        .route("/api/admin/tax-reports", get(api_admin_tax_reports))
        .route(
            "/api/admin/tax-reports/generate",
            post(api_admin_tax_reports_generate),
        )
        // Disputes
        .route("/api/admin/disputes", get(api_admin_disputes))
        .route(
            "/api/admin/disputes/:id/status",
            put(api_admin_disputes_status_update),
        )
        .route(
            "/api/admin/disputes/:id/evidence",
            get(api_admin_disputes_evidence_bundle).post(api_admin_disputes_generate_evidence),
        )
        // Approvals
        .route(
            "/api/admin/approvals",
            get(api_admin_approvals_list).post(api_admin_approvals_create),
        )
        .route(
            "/api/admin/approvals/:id/approve",
            post(api_admin_approvals_approve),
        )
        .route(
            "/api/admin/approvals/:id/reject",
            post(api_admin_approvals_reject),
        )
        // Legal
        .route(
            "/api/admin/legal/version",
            get(api_admin_legal_get_version_handler).post(api_admin_legal_update_version_handler),
        )
        // Storage Analytics
        .route("/api/admin/storage", get(api_admin_storage))
        // ── Admin Marketplace APIs ───────────────────────────────
        .route(
            "/api/admin/primary-escrow",
            get(primary_escrow::api_admin_primary_escrow_list),
        )
        .route(
            "/api/admin/primary-escrow/:asset_id/release-request",
            post(primary_escrow::api_admin_primary_escrow_release_request),
        )
        .route(
            "/api/admin/marketplace/stats",
            get(marketplace::api_admin_marketplace_stats),
        )
        .route(
            "/api/admin/marketplace/recent-trades",
            get(marketplace::api_admin_marketplace_recent_trades),
        )
        .route(
            "/api/admin/marketplace/trades/assets",
            get(marketplace::api_admin_marketplace_trade_assets),
        )
        .route(
            "/api/admin/marketplace/trades/export.csv",
            get(marketplace::api_admin_marketplace_trades_export_csv),
        )
        .route(
            "/api/admin/marketplace/trades",
            get(marketplace::api_admin_marketplace_trades),
        )
        .route(
            "/api/admin/marketplace/orders",
            get(marketplace::api_admin_marketplace_orders),
        )
        .route(
            "/api/admin/marketplace/orders/:order_id",
            delete(marketplace::api_admin_marketplace_order_cancel),
        )
        .route(
            "/api/admin/marketplace/orderbook/assets",
            get(marketplace::api_admin_marketplace_orderbook_assets),
        )
        .route(
            "/api/admin/marketplace/orderbook/:asset_id",
            get(marketplace::api_admin_marketplace_orderbook),
        )
        .route(
            "/api/admin/marketplace/orderbook/rebuild",
            post(marketplace::api_admin_marketplace_orderbook_rebuild),
        )
        .route(
            "/api/admin/marketplace/toggle-trading",
            post(marketplace::api_admin_marketplace_toggle_trading),
        )
        .route(
            "/api/admin/marketplace/health",
            get(marketplace::api_admin_marketplace_health),
        )
        .route(
            "/api/admin/marketplace/reconciliation",
            get(marketplace::api_admin_marketplace_reconciliation),
        )
        // ── 6A.13: Compliance & OJK APIs ─────────────────────────
        .route(
            "/api/admin/marketplace/compliance/ojk-report",
            get(marketplace::api_admin_marketplace_compliance_ojk),
        )
        .route(
            "/api/admin/marketplace/compliance/travel-rule",
            get(marketplace::api_admin_marketplace_compliance_travel_rule),
        )
        .route(
            "/api/admin/marketplace/compliance/tax-export",
            get(marketplace::api_admin_marketplace_compliance_tax),
        )
        // ── 6A.7: Pending Approvals ──────────────────────────────
        .route(
            "/api/admin/marketplace/approvals",
            get(marketplace::api_admin_marketplace_approvals),
        )
        .route(
            "/api/admin/marketplace/approvals/:order_id/approve",
            post(marketplace::api_admin_marketplace_approve_order),
        )
        .route(
            "/api/admin/marketplace/approvals/:order_id/reject",
            post(marketplace::api_admin_marketplace_reject_order),
        )
        // ── 6A.8: Fee Management ─────────────────────────────────
        .route(
            "/api/admin/marketplace/fees",
            get(marketplace::api_admin_marketplace_fees)
                .post(marketplace::api_admin_marketplace_create_fee),
        )
        // ── 6A.9: P2P Offers ─────────────────────────────────────
        .route(
            "/api/admin/marketplace/p2p",
            get(marketplace::api_admin_marketplace_p2p),
        )
        .route(
            "/api/admin/marketplace/p2p/:offer_id/cancel",
            post(marketplace::api_admin_marketplace_cancel_p2p),
        )
        // ── 6A.12: Alerts & Watchlist ────────────────────────────
        .route(
            "/api/admin/marketplace/alerts",
            get(marketplace::api_admin_marketplace_alerts),
        )
        .route(
            "/api/admin/marketplace/alerts/:alert_id",
            post(marketplace::api_admin_marketplace_alert_action),
        )
        .route(
            "/api/admin/marketplace/watchlist",
            get(marketplace::api_admin_marketplace_watchlist)
                .post(marketplace::api_admin_marketplace_add_watchlist),
        )
        // ── 6A.14: Marketplace Settings ──────────────────────────
        .route(
            "/api/admin/marketplace/settings",
            get(marketplace::api_admin_marketplace_settings)
                .post(marketplace::api_admin_marketplace_save_settings),
        )
        // ── Blockchain Treasury & Tokenization ───────────────────
        .route(
            "/api/admin/blockchain/treasury",
            get(blockchain::api_admin_blockchain_treasury),
        )
        .route(
            "/api/admin/blockchain/contracts/:address/detail",
            get(blockchain::api_admin_blockchain_clone_detail),
        )
        .route(
            "/api/admin/blockchain/tokenize-candidates",
            get(blockchain::api_admin_blockchain_tokenize_candidates),
        )
        .route(
            "/api/admin/blockchain/tokenize/:asset_id",
            get(blockchain::api_admin_blockchain_tokenize_check)
                .post(blockchain::api_admin_blockchain_tokenize),
        )
        .route(
            "/api/admin/blockchain/pause",
            post(blockchain::api_admin_blockchain_pause),
        )
        .route(
            "/api/admin/blockchain/unpause",
            post(blockchain::api_admin_blockchain_unpause),
        )
        // ── Blockchain Sync & Health (8C.5) ──────────────────────────
        .route(
            "/api/admin/blockchain/sync",
            get(blockchain::api_admin_blockchain_sync_status),
        )
        .route(
            "/api/admin/blockchain/force-kyc-sync/:user_id",
            post(blockchain::api_admin_blockchain_force_kyc_sync),
        )
        // ── Per-Clone Pause/Unpause (8C.4 SPV Isolation) ─────────────
        .route(
            "/api/admin/blockchain/contracts/:address/pause",
            post(blockchain::api_admin_blockchain_pause_clone),
        )
        .route(
            "/api/admin/blockchain/contracts/:address/unpause",
            post(blockchain::api_admin_blockchain_unpause_clone),
        )
        // ── IPFS: Pin metadata to Pinata ─────────────────────────────
        .route(
            "/api/admin/blockchain/pin-metadata/:asset_id",
            post(blockchain::api_admin_blockchain_pin_metadata),
        );

    // Debug-only endpoints: DB seeder is gated so it cannot ship to prod.
    #[cfg(debug_assertions)]
    {
        r = r.route("/api/admin/debug/seed", post(api_admin_debug_seed));
    }

    r
}
