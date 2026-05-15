/**
 * Admin Affiliate Applications — JS Controller
 * Fetches pending applications, renders them, and handles approve/reject flows.
 */
(function () {
  'use strict';

  let pendingApps = [];
  let currentAppId = null;
  let lastFocusedBeforeModal = null;
  let auditLog = [];

  const REFERRAL_CODE_PATTERN = /^[A-Z0-9_-]{3,20}$/;
  const REJECTION_REASON_MAX_LENGTH = 1000;

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
      if (!isValidPendingResponse(data)) {
        throw new Error('Unexpected affiliate applications response.');
      }

      pendingApps = data.pending;
      renderPending();
      updateKPIs(data.counts);
    } catch (err) {
      console.error('Failed to load affiliate applications:', err);
      renderStateRow('Failed to load applications. Please refresh.', 32);
    }
  }

  // ── Render table ─────────────────────────────────────────────
  function renderPending() {
    const tbody = document.getElementById('pending-body');
    const countEl = document.getElementById('pending-count');

    if (!pendingApps.length) {
      tbody.innerHTML =
        '<tr><td colspan="8" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No pending applications 🎉</td></tr>';
      countEl.textContent = '0 pending';
      return;
    }

    countEl.textContent = `${pendingApps.length} pending`;
    tbody.replaceChildren();

    pendingApps.forEach(app => {
      const date = app.created_at ? new Date(app.created_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' }) : '—';

      const row = document.createElement('tr');
      row.append(
        buildApplicantCell(app),
        buildRiskCell(app),
        buildTrafficCell(app),
        buildTextCell(app.audience_size || '—', 'color:var(--admin-text-secondary);font-size:13px;'),
        buildUrlCell(app.main_url),
        buildTextCell(app.company_name || '—', 'color:var(--admin-text-secondary);font-size:13px;'),
        buildTextCell(date, 'color:var(--admin-text-muted);font-size:12px;'),
        buildActionCell(app)
      );
      tbody.appendChild(row);
    });
  }

  // ── Risk cell from backend fraud_signals ─────────────────────
  function buildRiskCell(app) {
    const cell = document.createElement('td');
    const signals = Array.isArray(app.fraud_signals) ? app.fraud_signals : [];
    if (!signals.length) {
      const ok = applyStyle(document.createElement('span'), 'font-size:11px;padding:2px 8px;border-radius:10px;background:rgba(34,197,94,0.12);color:var(--admin-success);font-weight:600;');
      ok.textContent = 'Clean';
      ok.title = 'No backend risk signals detected';
      cell.appendChild(ok);
      return cell;
    }
    const score = signals.reduce((sum, s) => sum + (Number(s.score) || 0), 0);
    let color = 'var(--admin-text-secondary)';
    let label = `Minor (${score})`;
    if (score >= 50) { color = 'var(--admin-danger)'; label = `High (${score})`; }
    else if (score >= 25) { color = 'var(--admin-warning)'; label = `Suspicious (${score})`; }

    const badge = applyStyle(document.createElement('span'),
      `font-size:11px;padding:2px 8px;border-radius:10px;font-weight:600;cursor:help;background:${color}1a;color:${color};`);
    badge.textContent = label;
    badge.title = signals.map(s => `• ${s.label}`).join('\n');
    cell.appendChild(badge);
    return cell;
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
    clearApproveErrors();
    openModal(document.getElementById('approve-modal'));
  };

  window.closeApproveModal = function () {
    closeModal(document.getElementById('approve-modal'));
    currentAppId = null;
  };

  window.confirmApprove = async function () {
    const code = document.getElementById('approve-referral-code').value.trim().toUpperCase();
    const rate = parseInt(document.getElementById('approve-commission-rate').value);
    clearApproveErrors();

    if (!REFERRAL_CODE_PATTERN.test(code)) {
      setFieldError('approve-referral-error', 'Referral code must be 3-20 uppercase letters, numbers, underscores, or hyphens.');
      document.getElementById('approve-referral-code').focus();
      return;
    }
    if (isNaN(rate) || rate < 1 || rate > 450) {
      setFieldError('approve-commission-error', 'Commission rate must be 1-450 bps.');
      document.getElementById('approve-commission-rate').focus();
      return;
    }

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
      loadAuditLog();
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
    clearRejectErrors();
    openModal(document.getElementById('reject-modal'));
  };

  window.closeRejectModal = function () {
    closeModal(document.getElementById('reject-modal'));
    currentAppId = null;
  };

  window.confirmReject = async function () {
    const reason = document.getElementById('reject-reason').value.trim();
    clearRejectErrors();

    if (!reason) {
      setFieldError('reject-reason-error', 'A rejection reason is required.');
      document.getElementById('reject-reason').focus();
      return;
    }
    if (reason.length > REJECTION_REASON_MAX_LENGTH) {
      setFieldError('reject-reason-error', `Rejection reason must be ${REJECTION_REASON_MAX_LENGTH} characters or fewer.`);
      document.getElementById('reject-reason').focus();
      return;
    }

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
      loadAuditLog();
    } catch (err) {
      alert('Error: ' + err.message);
    } finally {
      btn.disabled = false;
      btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg> Reject';
    }
  };

  // ── Helpers ──────────────────────────────────────────────────
  function truncate(str, len) {
    if (!str) return '';
    return str.length > len ? str.substring(0, len) + '…' : str;
  }

  function isValidPendingResponse(data) {
    if (!data || !Array.isArray(data.pending) || !data.counts || typeof data.counts !== 'object') {
      return false;
    }

    return ['pending', 'active', 'rejected'].every(key =>
      Number.isInteger(data.counts[key]) && data.counts[key] >= 0
    );
  }

  function renderStateRow(message, padding) {
    const tbody = document.getElementById('pending-body');
    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 8;
    cell.style.cssText = `text-align:center;padding:${padding}px;color:var(--admin-text-muted);`;
    cell.textContent = message;
    row.appendChild(cell);
    tbody.replaceChildren(row);
  }

  function setFieldError(id, message) {
    const el = document.getElementById(id);
    if (!el) return;
    el.textContent = message;
    el.style.display = 'block';
  }

  function clearFieldError(id) {
    const el = document.getElementById(id);
    if (!el) return;
    el.textContent = '';
    el.style.display = 'none';
  }

  function clearApproveErrors() {
    clearFieldError('approve-referral-error');
    clearFieldError('approve-commission-error');
  }

  function clearRejectErrors() {
    clearFieldError('reject-reason-error');
  }

  function safeExternalUrl(value) {
    if (!value) return null;

    try {
      const url = new URL(String(value));
      return ['http:', 'https:'].includes(url.protocol) ? url.href : null;
    } catch (_) {
      return null;
    }
  }

  function applyStyle(el, style) {
    if (style) el.style.cssText = style;
    return el;
  }

  function buildTextCell(text, style) {
    const cell = document.createElement('td');
    applyStyle(cell, style);
    cell.textContent = text;
    return cell;
  }

  function buildApplicantCell(app) {
    const cell = document.createElement('td');
    const email = applyStyle(document.createElement('div'), 'font-weight:500;color:var(--admin-text-primary);font-size:13px;');
    email.textContent = app.email || '—';

    const id = applyStyle(document.createElement('div'), 'font-size:11px;color:var(--admin-text-muted);margin-top:2px;');
    id.textContent = `${String(app.id || '').substring(0, 8)}…`;

    cell.append(email, id);
    return cell;
  }

  function buildTrafficCell(app) {
    const cell = document.createElement('td');
    const badge = applyStyle(document.createElement('span'), 'font-size:11px;');
    badge.className = 'admin-badge admin-badge--info';
    badge.textContent = app.traffic_source || '—';
    cell.appendChild(badge);
    return cell;
  }

  function buildUrlCell(rawUrl) {
    const cell = document.createElement('td');
    const url = safeExternalUrl(rawUrl);

    if (!url) {
      const empty = applyStyle(document.createElement('span'), 'color:var(--admin-text-muted)');
      empty.textContent = '—';
      cell.appendChild(empty);
      return cell;
    }

    const link = applyStyle(document.createElement('a'), 'color:var(--admin-accent);text-decoration:none;');
    link.href = url;
    link.target = '_blank';
    link.rel = 'noopener';
    link.textContent = truncate(String(rawUrl), 30);
    cell.appendChild(link);
    return cell;
  }

  function buildActionCell(app) {
    const cell = document.createElement('td');
    const wrapper = applyStyle(document.createElement('div'), 'display:flex;gap:6px;');
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'admin-btn admin-btn--secondary admin-btn--sm';
    button.textContent = 'Review Application';
    button.addEventListener('click', () => openDetailsModal(app.id));
    wrapper.appendChild(button);
    cell.appendChild(wrapper);
    return cell;
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
    const activeModal = document.querySelector('#details-modal[style*="display: flex"], #approve-modal[style*="display: flex"], #reject-modal[style*="display: flex"], #info-modal[style*="display: flex"]');
    if (!activeModal) return false;

    if (activeModal.id === 'details-modal') closeDetailsModal();
    if (activeModal.id === 'approve-modal') closeApproveModal();
    if (activeModal.id === 'reject-modal') closeRejectModal();
    if (activeModal.id === 'info-modal') closeInfoModal();
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
    renderDetailsUrl(app.main_url);
    document.getElementById('details-company').textContent = app.company_name || '—';
    document.getElementById('details-tax').textContent = app.tax_id_last4 ? `***-**-${app.tax_id_last4}` : '—';
    document.getElementById('details-phone').textContent = app.phone_number || '—';

    // Backend fraud signals
    const fraudSection = document.getElementById('details-fraud-section');
    const fraudSignals = document.getElementById('details-fraud-signals');
    const signals = Array.isArray(app.fraud_signals) ? app.fraud_signals : [];
    if (signals.length) {
      fraudSection.style.display = 'block';
      fraudSignals.replaceChildren();
      signals.forEach(s => {
        const tag = document.createElement('span');
        tag.className = 'admin-badge admin-badge--warning';
        tag.style.cssText = 'font-size: 11px;';
        tag.textContent = `${s.label} (+${s.score})`;
        fraudSignals.appendChild(tag);
      });
    } else {
      fraudSection.style.display = 'none';
    }

    const approveBtn = document.getElementById('details-approve-btn');
    const rejectBtn = document.getElementById('details-reject-btn');
    const infoBtn = document.getElementById('details-info-btn');

    // Unbind and rebind to prevent stale listeners
    approveBtn.onclick = () => { closeDetailsModal(); openApproveModal(app.id, app.email || ''); };
    rejectBtn.onclick = () => { closeDetailsModal(); openRejectModal(app.id, app.email || ''); };
    infoBtn.onclick = () => { closeDetailsModal(); openInfoModal(app.id, app.email || ''); };

    openModal(document.getElementById('details-modal'));
  };

  // ── Request More Info Modal ──────────────────────────────────
  window.openInfoModal = function (appId, email) {
    currentAppId = appId;
    document.getElementById('info-modal-email').textContent = email;
    document.getElementById('info-message').value = '';
    document.getElementById('info-template').value = '';
    clearFieldError('info-message-error');
    openModal(document.getElementById('info-modal'));
  };

  window.closeInfoModal = function () {
    closeModal(document.getElementById('info-modal'));
    currentAppId = null;
  };

  window.confirmInfoRequest = async function () {
    const message = document.getElementById('info-message').value.trim();
    clearFieldError('info-message-error');

    if (!message) {
      setFieldError('info-message-error', 'A message is required.');
      document.getElementById('info-message').focus();
      return;
    }
    if (message.length > 1000) {
      setFieldError('info-message-error', 'Message must be 1000 characters or fewer.');
      document.getElementById('info-message').focus();
      return;
    }

    const btn = document.getElementById('info-confirm-btn');
    btn.disabled = true;
    btn.textContent = 'Sending…';

    try {
      const res = await fetch(`/api/admin/rewards/affiliates/${currentAppId}/request-info`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ message })
      });
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.error || 'Failed to send request');
      }
      closeInfoModal();
      loadAuditLog();
    } catch (err) {
      alert('Error: ' + err.message);
    } finally {
      btn.disabled = false;
      btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M2 4l6 4 6-4M2 4v8h12V4"/></svg> Send Request';
    }
  };

  // ── Audit log loader ─────────────────────────────────────────
  async function loadAuditLog() {
    const tbody = document.getElementById('audit-body');
    if (!tbody) return;
    try {
      const res = await fetch('/api/admin/audit-logs?action_prefix=affiliate.&per_page=20');
      if (!res.ok) throw new Error('Failed to fetch');
      const data = await res.json();
      auditLog = Array.isArray(data.logs) ? data.logs : [];
      renderAuditLog();
    } catch (err) {
      console.error('Failed to load audit log:', err);
      tbody.innerHTML = '<tr><td colspan="5" style="text-align:center;padding:24px;color:var(--admin-text-muted);font-size:12px;">Failed to load audit log.</td></tr>';
    }
  }

  function renderAuditLog() {
    const tbody = document.getElementById('audit-body');
    if (!tbody) return;
    if (!auditLog.length) {
      tbody.innerHTML = '<tr><td colspan="5" style="text-align:center;padding:24px;color:var(--admin-text-muted);font-size:12px;">No recent affiliate actions.</td></tr>';
      return;
    }
    tbody.replaceChildren();
    auditLog.forEach(entry => {
      const row = document.createElement('tr');
      row.append(
        buildAuditWhenCell(entry.created_at),
        buildAuditActionCell(entry.action),
        buildAuditAffiliateCell(entry.entity_id),
        buildTextCell(entry.actor_email || '—', 'font-size:12px;color:var(--admin-text-secondary);'),
        buildAuditDetailCell(entry)
      );
      tbody.appendChild(row);
    });
  }

  function buildAuditWhenCell(createdAt) {
    const cell = document.createElement('td');
    cell.style.cssText = 'font-size:12px;color:var(--admin-text-muted);';
    if (!createdAt) { cell.textContent = '—'; return cell; }
    const d = new Date(createdAt);
    cell.textContent = formatRelative(d);
    cell.title = d.toLocaleString();
    return cell;
  }

  function buildAuditActionCell(action) {
    const cell = document.createElement('td');
    const map = {
      'affiliate.approved': { color: 'var(--admin-success)', text: 'Approved' },
      'affiliate.rejected': { color: 'var(--admin-danger)', text: 'Rejected' },
      'affiliate.info_requested': { color: 'var(--admin-accent)', text: 'Info requested' },
      'affiliate.suspended': { color: 'var(--admin-warning)', text: 'Suspended' },
    };
    const cfg = map[action] || { color: 'var(--admin-text-secondary)', text: action || '—' };
    const badge = applyStyle(document.createElement('span'),
      `font-size:11px;padding:2px 8px;border-radius:10px;background:${cfg.color}1a;color:${cfg.color};font-weight:600;`);
    badge.textContent = cfg.text;
    cell.appendChild(badge);
    return cell;
  }

  function buildAuditAffiliateCell(entityId) {
    const cell = document.createElement('td');
    cell.style.cssText = 'font-size:12px;color:var(--admin-text-secondary);';
    if (!entityId) { cell.textContent = '—'; return cell; }
    const match = pendingApps.find(a => a.id === entityId);
    if (match) {
      cell.textContent = match.email || entityId;
    } else {
      cell.style.fontFamily = 'monospace';
      cell.textContent = `${String(entityId).substring(0, 8)}…`;
      cell.title = entityId;
    }
    return cell;
  }

  function buildAuditDetailCell(entry) {
    const cell = document.createElement('td');
    cell.style.cssText = 'font-size:12px;color:var(--admin-text-muted);max-width:320px;';
    let detail = '';
    if (entry.new_state && typeof entry.new_state === 'object') {
      if (entry.new_state.reason) detail = `Reason: ${entry.new_state.reason}`;
      else if (entry.new_state.message) detail = `Asked: ${entry.new_state.message}`;
      else if (entry.new_state.referral_code) detail = `Code: ${entry.new_state.referral_code}`;
    }
    cell.textContent = detail.length > 120 ? detail.substring(0, 120) + '…' : detail;
    if (detail.length > 120) cell.title = detail;
    return cell;
  }

  function formatRelative(date) {
    const sec = Math.floor((Date.now() - date.getTime()) / 1000);
    if (sec < 60) return `${sec}s ago`;
    if (sec < 3600) return `${Math.floor(sec / 60)}m ago`;
    if (sec < 86400) return `${Math.floor(sec / 3600)}h ago`;
    return `${Math.floor(sec / 86400)}d ago`;
  }

  window.closeDetailsModal = function () {
    closeModal(document.getElementById('details-modal'));
  };

  function renderDetailsUrl(rawUrl) {
    const container = document.getElementById('details-url');
    const url = safeExternalUrl(rawUrl);
    container.replaceChildren();

    if (!url) {
      container.textContent = '—';
      return;
    }

    const link = applyStyle(document.createElement('a'), 'color:var(--admin-accent)');
    link.href = url;
    link.target = '_blank';
    link.rel = 'noopener';
    link.textContent = String(rawUrl);
    container.appendChild(link);
  }

  // ── Close modals on backdrop click ───────────────────────────
  document.getElementById('details-close-btn').addEventListener('click', closeDetailsModal);
  document.getElementById('approve-cancel-btn').addEventListener('click', closeApproveModal);
  document.getElementById('approve-confirm-btn').addEventListener('click', confirmApprove);
  document.getElementById('reject-cancel-btn').addEventListener('click', closeRejectModal);
  document.getElementById('reject-confirm-btn').addEventListener('click', confirmReject);
  document.getElementById('info-cancel-btn')?.addEventListener('click', closeInfoModal);
  document.getElementById('info-confirm-btn')?.addEventListener('click', confirmInfoRequest);
  document.getElementById('info-template')?.addEventListener('change', e => {
    if (e.target.value) {
      const opt = e.target.options[e.target.selectedIndex];
      document.getElementById('info-message').value = opt.textContent;
    }
  });
  document.getElementById('approve-modal').addEventListener('click', function (e) {
    if (e.target === this) closeApproveModal();
  });
  document.getElementById('reject-modal').addEventListener('click', function (e) {
    if (e.target === this) closeRejectModal();
  });
  document.getElementById('details-modal').addEventListener('click', function (e) {
    if (e.target === this) closeDetailsModal();
  });
  document.getElementById('info-modal')?.addEventListener('click', function (e) {
    if (e.target === this) closeInfoModal();
  });
  document.getElementById('audit-refresh-btn')?.addEventListener('click', loadAuditLog);
  document.addEventListener('keydown', trapFocus);

  // ── Init ─────────────────────────────────────────────────────
  loadPending();
  loadAuditLog();
})();
