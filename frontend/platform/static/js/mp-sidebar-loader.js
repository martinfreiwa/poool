/**
 * Marketplace Sidebar Loader
 * Provides a separate sidebar for the /admin/marketplace/ section.
 */
(function () {
  function loadMarketplaceSidebar() {
    const placeholder = document.getElementById('admin-sidebar-placeholder');
    if (!placeholder) return;

    const currentPath = window.location.pathname;
    const isActive = (paths) => {
      const clean = currentPath.replace(/\/$/, '').replace('.html', '');
      return paths.some(p => clean === p.replace(/\/$/, '').replace('.html', ''));
    };

    placeholder.innerHTML = `
      <aside class="admin-sidebar" id="main-admin-sidebar">
        <div class="admin-sidebar-header">
          <a href="/admin/" class="admin-sidebar-logo">
            <img src="/static/images/Logo%20Pool.svg" alt="POOOL">
            <span class="admin-sidebar-logo-badge">Marketplace</span>
          </a>
        </div>

        <nav class="admin-sidebar-nav">
          <!-- Overview -->
          <div class="admin-nav-section">
            <a href="/admin/marketplace/" class="admin-nav-item ${isActive(['/admin/marketplace/', '/admin/marketplace/index']) ? 'active' : ''}" id="nav-mp-dashboard">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="6" height="6" rx="1.5"/><rect x="11" y="3" width="6" height="6" rx="1.5"/><rect x="3" y="11" width="6" height="6" rx="1.5"/><rect x="11" y="11" width="6" height="6" rx="1.5"/></svg>
              <span>Overview</span>
            </a>
          </div>

          <!-- Trading -->
          <div class="admin-nav-section">
            <span class="admin-nav-section-label">Trading</span>
            <a href="/admin/marketplace/orderbook.html" class="admin-nav-item ${isActive(['/admin/marketplace/orderbook']) ? 'active' : ''}">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M4 18v-4M8 18v-8M12 18V6M16 18V2M4 18h16"/></svg>
              <span>Orderbook</span>
            </a>
            <a href="/admin/marketplace/trades.html" class="admin-nav-item ${isActive(['/admin/marketplace/trades']) ? 'active' : ''}">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M7 7h10v10"/><path d="M7 17L17 7"/></svg>
              <span>Trade History</span>
            </a>
            <a href="/admin/marketplace/orders.html" class="admin-nav-item ${isActive(['/admin/marketplace/orders']) ? 'active' : ''}">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h4l2.5 12h11.5a2 2 0 001.9-1.3L23 5H5.5"/><circle cx="9" cy="20" r="2"/><circle cx="18" cy="20" r="2"/></svg>
              <span>Open Orders</span>
            </a>
            <a href="/admin/marketplace/p2p.html" class="admin-nav-item ${isActive(['/admin/marketplace/p2p']) ? 'active' : ''}">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2"/><circle cx="8.5" cy="7" r="4"/><path d="M20 8v6M23 11h-6"/></svg>
              <span>P2P Offers</span>
            </a>
          </div>

          <!-- Risk & Compliance -->
          <div class="admin-nav-section">
            <span class="admin-nav-section-label">Risk & Compliance</span>
            <a href="/admin/marketplace/approvals.html" class="admin-nav-item ${isActive(['/admin/marketplace/approvals']) ? 'active' : ''}">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M9 12l2 2 4-4"/></svg>
              <span>Approvals</span>
              <span class="admin-nav-badge admin-nav-badge--danger" id="mp-approvals-badge">3</span>
            </a>
            <a href="/admin/marketplace/alerts.html" class="admin-nav-item ${isActive(['/admin/marketplace/alerts']) ? 'active' : ''}">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0zM12 9v4M12 17h.01"/></svg>
              <span>Alerts</span>
              <span class="admin-nav-badge admin-nav-badge--warning" id="mp-alerts-badge">5</span>
            </a>
            <a href="/admin/marketplace/reconciliation.html" class="admin-nav-item ${isActive(['/admin/marketplace/reconciliation']) ? 'active' : ''}">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M9 11l3 3L22 4"/><path d="M21 12v7a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2h11"/></svg>
              <span>Reconciliation</span>
            </a>
            <a href="/admin/marketplace/compliance.html" class="admin-nav-item ${isActive(['/admin/marketplace/compliance']) ? 'active' : ''}">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="2" width="16" height="20" rx="2"/><path d="M8 6h8M8 10h8M8 14h4"/></svg>
              <span>Compliance</span>
            </a>
          </div>

          <!-- Configuration -->
          <div class="admin-nav-section">
            <span class="admin-nav-section-label">Configuration</span>
            <a href="/admin/marketplace/fees.html" class="admin-nav-item ${isActive(['/admin/marketplace/fees']) ? 'active' : ''}">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 015.83 1c0 2-3 3-3 3M12 17h.01"/></svg>
              <span>Fee Management</span>
            </a>
            <a href="/admin/marketplace/analytics.html" class="admin-nav-item ${isActive(['/admin/marketplace/analytics']) ? 'active' : ''}">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M18 20V10M12 20V4M6 20v-6"/></svg>
              <span>Analytics</span>
            </a>
            <a href="/admin/marketplace/settings.html" class="admin-nav-item ${isActive(['/admin/marketplace/settings']) ? 'active' : ''}">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M12 15a3 3 0 100-6 3 3 0 000 6z"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/></svg>
              <span>Settings</span>
            </a>
          </div>

          <!-- Back to Admin -->
          <div class="admin-nav-section" style="margin-top: auto; border-top: 1px solid var(--admin-sidebar-border); padding-top: 8px;">
            <a href="/admin/" class="admin-nav-item">
              <svg class="admin-nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"><path d="M19 12H5M12 19l-7-7 7-7"/></svg>
              <span>Back to Admin</span>
            </a>
          </div>
        </nav>

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

    document.dispatchEvent(new CustomEvent('admin:sidebar-ready'));

    // Sync theme label
    const isDark = document.documentElement.classList.contains('admin-dark');
    const label = document.getElementById('theme-toggle-label');
    if (label) label.textContent = isDark ? 'Dark' : 'Light';

    // Load user data
    if (window.userData) {
      const nameEl = document.getElementById('sidebar-user-name');
      const avatarEl = document.getElementById('sidebar-user-avatar');
      if (nameEl) nameEl.textContent = window.userData.full_name || 'Admin User';
      if (avatarEl && window.userData.profile_image) avatarEl.src = window.userData.profile_image;
    }
  }

  if (document.getElementById('admin-sidebar-placeholder')) {
    loadMarketplaceSidebar();
  } else {
    document.addEventListener('DOMContentLoaded', loadMarketplaceSidebar);
  }
})();
