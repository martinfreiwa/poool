use super::extractors::AdminUser;
use crate::auth::models::User;
use crate::auth::routes::AppState;
use axum::{
    extract::{Request, State},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::CookieJar;

/// GET /admin/  Admin dashboard (protected, requires admin role).
pub async fn page_admin_dashboard(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // AdminUser extractor already verified admin access
    render_admin_template(&state, "admin/index.html")
}

/// GET /admin/blog  Blog dashboard (protected, requires blog.manage).
pub async fn page_admin_blog(admin: AdminUser, State(state): State<AppState>) -> impl IntoResponse {
    if crate::auth::middleware::has_permission(&state.db, admin.user.id, "blog.view").await
        || crate::auth::middleware::has_permission(&state.db, admin.user.id, "blog.manage").await
    {
        render_admin_template(&state, "admin/blog.html")
    } else {
        Redirect::to("/admin/").into_response()
    }
}

/// GET /admin/blog-persona  Blog persona planner (protected, requires blog.view or blog.manage).
pub async fn page_admin_blog_persona(
    admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if crate::auth::middleware::has_permission(&state.db, admin.user.id, "blog.view").await
        || crate::auth::middleware::has_permission(&state.db, admin.user.id, "blog.manage").await
    {
        render_admin_template(&state, "admin/blog-persona.html")
    } else {
        Redirect::to("/admin/").into_response()
    }
}

/// GET /admin/blog-strategy  Blog strategy planner (protected, requires blog.view or blog.manage).
pub async fn page_admin_blog_strategy(
    admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if crate::auth::middleware::has_permission(&state.db, admin.user.id, "blog.view").await
        || crate::auth::middleware::has_permission(&state.db, admin.user.id, "blog.manage").await
    {
        render_admin_template(&state, "admin/blog-strategy.html")
    } else {
        Redirect::to("/admin/").into_response()
    }
}

/// GET /admin/blog-editor  Blog article editor (protected, requires blog.edit or blog.manage).
pub async fn page_admin_blog_editor(
    admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if crate::auth::middleware::has_permission(&state.db, admin.user.id, "blog.edit").await
        || crate::auth::middleware::has_permission(&state.db, admin.user.id, "blog.manage").await
    {
        render_admin_template(&state, "admin/blog-editor.html")
    } else {
        Redirect::to("/admin/blog.html").into_response()
    }
}

/// GET /admin/affiliate-compliance  Legacy affiliate compliance route.
///
/// Affiliate compliance is user-facing through `/affiliate/onboarding` and
/// `/affiliate/dashboard`; admins only review applications from the admin
/// applications board.
pub async fn page_admin_affiliate_compliance_redirect(_admin: AdminUser) -> impl IntoResponse {
    Redirect::to("/admin/affiliate-applications").into_response()
}

/// GET /admin/audit-logs  Audit trail (protected, requires audit.read).
pub async fn page_admin_audit_logs(
    admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if crate::auth::middleware::has_permission(&state.db, admin.user.id, "audit.read").await {
        render_admin_template(&state, "admin/audit-logs.html")
    } else {
        Redirect::to("/admin/").into_response()
    }
}

/// GET /admin/community/announcements  Community announcements admin page.
pub async fn page_admin_community_announcements(
    admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if crate::auth::middleware::has_permission(&state.db, admin.user.id, "community.manage").await {
        render_admin_template(&state, "admin/community/announcements.html")
    } else {
        Redirect::to("/admin/").into_response()
    }
}

/// GET /admin/community/amas  Community AMA management.
pub async fn page_admin_community_amas(
    admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if crate::auth::middleware::has_permission(&state.db, admin.user.id, "community.view").await {
        render_admin_template(&state, "admin/community/amas.html")
    } else {
        Redirect::to("/admin/").into_response()
    }
}

/// GET /admin/community/circles  Community circle management.
pub async fn page_admin_community_circles(
    admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if crate::auth::middleware::has_permission(&state.db, admin.user.id, "community.manage").await {
        render_admin_template(&state, "admin/community/circles.html")
    } else {
        Redirect::to("/admin/").into_response()
    }
}

/// GET /admin/community/challenges  Community challenge management.
pub async fn page_admin_community_challenges(
    admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if crate::auth::middleware::has_permission(&state.db, admin.user.id, "community.manage").await {
        render_admin_template(&state, "admin/community/challenges.html")
    } else {
        Redirect::to("/admin/").into_response()
    }
}

/// GET /admin/marketplace/compliance  Marketplace compliance exports.
///
/// This route intentionally uses the granular permission instead of `AdminUser`
/// so the dedicated compliance role can access the report surface without
/// broadening generic admin-page access.
pub async fn page_admin_marketplace_compliance(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = crate::auth::middleware::get_current_user(&jar, &state.db).await else {
        return Redirect::to("/auth/login").into_response();
    };

    if crate::auth::middleware::has_permission(&state.db, user.id, "marketplace.compliance").await {
        render_admin_template(&state, "admin/marketplace/compliance.html")
    } else {
        Redirect::to("/admin/").into_response()
    }
}

/// GET /admin/deposits  Deposit operations board.
///
/// Uses granular deposit permissions so finance/deposit operators can access
/// the page without broad generic-admin privileges.
pub async fn page_admin_deposits(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = crate::auth::middleware::get_current_user(&jar, &state.db).await else {
        return Redirect::to("/auth/login").into_response();
    };

    if crate::auth::middleware::has_permission(&state.db, user.id, "deposits.read").await
        || crate::auth::middleware::has_permission(&state.db, user.id, "deposits.write").await
        || crate::auth::middleware::has_permission(&state.db, user.id, "deposits.confirm").await
    {
        render_admin_template(&state, "admin/deposits.html")
    } else {
        Redirect::to("/admin/").into_response()
    }
}

/// GET /admin/{any}.html  Serve admin sub-pages (protected).
pub async fn page_admin_generic(
    jar: CookieJar,
    State(state): State<AppState>,
    req: Request,
) -> impl IntoResponse {
    let admin = match get_admin_page_user(&jar, &state).await {
        Ok(user) => user,
        Err(response) => return response,
    };

    let path = req.uri().path();
    let relative = path.trim_start_matches('/');

    // Path traversal protection
    if relative.contains("..") || !relative.starts_with("admin/") {
        return Redirect::to("/admin/").into_response();
    }

    // 301 redirect .html → clean URL (#28 — keep canonical URLs).
    if let Some(stem) = relative.strip_suffix(".html") {
        let q = req
            .uri()
            .query()
            .map(|s| format!("?{}", s))
            .unwrap_or_default();
        return Redirect::permanent(&format!("/{}{}", stem, q)).into_response();
    }

    // If the path doesn't end with .html, append it so clean URLs resolve correctly
    let file = if relative.ends_with('/') {
        format!("{}index.html", relative)
    } else {
        format!("{}.html", relative)
    };

    if relative.starts_with("admin/community/")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "community.view").await
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "community.manage").await
    {
        return Redirect::to("/admin/").into_response();
    }

    if (relative == "admin/affiliate-applications"
        || relative == "admin/affiliate-applications.html"
        || relative == "admin/affiliate-finance"
        || relative == "admin/affiliate-finance.html"
        || relative == "admin/affiliate-fraud"
        || relative == "admin/affiliate-fraud.html"
        || relative == "admin/admin-affiliate-fraud"
        || relative == "admin/admin-affiliate-fraud.html")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "affiliates.manage").await
    {
        return Redirect::to("/admin/").into_response();
    }

    if (relative == "admin/developer-submissions"
        || relative == "admin/developer-submissions.html"
        || relative == "admin/developer-submission-review"
        || relative == "admin/developer-submission-review.html")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "submissions.review").await
    {
        return Redirect::to("/admin/").into_response();
    }

    if (relative == "admin/users" || relative == "admin/users.html")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "users.view").await
    {
        return Redirect::to("/admin/").into_response();
    }

    if (relative == "admin/roles" || relative == "admin/roles.html")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "roles.edit").await
    {
        return Redirect::to("/admin/").into_response();
    }

    if (relative == "admin/notifications" || relative == "admin/notifications.html")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "notifications.view").await
    {
        return Redirect::to("/admin/").into_response();
    }

    if (relative == "admin/reports" || relative == "admin/reports.html")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "reports.generate").await
    {
        return Redirect::to("/admin/").into_response();
    }

    if (relative == "admin/marketplace/approvals" || relative == "admin/marketplace/approvals.html")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "marketplace.manage").await
    {
        return Redirect::to("/admin/").into_response();
    }

    if (relative == "admin/asset-tokenize" || relative == "admin/asset-tokenize.html")
        && !crate::admin::blockchain::has_blockchain_tokenize_permission(&state.db, admin.id).await
    {
        return Redirect::to("/admin/").into_response();
    }

    if (relative == "admin/blockchain-contracts" || relative == "admin/blockchain-contracts.html")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "treasury.read").await
    {
        return Redirect::to("/admin/").into_response();
    }

    if (relative == "admin/blockchain-sync" || relative == "admin/blockchain-sync.html")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "treasury.read").await
    {
        return Redirect::to("/admin/").into_response();
    }

    if (relative == "admin/marketplace/"
        || relative == "admin/marketplace/index"
        || relative == "admin/marketplace/index.html")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "marketplace.view").await
    {
        return Redirect::to("/admin/").into_response();
    }

    if relative.starts_with("admin/marketplace/")
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "marketplace.view").await
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "marketplace.manage").await
        && !crate::auth::middleware::has_permission(&state.db, admin.id, "marketplace.compliance")
            .await
    {
        return Redirect::to("/admin/").into_response();
    }

    render_admin_template(&state, &file)
}

