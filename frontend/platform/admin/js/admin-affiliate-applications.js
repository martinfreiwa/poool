/**
 * Admin Affiliate Applications — JS Controller
 * Fetches pending applications, renders them, and handles approve/reject flows.
 */
(function () {
  'use strict';

  let pendingApps = [];
  let currentAppId = null;
  let lastFocusedBeforeModal = null;

  const FOCUSABLE_SELECTOR = [
    'a[href]',
    'button:not([disabled])',
    'textarea:not([disabled])',
    'input:not([disabled])',
    'select:not([disabled])',
    '[tabindex]:not([tabindex="-1"])'
  ].join(',');

  // ── Load pending applications ────────────────────────────────
  async function loadPending() {
    try {
      const res = await fetch('/api/admin/rewards/affiliates/pending');
      if (!res.ok) throw new Error('Failed to fetch');
      const data = await res.json();
      pendingApps = data.pending || [];
      renderPending();
      updateKPIs(data.counts || {});
    } catch (err) {
      console.error('Failed to load affiliate applications:', err);
      document.getElementById('pending-body').innerHTML =
        '<tr><td colspan="7" style="text-align:center;padding:32px;color:var(--admin-text-muted);">Failed to load applications. Please refresh.</td></tr>';
    }
  }

  // ── Render table ─────────────────────────────────────────────
  function renderPending() {
    const tbody = document.getElementById('pending-body');
    const countEl = document.getElementById('pending-count');

    if (!pendingApps.length) {
      tbody.innerHTML =
        '<tr><td colspan="7" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No pending applications 🎉</td></tr>';
      countEl.textContent = '0 pending';
      return;
    }

    countEl.textContent = `${pendingApps.length} pending`;

    tbody.innerHTML = pendingApps.map(app => {
      const date = app.created_at ? new Date(app.created_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' }) : '—';
      const url = app.main_url
        ? `<a href="${escapeHtml(app.main_url)}" target="_blank" rel="noopener" style="color:var(--admin-accent);text-decoration:none;">${truncate(app.main_url, 30)}</a>`
        : '<span style="color:var(--admin-text-muted)">—</span>';

      return `<tr>
        <td>
          <div style="font-weight:500;color:var(--admin-text-primary);font-size:13px;">${escapeHtml(app.email || '—')}</div>
          <div style="font-size:11px;color:var(--admin-text-muted);margin-top:2px;">${escapeHtml(app.id || '').substring(0, 8)}…</div>
        </td>
        <td><span class="admin-badge admin-badge--info" style="font-size:11px;">${escapeHtml(app.traffic_source || '—')}</span></td>
        <td style="color:var(--admin-text-secondary);font-size:13px;">${escapeHtml(app.audience_size || '—')}</td>
        <td>${url}</td>
        <td style="color:var(--admin-text-secondary);font-size:13px;">${escapeHtml(app.company_name || '—')}</td>
        <td style="color:var(--admin-text-muted);font-size:12px;">${date}</td>
        <td>
          <div style="display:flex;gap:6px;">
            <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="openDetailsModal('${app.id}')">
              <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M7 2a5 5 0 100 10A5 5 0 007 2zm0 0l5 5"/></svg>
              Review Application
            </button>
          </div>
        </td>
      </tr>`;
    }).join('');
  }

  // ── KPI Update ───────────────────────────────────────────────
  function updateKPIs(counts) {
    document.getElementById('kpi-pending').textContent = counts.pending ?? pendingApps.length;
    document.getElementById('kpi-active').textContent = counts.active ?? '—';
    document.getElementById('kpi-rejected').textContent = counts.rejected ?? '—';
  }

  // ── Approve Modal ────────────────────────────────────────────
  window.openApproveModal = function (appId, email) {
    currentAppId = appId;
    document.getElementById('approve-modal-email').textContent = email;
    document.getElementById('approve-referral-code').value = '';
    document.getElementById('approve-commission-rate').value = '50';
    openModal(document.getElementById('approve-modal'));
  };

  window.closeApproveModal = function () {
    closeModal(document.getElementById('approve-modal'));
    currentAppId = null;
  };

  window.confirmApprove = async function () {
    const code = document.getElementById('approve-referral-code').value.trim().toUpperCase();
    const rate = parseInt(document.getElementById('approve-commission-rate').value);

    if (!code) return alert('Referral code is required.');
    if (isNaN(rate) || rate < 1 || rate > 450) return alert('Commission rate must be 1–450 bps.');

    const btn = document.getElementById('approve-confirm-btn');
    btn.disabled = true;
    btn.textContent = 'Approving…';

    try {
      const res = await fetch(`/api/admin/rewards/affiliates/${currentAppId}/approve`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ referral_code: code, commission_rate_bps: rate })
      });

      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.error || 'Approval failed');
      }

      closeApproveModal();
      loadPending(); // Refresh the list
    } catch (err) {
      alert('Error: ' + err.message);
    } finally {
      btn.disabled = false;
      btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 8l3.5 3.5L13 4"/></svg> Approve';
    }
  };

  // ── Reject Modal ─────────────────────────────────────────────
  window.openRejectModal = function (appId, email) {
    currentAppId = appId;
    document.getElementById('reject-modal-email').textContent = email;
    document.getElementById('reject-reason').value = '';
    openModal(document.getElementById('reject-modal'));
  };

  window.closeRejectModal = function () {
    closeModal(document.getElementById('reject-modal'));
    currentAppId = null;
  };

  window.confirmReject = async function () {
    const reason = document.getElementById('reject-reason').value.trim();
    if (!reason) return alert('A rejection reason is required.');

    const btn = document.getElementById('reject-confirm-btn');
    btn.disabled = true;
    btn.textContent = 'Rejecting…';

    try {
      const res = await fetch(`/api/admin/rewards/affiliates/${currentAppId}/reject`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ reason })
      });

      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.error || 'Rejection failed');
      }

      closeRejectModal();
      loadPending();
    } catch (err) {
      alert('Error: ' + err.message);
    } finally {
      btn.disabled = false;
      btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg> Reject';
    }
  };

  // ── Helpers ──────────────────────────────────────────────────
  function escapeHtml(str) {
    return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  function truncate(str, len) {
    if (!str) return '';
    return str.length > len ? str.substring(0, len) + '…' : str;
  }

  function getFocusableElements(modal) {
    return Array.from(modal.querySelectorAll(FOCUSABLE_SELECTOR))
      .filter(el => el.offsetParent !== null || el === document.activeElement);
  }

  function openModal(modal) {
    lastFocusedBeforeModal = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    modal.style.display = 'flex';
    const focusTarget = getFocusableElements(modal)[0] || modal;
    focusTarget.focus({ preventScroll: true });
  }

  function closeModal(modal) {
    modal.style.display = 'none';
    if (lastFocusedBeforeModal && document.contains(lastFocusedBeforeModal)) {
      lastFocusedBeforeModal.focus({ preventScroll: true });
    }
    lastFocusedBeforeModal = null;
  }

  function closeActiveModal() {
    const activeModal = document.querySelector('#details-modal[style*="display: flex"], #approve-modal[style*="display: flex"], #reject-modal[style*="display: flex"]');
    if (!activeModal) return false;

    if (activeModal.id === 'details-modal') closeDetailsModal();
    if (activeModal.id === 'approve-modal') closeApproveModal();
    if (activeModal.id === 'reject-modal') closeRejectModal();
    return true;
  }

  function trapFocus(event) {
    if (event.key === 'Escape') {
      closeActiveModal();
      return;
    }

    if (event.key !== 'Tab') return;

    const activeModal = document.querySelector('#details-modal[style*="display: flex"], #approve-modal[style*="display: flex"], #reject-modal[style*="display: flex"]');
    if (!activeModal) return;

    const focusable = getFocusableElements(activeModal);
    if (!focusable.length) {
      event.preventDefault();
      activeModal.focus({ preventScroll: true });
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus({ preventScroll: true });
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus({ preventScroll: true });
    }
  }

  // ── Details Modal ────────────────────────────────────────────
  window.openDetailsModal = function (appId) {
    const app = pendingApps.find(a => a.id === appId);
    if (!app) return;
    
    currentAppId = appId;
    document.getElementById('details-name').textContent = app.user_name || '—';
    document.getElementById('details-email').textContent = app.email || '—';
    document.getElementById('details-date').textContent = app.created_at ? new Date(app.created_at).toLocaleString() : '—';
    document.getElementById('details-traffic').textContent = app.traffic_source || '—';
    document.getElementById('details-audience').textContent = app.audience_size || '—';
    document.getElementById('details-url').innerHTML = app.main_url ? `<a href="${escapeHtml(app.main_url)}" target="_blank" style="color:var(--admin-accent)">${escapeHtml(app.main_url)}</a>` : '—';
    document.getElementById('details-company').textContent = app.company_name || '—';
    document.getElementById('details-tax').textContent = app.tax_id || '—';
    document.getElementById('details-phone').textContent = app.phone_number || '—';
    
    const approveBtn = document.getElementById('details-approve-btn');
    const rejectBtn = document.getElementById('details-reject-btn');
    
    // Unbind and rebind to prevent stale listeners
    approveBtn.onclick = () => { closeDetailsModal(); openApproveModal(app.id, app.email || ''); };
    rejectBtn.onclick = () => { closeDetailsModal(); openRejectModal(app.id, app.email || ''); };

    openModal(document.getElementById('details-modal'));
  };

  window.closeDetailsModal = function () {
    closeModal(document.getElementById('details-modal'));
  };

  // ── Close modals on backdrop click ───────────────────────────
  document.getElementById('approve-modal').addEventListener('click', function (e) {
    if (e.target === this) closeApproveModal();
  });
  document.getElementById('reject-modal').addEventListener('click', function (e) {
    if (e.target === this) closeRejectModal();
  });
  document.getElementById('details-modal').addEventListener('click', function (e) {
    if (e.target === this) closeDetailsModal();
  });
  document.addEventListener('keydown', trapFocus);

  // ── Init ─────────────────────────────────────────────────────
  loadPending();
})();
