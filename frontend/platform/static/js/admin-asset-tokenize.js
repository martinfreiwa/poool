/**
 * Admin Asset Tokenize — Fetches pre-flight checks and handles tokenization.
 * Works on both localhost and production (relative URLs).
 */
(function () {
  'use strict';

  const TOKENIZE_API_BASE = '/api/admin/blockchain/tokenize/';

  // Get asset_id from URL query param
  const urlParams = new URLSearchParams(window.location.search);
  const assetId = urlParams.get('id') || urlParams.get('asset_id');

  document.addEventListener('DOMContentLoaded', () => {
    if (!assetId) {
      showError('No asset ID provided. Use ?id=<uuid> in the URL.');
      return;
    }
    loadTokenizeCheck();
  });

  async function loadTokenizeCheck() {
    try {
      const res = await fetch(TOKENIZE_API_BASE + assetId, {
        credentials: 'include',
        headers: { 'Accept': 'application/json' }
      });
      if (!res.ok) {
        const body = await res.text();
        throw new Error(`HTTP ${res.status}: ${body}`);
      }
      const data = await res.json();
      renderTokenizePage(data);
    } catch (err) {
      console.error('Failed to load tokenize check:', err);
      showError('Failed to load asset data: ' + err.message);
    }
  }

  function renderTokenizePage(data) {
    // Page title
    const title = document.getElementById('page-title');
    if (title) title.textContent = `Tokenize: ${data.title}`;

    const breadcrumbName = document.getElementById('breadcrumb-asset-name');
    if (breadcrumbName) {
      breadcrumbName.textContent = data.title;
      breadcrumbName.href = `/admin/asset-details?id=${data.asset_id}`;
    }

    // Asset summary
    const totalValue = (data.total_value_cents / 100).toLocaleString('en-US', { style: 'currency', currency: 'USD' });
    const tokenPrice = (data.token_price_cents / 100).toLocaleString('en-US', { style: 'currency', currency: 'USD' });
    
    setEl('summary-valuation', totalValue);
    setEl('summary-price', tokenPrice);
    setEl('summary-supply', data.tokens_total.toLocaleString());

    // Network info
    const network = getNetworkFromEnv();
    setEl('summary-network', network.label);

    // Already tokenized?
    if (data.already_tokenized) {
      renderAlreadyTokenized(data);
      return;
    }

    // Pre-flight checklist
    renderChecklist(data.checks, data);

    // Deploy button
    const deployArea = document.getElementById('deploy-area');
    if (deployArea) {
      const allPassed = data.checks.all_passed;
      deployArea.innerHTML = `
        <div style="margin-bottom: 16px;">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--admin-accent)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 2L2 7l10 5 10-5-10-5z" />
            <path d="M2 17l10 5 10-5" />
            <path d="M2 12l10 5 10-5" />
          </svg>
        </div>
        <p style="font-size: 14px; color: var(--admin-text-secondary); margin: 0 0 20px; max-width: 500px; margin-left: auto; margin-right: auto;">
          Tokenizing will call <strong>createAsset()</strong> on the POOOLProperty1155 smart contract on the Polygon blockchain.
          This assigns an on-chain token ID and mints ${data.tokens_total.toLocaleString()} tokens.
        </p>
        <button class="deploy-btn-main" id="btn-tokenize" ${allPassed ? '' : 'disabled'} onclick="window._tokenize()">
          🚀 Tokenize Asset — Deploy On-Chain
        </button>
        ${!allPassed ? '<p style="font-size: 12px; color: var(--admin-danger); margin: 12px 0 0;">All pre-flight checks must pass before tokenization.</p>' : ''}
      `;
    }
  }

  function renderChecklist(checks, data) {
    const list = document.getElementById('checklist');
    if (!list) return;

    const items = [
      { label: 'Asset Approved & Published', sub: 'Asset must be approved by an admin before tokenization.', pass: checks.asset_approved },
      { label: 'Token Supply Defined', sub: `${data.tokens_total.toLocaleString()} tokens configured.`, pass: checks.has_token_supply },
      { label: 'Token Price Set', sub: `Price: ${(data.token_price_cents / 100).toLocaleString('en-US', { style: 'currency', currency: 'USD' })} per token.`, pass: checks.has_price },
    ];

    // Show pre-flight checks
    list.innerHTML = items.map(item => `
      <div class="preflight-item">
        <div class="preflight-icon preflight-icon--${item.pass ? 'pass' : 'fail'}">
          <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2">
            ${item.pass ? '<path d="M3 8l3.5 3.5L13 4" />' : '<path d="M4 4l8 8M12 4l-8 8" />'}
          </svg>
        </div>
        <div style="flex: 1;">
          <div class="preflight-label">${item.label}</div>
          <div class="preflight-sublabel">${item.sub}</div>
        </div>
        <span class="admin-badge admin-badge--${item.pass ? 'success' : 'danger'}">${item.pass ? 'Pass' : 'Fail'}</span>
      </div>
    `).join('');

    // Tokenization status — separate, prominent status indicator
    const isTokenized = !checks.not_already_tokenized;
    list.innerHTML += `
      <div class="preflight-item" style="border-top: 1px solid var(--admin-border); margin-top: 8px; padding-top: 16px;">
        <div class="preflight-icon preflight-icon--${isTokenized ? 'pass' : 'fail'}">
          <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2">
            ${isTokenized ? '<path d="M3 8l3.5 3.5L13 4" />' : '<path d="M4 4l8 8M12 4l-8 8" />'}
          </svg>
        </div>
        <div style="flex: 1;">
          <div class="preflight-label">${isTokenized ? 'Tokenized' : 'Not Tokenized'}</div>
          <div class="preflight-sublabel">${isTokenized ? 'Asset has an on-chain token ID.' : 'Asset has not been deployed on-chain yet.'}</div>
        </div>
        <span class="admin-badge admin-badge--${isTokenized ? 'success' : 'danger'}">${isTokenized ? 'Deployed' : 'Pending'}</span>
      </div>
    `;
  }

  function renderAlreadyTokenized(data) {
    const network = getNetworkFromEnv();

    // Hide checklist, show result
    const checklistCard = document.getElementById('checklist-card');
    if (checklistCard) checklistCard.style.display = 'none';

    const deployArea = document.getElementById('deploy-area');
    if (deployArea) {
      deployArea.innerHTML = `
        <div class="deploy-result">
          <div style="display: flex; align-items: center; gap: 10px; margin-bottom: 12px;">
            <svg width="22" height="22" viewBox="0 0 16 16" fill="none" stroke="#10b981" stroke-width="2"><path d="M3 8l3.5 3.5L13 4" /></svg>
            <span style="font-size: 16px; font-weight: 700; color: #10b981;">Asset Already Tokenized</span>
          </div>
          <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
            <div>
              <span style="font-size: 11px; font-weight: 600; color: var(--admin-text-muted); text-transform: uppercase;">Token ID</span>
              <div class="deploy-result-address">${data.chain_token_id || '—'}</div>
            </div>
            <div>
              <span style="font-size: 11px; font-weight: 600; color: var(--admin-text-muted); text-transform: uppercase;">Contract Address</span>
              <div class="deploy-result-address">
                <a href="${network.explorer}/address/${data.chain_contract_address}" target="_blank" style="color: var(--admin-accent); text-decoration: none;">
                  ${data.chain_contract_address || '—'}
                </a>
              </div>
            </div>
          </div>
          <div style="margin-top: 16px;">
            <a href="/admin/blockchain-treasury" class="admin-btn admin-btn--secondary">
              Go to Blockchain Treasury →
            </a>
          </div>
        </div>
      `;
    }

    // Update timeline
    const tokenizeStep = document.getElementById('timeline-tokenize');
    if (tokenizeStep) {
      tokenizeStep.className = 'timeline-step timeline-step--done';
      tokenizeStep.innerHTML = '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 8l3.5 3.5L13 4" /></svg> Tokenized';
    }
    const liveStep = document.getElementById('timeline-live');
    if (liveStep) {
      liveStep.className = 'timeline-step timeline-step--active';
    }
  }

  // ── Tokenize action ──────────────────────────────────────────
  window._tokenize = async function () {
    if (!confirm('Tokenize this asset on the Polygon blockchain?\n\nThis will call createAsset() on the smart contract and assign an on-chain token ID.')) return;

    const btn = document.getElementById('btn-tokenize');
    if (btn) { btn.disabled = true; btn.textContent = '⏳ Tokenizing…'; }

    try {
      const csrf = document.querySelector('meta[name="csrf-token"]')?.content || '';
      const res = await fetch(TOKENIZE_API_BASE + assetId, {
        method: 'POST',
        credentials: 'include',
        headers: {
          'Content-Type': 'application/json',
          'X-CSRF-Token': csrf,
        },
      });
      const data = await res.json();
      if (data.success) {
        alert('✅ Asset tokenized successfully!\n\nToken ID: ' + data.chain_token_id + '\nTx: ' + data.chain_tx_hash);
        location.reload();
      } else {
        alert('❌ Tokenization failed: ' + JSON.stringify(data));
        if (btn) { btn.disabled = false; btn.textContent = '🚀 Tokenize Asset — Deploy On-Chain'; }
      }
    } catch (err) {
      alert('❌ Error: ' + err.message);
      if (btn) { btn.disabled = false; btn.textContent = '🚀 Tokenize Asset — Deploy On-Chain'; }
    }
  };

  // ── Helpers ─────────────────────────────────────────────────
  function setEl(id, text) {
    const el = document.getElementById(id);
    if (el) el.textContent = text;
  }

  function showError(msg) {
    const content = document.querySelector('.admin-content');
    if (content) {
      content.innerHTML = `
        <div class="admin-page-header">
          <h1 class="admin-page-title">Tokenize Asset</h1>
          <p class="admin-page-subtitle" style="color: var(--admin-danger);">⚠️ ${msg}</p>
        </div>
      `;
    }
  }

  function getNetworkFromEnv() {
    // We detect this from the page URL domain — production = mainnet, else testnet
    const isProduction = window.location.hostname === 'platform.poool.app';
    return {
      label: isProduction ? 'Polygon PoS (Mainnet)' : 'Polygon Amoy (Testnet)',
      explorer: isProduction ? 'https://polygonscan.com' : 'https://amoy.polygonscan.com',
    };
  }
})();