async fn get_admin_page_user(
    jar: &CookieJar,
    state: &AppState,
) -> Result<User, axum::response::Response> {
    let Some(user) = crate::auth::middleware::get_current_user(jar, &state.db).await else {
        return Err(Redirect::to("/auth/login").into_response());
    };

    let is_admin = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM user_roles ur
            JOIN roles r ON r.id = ur.role_id
            WHERE ur.user_id = $1
            AND r.name IN ('admin', 'super_admin')
            AND ur.is_active = TRUE
        )
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if !is_admin {
        return Err(Redirect::to("/admin/").into_response());
    }

    Ok(user)
}

/// Render an admin template. Admin access is assumed to be already verified
/// by the caller.
fn render_admin_template(state: &AppState, file: &str) -> axum::response::Response {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/poool_debug.txt")
    {
        let _ = writeln!(f, "Attempting to load template for: {}", file);
    }
    use axum::response::Html;
    match state.templates.get_template(file) {
        Ok(template) => match template.render(minijinja::context! {
            metabase_base_url => state.config.metabase_base_url,
            metabase_public_dashboard_path => state.config.metabase_public_dashboard_path,
            metabase_dashboard_id => state.config.metabase_dashboard_id,
        }) {
            Ok(content) => Html(content).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error for admin {}: {}", file, e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("<h1>Internal Server Error: {}</h1>", e)),
                )
                    .into_response()
            }
        },
        Err(e) => {
            tracing::error!("Template GET error for admin file {}: {:?}", file, e);
            (
                axum::http::StatusCode::NOT_FOUND,
                Html(format!("<h1>Page not found</h1><p>Debug info: Tried file '{}', minijinja error: {}</p>", file, e)),
            )
                .into_response()
        }
    }
}
