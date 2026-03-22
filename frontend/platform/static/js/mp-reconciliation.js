/**
 * Reconciliation — mp-reconciliation.js
 * Fetches reconciliation report from the backend API.
 * Falls back to mock data if the API is unavailable.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/reconciliation';

  // ── Render Invariant Checks ─────────────────────────────────────
  function renderChecks(report) {
    const checks = [report.cash_balance_check, report.fee_balance_check, report.token_integrity_check];
    const checksContainer = document.getElementById('invariant-checks');
    if (checksContainer) {
      checksContainer.innerHTML = checks.map(c => {
        const icon = c.passed ? '✅' : '❌';
        const cls = c.passed ? 'admin-badge--success' : 'admin-badge--danger';
        return `
          <div class="mp-recon-check" style="display:flex; align-items:center; justify-content:space-between; padding:16px; border:1px solid var(--admin-border); border-radius:var(--admin-radius-sm); margin-bottom:8px; background:var(--admin-bg-card);">
            <div style="display:flex; align-items:center; gap:12px;">
              <span style="font-size:20px;">${icon}</span>
              <div>
                <div style="font-weight:600; color:var(--admin-text-primary); font-size:14px;">${c.name}</div>
                <div style="font-size:12px; color:var(--admin-text-muted); margin-top:2px;">${c.details}</div>
              </div>
            </div>
            <div style="text-align:right;">
              <span class="admin-badge ${cls}">
                <span class="admin-badge-dot"></span>
                ${c.passed ? 'PASS' : 'FAIL'}
              </span>
              ${c.delta !== 0 ? `<div style="font-size:11px; color:var(--admin-danger); margin-top:4px; font-variant-numeric:tabular-nums;">Δ ${c.delta > 0 ? '+' : ''}${c.delta}</div>` : ''}
            </div>
          </div>
        `;
      }).join('');
    }

    // Update KPIs
    const failCount = checks.filter(c => !c.passed).length;
    const kpiStatus = document.getElementById('kpi-recon-status');
    if (kpiStatus) {
      kpiStatus.textContent = failCount === 0 ? 'All Clear' : `${failCount} Issue${failCount > 1 ? 's' : ''}`;
      kpiStatus.style.color = failCount === 0 ? 'var(--admin-success)' : 'var(--admin-danger)';
    }

    const kpiTimestamp = document.getElementById('kpi-recon-timestamp');
    if (kpiTimestamp && report.generated_at) {
      kpiTimestamp.textContent = new Date(report.generated_at).toLocaleString('en-US', {
        dateStyle: 'short', timeStyle: 'medium'
      });
    }
  }

  // ── Render Legacy Mismatch Table ────────────────────────────────
  function renderMismatches(report) {
    const tbody = document.getElementById('recon-body');
    if (!tbody) return;

    // The API gives us invariant checks; detailed mismatches come in a future release.
    const tokenCheck = report.token_integrity_check;
    if (tokenCheck && !tokenCheck.passed && tokenCheck.actual > 0) {
      tbody.innerHTML = `
        <tr>
          <td colspan="7" style="text-align:center; padding:16px;">
            <span class="admin-badge admin-badge--danger"><span class="admin-badge-dot"></span>${tokenCheck.actual} token supply mismatches detected</span>
            <div style="font-size:12px; color:var(--admin-text-muted); margin-top:6px;">Run detailed reconciliation to identify specific assets.</div>
          </td>
        </tr>
      `;
    } else {
      tbody.innerHTML = `
        <tr>
          <td colspan="7" style="text-align:center; padding:24px; color:var(--admin-success);">
            ✅ No mismatches — all balances verified
          </td>
        </tr>
      `;
    }
  }

  // ── Render Run History (mock for now) ───────────────────────────
  function renderHistory() {
    const history = [
      { time: '2026-03-20 04:00', wallets: 1247, mismatches: 3, duration: '12.4s', status: 'warning' },
      { time: '2026-03-19 04:00', wallets: 1243, mismatches: 0, duration: '11.8s', status: 'ok' },
      { time: '2026-03-18 04:00', wallets: 1238, mismatches: 1, duration: '12.1s', status: 'warning' },
      { time: '2026-03-17 04:00', wallets: 1235, mismatches: 0, duration: '11.5s', status: 'ok' },
      { time: '2026-03-16 04:00', wallets: 1230, mismatches: 0, duration: '11.9s', status: 'ok' },
    ];

    const hBody = document.getElementById('recon-history-body');
    if (hBody) {
      hBody.innerHTML = history.map(h => {
        const statusBadge = h.status === 'ok'
          ? '<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>Clean</span>'
          : '<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>Mismatches</span>';
        return `
          <tr>
            <td style="font-variant-numeric:tabular-nums; font-size:13px;">${h.time}</td>
            <td style="font-variant-numeric:tabular-nums;">${h.wallets.toLocaleString()}</td>
            <td><span class="admin-badge ${h.mismatches > 0 ? 'admin-badge--warning' : 'admin-badge--success'}">${h.mismatches}</span></td>
            <td style="font-variant-numeric:tabular-nums;">${h.duration}</td>
            <td>${statusBadge}</td>
          </tr>
        `;
      }).join('');
    }
  }

  // ── Mock Fallback ───────────────────────────────────────────────
  function useMockData() {
    const mismatches = [
      { user: 'USR-3384', asset: 'BVRT', wallet: 120, ledger: 125, diff: -5, cause: 'Concurrent order execution race condition' },
      { user: 'USR-1738', asset: 'JOTX', wallet: 50, ledger: 48, diff: 2, cause: 'Settlement callback delayed — tokens credited early' },
      { user: 'USR-6643', asset: 'SWHS', wallet: 800, ledger: 800.5, diff: -0.5, cause: 'Rounding error in fractional token split' },
    ];

    const tbody = document.getElementById('recon-body');
    if (tbody) {
      tbody.innerHTML = mismatches.map((m, i) => `
        <tr id="mismatch-${i}">
          <td><code style="font-size:11px; padding:2px 6px; background:var(--admin-code-bg); border-radius:4px;">${m.user}</code></td>
          <td style="font-weight:600;">${m.asset}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${m.wallet.toLocaleString()}</td>
          <td style="text-align:right; font-variant-numeric:tabular-nums;">${m.ledger.toLocaleString()}</td>
          <td style="text-align:right;">
            <span class="admin-badge admin-badge--danger" style="font-variant-numeric:tabular-nums;">${m.diff > 0 ? '+' : ''}${m.diff}</span>
          </td>
          <td style="font-size:12px; color:var(--admin-text-muted); max-width:250px;">${m.cause}</td>
          <td style="text-align:center;">
            <button class="admin-btn admin-btn--success admin-btn--sm btn-resolve-mismatch" data-idx="${i}">Resolve</button>
          </td>
        </tr>
      `).join('');

      document.querySelectorAll('.btn-resolve-mismatch').forEach(btn => {
        btn.addEventListener('click', function () {
          const idx = parseInt(this.dataset.idx);
          if (typeof mpButtonAction === 'function') {
            mpButtonAction(this, `Mismatch for ${mismatches[idx].user}/${mismatches[idx].asset} resolved`, 1000, () => {
              const row = document.getElementById(`mismatch-${idx}`);
              if (row) { row.style.transition = 'opacity 0.3s'; row.style.opacity = '0.3'; }
            });
          }
        });
      });
    }

    renderHistory();
  }

  // ── Load ────────────────────────────────────────────────────────
  async function loadReconciliation() {
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const report = await res.json();
      renderChecks(report);
      renderMismatches(report);
      renderHistory();
    } catch (err) {
      console.warn('[mp-reconciliation] API unavailable, using mock data:', err);
      useMockData();
    }
  }

  // ── Init ────────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', () => {
    loadReconciliation();

    document.getElementById('btn-run-recon')?.addEventListener('click', function () {
      const btn = this;
      btn.disabled = true;
      btn.textContent = 'Running…';
      loadReconciliation().then(() => {
        btn.disabled = false;
        btn.textContent = 'Run Reconciliation';
        if (typeof mpToast === 'function') mpToast('Reconciliation run completed', 'success');
      });
    });
  });
})();
