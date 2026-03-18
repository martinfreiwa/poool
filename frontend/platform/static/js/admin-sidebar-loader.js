/**
 * Admin Sidebar Loader
 * Consolidates the admin sidebar into a single component for every page.
 */

(function () {
    function loadSidebar() {
        const sidebarPlaceholder = document.getElementById(
            "admin-sidebar-placeholder",
        );
        if (!sidebarPlaceholder) return;

        const currentPath = window.location.pathname;

        // Robust path matching helper
        const isPathActive = (paths) => {
            const cleanCurrent = currentPath.replace(/\/$/, "").replace(".html", "");
            return paths.some((path) => {
                const cleanTarget = path.replace(/\/$/, "").replace(".html", "");
                return cleanCurrent === cleanTarget;
            });
        };

        const sidebarHtml = `
            <aside class="admin-sidebar" id="main-admin-sidebar">
                <!-- Logo -->
                <div class="admin-sidebar-header">
                    <a href="/admin/" class="admin-sidebar-logo">
                        <img src="/static/images/Logo%20Pool.svg" alt="POOOL">
                        <span class="admin-sidebar-logo-badge">Admin</span>
                    </a>
                </div>

                <!-- Navigation -->
                <nav class="admin-sidebar-nav">
                    <!-- Overview -->
                    <div class="admin-nav-section">
                        <a href="/admin/" class="admin-nav-item ${isPathActive(["/admin/", "/admin/index.html"]) ? "active" : ""}" id="nav-dashboard">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="6" height="6" rx="1.5"/><rect x="11" y="3" width="6" height="6" rx="1.5"/><rect x="3" y="11" width="6" height="6" rx="1.5"/><rect x="11" y="11" width="6" height="6" rx="1.5"/></svg>
                            <span>Dashboard</span>
                        </a>
                    </div>

                    <!-- People -->
                    <div class="admin-nav-section">
                        <span class="admin-nav-section-label">People</span>
                        <a href="/admin/users.html" class="admin-nav-item ${isPathActive(["/admin/users.html", "/admin/user-details.html"]) ? "active" : ""}" id="nav-users">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21v-2a4 4 0 00-4-4H9a4 4 0 00-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
                            <span>Users</span>
                        </a>
                        <a href="/admin/kyc.html" class="admin-nav-item ${isPathActive(["/admin/kyc.html"]) ? "active" : ""}" id="nav-kyc">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M9 11l3 3L22 4"/><rect x="2" y="4" width="14" height="16" rx="2"/></svg>
                            <span>KYC & AML</span>
                        </a>
                        <a href="/admin/support.html" class="admin-nav-item ${isPathActive(["/admin/support.html", "/admin/support-ticket.html"]) ? "active" : ""}" id="nav-support">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M18 10c0 4.418-3.582 8-8 8a8 8 0 110-16c4.418 0 8 3.582 8 8z"/><path d="M10 14v.01M10 6v6"/></svg>
                            <span>Support</span>
                        </a>
                    </div>

                    <!-- Assets -->
                    <div class="admin-nav-section">
                        <span class="admin-nav-section-label">Assets</span>
                        <a href="/admin/developer-submissions.html" class="admin-nav-item ${isPathActive(["/admin/developer-submissions.html", "/admin/developer-submission-review.html"]) ? "active" : ""}" id="nav-submissions">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6z"/><path d="M14 2v6h6M8 12h8M8 16h6"/></svg>
                            <span>Submissions</span>
                        </a>
                        <a href="/admin/asset-change-requests.html" class="admin-nav-item ${isPathActive(["/admin/asset-change-requests.html", "/admin/asset-change-review.html"]) ? "active" : ""}" id="nav-change-requests">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
                            <span>Change Requests</span>
                            <span id="change-requests-badge" style="display:none;min-width:18px;height:18px;line-height:18px;text-align:center;padding:0 5px;background:#f59e0b;color:#fff;border-radius:10px;font-size:10px;font-weight:700;margin-left:auto;"></span>
                        </a>
                        <a href="/admin/assets.html" class="admin-nav-item ${isPathActive(["/admin/assets.html", "/admin/asset-details.html"]) ? "active" : ""}" id="nav-assets">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="7" width="20" height="14" rx="2"/><path d="M16 21V5a2 2 0 00-2-2h-4a2 2 0 00-2 2v16"/></svg>
                            <span>Live Assets</span>
                        </a>
                    </div>

                    <!-- Finance -->
                    <div class="admin-nav-section">
                        <span class="admin-nav-section-label">Finance</span>
                        <a href="/admin/orders.html" class="admin-nav-item ${isPathActive(["/admin/orders.html"]) ? "active" : ""}" id="nav-orders">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h4l2.5 12h11.5a2 2 0 001.9-1.3L23 5H5.5"/><circle cx="9" cy="20" r="2"/><circle cx="18" cy="20" r="2"/></svg>
                            <span>Orders</span>
                        </a>
                        <a href="/admin/deposits.html" class="admin-nav-item ${isPathActive(["/admin/deposits.html"]) ? "active" : ""}" id="nav-deposits">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v20M8 6l4-4 4 4"/><rect x="3" y="10" width="18" height="10" rx="3"/></svg>
                            <span>Deposits</span>
                        </a>
                        <a href="/admin/treasury.html" class="admin-nav-item ${isPathActive(["/admin/treasury.html"]) ? "active" : ""}" id="nav-treasury">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="4" width="20" height="16" rx="2"/><path d="M2 10h20M12 10v10M7 2v4M17 2v4"/></svg>
                            <span>Treasury</span>
                        </a>
                        <a href="/admin/dividends.html" class="admin-nav-item ${isPathActive(["/admin/dividends.html"]) ? "active" : ""}" id="nav-dividends">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M8 12h8M12 8v8"/></svg>
                            <span>Dividends</span>
                        </a>
                        <a href="/admin/rewards.html" class="admin-nav-item ${isPathActive(["/admin/rewards.html"]) ? "active" : ""}" id="nav-rewards">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/></svg>
                            <span>Rewards</span>
                        </a>
                    </div>

                    <!-- Security & Access -->
                    <div class="admin-nav-section">
                        <span class="admin-nav-section-label">Security & Access</span>
                        <a href="/admin/admins.html" class="admin-nav-item ${isPathActive(["/admin/admins.html"]) ? "active" : ""}" id="nav-admins">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2"/><circle cx="12" cy="7" r="4"/><path d="M12 3v2M12 9v2M15 6h-2M9 6H7"/></svg>
                            <span>Admin Directory</span>
                        </a>
                        <a href="/admin/roles.html" class="admin-nav-item ${isPathActive(["/admin/roles.html"]) ? "active" : ""}" id="nav-roles">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3a9 9 0 100 18 9 9 0 000-18zm0 14a5 5 0 110-10 5 5 0 010 10z"/><path d="M12 8v2M12 14v.01"/></svg>
                            <span>Roles & Permissions</span>
                        </a>
                        <a href="/admin/audit-logs.html" class="admin-nav-item ${isPathActive(["/admin/audit-logs.html"]) ? "active" : ""}" id="nav-audit">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="2" width="16" height="20" rx="2"/><path d="M8 6h8M8 10h8M8 14h4"/></svg>
                            <span>Audit Logs</span>
                        </a>
                        <a href="/admin/approvals.html" class="admin-nav-item ${isPathActive(["/admin/approvals.html"]) ? "active" : ""}" id="nav-approvals">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M9 12l2 2 4-4"/></svg>
                            <span>Approval Queue</span>
                            <span id="approvals-badge" style="display:none;min-width:18px;height:18px;line-height:18px;text-align:center;padding:0 5px;background:#ef4444;color:#fff;border-radius:10px;font-size:10px;font-weight:700;margin-left:auto;"></span>
                        </a>
                    </div>

                    <!-- System -->
                    <div class="admin-nav-section">
                        <span class="admin-nav-section-label">System</span>
                        <a href="/admin/notifications.html" class="admin-nav-item ${isPathActive(["/admin/notifications.html"]) ? "active" : ""}" id="nav-notifications">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9z"/><path d="M13.73 21a2 2 0 01-3.46 0"/></svg>
                            <span>Notifications</span>
                        </a>
                        <a href="/admin/reports.html" class="admin-nav-item ${isPathActive(["/admin/reports.html"]) ? "active" : ""}" id="nav-reports">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M4 18v-4M8 18v-8M12 18V6M16 18V2M4 18h16"/><circle cx="16" cy="2" r="2"/></svg>
                            <span>Reports</span>
                        </a>
                        <a href="/admin/email-marketing.html" class="admin-nav-item ${isPathActive(["/admin/email-marketing.html"]) ? "active" : ""}" id="nav-email">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="4" width="20" height="16" rx="2"/><path d="M22 6l-10 7L2 6"/></svg>
                            <span>Emails & Marketing</span>
                        </a>
                        <a href="/admin/storage.html" class="admin-nav-item ${isPathActive(["/admin/storage.html"]) ? "active" : ""}" id="nav-storage">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></svg>
                            <span>Storage</span>
                        </a>
                        <a href="/admin/system.html" class="admin-nav-item ${isPathActive(["/admin/system.html"]) ? "active" : ""}" id="nav-system">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h4l2-5 3 10 2-5h4"/><rect x="2" y="4" width="20" height="16" rx="2"/></svg>
                            <span>System Health</span>
                        </a>
                        <a href="/admin/settings.html" class="admin-nav-item ${isPathActive(["/admin/settings.html"]) ? "active" : ""}" id="nav-settings">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 15a3 3 0 100-6 3 3 0 000 6z"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/></svg>
                            <span>Settings</span>
                        </a>
                    </div>
                </nav>

                <!-- Footer -->
                <div class="admin-sidebar-footer">
                    <button class="admin-theme-toggle" id="admin-theme-toggle" title="Toggle dark/light mode">
                        <div class="admin-theme-toggle-track">
                            <div class="admin-theme-toggle-thumb"></div>
                        </div>
                        <span id="theme-toggle-label">Light</span>
                    </button>
                    <div class="admin-sidebar-user">
                        <img src="/static/images/Image.webp" alt="Admin" class="admin-sidebar-avatar" id="sidebar-user-avatar">
                        <div class="admin-sidebar-user-info">
                            <div class="admin-sidebar-user-name" id="sidebar-user-name">Admin User</div>
                            <div class="admin-sidebar-user-role" id="sidebar-user-role">Super Admin</div>
                        </div>
                    </div>
                </div>
            </aside>
        `;

        sidebarPlaceholder.innerHTML = sidebarHtml;

        // Notify the permission guard (and any other listeners) that the
        // sidebar DOM is now available so they can apply visibility rules.
        document.dispatchEvent(new CustomEvent("admin:sidebar-ready"));

        // Sync theme label immediately
        const isDark = document.documentElement.classList.contains("admin-dark");
        const label = document.getElementById("theme-toggle-label");
        if (label) label.textContent = isDark ? "Dark" : "Light";

        // Load user data if available
        if (window.userData) {
            const nameEl = document.getElementById("sidebar-user-name");
            const avatarEl = document.getElementById("sidebar-user-avatar");
            if (nameEl)
                nameEl.textContent = window.userData.full_name || "Admin User";
            if (avatarEl && window.userData.profile_image)
                avatarEl.src = window.userData.profile_image;
        }
    }

    // Dynamically update notification bell badge count across all admin pages
    async function updateNotificationBadges() {
        try {
            // Fetch unread notification count
            const notifResp = await fetch("/api/admin/notifications");
            if (notifResp.ok) {
                const data = await notifResp.json();
                const unread = (data.notifications || []).filter(n => !n.is_read).length;
                // Update all notification badge elements on the page
                document.querySelectorAll(".admin-notification-badge").forEach(badge => {
                    if (unread > 0) {
                        badge.textContent = unread > 99 ? "99+" : unread;
                        badge.style.display = "";
                    } else {
                        badge.style.display = "none";
                    }
                });
            }
        } catch (e) {
            // Silently fail — notification badges stay hidden
        }

        try {
            // Fetch pending approval count for the sidebar badge
            const approvalResp = await fetch("/api/admin/approvals");
            if (approvalResp.ok) {
                const data = await approvalResp.json();
                const pending = data.pending_count || 0;
                const badge = document.getElementById("approvals-badge");
                if (badge) {
                    if (pending > 0) {
                        badge.textContent = pending;
                        badge.style.display = "";
                    } else {
                        badge.style.display = "none";
                    }
                }
            }
        } catch (e) {
            // Silently fail
        }

        try {
            // Fetch pending change requests count for sidebar badge
            const crResp = await fetch("/api/admin/change-requests");
            if (crResp.ok) {
                const data = await crResp.json();
                const pending = data.pending_count || 0;
                const badge = document.getElementById("change-requests-badge");
                if (badge) {
                    if (pending > 0) {
                        badge.textContent = pending;
                        badge.style.display = "";
                    } else {
                        badge.style.display = "none";
                    }
                }
            }
        } catch (e) {
            // Silently fail
        }
    }

    // Run on script load if placeholder exists, or on DOMContentLoaded
    if (document.getElementById("admin-sidebar-placeholder")) {
        loadSidebar();
        // Update badges after sidebar is loaded (slight delay to ensure DOM ready)
        setTimeout(updateNotificationBadges, 500);
    } else {
        document.addEventListener("DOMContentLoaded", () => {
            loadSidebar();
            setTimeout(updateNotificationBadges, 500);
        });
    }
})();
