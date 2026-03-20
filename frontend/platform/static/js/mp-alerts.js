/**
 * Alerts & Watchlist — mp-alerts.js
 */
(function () {
  'use strict';

  const alerts = [
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
    return '<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>Warning</span>';
  }

  function statusBadge(status) {
    if (status === 'new') return '<span class="admin-badge admin-badge--danger"><span class="admin-badge-dot"></span>New</span>';
    if (status === 'acknowledged') return '<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>Acknowledged</span>';
    return '<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>Resolved</span>';
  }

  function render() {
    // KPIs
    const total = alerts.length;
    const critical = alerts.filter(a => a.severity === 'critical').length;
    const unresolved = alerts.filter(a => a.status !== 'resolved').length;
    document.getElementById('kpi-total-alerts').textContent = total;
    document.getElementById('kpi-critical').textContent = critical;
    document.getElementById('kpi-unresolved').textContent = unresolved;

    const tbody = document.getElementById('alerts-body');
    if (!tbody) return;

    tbody.innerHTML = alerts.map((a, i) => {
      const usersHTML = a.users.map(u => `<code style="font-size:10px; padding:1px 5px; background:var(--admin-code-bg); border-radius:3px;">${u}</code>`).join(' ');
      const isResolved = a.status === 'resolved';
      return `
        <tr id="alert-row-${i}" style="${isResolved ? 'opacity:0.5;' : ''}">
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${a.id}</code></td>
          <td>${severityBadge(a.severity)}</td>
          <td style="font-weight:600; color:var(--admin-text-primary);">${a.type}</td>
          <td style="max-width:300px; font-size:12px;">${a.desc}</td>
          <td>${usersHTML}</td>
          <td style="font-size:12px; color:var(--admin-text-muted);">${a.detected}</td>
          <td>${statusBadge(a.status)}</td>
          <td style="text-align:center;">
            <div style="display:flex; gap:6px; justify-content:center;">
              ${a.status === 'new' ? `<button class="admin-btn admin-btn--secondary admin-btn--sm btn-ack" data-idx="${i}">Acknowledge</button>` : ''}
              ${a.status !== 'resolved' ? `<button class="admin-btn admin-btn--success admin-btn--sm btn-resolve" data-idx="${i}">Resolve</button>` : '<span style="font-size:11px; color:var(--admin-text-muted);">Done</span>'}
            </div>
          </td>
        </tr>
      `;
    }).join('');

    document.querySelectorAll('.btn-ack').forEach(btn => {
      btn.addEventListener('click', function () {
        const idx = parseInt(this.dataset.idx);
        mpButtonAction(this, `Alert ${alerts[idx].id} acknowledged`, 800, () => {
          alerts[idx].status = 'acknowledged';
          render();
        });
      });
    });

    document.querySelectorAll('.btn-resolve').forEach(btn => {
      btn.addEventListener('click', function () {
        const idx = parseInt(this.dataset.idx);
        mpButtonAction(this, `Alert ${alerts[idx].id} resolved`, 800, () => {
          alerts[idx].status = 'resolved';
          render();
        });
      });
    });
  }

  document.addEventListener('DOMContentLoaded', render);
})();
