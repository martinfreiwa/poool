/**
 * Admin Asset Tokenize - safe API-backed preflight and tokenization.
 */
(function () {
  'use strict';

  const TOKENIZE_API_BASE = '/api/admin/blockchain/tokenize/';
  const CANDIDATES_API = '/api/admin/blockchain/tokenize-candidates';

  const urlParams = new URLSearchParams(window.location.search);
  let assetId = urlParams.get('id') || urlParams.get('asset_id');
  let lastError = null;

  document.addEventListener('DOMContentLoaded', () => {
    if (!assetId) {
      loadTokenizeCandidates();
      return;
    }
    loadTokenizeCheck();
  });

  async function fetchJSON(url, options = {}) {
    const res = await fetch(url, {
      credentials: 'include',
      headers: { Accept: 'application/json', ...(options.headers || {}) },
      ...options,
    });
    const contentType = res.headers.get('content-type') || '';
    const body = contentType.includes('application/json')
      ? await res.json()
      : { error: await res.text() };
    if (!res.ok) {
      throw new Error(body.error || `HTTP ${res.status}`);
    }
    return body;
  }

  async function loadTokenizeCandidates() {
    setBreadcrumbForPicker();
    setEl('page-title', 'Select Asset to Tokenize');
    try {
      const data = await fetchJSON(CANDIDATES_API);
      renderCandidatePicker(data.assets || []);
    } catch (err) {
      lastError = err;
      console.error('Failed to load tokenization candidates:', err);
      showError(`Failed to load tokenizable assets: ${err.message}`);
    }
  }

  async function loadTokenizeCheck() {
    try {
      const data = await fetchJSON(TOKENIZE_API_BASE + encodeURIComponent(assetId));
      renderTokenizePage(data);
    } catch (err) {
      lastError = err;
      console.error('Failed to load tokenize check:', err);
      showError(`Failed to load asset data: ${err.message}`);
    }
  }

  function renderTokenizePage(data) {
    setEl('page-title', `Tokenize: ${data.title}`);

    const breadcrumbName = document.getElementById('breadcrumb-asset-name');
    if (breadcrumbName) {
      breadcrumbName.textContent = data.title;
      breadcrumbName.href = `/admin/asset-details?id=${encodeURIComponent(data.asset_id)}`;
    }

    setEl('summary-valuation', formatCurrency(data.total_value_cents));
    setEl('summary-price', formatCurrency(data.token_price_cents));
    setEl('summary-supply', Number(data.tokens_total || 0).toLocaleString());
    setEl('summary-network', formatNetwork(data.chain_network));

    if (data.already_tokenized) {
      renderAlreadyTokenized(data);
      return;
    }

    const checklistCard = document.getElementById('checklist-card');
    if (checklistCard) checklistCard.style.display = '';
    renderChecklist(data.checks || {}, data);
    renderDeployArea(data);
  }

  function renderCandidatePicker(assets) {
    const checklistCard = document.getElementById('checklist-card');
    if (checklistCard) checklistCard.style.display = 'none';
    clearSummaries();

    const deployArea = document.getElementById('deploy-area');
    if (!deployArea) return;
    deployArea.textContent = '';
    deployArea.classList.add('deploy-button-area--picker');

    const header = document.createElement('div');
    header.className = 'tokenize-picker-header';

    const title = document.createElement('h2');
    title.textContent = 'Tokenizable Assets';
    const sub = document.createElement('p');
    sub.textContent = 'Select a published asset to review its production pre-flight checks before deployment.';
    header.append(title, sub);
    deployArea.appendChild(header);

    if (!assets.length) {
      const empty = document.createElement('p');
      empty.className = 'tokenize-status tokenize-status--muted';
      empty.textContent = 'No published assets are currently available for tokenization review.';
      deployArea.appendChild(empty);
      return;
    }

    const list = document.createElement('div');
    list.className = 'tokenize-candidate-list';
    assets.forEach((asset) => {
      const link = document.createElement('a');
      link.className = 'tokenize-candidate';
      link.href = `/admin/asset-tokenize?id=${encodeURIComponent(asset.asset_id)}`;

      const main = document.createElement('div');
      const name = document.createElement('strong');
      name.textContent = asset.title;
      const meta = document.createElement('span');
      meta.textContent = `${formatStatus(asset.funding_status)} · ${Number(asset.tokens_total || 0).toLocaleString()} tokens · ${formatCurrency(asset.token_price_cents)} each`;
      main.append(name, meta);

      const badge = document.createElement('span');
      badge.className = `admin-badge admin-badge--${asset.already_tokenized ? 'success' : 'warning'}`;
      badge.textContent = asset.already_tokenized ? 'Tokenized' : 'Review';

      link.append(main, badge);
      list.appendChild(link);
    });
    deployArea.appendChild(list);
  }

  function renderDeployArea(data) {
    const deployArea = document.getElementById('deploy-area');
    if (!deployArea) return;

    deployArea.classList.remove('deploy-button-area--picker');
    deployArea.textContent = '';

    const icon = document.createElement('div');
    icon.className = 'deploy-icon';
    icon.setAttribute('aria-hidden', 'true');
    icon.textContent = '⬢';

    const description = document.createElement('p');
    description.className = 'deploy-description';
    description.append('Tokenizing will call ');
    const method = document.createElement('strong');
    method.textContent = 'deployAsset()';
    description.append(method, ' on the configured AssetFactory contract. This assigns an on-chain token ID and mints ');
    const supply = document.createElement('strong');
    supply.textContent = Number(data.tokens_total || 0).toLocaleString();
    description.append(supply, ' tokens.');

    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'deploy-btn-main';
    button.id = 'btn-tokenize';
    button.disabled = !data.checks?.all_passed;
    button.textContent = 'Tokenize Asset - Deploy On-Chain';
    button.addEventListener('click', tokenizeAsset);

    deployArea.append(icon, description, button);
    if (!data.checks?.all_passed) {
      const blocked = document.createElement('p');
      blocked.className = 'tokenize-status tokenize-status--danger';
      blocked.textContent = 'All production pre-flight checks must pass before tokenization.';
      deployArea.appendChild(blocked);
    }
  }

  function renderChecklist(checks, data) {
    const list = document.getElementById('checklist');
    if (!list) return;
    list.textContent = '';

    const items = [
      ['Asset Approved & Published', 'Asset must be approved by an admin before tokenization.', checks.asset_approved],
      ['Token Supply Defined', `${Number(data.tokens_total || 0).toLocaleString()} tokens configured.`, checks.has_token_supply],
      ['Token Price Set', `Price: ${formatCurrency(data.token_price_cents)} per token.`, checks.has_price],
      ['Legal Documents Present', 'At least one asset document is attached for operator review.', checks.legal_documents_present],
      ['Funding Status Ready', `Current status: ${formatStatus(data.funding_status)}.`, checks.funding_ready],
      ['Metadata URI Ready', data.metadata_uri || 'Metadata endpoint will be generated for this asset.', checks.metadata_uri_ready],
      ['Chain Configuration Ready', `${formatNetwork(data.chain_network)} deployment config is available.`, checks.chain_configured],
      ['Operator Permission Verified', 'Current admin has the dedicated blockchain.tokenize permission.', checks.operator_can_tokenize],
      ['Not Already Tokenized', checks.not_already_tokenized ? 'No chain token is currently stored.' : 'Asset already has chain metadata.', checks.not_already_tokenized],
    ];

    items.forEach(([label, sub, pass], index) => {
      list.appendChild(createCheckItem(label, sub, Boolean(pass), index === items.length - 1));
    });
  }

  function createCheckItem(label, sub, pass, separated) {
    const item = document.createElement('div');
    item.className = 'preflight-item';
    if (separated) item.classList.add('preflight-item--separated');

    const icon = document.createElement('div');
    icon.className = `preflight-icon preflight-icon--${pass ? 'pass' : 'fail'}`;
    icon.textContent = pass ? '✓' : '×';

    const copy = document.createElement('div');
    copy.className = 'preflight-copy';
    const title = document.createElement('div');
    title.className = 'preflight-label';
    title.textContent = label;
    const desc = document.createElement('div');
    desc.className = 'preflight-sublabel';
    desc.textContent = sub;
    copy.append(title, desc);

    const badge = document.createElement('span');
    badge.className = `admin-badge admin-badge--${pass ? 'success' : 'danger'}`;
    badge.textContent = pass ? 'Pass' : 'Fail';

    item.append(icon, copy, badge);
    return item;
  }

  function renderAlreadyTokenized(data) {
    const checklistCard = document.getElementById('checklist-card');
    if (checklistCard) checklistCard.style.display = 'none';

    const deployArea = document.getElementById('deploy-area');
    if (deployArea) {
      deployArea.textContent = '';
      const result = document.createElement('div');
      result.className = 'deploy-result';

      const heading = document.createElement('div');
      heading.className = 'deploy-result-heading';
      const mark = document.createElement('span');
      mark.setAttribute('aria-hidden', 'true');
      mark.textContent = '✓';
      const label = document.createElement('span');
      label.textContent = 'Asset Already Tokenized';
      heading.append(mark, label);

      const grid = document.createElement('div');
      grid.className = 'deploy-result-grid';
      grid.append(
        resultField('Token ID', data.chain_token_id || '-'),
        resultField('Contract Address', data.chain_contract_address || '-', data.explorer_url)
      );

      const footer = document.createElement('div');
      footer.className = 'deploy-result-footer';
      const treasury = document.createElement('a');
      treasury.href = '/admin/blockchain-treasury';
      treasury.className = 'admin-btn admin-btn--secondary';
      treasury.textContent = 'Go to Blockchain Treasury';
      footer.appendChild(treasury);

      result.append(heading, grid, footer);
      deployArea.appendChild(result);
    }

    const tokenizeStep = document.getElementById('timeline-tokenize');
    if (tokenizeStep) {
      tokenizeStep.className = 'timeline-step timeline-step--done';
      tokenizeStep.textContent = 'Tokenized';
    }
    const liveStep = document.getElementById('timeline-live');
    if (liveStep) liveStep.className = 'timeline-step timeline-step--active';
  }

  function resultField(label, value, explorerUrl) {
    const wrap = document.createElement('div');
    const caption = document.createElement('span');
    caption.className = 'deploy-result-label';
    caption.textContent = label;
    const valueBox = document.createElement('div');
    valueBox.className = 'deploy-result-address';
    if (explorerUrl && isAddress(value)) {
      const link = document.createElement('a');
      link.href = `${explorerUrl}/address/${encodeURIComponent(value)}`;
      link.target = '_blank';
      link.rel = 'noopener noreferrer';
      link.textContent = value;
      valueBox.appendChild(link);
    } else {
      valueBox.textContent = value;
    }
    wrap.append(caption, valueBox);
    return wrap;
  }

  async function tokenizeAsset() {
    const confirmed = await confirmTokenize();
    if (!confirmed) return;

    const btn = document.getElementById('btn-tokenize');
    setButtonBusy(btn, true);

    try {
      const data = await fetchJSON(TOKENIZE_API_BASE + encodeURIComponent(assetId), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-CSRF-Token': getCsrfToken(),
        },
      });
      renderSuccess(data);
      await loadTokenizeCheck();
    } catch (err) {
      lastError = err;
      renderInlineError(`Tokenization failed: ${err.message}`);
    } finally {
      setButtonBusy(btn, false);
    }
  }

  function renderSuccess(data) {
    const deployArea = document.getElementById('deploy-area');
    if (!deployArea) return;
    const success = document.createElement('p');
    success.className = 'tokenize-status tokenize-status--success';
    success.textContent = `Asset tokenized. Token ID ${data.chain_token_id}; transaction ${data.chain_tx_hash}.`;
    deployArea.appendChild(success);
  }

  function renderInlineError(message) {
    const deployArea = document.getElementById('deploy-area');
    if (!deployArea) return;
    const existing = deployArea.querySelector('.tokenize-status--danger');
    if (existing) existing.remove();
    const error = document.createElement('p');
    error.className = 'tokenize-status tokenize-status--danger';
    error.setAttribute('role', 'alert');
    error.textContent = message;
    deployArea.appendChild(error);
  }

  function confirmTokenize() {
    return new Promise((resolve) => {
      const overlay = document.createElement('div');
      overlay.className = 'ds-modal-overlay active';
      overlay.setAttribute('role', 'dialog');
      overlay.setAttribute('aria-modal', 'true');
      overlay.setAttribute('aria-labelledby', 'tokenize-confirm-title');

      const dialog = document.createElement('div');
      dialog.className = 'ds-modal ds-modal--sm';

      // Header
      const header = document.createElement('div');
      header.className = 'ds-modal__header';
      const headerText = document.createElement('div');
      const titleEl = document.createElement('h3');
      titleEl.id = 'tokenize-confirm-title';
      titleEl.className = 'ds-modal__title';
      titleEl.textContent = 'Deploy Asset On-Chain';
      headerText.appendChild(titleEl);
      const closeBtn = document.createElement('button');
      closeBtn.type = 'button';
      closeBtn.className = 'ds-modal__close';
      closeBtn.setAttribute('aria-label', 'Close');
      closeBtn.innerHTML = '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>';
      header.append(headerText, closeBtn);
      dialog.appendChild(header);

      // Body
      const body = document.createElement('div');
      body.className = 'ds-modal__body';
      const bodyText = document.createElement('p');
      bodyText.style.margin = '0';
      bodyText.style.color = '#475467';
      bodyText.style.fontSize = '14px';
      bodyText.style.lineHeight = '1.6';
      bodyText.textContent = 'This will submit a tokenization transaction for the selected asset. Continue only after reviewing every pre-flight check.';
      body.appendChild(bodyText);
      dialog.appendChild(body);

      // Footer
      const footer = document.createElement('div');
      footer.className = 'ds-modal__footer ds-modal__footer--bordered';
      const cancel = document.createElement('button');
      cancel.type = 'button';
      cancel.className = 'ds-btn ds-btn--secondary';
      cancel.textContent = 'Cancel';
      const confirm = document.createElement('button');
      confirm.type = 'button';
      confirm.className = 'ds-btn ds-btn--primary';
      confirm.textContent = 'Deploy Asset';
      footer.append(cancel, confirm);
      dialog.appendChild(footer);

      const onKeydown = (event) => {
        if (event.key === 'Escape' && document.body.contains(overlay)) close(false);
      };
      const close = (value) => {
        document.removeEventListener('keydown', onKeydown);
        overlay.classList.remove('active');
        setTimeout(() => overlay.remove(), 200);
        resolve(value);
      };
      closeBtn.addEventListener('click', () => close(false));
      cancel.addEventListener('click', () => close(false));
      confirm.addEventListener('click', () => close(true));
      overlay.addEventListener('click', (event) => {
        if (event.target === overlay) close(false);
      });
      document.addEventListener('keydown', onKeydown);

      overlay.appendChild(dialog);
      document.body.appendChild(overlay);
      confirm.focus();
    });
  }

  function showError(msg) {
    const content = document.querySelector('.admin-content');
    if (!content) return;
    content.textContent = '';

    const header = document.createElement('div');
    header.className = 'admin-page-header';
    const title = document.createElement('h1');
    title.className = 'admin-page-title';
    title.textContent = 'Tokenize Asset';
    const subtitle = document.createElement('p');
    subtitle.className = 'admin-page-subtitle tokenize-status--danger';
    subtitle.setAttribute('role', 'alert');
    subtitle.textContent = msg;
    header.append(title, subtitle);
    content.appendChild(header);
  }

  function setBreadcrumbForPicker() {
    const breadcrumbName = document.getElementById('breadcrumb-asset-name');
    if (breadcrumbName) {
      breadcrumbName.textContent = 'Select Asset';
      breadcrumbName.href = '/admin/assets';
    }
  }

  function setButtonBusy(btn, busy) {
    if (!btn) return;
    btn.disabled = busy;
    btn.setAttribute('aria-busy', busy ? 'true' : 'false');
    btn.textContent = busy ? 'Tokenizing...' : 'Tokenize Asset - Deploy On-Chain';
  }

  function getCsrfToken() {
    const meta = document.querySelector('meta[name="csrf-token"]')?.content;
    if (meta) return meta;
    const cookie = document.cookie
      .split('; ')
      .find((row) => row.startsWith('csrf_token='));
    return cookie ? decodeURIComponent(cookie.split('=').slice(1).join('=')) : '';
  }

  function setEl(id, text) {
    const el = document.getElementById(id);
    if (el) el.textContent = text;
  }

  function clearSummaries() {
    ['summary-valuation', 'summary-price', 'summary-supply', 'summary-network'].forEach((id) => setEl(id, '-'));
  }

  function formatCurrency(cents) {
    return ((Number(cents) || 0) / 100).toLocaleString('en-US', {
      style: 'currency',
      currency: 'USD',
    });
  }

  function formatStatus(status) {
    return String(status || 'unknown').replace(/_/g, ' ');
  }

  function formatNetwork(network) {
    return network === 'polygon' || network === 'polygon_mainnet'
      ? 'Polygon PoS (Mainnet)'
      : 'Polygon Amoy (Testnet)';
  }

  function isAddress(value) {
    return /^0x[a-fA-F0-9]{40}$/.test(String(value || ''));
  }

  window.PooolAssetTokenize = {
    reload: () => (assetId ? loadTokenizeCheck() : loadTokenizeCandidates()),
    getLastError: () => lastError,
  };
})();
