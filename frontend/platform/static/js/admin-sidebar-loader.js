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

        // Restore cached user data synchronously to avoid flash of placeholder values
        var cachedUser = {};
        try { cachedUser = JSON.parse(localStorage.getItem('poool_user_cache') || '{}'); } catch (_) {}
        var cachedName   = cachedUser.full_name || cachedUser.name || 'Admin User';
        var cachedRole   = cachedUser.role ? cachedUser.role.replace('_', ' ').replace(/\b\w/g, function(l) { return l.toUpperCase(); }) : 'Super Admin';
        var cachedAvatar = cachedUser.avatar_src || '/static/images/ui/Image.webp';

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
                        <span class="admin-sidebar-logo-badge" id="admin-sidebar-env-pill" data-env="loading">Admin</span>
                    </a>
                    <button type="button" class="admin-sidebar-collapse-btn" id="admin-sidebar-collapse" aria-label="Collapse sidebar" title="Collapse / expand sidebar (mod-B)">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg>
                    </button>
                </div>

                <!-- Navigation -->
                <nav class="admin-sidebar-nav" aria-label="Admin navigation">
                    <!-- Overview -->
                    <div class="admin-nav-section">
                        <a href="/admin/" class="admin-nav-item ${isPathActive(["/admin/", "/admin/index.html"]) ? "active" : ""}" id="nav-dashboard">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="6" height="6" rx="1.5"/><rect x="11" y="3" width="6" height="6" rx="1.5"/><rect x="3" y="11" width="6" height="6" rx="1.5"/><rect x="11" y="11" width="6" height="6" rx="1.5"/></svg>
                            <span>Dashboard</span>
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
                            <span id="change-requests-badge" class="admin-nav-badge admin-nav-badge--warning" style="display:none"></span>
                        </a>
                        <a href="/admin/assets.html" class="admin-nav-item ${isPathActive(["/admin/assets.html", "/admin/asset-details.html"]) ? "active" : ""}" id="nav-assets">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="7" width="20" height="14" rx="2"/><path d="M16 21V5a2 2 0 00-2-2h-4a2 2 0 00-2 2v16"/></svg>
                            <span>Live Assets</span>
                        </a>
                        <a href="/admin/asset-tokenize.html" class="admin-nav-item ${isPathActive(["/admin/asset-tokenize.html"]) ? "active" : ""}" id="nav-asset-tokenize">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/><path d="M7.5 4.21l4.5 2.6 4.5-2.6M7.5 19.79V14.6L3 12M21 12l-4.5 2.6v5.19M12 6.81v5.2"/></svg>
                            <span>Asset Tokenization</span>
                        </a>
                        <a href="/admin/villa-operations-queue" class="admin-nav-item ${isPathActive(["/admin/villa-operations-queue", "/admin/villa-operations-entry"]) ? "active" : ""}" id="nav-ops-queue">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="18" rx="2"/><path d="M16 2v4M8 2v4M3 10h18"/><path d="M8 14h.01M12 14h.01M16 14h.01M8 18h.01M12 18h.01"/></svg>
                            <span>Operations Queue</span>
                        </a>
                    </div>

                    <!-- Finance -->
                    <div class="admin-nav-section">
                        <span class="admin-nav-section-label">Finance</span>
                        <a href="/admin/orders.html" class="admin-nav-item ${isPathActive(["/admin/orders.html"]) ? "active" : ""}" id="nav-orders" title="Primary investment purchase orders">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h4l2.5 12h11.5a2 2 0 001.9-1.3L23 5H5.5"/><circle cx="9" cy="20" r="2"/><circle cx="18" cy="20" r="2"/></svg>
                            <span>Investment Orders</span>
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
                            <span id="affiliate-apps-badge" class="admin-nav-badge admin-nav-badge--warning" style="display:none"></span>
                        </a>
                        <a href="/admin/affiliate-finance.html" class="admin-nav-item ${isPathActive(["/admin/affiliate-finance.html"]) ? "active" : ""}" id="nav-affiliate-finance">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="1" x2="12" y2="23"></line><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"></path></svg>
                            <span>Affiliate Finance</span>
                        </a>
                        <a href="/admin/affiliate-fraud.html" class="admin-nav-item ${isPathActive(["/admin/affiliate-fraud.html", "/admin/admin-affiliate-fraud.html"]) ? "active" : ""}" id="nav-affiliate-fraud">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
                            <span>Syndicate Fraud</span>
                        </a>
                        <a href="/admin/affiliate-teams" class="admin-nav-item ${isPathActive(["/admin/affiliate-teams"]) ? "active" : ""}" id="nav-affiliate-teams">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>
                            <span>Affiliate Teams</span>
                        </a>
                    </div>

                    <!-- Marketplace -->
                    <div class="admin-nav-section">
                        <span class="admin-nav-section-label">Marketplace</span>
                        <span class="admin-nav-subsection-label">Trading</span>
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
                        <a href="/admin/marketplace/orders.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/orders.html"]) ? "active" : ""}" id="nav-mp-orders" title="Live secondary-market trade orders">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h4l2.5 12h11.5a2 2 0 001.9-1.3L23 5H5.5"/><circle cx="9" cy="20" r="2"/><circle cx="18" cy="20" r="2"/></svg>
                            <span>Trade Orders</span>
                        </a>
                        <span class="admin-nav-subsection-label">Approvals & Reconciliation</span>
                        <a href="/admin/marketplace/approvals.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/approvals.html"]) ? "active" : ""}" id="nav-mp-approvals">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M9 12l2 2 4-4"/></svg>
                            <span>MP Approvals</span>
                        </a>
                        <a href="/admin/marketplace/primary-escrow.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/primary-escrow.html"]) ? "active" : ""}" id="nav-mp-primary-escrow">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="6" width="18" height="14" rx="2"/><path d="M16 10a4 4 0 01-8 0"/><path d="M9 6V4a3 3 0 016 0v2"/></svg>
                            <span>Primary Escrow</span>
                        </a>
                        <span class="admin-nav-subsection-label">Insights & Config</span>
                        <a href="/admin/marketplace/fees.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/fees.html"]) ? "active" : ""}" id="nav-mp-fees">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 015.83 1c0 2-3 3-3 3M12 17h.01"/></svg>
                            <span>Fees</span>
                        </a>
                        <a href="/admin/marketplace/alerts.html" class="admin-nav-item ${isPathActive(["/admin/marketplace/alerts.html"]) ? "active" : ""}" id="nav-mp-alerts" title="Alerts, Watchlist & Detection Rules">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0zM12 9v4M12 17h.01"/></svg>
                            <span>Alerts</span>
                            <span id="nav-mp-alerts-badge" style="margin-left:auto;font-size:10px;padding:1px 6px;border-radius:10px;background:var(--admin-danger);color:#fff;display:none;">0</span>
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
                        <a href="/admin/compliance.html" class="admin-nav-item ${isPathActive(["/admin/compliance.html"]) ? "active" : ""}" id="nav-compliance">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2L4 6v6c0 5 3.5 9 8 10 4.5-1 8-5 8-10V6l-8-4z"/><path d="M9 12l2 2 4-4"/></svg>
                            <span>Compliance Queue</span>
                        </a>
                        <a href="/admin/support.html" class="admin-nav-item ${isPathActive(["/admin/support.html", "/admin/support-ticket.html"]) ? "active" : ""}" id="nav-support">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M18 10c0 4.418-3.582 8-8 8a8 8 0 110-16c4.418 0 8 3.582 8 8z"/><path d="M10 14v.01M10 6v6"/></svg>
                            <span>Support</span>
                        </a>
                    </div>

                    <!-- Community (collapsible nav group) -->
                    <div class="admin-nav-section admin-nav-group-section" data-group-id="community">
                        <button type="button" class="admin-nav-item admin-nav-group-trigger ${isPathActive(["/admin/community/", "/admin/community/index.html", "/admin/community/announcements.html", "/admin/community/posts.html", "/admin/community/post-detail.html", "/admin/community/comments.html", "/admin/community/reports.html", "/admin/community/appeals.html", "/admin/community/users.html", "/admin/community/user-detail.html", "/admin/community/badges.html", "/admin/community/amas.html", "/admin/community/challenges.html", "/admin/community/circles.html", "/admin/community/circle-detail.html", "/admin/community/leaderboard.html"]) ? "active" : ""}" id="nav-community-trigger">
                            <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 00-3-3.87"/><path d="M16 3.13a4 4 0 010 7.75"/></svg>
                            <span>Community</span>
                            <svg class="admin-nav-group-caret" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="6 9 12 15 18 9"></polyline></svg>
                        </button>
                        <div class="admin-nav-group-items">
                            <a href="/admin/community/" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/", "/admin/community/index.html"]) ? "active" : ""}" id="nav-com-overview">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/></svg>
                                <span>Overview</span>
                            </a>
                            <a href="/admin/community/announcements.html" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/announcements.html"]) ? "active" : ""}" id="nav-com-announcements">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
                                <span>Announcements</span>
                            </a>
                            <a href="/admin/community/posts.html" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/posts.html", "/admin/community/post-detail.html"]) ? "active" : ""}" id="nav-com-posts">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path></svg>
                                <span>Posts</span>
                            </a>
                            <a href="/admin/community/comments.html" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/comments.html"]) ? "active" : ""}" id="nav-com-comments">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"></path></svg>
                                <span>Comments</span>
                            </a>
                            <a href="/admin/community/reports.html" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/reports.html"]) ? "active" : ""}" id="nav-com-reports">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>
                                <span>Moderation Queue</span>
                                <span id="com-reports-badge" class="admin-nav-badge admin-nav-badge--danger" style="display:none"></span>
                            </a>
                            <a href="/admin/community/appeals.html" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/appeals.html"]) ? "active" : ""}" id="nav-com-appeals">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><polyline points="10 9 9 9 8 9"/></svg>
                                <span>Ban Appeals</span>
                            </a>
                            <a href="/admin/community/users.html" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/users.html", "/admin/community/user-detail.html"]) ? "active" : ""}" id="nav-com-users">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M23 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path></svg>
                                <span>Community Users</span>
                            </a>
                            <a href="/admin/community/badges.html" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/badges.html"]) ? "active" : ""}" id="nav-com-badges">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="8" r="7"></circle><polyline points="8.21 13.89 7 23 12 20 17 23 15.79 13.88"></polyline></svg>
                                <span>Badges</span>
                            </a>
                            <a href="/admin/community/amas.html" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/amas.html"]) ? "active" : ""}" id="nav-com-amas">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"></path><path d="M19 10v2a7 7 0 0 1-14 0v-2"></path><line x1="12" y1="19" x2="12" y2="23"></line><line x1="8" y1="23" x2="16" y2="23"></line></svg>
                                <span>Expert AMAs</span>
                            </a>
                            <a href="/admin/community/challenges.html" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/challenges.html"]) ? "active" : ""}" id="nav-com-challenges">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon></svg>
                                <span>Challenges</span>
                            </a>
                            <a href="/admin/community/circles.html" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/circles.html", "/admin/community/circle-detail.html"]) ? "active" : ""}" id="nav-com-circles">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><circle cx="12" cy="12" r="3"></circle></svg>
                                <span>Circles</span>
                            </a>
                            <a href="/admin/community/leaderboard.html" class="admin-nav-item admin-nav-sub-item ${isPathActive(["/admin/community/leaderboard.html"]) ? "active" : ""}" id="nav-com-leaderboard">
                                <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/><line x1="2" y1="20" x2="22" y2="20"/></svg>
                                <span>Leaderboard</span>
                            </a>
                        </div>
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
                            <span id="approvals-badge" class="admin-nav-badge admin-nav-badge--danger" style="display:none"></span>
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
                    <div class="admin-sidebar-user-menu" id="admin-sidebar-user-menu">
                        <button type="button" class="admin-sidebar-user" id="admin-sidebar-user-btn" aria-haspopup="menu" aria-expanded="false">
                            <span class="admin-sidebar-avatar-wrap">
                                <img src="${cachedAvatar}" alt="${cachedName}" class="admin-sidebar-avatar" id="sidebar-user-avatar" onerror="this.style.display='none'">
                                <span class="admin-sidebar-avatar-status admin-sidebar-avatar-status--online" id="sidebar-user-status" aria-label="Online" title="Online"></span>
                            </span>
                            <div class="admin-sidebar-user-info">
                                <div class="admin-sidebar-user-name" id="sidebar-user-name">${cachedName}</div>
                                <div class="admin-sidebar-user-role" id="sidebar-user-role">${cachedRole}</div>
                            </div>
                            <svg class="admin-sidebar-user-caret" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="18 15 12 9 6 15"/></svg>
                        </button>
                        <div class="admin-sidebar-user-popover" id="admin-sidebar-user-popover" role="menu">
                            <a href="/admin/profile" class="admin-sidebar-user-popover__item" role="menuitem">Profile</a>
                            <a href="/admin/audit-logs" class="admin-sidebar-user-popover__item" role="menuitem">My audit log</a>
                            <a href="/admin/" class="admin-sidebar-user-popover__item" role="menuitem">Switch account</a>
                            <div class="admin-sidebar-user-popover__sep"></div>
                            <button type="button" class="admin-sidebar-user-popover__item" id="admin-theme-toggle" role="menuitemcheckbox" aria-checked="false" style="display:flex;align-items:center;justify-content:space-between;gap:8px;">
                                <span style="display:inline-flex;align-items:center;gap:8px;">
                                    <span class="admin-theme-toggle-icon admin-theme-toggle-icon--sun" aria-hidden="true">
                                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/></svg>
                                    </span>
                                    <span class="admin-theme-toggle-icon admin-theme-toggle-icon--moon" aria-hidden="true">
                                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z"/></svg>
                                    </span>
                                    <span id="theme-toggle-label">Light mode</span>
                                </span>
                                <span class="admin-theme-toggle-track" aria-hidden="true"><span class="admin-theme-toggle-thumb"></span></span>
                            </button>
                            <div class="admin-sidebar-user-popover__sep"></div>
                            <button type="button" class="admin-sidebar-user-popover__item admin-sidebar-user-popover__item--danger" id="admin-sidebar-logout" role="menuitem">Sign out</button>
                        </div>
                    </div>
                </div>
            </aside>
        `;

        sidebarPlaceholder.innerHTML = sidebarHtml;

        // Mark active nav item for screen readers
        sidebarPlaceholder.querySelectorAll(".admin-nav-item.active").forEach(el => {
            el.setAttribute("aria-current", "page");
        });

        // Notify the permission guard (and any other listeners) that the
        // sidebar DOM is now available so they can apply visibility rules.
        document.dispatchEvent(new CustomEvent("admin:sidebar-ready"));

        // Sidebar collapse (#25)
        const COLLAPSE_KEY = "admin_sidebar_collapsed";
        const applyCollapsed = (on) => {
            document.body.classList.toggle("admin-sidebar-collapsed", on);
            const btn = document.getElementById("admin-sidebar-collapse");
            if (btn) {
                btn.setAttribute("aria-label", on ? "Expand sidebar" : "Collapse sidebar");
                btn.setAttribute("aria-expanded", on ? "false" : "true");
            }
        };
        applyCollapsed(localStorage.getItem(COLLAPSE_KEY) === "1");
        document.getElementById("admin-sidebar-collapse")?.addEventListener("click", () => {
            const next = !document.body.classList.contains("admin-sidebar-collapsed");
            localStorage.setItem(COLLAPSE_KEY, next ? "1" : "0");
            applyCollapsed(next);
        });
        document.addEventListener("keydown", (e) => {
            if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "b") {
                e.preventDefault();
                const next = !document.body.classList.contains("admin-sidebar-collapsed");
                localStorage.setItem(COLLAPSE_KEY, next ? "1" : "0");
                applyCollapsed(next);
            }
        });

        // Profile popover (#26)
        const userBtn = document.getElementById("admin-sidebar-user-btn");
        const userPop = document.getElementById("admin-sidebar-user-popover");
        const userMenu = document.getElementById("admin-sidebar-user-menu");
        if (userBtn && userPop && userMenu) {
            userBtn.addEventListener("click", (e) => {
                e.stopPropagation();
                const open = userMenu.classList.toggle("is-open");
                userBtn.setAttribute("aria-expanded", open ? "true" : "false");
            });
            document.addEventListener("click", (e) => {
                if (!userMenu.contains(e.target)) {
                    userMenu.classList.remove("is-open");
                    userBtn.setAttribute("aria-expanded", "false");
                }
            });
        }
        document.getElementById("admin-sidebar-logout")?.addEventListener("click", async () => {
            try {
                await fetch("/api/auth/logout", { method: "POST", credentials: "same-origin" });
            } catch (_) { /* ignore */ }
            window.location.href = "/login";
        });

        // Sync theme label immediately. Label is the action ("Switch to X")
        // so it's unambiguous what clicking will do.
        const syncThemeLabel = () => {
            const isDark = document.documentElement.classList.contains("admin-dark");
            const label = document.getElementById("theme-toggle-label");
            const btn = document.getElementById("admin-theme-toggle");
            if (label) label.textContent = isDark ? "Switch to Light" : "Switch to Dark";
            if (btn) {
                btn.setAttribute("aria-label", isDark ? "Switch to light mode" : "Switch to dark mode");
                btn.setAttribute("aria-checked", isDark ? "true" : "false");
            }
        };
        syncThemeLabel();
        const themeToggle = document.getElementById("admin-theme-toggle");
        if (themeToggle) {
            // Defer to next tick so admin-theme.js has flipped the class first
            themeToggle.addEventListener("click", () => setTimeout(syncThemeLabel, 0));
        }

        // Update sidebar name/avatar from a user data object
        function updateSidebarUser(userData) {
            const nameEl = document.getElementById("sidebar-user-name");
            const roleEl = document.getElementById("sidebar-user-role");
            const avatarEl = document.getElementById("sidebar-user-avatar");
            if (nameEl) nameEl.textContent = userData.full_name || "Admin User";
            if (roleEl && userData.role) roleEl.textContent = userData.role;
            if (avatarEl && userData.profile_image) avatarEl.src = userData.profile_image;
        }

        // Apply synchronously if already available, and listen for async load
        if (window.userData) updateSidebarUser(window.userData);
        document.addEventListener("admin:user-loaded", (e) => updateSidebarUser(e.detail));
    }

    // Dynamically update notification bell badge count across all admin pages
    async function updateNotificationBadges() {
        const canFetchBadge = (permission) => {
            const perms = window.adminPermissions;
            return !!(perms && perms.loaded && perms.has(permission));
        };

        const setBadge = (id, n) => {
            const badge = document.getElementById(id);
            if (!badge) return;
            if (n > 0) {
                badge.textContent = n > 99 ? "99+" : n;
                badge.style.display = "";
            } else {
                badge.style.display = "none";
            }
        };
        const fetchJson = async (url) => {
            try {
                const r = await fetch(url);
                return r.ok ? await r.json() : null;
            } catch (e) {
                return null;
            }
        };

        const tasks = [
            fetchJson("/api/admin/notifications").then(data => {
                if (!data) return;
                const unread = (data.notifications || []).filter(n => !n.is_read).length;
                document.querySelectorAll(".admin-notification-badge").forEach(badge => {
                    if (unread > 0) {
                        badge.textContent = unread > 99 ? "99+" : unread;
                        badge.style.display = "";
                    } else {
                        badge.style.display = "none";
                    }
                });
            }),
            fetchJson("/api/admin/approvals").then(data => {
                if (!data) return;
                setBadge("approvals-badge", data.pending_count || 0);
            }),
            fetchJson("/api/admin/change-requests").then(data => {
                if (!data) return;
                setBadge("change-requests-badge", data.pending_count || 0);
            }),
            fetchJson("/api/admin/community/reports").then(data => {
                if (!data) return;
                const pending = Array.isArray(data)
                    ? data.filter(r => r.status === "pending").length
                    : 0;
                setBadge("com-reports-badge", pending);
            }),
            fetchJson("/api/admin/marketplace/alerts").then(data => {
                if (!Array.isArray(data)) return;
                const unresolved = data.filter(a => !["resolved", "false_positive"].includes(a.status)).length;
                const badge = document.getElementById("nav-mp-alerts-badge");
                if (!badge) return;
                if (unresolved > 0) {
                    badge.textContent = unresolved > 99 ? "99+" : unresolved;
                    badge.style.display = "";
                    const critical = data.filter(a => (a.severity || "").toLowerCase() === "critical" && !["resolved", "false_positive"].includes(a.status)).length;
                    badge.style.background = critical > 0 ? "var(--admin-danger)" : "var(--admin-warning)";
                } else {
                    badge.style.display = "none";
                }
            }),
        ];

        if (canFetchBadge("affiliates.manage")) {
            tasks.push(
                fetchJson("/api/admin/rewards/affiliates/pending").then(data => {
                    if (!data) return;
                    setBadge("affiliate-apps-badge", (data.pending || []).length);
                })
            );
        }

        await Promise.all(tasks);
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
            // No saved position — scroll the active item into view inside the nav container
            const activeItem = sidebar.querySelector(".admin-nav-item.active");
            if (activeItem) {
                const itemTop = activeItem.offsetTop;
                const itemHeight = activeItem.offsetHeight;
                const viewHeight = sidebar.clientHeight;
                if (itemTop < sidebar.scrollTop + 16 || itemTop + itemHeight > sidebar.scrollTop + viewHeight - 16) {
                    sidebar.scrollTop = Math.max(0, itemTop - viewHeight / 2 + itemHeight / 2);
                }
            }
        }

        // Intercept all sidebar link clicks to save scroll position BEFORE navigation
        sidebar.querySelectorAll("a.admin-nav-item").forEach(link => {
            link.addEventListener("click", () => {
                sessionStorage.setItem(SCROLL_KEY, sidebar.scrollTop);
            });
        });
    }

    // Restore once the sidebar HTML is injected (admin:sidebar-ready fires in loadSidebar)
    document.addEventListener("admin:sidebar-ready", restoreSidebarScroll);

    // Run on script load if placeholder exists, or on DOMContentLoaded
    function bootstrap() {
        loadSidebar();
        updateNotificationBadges();
        injectHealthPill();
        wireSidebarCollapse();
        wireNavGroups();
        wireAvatarStatus();
    }

    // ── Avatar online/away status (heuristic) ────────────────────────────
    function wireAvatarStatus() {
        let lastActivity = Date.now();
        const bump = () => { lastActivity = Date.now(); };
        ["mousemove", "keydown", "touchstart", "scroll"].forEach((ev) =>
            window.addEventListener(ev, bump, { passive: true }));
        document.addEventListener("visibilitychange", bump);

        function tick() {
            const dot = document.getElementById("sidebar-user-status");
            if (!dot) return;
            const idle = Date.now() - lastActivity;
            const hidden = document.visibilityState === "hidden";
            let cls = "online", label = "Online";
            if (hidden || idle > 30 * 60_000) { cls = "offline"; label = "Away >30m"; }
            else if (idle > 5 * 60_000) { cls = "away"; label = "Idle"; }
            dot.classList.remove(
                "admin-sidebar-avatar-status--online",
                "admin-sidebar-avatar-status--away",
                "admin-sidebar-avatar-status--offline"
            );
            dot.classList.add(`admin-sidebar-avatar-status--${cls}`);
            dot.setAttribute("aria-label", label);
            dot.setAttribute("title", label);
        }
        // Avatar status / badge ticker — paused when tab hidden.
        let tickTimer = setInterval(tick, 15_000);
        document.addEventListener("visibilitychange", () => {
            if (document.visibilityState === "hidden") {
                if (tickTimer) { clearInterval(tickTimer); tickTimer = null; }
            } else if (!tickTimer) {
                tick();
                tickTimer = setInterval(tick, 15_000);
            }
        });
        tick();
    }
    if (document.getElementById("admin-sidebar-placeholder")) {
        bootstrap();
    } else {
        document.addEventListener("DOMContentLoaded", bootstrap);
    }
    document.addEventListener("admin:permissions-loaded", updateNotificationBadges);

    // ── Global System Health Pill ────────────────────────────────────────
    async function injectHealthPill() {
        const topbar = document.querySelector(".admin-topbar");
        if (!topbar) return;
        let right = topbar.querySelector(".admin-topbar-right");
        if (!right) {
            right = document.createElement("div");
            right.className = "admin-topbar-right";
            topbar.appendChild(right);
        }
        if (right.querySelector(".admin-health-pill")) return;

        const pill = document.createElement("button");
        pill.type = "button";
        pill.className = "admin-health-pill admin-health-pill--unknown";
        pill.setAttribute("aria-label", "System health");
        pill.setAttribute("title", "System health (click for details)");
        pill.innerHTML = `<span class="admin-health-dot" aria-hidden="true"></span><span class="admin-health-label">Health</span>`;
        right.insertBefore(pill, right.firstChild);

        const popover = document.createElement("div");
        popover.className = "admin-health-popover";
        popover.hidden = true;
        popover.setAttribute("role", "dialog");
        popover.innerHTML = `<div class="admin-health-popover-title">System Health</div><div class="admin-health-popover-body">Checking…</div>`;
        right.appendChild(popover);

        pill.addEventListener("click", (e) => {
            e.stopPropagation();
            popover.hidden = !popover.hidden;
        });
        document.addEventListener("click", (e) => {
            if (!popover.hidden && !popover.contains(e.target) && e.target !== pill) {
                popover.hidden = true;
            }
        });

        async function probe() {
            try {
                const r = await fetch("/health", { cache: "no-store" });
                const data = await r.json().catch(() => ({}));
                const ok = r.ok && data.status === "ok";
                const degraded = r.ok && data.status === "degraded";
                pill.classList.remove(
                    "admin-health-pill--ok",
                    "admin-health-pill--degraded",
                    "admin-health-pill--down",
                    "admin-health-pill--unknown"
                );
                pill.classList.add(
                    ok ? "admin-health-pill--ok" :
                    degraded ? "admin-health-pill--degraded" :
                    "admin-health-pill--down"
                );

                // Env-aware logo pill (PROD red, STAGING amber, DEV neutral)
                paintEnvPill(data.app_env || "development");
                const lab = pill.querySelector(".admin-health-label");
                lab.textContent = ok ? "OK" : degraded ? "Degraded" : "Down";

                const c = data.components || {};
                const env = c.env || {};
                const item = (label, val) => {
                    const cls =
                        val === "ok" ? "ok" :
                        val === "not_configured" ? "warn" :
                        val === "missing" || val === "error" ? "down" : "unknown";
                    return `<li><span class="admin-health-item-dot admin-health-item-dot--${cls}"></span>${label}<span class="admin-health-item-val">${val || "—"}</span></li>`;
                };
                popover.querySelector(".admin-health-popover-body").innerHTML = `
                    <ul class="admin-health-items">
                        ${item("Database", c.database || (r.ok ? "ok" : "error"))}
                        ${item("Redis", c.redis)}
                        ${item("Encryption key", env.TOTP_SECRET_ENCRYPTION_KEY_OR_ENCRYPTION_KEY)}
                        ${item("Session secret", env.SESSION_SECRET_OR_JWT_SECRET)}
                    </ul>
                    <div class="admin-health-version">v${data.version || "?"} · ${new Date().toLocaleTimeString()}</div>`;
            } catch (e) {
                pill.classList.add("admin-health-pill--down");
                pill.querySelector(".admin-health-label").textContent = "Down";
                popover.querySelector(".admin-health-popover-body").innerHTML =
                    `<div class="admin-health-error">Probe failed: ${e.message}</div>`;
            }
        }
        probe();
        let probeTimer = setInterval(probe, 60_000);
        document.addEventListener("visibilitychange", () => {
            if (document.visibilityState === "hidden") {
                if (probeTimer) { clearInterval(probeTimer); probeTimer = null; }
            } else if (!probeTimer) {
                probe();
                probeTimer = setInterval(probe, 60_000);
            }
        });
    }

    // ── Env-aware logo pill ───────────────────────────────────────────────
    function paintEnvPill(env) {
        const pill = document.getElementById("admin-sidebar-env-pill");
        if (!pill) return;
        const e = String(env).toLowerCase();
        let label = "Admin";
        let bg = "";
        let color = "";
        let title = `Environment: ${env}`;
        if (e === "production" || e === "prod") {
            label = "PROD";
            bg = "#dc2626"; color = "#fff";
            title = "PRODUCTION — actions affect live users. Tread carefully.";
        } else if (e === "staging" || e === "stage") {
            label = "STAGING";
            bg = "#d97706"; color = "#fff";
        } else if (e === "development" || e === "dev" || e === "local") {
            label = "DEV";
            bg = "#3b82f6"; color = "#fff";
        } else {
            label = String(env).toUpperCase().slice(0, 8);
        }
        pill.textContent = label;
        pill.dataset.env = e;
        pill.title = title;
        if (bg) {
            pill.style.background = bg;
            pill.style.color = color;
            pill.style.fontWeight = "700";
            pill.style.letterSpacing = "0.05em";
        }
    }

    // ── Collapsible nav groups (dropdown parent items, persists in localStorage) ──
    function wireNavGroups() {
        const sidebar = document.getElementById("main-admin-sidebar");
        if (!sidebar) return;
        const KEY = "admin_sidebar_nav_groups_collapsed";
        const collapsed = new Set(JSON.parse(localStorage.getItem(KEY) || "[]"));

        sidebar.querySelectorAll(".admin-nav-group-section[data-group-id]").forEach((section) => {
            const groupId = section.dataset.groupId;
            const trigger = section.querySelector(".admin-nav-group-trigger");
            if (!trigger) return;

            // Auto-expand if a child page is currently active
            const hasActive = section.querySelector(".admin-nav-group-items .admin-nav-item.active");
            if (hasActive) {
                collapsed.delete(groupId);
            }

            const isCollapsed = collapsed.has(groupId);
            trigger.setAttribute("aria-expanded", String(!isCollapsed));
            if (isCollapsed) section.classList.add("admin-nav-group--collapsed");

            trigger.addEventListener("click", () => {
                const nowCollapsed = section.classList.toggle("admin-nav-group--collapsed");
                trigger.setAttribute("aria-expanded", String(!nowCollapsed));
                if (nowCollapsed) collapsed.add(groupId);
                else collapsed.delete(groupId);
                localStorage.setItem(KEY, JSON.stringify([...collapsed]));
            });
        });

        // Persist final collapsed state (after auto-expand adjustments)
        localStorage.setItem(KEY, JSON.stringify([...collapsed]));
    }

    // ── Collapsible sidebar sections (persists open/closed in localStorage) ──
    function wireSidebarCollapse() {
        const sidebar = document.getElementById("main-admin-sidebar");
        if (!sidebar) return;
        const KEY = "admin_sidebar_collapsed_sections";
        const collapsed = new Set(JSON.parse(localStorage.getItem(KEY) || "[]"));

        sidebar.querySelectorAll(".admin-nav-section").forEach((sec) => {
            const label = sec.querySelector(".admin-nav-section-label");
            if (!label) return;
            const id = label.textContent.trim();
            const btn = document.createElement("button");
            btn.type = "button";
            btn.className = "admin-nav-section-toggle";
            btn.setAttribute("aria-expanded", String(!collapsed.has(id)));
            btn.innerHTML = `<span>${id}</span><span class="admin-nav-section-caret" aria-hidden="true">▾</span>`;
            label.replaceWith(btn);
            if (collapsed.has(id)) sec.classList.add("admin-nav-section--collapsed");
            btn.addEventListener("click", () => {
                const isCollapsed = sec.classList.toggle("admin-nav-section--collapsed");
                btn.setAttribute("aria-expanded", String(!isCollapsed));
                if (isCollapsed) collapsed.add(id);
                else collapsed.delete(id);
                localStorage.setItem(KEY, JSON.stringify([...collapsed]));
            });
        });
    }
})();
