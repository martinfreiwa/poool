/**
 * Alerts & Watchlist — mp-alerts.js
 * Fetches marketplace alerts from the backend API.
 * Falls back to mock data if the API is unavailable.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/alerts';
  let alerts = [];
  let usingMockData = false;

  // ── Mock Data ───────────────────────────────────────────────────
  const MOCK_ALERTS = [
    { id: 'ALR-001', severity: 'critical', type: 'Wash Trading', desc: 'Same IP address executing buy/sell on BVRT within 30s window', users: ['USR-8291', 'USR-4410'], detected: '12 min ago', status: 'new' },
    { id: 'ALR-002', severity: 'critical', type: 'Negative Balance', desc: 'User wallet balance went negative after concurrent order execution', users: ['USR-3384'], detected: '45 min ago', status: 'new' },
    { id: 'ALR-003', severity: 'warning', type: 'Price Manipulation', desc: 'Rapid price movement on SWHS — 15% spike in 5 minutes', users: ['USR-6643', 'USR-7829'], detected: '2h ago', status: 'new' },
    { id: 'ALR-004', severity: 'warning', type: 'Large Order', desc: 'Single order exceeds 10% of daily volume on JOTX', users: ['USR-1738'], detected: '3h ago', status: 'acknowledged' },
    { id: 'ALR-005', severity: 'warning', type: 'Unusual Pattern', desc: 'User placing and cancelling orders repeatedly (potential spoofing)', users: ['USR-5561'], detected: '5h ago', status: 'acknowledged' },
    { id: 'ALR-006', severity: 'critical', type: 'Settlement Failure', desc: 'On-chain settlement for trade TRD-100042 failed after 3 retries', users: ['USR-2201', 'USR-9987'], detected: '6h ago', status: 'new' },
    { id: 'ALR-007', severity: 'warning', type: 'KYC Mismatch', desc: 'Trading user has pending KYC re-verification — tier 2 required for volume', users: ['USR-7829'], detected: '8h ago', status: 'resolved' },
    { id: 'ALR-008', severity: 'warning', type: 'API Abuse', desc: 'Rate limit exceeded — 500+ API calls in 1 minute from same session', users: ['USR-4410'], detected: '12h ago', status: 'resolved' },
  ];

  function severityBadge(sev) {
    if (sev === 'critical') return '<span class="admin-badge admin-badge--danger"><span class="admin-badge-dot"></span>Critical</span>';
    if (sev === 'warning') return '<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>Warning</span>';
    return '<span class="admin-badge admin-badge--info"><span class="admin-badge-dot"></span>Info</span>';
  }

  function statusBadge(status) {
    if (status === 'new') return '<span class="admin-badge admin-badge--danger"><span class="admin-badge-dot"></span>New</span>';
    if (status === 'acknowledged') return '<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>Acknowledged</span>';
    if (status === 'false_positive') return '<span class="admin-badge admin-badge--neutral"><span class="admin-badge-dot"></span>False Pos</span>';
    return '<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>Resolved</span>';
  }

  function timeAgo(dateStr) {
    const diff = (Date.now() - new Date(dateStr).getTime()) / 1000;
    if (diff < 60) return Math.floor(diff) + 's ago';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
    if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
    return Math.floor(diff / 86400) + 'd ago';
  }

  function render() {
    // KPIs
    const total = alerts.length;
    const critical = alerts.filter(a => (a.severity || '').toLowerCase() === 'critical').length;
    const unresolved = alerts.filter(a => !['resolved', 'false_positive'].includes(a.status)).length;
    const kTotal = document.getElementById('kpi-total-alerts');
    const kCritical = document.getElementById('kpi-critical');
    const kUnresolved = document.getElementById('kpi-unresolved');
    if (kTotal) kTotal.textContent = total;
    if (kCritical) kCritical.textContent = critical;
    if (kUnresolved) kUnresolved.textContent = unresolved;

    const tbody = document.getElementById('alerts-body');
    if (!tbody) return;

    tbody.innerHTML = alerts.map((a, i) => {
      let alertId, severity, alertType, desc, usersHTML, detected, status;
      if (usingMockData) {
        alertId = a.id; severity = a.severity; alertType = a.type; desc = a.desc;
        usersHTML = a.users.map(u => `<code style="font-size:10px; padding:1px 5px; background:var(--admin-code-bg); border-radius:3px;">${u}</code>`).join(' ');
        detected = a.detected; status = a.status;
      } else {
        alertId = a.id.substring(0, 8);
        severity = a.severity;
        alertType = a.alert_type;
        desc = a.message;
        usersHTML = a.user_id
          ? `<code style="font-size:10px; padding:1px 5px; background:var(--admin-code-bg); border-radius:3px;">${a.user_id.substring(0, 8)}</code>`
          : '—';
        detected = timeAgo(a.created_at);
        status = a.status;
      }
      const isResolved = ['resolved', 'false_positive'].includes(status);
      return `
        <tr id="alert-row-${i}" style="${isResolved ? 'opacity:0.5;' : ''}">
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${alertId}</code></td>
          <td>${severityBadge(severity)}</td>
          <td style="font-weight:600; color:var(--admin-text-primary);">${alertType}</td>
          <td style="max-width:300px; font-size:12px;">${desc}</td>
          <td>${usersHTML}</td>
          <td style="font-size:12px; color:var(--admin-text-muted);">${detected}</td>
          <td>${statusBadge(status)}</td>
          <td style="text-align:center;">
            <div style="display:flex; gap:6px; justify-content:center;">
              ${status === 'new' ? `<button class="admin-btn admin-btn--secondary admin-btn--sm btn-ack" data-idx="${i}">Acknowledge</button>` : ''}
              ${!isResolved ? `<button class="admin-btn admin-btn--success admin-btn--sm btn-resolve" data-idx="${i}">Resolve</button>` : '<span style="font-size:11px; color:var(--admin-text-muted);">Done</span>'}
            </div>
          </td>
        </tr>
      `;
    }).join('');

    bindActions();
  }

  function bindActions() {
    document.querySelectorAll('.btn-ack').forEach(btn => {
      btn.addEventListener('click', async function () {
        const idx = parseInt(this.dataset.idx);
        await handleAlertAction(this, idx, 'acknowledge');
      });
    });

    document.querySelectorAll('.btn-resolve').forEach(btn => {
      btn.addEventListener('click', async function () {
        const idx = parseInt(this.dataset.idx);
        await handleAlertAction(this, idx, 'resolve');
      });
    });
  }

  async function handleAlertAction(btn, idx, action) {
    const alert = alerts[idx];

    if (!usingMockData) {
      try {
        const res = await fetch(`${API}/${alert.id}`, {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ action }),
        });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const result = await res.json();
        alerts[idx].status = result.status;
        render();
        mpToast(`Alert ${action}d`, 'success');
        return;
      } catch (err) {
        mpToast(`Failed: ${err.message}`, 'error');
        return;
      }
    }

    // Mock path
    if (typeof mpButtonAction === 'function') {
      const label = usingMockData ? alert.id : alert.id.substring(0, 8);
      mpButtonAction(btn, `Alert ${label} ${action}d`, 800, () => {
        alerts[idx].status = action === 'acknowledge' ? 'acknowledged' : 'resolved';
        render();
      });
    }
  }

  // ── Load ────────────────────────────────────────────────────────
  async function loadAlerts() {
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      alerts = await res.json();
      usingMockData = false;
    } catch (err) {
      console.warn('[mp-alerts] API unavailable, using mock data:', err);
      alerts = [...MOCK_ALERTS];
      usingMockData = true;
    }
    render();
  }

  document.addEventListener('DOMContentLoaded', loadAlerts);
})();
