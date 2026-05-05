/**
 * Admin Blockchain Treasury — Fetches real data from the backend API.
 * Works on both localhost and production (uses relative URLs).
 */
(function () {
  'use strict';

  const TREASURY_API        = '/api/admin/blockchain/treasury';
  const PAUSE_API           = '/api/admin/blockchain/pause';
  const UNPAUSE_API         = '/api/admin/blockchain/unpause';
  const PRIMARY_QUEUE_API   = '/api/admin/blockchain/primary-settle/queue';
  const PRIMARY_RUN_API     = '/api/admin/blockchain/primary-settle/run';

  // ── DOM references ──────────────────────────────────────────
  const el = (id) => document.getElementById(id);

  function csrfHeaders(headers = {}) {
    const token = typeof window.getCsrfToken === 'function' ? window.getCsrfToken() : getCsrfTokenFromCookie();
    return token ? { ...headers, 'X-CSRF-Token': token } : headers;
  }

  function getCsrfTokenFromCookie() {
    const value = `; ${document.cookie}`;
    const parts = value.split('; csrf_token=');
    return parts.length === 2 ? parts.pop().split(';').shift() : '';
  }

  document.addEventListener('DOMContentLoaded', () => {
    loadTreasury();
    loadPrimaryQueue();
  });

  async function loadTreasury() {
    try {
      const res = await fetch(TREASURY_API, {
        credentials: 'include',
        headers: { 'Accept': 'application/json' }
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      renderTreasury(data);
    } catch (err) {
      console.error('Failed to load blockchain treasury:', err);
      const content = document.querySelector('.admin-content');
      if (content) {
        content.innerHTML = `
          <div class="admin-page-header">
            <h1 class="admin-page-title">Blockchain Treasury</h1>
            <p class="admin-page-subtitle" style="color: var(--admin-danger);">
              ⚠️ Failed to load blockchain data: ${err.message}
            </p>
          </div>
          <div class="admin-card" style="padding: 40px; text-align: center;">
            <p style="color: var(--admin-text-muted);">
              Make sure the blockchain environment variables are configured (CHAIN_CONTRACT_ADDRESS, CHAIN_SETTLEMENT_ADDRESS).
            </p>
            <button class="admin-btn admin-btn--primary" onclick="location.reload()" style="margin-top: 16px;">
              Retry
            </button>
          </div>
        `;
      }
    }
  }

  function renderTreasury(data) {
    const explorerUrl = data.explorer_url;
    const networkLabel = data.network === 'polygon' ? 'Polygon PoS' : 'Polygon Amoy (Testnet)';
    const networkBadge = data.network === 'polygon'
      ? '<span class="admin-badge admin-badge--success">Mainnet</span>'
      : '<span class="admin-badge admin-badge--warning">Testnet</span>';

    // Wallet section
    const walletAddr = el('wallet-address');
    if (walletAddr) walletAddr.textContent = data.wallet_address;

    const walletNetwork = el('wallet-network');
    if (walletNetwork) walletNetwork.innerHTML = networkLabel + ' ' + networkBadge;

    // Contract link
    const contractLink = el('contract-link');
    if (contractLink) {
      contractLink.href = `${explorerUrl}/address/${data.contract_address}`;
      contractLink.textContent = truncateAddr(data.contract_address);
    }

    const viewExplorerBtn = el('view-explorer-btn');
    if (viewExplorerBtn) {
      viewExplorerBtn.href = `${explorerUrl}/address/${data.wallet_address}`;
    }

    // KPI Cards
    setKpi('kpi-tokenized', data.tokenized_assets_count);
    setKpi('kpi-total-supply', sumTokens(data.tokenized_assets, 'tokens_total'));
    setKpi('kpi-tokens-sold', sumTokensSold(data.tokenized_assets));
    setKpi('kpi-pending-trades', data.pending_trades);
    setKpi('kpi-confirmed-trades', data.confirmed_trades);
    setKpi('kpi-batches', data.total_batches);
    setKpi('kpi-whitelisted', data.whitelisted_users_count);

    // Subtitle for sold
    const soldPct = data.tokenized_assets_count > 0
      ? ((sumTokensSold(data.tokenized_assets) / sumTokens(data.tokenized_assets, 'tokens_total')) * 100).toFixed(1)
      : '0.0';
    setKpiSub('kpi-tokens-sold-sub', `${soldPct}% of total supply`);

    // Settlement status
    const statusEl = el('settlement-status');
    if (statusEl) {
      statusEl.innerHTML = data.settlement_enabled
        ? '<span class="admin-badge admin-badge--success">Active</span>'
        : '<span class="admin-badge admin-badge--neutral">Disabled</span>';
    }

    // Contracts table
    renderContractsTable(data.tokenized_assets, explorerUrl);

    // Batches table
    renderBatchesTable(data.recent_batches, explorerUrl);

    // Emergency controls
    renderEmergencyControls(data);
  }

  function renderContractsTable(assets, explorerUrl) {
    const tbody = el('contracts-tbody');
    if (!tbody) return;

    if (assets.length === 0) {
      tbody.innerHTML = `
        <tr>
          <td colspan="7" style="text-align: center; padding: 40px; color: var(--admin-text-muted);">
            No assets have been tokenized on-chain yet.
            <br><a href="/admin/assets" style="color: var(--admin-accent);">Go to Assets →</a>
          </td>
        </tr>
      `;
      return;
    }

    tbody.innerHTML = assets.map(a => {
      const sold = a.tokens_total - a.tokens_available;
      const soldPct = a.tokens_total > 0 ? ((sold / a.tokens_total) * 100).toFixed(1) : '0.0';
      const statusDot = a.funding_status === 'exited' ? 'paused' : 'live';
      const statusLabel = a.funding_status === 'exited' ? 'Exited' : 'Live';
      const statusColor = a.funding_status === 'exited' ? '#f59e0b' : '#10b981';

      return `
        <tr>
          <td style="font-weight: 600; font-size: 13px;">${esc(a.title)}</td>
          <td>
            <a href="${explorerUrl}/address/${esc(a.chain_contract_address || '')}" target="_blank" class="basescan-link">
              ${truncateAddr(a.chain_contract_address || '—')}
            </a>
          </td>
          <td><span class="admin-badge admin-badge--neutral">${esc(a.chain_token_id || '—')}</span></td>
          <td style="font-weight: 600;">${fmt(a.tokens_total)}</td>
          <td>
            <div style="font-size: 12px; margin-bottom: 4px;">
              <span style="color: var(--admin-accent); font-weight: 600;">${fmt(sold)} sold</span>
              <span style="color: var(--admin-text-muted);"> / ${fmt(a.tokens_available)} available</span>
            </div>
            <div class="token-supply-bar">
              <div class="sold" style="width: ${soldPct}%;"></div>
            </div>
          </td>
          <td>
            <span class="contract-status-dot contract-status-dot--${statusDot}"></span>
            <span style="font-size: 12px; font-weight: 600; color: ${statusColor};">${statusLabel}</span>
          </td>
          <td style="font-size: 12px; color: var(--admin-text-muted);">${esc(a.created_at)}</td>
        </tr>
      `;
    }).join('');
  }

  function renderBatchesTable(batches, explorerUrl) {
    const tbody = el('batches-tbody');
    if (!tbody) return;

    if (batches.length === 0) {
      tbody.innerHTML = `
        <tr>
          <td colspan="6" style="text-align: center; padding: 40px; color: var(--admin-text-muted);">
            No settlement batches have been processed yet.
          </td>
        </tr>
      `;
      return;
    }

    tbody.innerHTML = batches.map(b => {
      const statusBadge = b.status === 'confirmed'
        ? '<span class="admin-badge admin-badge--success">Confirmed</span>'
        : b.status === 'failed'
        ? '<span class="admin-badge admin-badge--danger">Failed</span>'
        : b.status === 'submitted'
        ? '<span class="admin-badge admin-badge--warning">Submitted</span>'
        : '<span class="admin-badge admin-badge--neutral">Pending</span>';

      const txLink = b.tx_hash
        ? `<a href="${explorerUrl}/tx/${b.tx_hash}" target="_blank" class="basescan-link">${truncateAddr(b.tx_hash)}</a>`
        : '—';

      const gasInfo = b.gas_used ? `${fmt(b.gas_used)} gas` : '—';

      return `
        <tr>
          <td style="font-size: 12px;">${esc(b.created_at)}</td>
          <td style="font-weight: 600;">${b.batch_size}</td>
          <td>${statusBadge}</td>
          <td>${txLink}</td>
          <td style="font-size: 12px;">${gasInfo}</td>
          <td style="font-size: 12px; color: var(--admin-text-muted);">
            ${b.error_message ? '<span style="color: var(--admin-danger);" title="' + esc(b.error_message) + '">Error</span>' : '—'}
          </td>
        </tr>
      `;
    }).join('');
  }

  function renderEmergencyControls(data) {
    const container = el('emergency-controls');
    if (!container) return;

    container.innerHTML = `
      <div class="admin-card" style="padding: 16px; border-color: rgba(239, 68, 68, 0.2);">
        <div style="font-weight: 600; font-size: 14px; margin-bottom: 4px;">POOOLProperty1155</div>
        <div style="font-size: 11px; color: var(--admin-text-muted); margin-bottom: 12px;">
          ${truncateAddr(data.contract_address)}
        </div>
        <div style="display: flex; gap: 8px;">
          <button class="admin-btn admin-btn--secondary" id="btn-pause"
            style="flex: 1; color: var(--admin-danger); border-color: var(--admin-danger);"
            onclick="window._blockchainPause()">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="3" width="10" height="10" rx="1" />
            </svg>
            PAUSE
          </button>
          <button class="admin-btn admin-btn--secondary" id="btn-unpause"
            style="flex: 1; color: var(--admin-success); border-color: var(--admin-success);"
            onclick="window._blockchainUnpause()">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M5 3l8 5-8 5V3z" />
            </svg>
            UNPAUSE
          </button>
        </div>
      </div>
    `;
  }

  // ── Primary-issuance settlement panel ─────────────────────
  async function loadPrimaryQueue() {
    try {
      const res = await fetch(PRIMARY_QUEUE_API, {
        credentials: 'include',
        headers: { 'Accept': 'application/json' },
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      renderPrimaryQueue(data);
    } catch (err) {
      console.error('Failed to load primary settlement queue:', err);
      const tbody = el('primary-upcoming-tbody');
      if (tbody) {
        tbody.innerHTML = `<tr><td colspan="6" style="text-align:center; padding:16px; color:var(--admin-danger);">Failed to load: ${esc(err.message)}</td></tr>`;
      }
    }
  }

  function renderPrimaryQueue(data) {
    const counts = data.counts || {};
    setKpi('primary-count-pending',   counts.pending   || 0);
    setKpi('primary-count-submitted', counts.submitted || 0);
    setKpi('primary-count-confirmed', counts.confirmed || 0);
    setKpi('primary-count-failed',    counts.failed    || 0);

    renderPrimaryFailures(data.recent_failures || [], data.blockers || {});

    const tbody = el('primary-upcoming-tbody');
    if (!tbody) return;
    const upcoming = data.upcoming || [];
    if (upcoming.length === 0) {
      tbody.innerHTML = `
        <tr>
          <td colspan="6" style="text-align:center; padding:20px; color:var(--admin-text-muted);">
            No primary order items pending on-chain settlement.
          </td>
        </tr>
      `;
      return;
    }
    tbody.innerHTML = upcoming.map(item => {
      const eligible = item.settle_eligible_at ? new Date(item.settle_eligible_at).toLocaleString() : '—';
      const statusBadge = item.ready_now
        ? '<span class="admin-badge admin-badge--success">Ready</span>'
        : '<span class="admin-badge admin-badge--warning">Waiting</span>';
      const attempts = item.attempt_count || 0;
      const attemptStyle = attempts > 0 ? 'color: var(--admin-danger); font-weight: 700;' : '';
      return `
        <tr>
          <td style="font-family:'SF Mono',monospace; font-size:12px;">${esc(item.order_number)}</td>
          <td style="font-size:13px;">${esc(item.asset_title || '—')}</td>
          <td style="font-weight:600;">${fmt(item.tokens)}</td>
          <td style="font-size:12px; color:var(--admin-text-muted);">${esc(eligible)}</td>
          <td style="${attemptStyle}">${attempts}</td>
          <td>${statusBadge}</td>
        </tr>
      `;
    }).join('');
  }

  function renderPrimaryFailures(failures, blockers) {
    const host = el('primary-failures-host');
    if (!host) return;

    const blockerLines = [];
    if (blockers.no_buyer_wallet) {
      blockerLines.push(`<li>${blockers.no_buyer_wallet} item(s) have no buyer wallet bound (user must complete SIWE wallet binding)</li>`);
    }
    if (blockers.no_asset_contract) {
      blockerLines.push(`<li>${blockers.no_asset_contract} item(s) target an asset that has no on-chain contract yet</li>`);
    }
    if (blockers.wallet_not_whitelisted) {
      blockerLines.push(`<li>${blockers.wallet_not_whitelisted} item(s) have a bound wallet but the on-chain whitelist hasn't synced yet (KYC whitelist worker)</li>`);
    }

    if (failures.length === 0 && blockerLines.length === 0) {
      host.innerHTML = '';
      return;
    }

    const failureRows = failures.map(f => {
      const when = f.created_at ? new Date(f.created_at).toLocaleString() : '—';
      const err = f.error_message || '(no message recorded)';
      const txCell = f.tx_hash ? truncateAddr(f.tx_hash) : '<em>not broadcast</em>';
      return `
        <tr>
          <td style="font-size:12px; color:var(--admin-text-muted); white-space:nowrap;">${esc(when)}</td>
          <td style="font-weight:600;">${f.batch_size}</td>
          <td style="font-size:12px;">${txCell}</td>
          <td style="font-size:12px; color:var(--admin-danger); font-family:'SF Mono',monospace;">${esc(err)}</td>
        </tr>
      `;
    }).join('');

    host.innerHTML = `
      ${blockerLines.length ? `
        <div style="border:1px solid rgba(245,158,11,0.3); background:rgba(245,158,11,0.05); border-radius:var(--admin-radius-md); padding:12px 16px; margin-bottom:16px;">
          <div style="font-weight:600; font-size:13px; color:#b45309; margin-bottom:6px;">Eligibility blockers (items stuck NULL)</div>
          <ul style="margin:0; padding-left:18px; font-size:12px; color:var(--admin-text-secondary);">${blockerLines.join('')}</ul>
        </div>
      ` : ''}
      ${failures.length ? `
        <div style="border:1px solid rgba(239,68,68,0.25); background:rgba(239,68,68,0.04); border-radius:var(--admin-radius-md); padding:12px 16px; margin-bottom:16px;">
          <div style="font-weight:600; font-size:13px; color:var(--admin-danger); margin-bottom:8px;">Recent failed batches (last 5)</div>
          <table class="admin-table" style="margin:0;">
            <thead>
              <tr><th>When</th><th>Size</th><th>Tx</th><th>Error</th></tr>
            </thead>
            <tbody>${failureRows}</tbody>
          </table>
        </div>
      ` : ''}
    `;
  }

  window._runPrimarySettlement = async function () {
    const btn = el('btn-primary-settle-run');
    const ignoreDelay = el('primary-settle-ignore-delay');
    const useIgnore = ignoreDelay && ignoreDelay.checked;
    if (useIgnore) {
      if (!confirm('⚠️ Ignore T+1 delay?\n\nThis bypasses the bank-wire reversal safety window. Use only for testing or after manual reconciliation.\n\nContinue?')) {
        return;
      }
    } else if (!confirm('Run primary-issuance settlement now?\n\nThis will broadcast a settleBatch() tx for any eligible items.')) {
      return;
    }
    if (btn) {
      btn.disabled = true;
      btn.textContent = 'Running…';
    }
    try {
      const url = useIgnore ? `${PRIMARY_RUN_API}?ignore_delay=true` : PRIMARY_RUN_API;
      const res = await fetch(url, {
        method: 'POST',
        credentials: 'include',
        headers: csrfHeaders({ 'Content-Type': 'application/json' }),
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) {
        throw new Error(data.error || data.message || `HTTP ${res.status}`);
      }
      alert('✅ ' + (data.message || `Settled ${data.settled || 0} item(s)`));
      loadPrimaryQueue();
      loadTreasury();
    } catch (err) {
      alert('❌ Run failed: ' + err.message);
    } finally {
      if (btn) {
        btn.disabled = false;
        btn.innerHTML = `
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 3l8 5-8 5V3z" />
          </svg>
          Run now
        `;
      }
    }
  };

  // ── Emergency actions ──────────────────────────────────────
  window._blockchainPause = async function () {
    if (!confirm('⚠️ EMERGENCY PAUSE\n\nThis will freeze ALL token transfers on the smart contract.\n\nAre you absolutely sure?')) return;
    if (!confirm('FINAL CONFIRMATION: Pause the contract NOW?')) return;
    try {
      const res = await fetch(PAUSE_API, {
        method: 'POST',
        credentials: 'include',
        headers: csrfHeaders({ 'Content-Type': 'application/json' }),
      });
      const data = await res.json();
      if (data.success) {
        alert('✅ Contract PAUSED successfully.\nTx: ' + data.tx_hash);
        location.reload();
      } else {
        alert('❌ Pause failed: ' + JSON.stringify(data));
      }
    } catch (err) {
      alert('❌ Error: ' + err.message);
    }
  };

  window._blockchainUnpause = async function () {
    if (!confirm('Unpause the contract? Token transfers will resume.')) return;
    try {
      const res = await fetch(UNPAUSE_API, {
        method: 'POST',
        credentials: 'include',
        headers: csrfHeaders({ 'Content-Type': 'application/json' }),
      });
      const data = await res.json();
      if (data.success) {
        alert('✅ Contract UNPAUSED.\nTx: ' + data.tx_hash);
        location.reload();
      } else {
        alert('❌ Unpause failed: ' + JSON.stringify(data));
      }
    } catch (err) {
      alert('❌ Error: ' + err.message);
    }
  };

  // ── Utility ────────────────────────────────────────────────
  function setKpi(id, value) {
    const el_ = el(id);
    if (el_) el_.textContent = typeof value === 'number' ? fmt(value) : value;
  }
  function setKpiSub(id, text) {
    const el_ = el(id);
    if (el_) el_.textContent = text;
  }
  function sumTokens(assets, field) {
    return assets.reduce((sum, a) => sum + (a[field] || 0), 0);
  }
  function sumTokensSold(assets) {
    return assets.reduce((sum, a) => sum + (a.tokens_total - a.tokens_available), 0);
  }
  function fmt(n) {
    return n.toLocaleString('en-US');
  }
  function truncateAddr(addr) {
    if (!addr || addr.length < 12) return addr || '—';
    return addr.slice(0, 6) + '…' + addr.slice(-4);
  }
  function esc(str) {
    const d = document.createElement('div');
    d.textContent = str || '';
    return d.innerHTML;
  }

  // Copy address
  window._copyAddr = function (addr) {
    navigator.clipboard.writeText(addr).then(() => {
      const btn = document.querySelector('.copy-btn');
      if (btn) { btn.textContent = '✓'; setTimeout(() => { btn.textContent = '📋'; }, 1500); }
    });
  };
})();
