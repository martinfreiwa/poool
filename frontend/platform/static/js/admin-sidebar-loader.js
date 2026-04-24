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
                        <img src="/static/images/logos/Logo%20Pool.svg" alt="POOOL">
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

                    <!-- Blog -->
                    <div class="admin-nav-section">
                        <span class="admin-nav-section-label">Blog</span>
                        <a href="/admin/blog.html" class="admin-nav-item ${isPathActive(["/admin/blog.html", "/admin/blog-editor.html"]) ? "active" : ""}" id="nav-blog">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5A2.5 2.5 0 016.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 014 19.5v-15A2.5 2.5 0 016.5 2z"/><path d="M8 7h8M8 11h6"/></svg>
                            <span>Blog</span>
                        </a>
                        <a href="/admin/blog-persona.html" class="admin-nav-item ${isPathActive(["/admin/blog-persona.html"]) ? "active" : ""}" id="nav-blog-persona">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21a8 8 0 1 0-16 0"/><circle cx="12" cy="7" r="4"/><path d="M15 11l2 2 4-4"/></svg>
                            <span>Persona</span>
                        </a>
                        <a href="/admin/blog-strategy.html" class="admin-nav-item ${isPathActive(["/admin/blog-strategy.html"]) ? "active" : ""}" id="nav-blog-strategy">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3v18h18"/><path d="M7 15l4-4 3 3 5-7"/><path d="M17 7h2v2"/></svg>
                            <span>Strategy</span>
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

                    <!-- Community -->
                    <div class="admin-nav-section">
                        <span class="admin-nav-section-label">Community</span>
                        <a href="/admin/community/" class="admin-nav-item ${isPathActive(["/admin/community/", "/admin/community/index.html"]) ? "active" : ""}" id="nav-com-overview">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 00-3-3.87"/><path d="M16 3.13a4 4 0 010 7.75"/></svg>
                            <span>Overview</span>
                        </a>
                        <a href="/admin/community/announcements.html" class="admin-nav-item ${isPathActive(["/admin/community/announcements.html"]) ? "active" : ""}" id="nav-com-announcements">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
                            <span>Announcements</span>
                        </a>
                        <a href="/admin/community/posts.html" class="admin-nav-item ${isPathActive(["/admin/community/posts.html", "/admin/community/post-detail.html"]) ? "active" : ""}" id="nav-com-posts">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path></svg>
                            <span>Posts</span>
                        </a>
                        <a href="/admin/community/comments.html" class="admin-nav-item ${isPathActive(["/admin/community/comments.html"]) ? "active" : ""}" id="nav-com-comments">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"></path></svg>
                            <span>Comments</span>
                        </a>
                        <a href="/admin/community/reports.html" class="admin-nav-item ${isPathActive(["/admin/community/reports.html"]) ? "active" : ""}" id="nav-com-reports">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>
                            <span>Moderation Queue</span>
                            <span id="com-reports-badge" style="display:none;min-width:18px;height:18px;line-height:18px;text-align:center;padding:0 5px;background:#ef4444;color:#fff;border-radius:10px;font-size:10px;font-weight:700;margin-left:auto;"></span>
                        </a>
                        <a href="/admin/community/users.html" class="admin-nav-item ${isPathActive(["/admin/community/users.html", "/admin/community/user-detail.html"]) ? "active" : ""}" id="nav-com-users">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M23 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path></svg>
                            <span>Community Users</span>
                        </a>
                        <a href="/admin/community/badges.html" class="admin-nav-item ${isPathActive(["/admin/community/badges.html"]) ? "active" : ""}" id="nav-com-badges">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="8" r="7"></circle><polyline points="8.21 13.89 7 23 12 20 17 23 15.79 13.88"></polyline></svg>
                            <span>Badges</span>
                        </a>
                        <a href="/admin/community/amas.html" class="admin-nav-item ${isPathActive(["/admin/community/amas.html"]) ? "active" : ""}" id="nav-com-amas">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"></path><path d="M19 10v2a7 7 0 0 1-14 0v-2"></path><line x1="12" y1="19" x2="12" y2="23"></line><line x1="8" y1="23" x2="16" y2="23"></line></svg>
                            <span>Expert AMAs</span>
                        </a>
                        <a href="/admin/community/challenges.html" class="admin-nav-item ${isPathActive(["/admin/community/challenges.html"]) ? "active" : ""}" id="nav-com-challenges">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon></svg>
                            <span>Challenges</span>
                        </a>
                        <a href="/admin/community/circles.html" class="admin-nav-item ${isPathActive(["/admin/community/circles.html", "/admin/community/circle-detail.html"]) ? "active" : ""}" id="nav-com-circles">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><circle cx="12" cy="12" r="3"></circle></svg>
                            <span>Circles</span>
                        </a>
                        <a href="/admin/community/leaderboard.html" class="admin-nav-item ${isPathActive(["/admin/community/leaderboard.html"]) ? "active" : ""}" id="nav-com-leaderboard">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v20"></path><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"></path></svg>
                            <span>Leaderboard</span>
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
                        <a href="/admin/asset-tokenize.html" class="admin-nav-item ${isPathActive(["/admin/asset-tokenize.html"]) ? "active" : ""}" id="nav-asset-tokenize">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/><path d="M7.5 4.21l4.5 2.6 4.5-2.6M7.5 19.79V14.6L3 12M21 12l-4.5 2.6v5.19M12 6.81v5.2"/></svg>
                            <span>Asset Tokenize</span>
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
                        <a href="/admin/blockchain-treasury.html" class="admin-nav-item ${isPathActive(["/admin/blockchain-treasury.html"]) ? "active" : ""}" id="nav-blockchain">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>
                            <span>Blockchain Treasury</span>
                        </a>
                        <a href="/admin/blockchain-contracts.html" class="admin-nav-item ${isPathActive(["/admin/blockchain-contracts.html", "/admin/blockchain-contract-detail.html"]) ? "active" : ""}" id="nav-blockchain-contracts">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/></svg>
                            <span>Live Contracts</span>
                        </a>
                        <a href="/admin/blockchain-sync.html" class="admin-nav-item ${isPathActive(["/admin/blockchain-sync.html"]) ? "active" : ""}" id="nav-blockchain-sync">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.92-10.26l-3.42 3.42"/><line x1="12" y1="12" x2="16" y2="16"/></svg>
                            <span>Web3 Sync & Health</span>
                        </a>
                        <a href="/admin/dividends.html" class="admin-nav-item ${isPathActive(["/admin/dividends.html"]) ? "active" : ""}" id="nav-dividends">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M8 12h8M12 8v8"/></svg>
                            <span>Dividends</span>
                        </a>
                        <a href="/admin/rewards.html" class="admin-nav-item ${isPathActive(["/admin/rewards.html"]) ? "active" : ""}" id="nav-rewards">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/></svg>
                            <span>Rewards</span>
                        </a>
                        <a href="/admin/pending-settlements.html" class="admin-nav-item ${isPathActive(["/admin/pending-settlements.html"]) ? "active" : ""}" id="nav-settlements">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
                            <span>Pending Settlements</span>
                        </a>
                        <a href="/admin/affiliate-applications.html" class="admin-nav-item ${isPathActive(["/admin/affiliate-applications.html"]) ? "active" : ""}" id="nav-affiliate-apps">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 00-3-3.87"/><path d="M16 3.13a4 4 0 010 7.75"/></svg>
                            <span>Affiliate Apps</span>
                            <span id="affiliate-apps-badge" style="display:none;min-width:18px;height:18px;line-height:18px;text-align:center;padding:0 5px;background:#f59e0b;color:#fff;border-radius:10px;font-size:10px;font-weight:700;margin-left:auto;"></span>
                        </a>
                        <a href="/admin/affiliate-finance.html" class="admin-nav-item ${isPathActive(["/admin/affiliate-finance.html"]) ? "active" : ""}" id="nav-affiliate-finance">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="1" x2="12" y2="23"></line><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"></path></svg>
                            <span>Affiliate Finance</span>
                        </a>
                        <a href="/admin/admin-affiliate-fraud.html" class="admin-nav-item ${isPathActive(["/admin/admin-affiliate-fraud.html"]) ? "active" : ""}" id="nav-affiliate-fraud">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
                            <span>Syndicate Fraud</span>
                        </a>
                    </div>

                    <!-- Marketplace -->
                    <div class="admin-nav-section">
                        <span class="admin-nav-section-label">Marketplace</span>
                        <a href="/admin/marketplace/" class="admin-nav-item ${isPathActive(["/admin/marketplace/", "/admin/marketplace/index.html"]) ? "active" : ""}" id="nav-mp-overview">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M18 20V10M12 20V4M6 20v-6"/></svg>
                            <span>MP Overview</span>
                        </a>
                        <a href="/admin/marketplace/orderbook.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/orderbook.html"]) ? "active" : ""}" id="nav-mp-orderbook">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M4 18v-4M8 18v-8M12 18V6M16 18V2M4 18h16"/></svg>
                            <span>Orderbook</span>
                        </a>
                        <a href="/admin/marketplace/trades.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/trades.html"]) ? "active" : ""}" id="nav-mp-trades">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M7 7h10v10"/><path d="M7 17L17 7"/></svg>
                            <span>Trades</span>
                        </a>
                        <a href="/admin/marketplace/orders.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/orders.html"]) ? "active" : ""}" id="nav-mp-orders">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h4l2.5 12h11.5a2 2 0 001.9-1.3L23 5H5.5"/><circle cx="9" cy="20" r="2"/><circle cx="18" cy="20" r="2"/></svg>
                            <span>Open Orders</span>
                        </a>
                        <a href="/admin/marketplace/approvals.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/approvals.html"]) ? "active" : ""}" id="nav-mp-approvals">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M9 12l2 2 4-4"/></svg>
                            <span>MP Approvals</span>
                        </a>
                        <a href="/admin/marketplace/primary-escrow.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/primary-escrow.html"]) ? "active" : ""}" id="nav-mp-primary-escrow">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="6" width="18" height="14" rx="2"/><path d="M16 10a4 4 0 01-8 0"/><path d="M9 6V4a3 3 0 016 0v2"/></svg>
                            <span>Primary Escrow</span>
                        </a>
                        <a href="/admin/marketplace/fees.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/fees.html"]) ? "active" : ""}" id="nav-mp-fees">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 015.83 1c0 2-3 3-3 3M12 17h.01"/></svg>
                            <span>Fees</span>
                        </a>
                        <a href="/admin/marketplace/alerts.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/alerts.html"]) ? "active" : ""}" id="nav-mp-alerts">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0zM12 9v4M12 17h.01"/></svg>
                            <span>Alerts</span>
                        </a>
                        <a href="/admin/marketplace/p2p.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/p2p.html"]) ? "active" : ""}" id="nav-mp-p2p">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75"/></svg>
                            <span>P2P Offers</span>
                        </a>
                        <a href="/admin/marketplace/reconciliation.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/reconciliation.html"]) ? "active" : ""}" id="nav-mp-recon">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M16 16v3a2 2 0 01-2 2H6a2 2 0 01-2-2V7a2 2 0 012-2h2m0 0h6a2 2 0 012 2v6"/><path d="M9 12l2 2 4-4"/></svg>
                            <span>Reconciliation</span>
                        </a>
                        <a href="/admin/marketplace/compliance.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/compliance.html"]) ? "active" : ""}" id="nav-mp-compliance">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                            <span>Compliance</span>
                        </a>
                        <a href="/admin/marketplace/analytics.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/analytics.html"]) ? "active" : ""}" id="nav-mp-analytics">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M8 12l2 2 4-4"/></svg>
                            <span>Analytics</span>
                        </a>
                        <a href="/admin/marketplace/settings.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/settings.html"]) ? "active" : ""}" id="nav-mp-settings">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 15a3 3 0 100-6 3 3 0 000 6z"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/></svg>
                            <span>MP Settings</span>
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

                    <!-- Templates -->
                    <div class="admin-nav-section">
                        <span class="admin-nav-section-label">Templates</span>
                        <a href="/admin/templates/icons.html" class="admin-nav-item ${isPathActive(["/admin/templates/icons.html"]) ? "active" : ""}" id="nav-tpl-icons">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7" rx="2"/><rect x="14" y="3" width="7" height="7" rx="2"/><rect x="3" y="14" width="7" height="7" rx="2"/><rect x="14" y="14" width="7" height="7" rx="2"/></svg>
                            <span>Icons</span>
                        </a>
                        <a href="/statistics-template.html" class="admin-nav-item ${isPathActive(["/statistics-template.html"]) ? "active" : ""}" id="nav-tpl-statistics">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="20" x2="18" y2="10"></line><line x1="12" y1="20" x2="12" y2="4"></line><line x1="6" y1="20" x2="6" y2="14"></line></svg>
                            <span>Statistics & Analytics</span>
                        </a>
                        <a href="/forms-template.html" class="admin-nav-item ${isPathActive(["/forms-template.html"]) ? "active" : ""}" id="nav-tpl-forms">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path><rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect></svg>
                            <span>Forms & Inputs</span>
                        </a>
                        <a href="/table-template.html" class="admin-nav-item ${isPathActive(["/table-template.html"]) ? "active" : ""}" id="nav-tpl-table">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"></path><line x1="22" y1="10" x2="2" y2="10"></line><line x1="8" y1="20" x2="8" y2="10"></line></svg>
                            <span>Data Tables</span>
                        </a>
                        <a href="/overlays-template.html" class="admin-nav-item ${isPathActive(["/overlays-template.html"]) ? "active" : ""}" id="nav-tpl-overlays">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"></path><polyline points="22 6 12 13 2 6"></polyline></svg>
                            <span>Overlays & Modals</span>
                        </a>
                        <a href="/fonts-template.html" class="admin-nav-item ${isPathActive(["/fonts-template.html"]) ? "active" : ""}" id="nav-tpl-fonts">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 7 4 4 20 4 20 7"></polyline><line x1="9" y1="20" x2="15" y2="20"></line><line x1="12" y1="4" x2="12" y2="20"></line></svg>
                            <span>Fonts & Typography</span>
                        </a>
                    </div>
                </nav>

                <!-- Footer -->
                <div class="admin-sidebar-footer">
                    <button type="button" class="admin-theme-toggle" id="admin-theme-toggle" title="Toggle dark/light mode">
                        <div class="admin-theme-toggle-track">
                            <div class="admin-theme-toggle-thumb"></div>
                        </div>
                        <span id="theme-toggle-label">Light</span>
                    </button>
                    <div class="admin-sidebar-user">
                        <img src="/static/images/ui/Image.webp" alt="Admin" class="admin-sidebar-avatar" id="sidebar-user-avatar">
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

        try {
            // Fetch pending content reports count for sidebar badge
            const comReportsResp = await fetch("/api/admin/community/reports");
            if (comReportsResp.ok) {
                const data = await comReportsResp.json();
                const pending = data.length || 0;
                const badge = document.getElementById("com-reports-badge");
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
            // Fetch pending affiliate applications count for sidebar badge
            const affResp = await fetch("/api/admin/rewards/affiliates/pending");
            if (affResp.ok) {
                const data = await affResp.json();
                const pending = (data.pending || []).length;
                const badge = document.getElementById("affiliate-apps-badge");
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
        setTimeout(updateNotificationBadges, 500);
    } else {
        document.addEventListener("DOMContentLoaded", () => {
            loadSidebar();
            setTimeout(updateNotificationBadges, 500);
        });
    }

    // ==== Sidebar Scroll Persistence ====
    // Save sidebar scroll position before navigating away
    // Restore it on the next page load so the sidebar doesn't jump to top
    const SCROLL_KEY = "admin-sidebar-scroll";

    function restoreSidebarScroll() {
        const sidebar = document.querySelector(".admin-sidebar-nav");
        if (!sidebar) return;

        const saved = sessionStorage.getItem(SCROLL_KEY);
        if (saved !== null) {
            sidebar.scrollTop = parseInt(saved, 10);
        } else {
            // No saved position — scroll the active item into view
            const activeItem = sidebar.querySelector(".admin-nav-item.active");
            if (activeItem) {
                activeItem.scrollIntoView({ block: "center", behavior: "instant" });
            }
        }

        // Intercept all sidebar link clicks to save scroll position BEFORE navigation
        sidebar.querySelectorAll("a.admin-nav-item").forEach(link => {
            link.addEventListener("click", () => {
                sessionStorage.setItem(SCROLL_KEY, sidebar.scrollTop);
            });
        });
    }

    // Restore after a tiny delay to ensure sidebar is fully rendered
    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", () => setTimeout(restoreSidebarScroll, 50));
    } else {
        setTimeout(restoreSidebarScroll, 50);
    }
})();
