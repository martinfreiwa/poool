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

  // Picker state
  const pickerState = {
    assets: [],
    deployer: null,
    search: '',
    statusFilter: 'tokenizable', // tokenizable | all | tokenized
    sort: 'updated_desc', // updated_desc | created_desc | price_desc | price_asc | tokens_desc
    selectedId: null,
    detailCache: {},
    lastLoadedAt: null,
    autoRefresh: false,
    refreshTimer: null,
    focusedIndex: -1,
    hoverEl: null,
    hoverTimer: null,
  };

  const REFRESH_INTERVAL_MS = 30000;

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
    renderPickerSkeleton();
    try {
      const data = await fetchJSON(CANDIDATES_API);
      pickerState.assets = data.assets || [];
      pickerState.deployer = {
        wallet_address: data.wallet_address,
        contract_address: data.contract_address,
        network: data.network,
        explorer_url: data.explorer_url,
        balance_wei: data.deployer_balance_wei,
        balance_checked_at: data.deployer_balance_checked_at,
      };
      pickerState.lastLoadedAt = Date.now();
      renderCandidatePicker();
      bindGlobalShortcuts();
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

  function renderPickerSkeleton() {
    const checklistCard = document.getElementById('checklist-card');
    if (checklistCard) checklistCard.style.display = 'none';
    const deployArea = document.getElementById('deploy-area');
    if (!deployArea) return;
    deployArea.textContent = '';
    deployArea.classList.add('deploy-button-area--picker');
    const wrap = document.createElement('div');
    for (let i = 0; i < 6; i += 1) {
      const row = document.createElement('div');
      row.className = 'skel-row';
      row.innerHTML = `
        <div style="flex:1;min-width:0">
          <div class="skel-bar skel-bar--title"></div>
          <div class="skel-bar skel-bar--meta"></div>
        </div>
        <div class="skel-bar skel-bar--badge"></div>`;
      wrap.appendChild(row);
    }
    deployArea.appendChild(wrap);
  }

  function renderCandidatePicker() {
    const checklistCard = document.getElementById('checklist-card');
    if (checklistCard) checklistCard.style.display = 'none';
    const timeline = document.getElementById('status-timeline');
    if (timeline) timeline.style.display = 'none';
    clearSummaries();

    const deployArea = document.getElementById('deploy-area');
    if (!deployArea) return;
    deployArea.textContent = '';
    deployArea.classList.add('deploy-button-area--picker');

    // Refresh strip
    deployArea.appendChild(buildRefreshStrip());

    // Deployer wallet strip
    const strip = buildDeployerStrip();
    if (strip) deployArea.appendChild(strip);

    // Global low-gas banner (blocks deploys for ALL assets)
    const lowGasBanner = buildLowGasBanner();
    if (lowGasBanner) deployArea.appendChild(lowGasBanner);

    const header = document.createElement('div');
    header.className = 'tokenize-picker-header';
    const title = document.createElement('h2');
    title.textContent = 'Tokenizable Assets';
    const sub = document.createElement('p');
    sub.textContent = 'Select a published asset to inspect its pre-flight checks. Click a row to review inline; no navigation required.';
    header.append(title, sub);
    deployArea.appendChild(header);

    // Controls: search + sort + filter chips
    deployArea.appendChild(buildPickerControls());

    const listWrap = document.createElement('div');
    listWrap.id = 'candidate-list-wrap';
    deployArea.appendChild(listWrap);

    // Sticky action bar mount
    const stickyMount = document.createElement('div');
    stickyMount.id = 'sticky-action-mount';
    deployArea.appendChild(stickyMount);

    renderCandidateList();
    renderStickyActionBar();
  }

  function buildRefreshStrip() {
    const strip = document.createElement('div');
    strip.className = 'refresh-strip';

    const pulse = document.createElement('span');
    pulse.className = `refresh-strip__pulse ${pickerState.autoRefresh ? '' : 'refresh-strip__pulse--paused'}`;
    pulse.id = 'refresh-pulse';

    const label = document.createElement('span');
    label.id = 'refresh-label';
    label.textContent = formatRefreshLabel();

    const reload = document.createElement('button');
    reload.type = 'button';
    reload.className = 'refresh-strip__btn';
    reload.textContent = 'Refresh';
    reload.addEventListener('click', () => {
      const id = pickerState.selectedId;
      pickerState.detailCache = {};
      loadTokenizeCandidates().then(() => {
        if (id) {
          pickerState.selectedId = id;
          renderCandidateList();
          renderStickyActionBar();
        }
      });
    });

    const auto = document.createElement('button');
    auto.type = 'button';
    auto.className = 'refresh-strip__btn';
    auto.id = 'auto-refresh-btn';
    auto.textContent = pickerState.autoRefresh ? 'Auto-refresh: On' : 'Auto-refresh: Off';
    auto.addEventListener('click', toggleAutoRefresh);

    const shortcut = document.createElement('span');
    shortcut.className = 'refresh-strip__shortcut';
    shortcut.innerHTML = 'Shortcuts: <span class="kbd">/</span> search · <span class="kbd">J</span>/<span class="kbd">K</span> nav · <span class="kbd">Enter</span> select · <span class="kbd">?</span> help';

    strip.append(pulse, label, reload, auto, shortcut);
    return strip;
  }

  function formatRefreshLabel() {
    if (!pickerState.lastLoadedAt) return 'Loading…';
    const secs = Math.round((Date.now() - pickerState.lastLoadedAt) / 1000);
    if (secs < 5) return 'Updated just now';
    if (secs < 60) return `Updated ${secs}s ago`;
    return `Updated ${Math.floor(secs / 60)}m ago`;
  }

  function toggleAutoRefresh() {
    pickerState.autoRefresh = !pickerState.autoRefresh;
    if (pickerState.refreshTimer) {
      clearInterval(pickerState.refreshTimer);
      pickerState.refreshTimer = null;
    }
    if (pickerState.autoRefresh) {
      pickerState.refreshTimer = setInterval(() => {
        loadTokenizeCandidates();
      }, REFRESH_INTERVAL_MS);
    }
    const btn = document.getElementById('auto-refresh-btn');
    if (btn) btn.textContent = pickerState.autoRefresh ? 'Auto-refresh: On' : 'Auto-refresh: Off';
    const pulse = document.getElementById('refresh-pulse');
    if (pulse) {
      pulse.className = `refresh-strip__pulse ${pickerState.autoRefresh ? '' : 'refresh-strip__pulse--paused'}`;
    }
  }

  // Tick the "updated Xs ago" label every 5s
  setInterval(() => {
    const lbl = document.getElementById('refresh-label');
    if (lbl) lbl.textContent = formatRefreshLabel();
  }, 5000);

  function renderStickyActionBar() {
    const mount = document.getElementById('sticky-action-mount');
    if (!mount) return;
    mount.textContent = '';
    if (!pickerState.selectedId) return;

    const asset = pickerState.assets.find((a) => a.asset_id === pickerState.selectedId);
    if (!asset) return;
    const detail = pickerState.detailCache[pickerState.selectedId];
    const allPassed = detail?.checks?.all_passed;
    const tokenized = asset.already_tokenized;

    const bar = document.createElement('div');
    bar.className = 'sticky-action-bar';

    const info = document.createElement('div');
    info.className = 'sticky-action-bar__info';
    const t = document.createElement('div');
    t.className = 'sticky-action-bar__title';
    t.textContent = asset.title;
    const s = document.createElement('div');
    s.className = 'sticky-action-bar__sub';
    if (tokenized) {
      s.textContent = 'Already tokenized on-chain';
    } else if (!detail) {
      s.textContent = 'Loading checks…';
    } else if (allPassed) {
      s.textContent = 'All pre-flight checks passed';
    } else {
      const failed = countFailedChecks(detail.checks);
      s.textContent = `${failed} pre-flight check${failed === 1 ? '' : 's'} failing`;
    }
    const wrap = document.createElement('div');
    wrap.style.minWidth = '0';
    wrap.append(t, s);
    info.appendChild(wrap);

    const actions = document.createElement('div');
    actions.className = 'sticky-action-bar__actions';

    const dismiss = document.createElement('button');
    dismiss.type = 'button';
    dismiss.className = 'sticky-action-bar__btn sticky-action-bar__btn--ghost';
    dismiss.textContent = 'Deselect';
    dismiss.addEventListener('click', () => {
      pickerState.selectedId = null;
      clearSummaries();
      renderCandidateList();
      renderStickyActionBar();
    });

    const open = document.createElement('a');
    open.href = `/admin/asset-tokenize?id=${encodeURIComponent(asset.asset_id)}`;
    open.className = 'sticky-action-bar__btn sticky-action-bar__btn--primary';
    open.textContent = tokenized ? 'View deployment →' : (allPassed ? 'Continue to Deploy →' : 'Open & resolve →');

    actions.append(dismiss, open);
    bar.append(info, actions);
    mount.appendChild(bar);
  }

  function countFailedChecks(checks) {
    if (!checks) return 0;
    const keys = ['asset_approved', 'has_token_supply', 'has_price', 'legal_documents_present',
      'funding_ready', 'metadata_uri_ready', 'chain_configured', 'operator_can_tokenize', 'not_already_tokenized'];
    return keys.reduce((n, k) => n + (checks[k] ? 0 : 1), 0);
  }

  function buildDeployerStrip() {
    const d = pickerState.deployer;
    if (!d) return null;
    const strip = document.createElement('div');
    strip.className = 'deployer-strip';

    const isMainnet = d.network === 'polygon' || d.network === 'polygon_mainnet';
    const netBadge = document.createElement('span');
    netBadge.className = `deployer-strip__net-badge ${isMainnet ? 'deployer-strip__net-badge--mainnet' : 'deployer-strip__net-badge--testnet'}`;
    netBadge.textContent = isMainnet ? 'Polygon Mainnet' : 'Polygon Amoy (Testnet)';

    strip.appendChild(stripField('Network', netBadge));
    strip.appendChild(stripField('Deployer Wallet', addressLink(d.wallet_address, d.explorer_url)));
    strip.appendChild(stripField('Contract', addressLink(d.contract_address, d.explorer_url)));
    strip.appendChild(stripField('Gas Balance', buildGasBalanceNode(d)));
    return strip;
  }

  function buildLowGasBanner() {
    const d = pickerState.deployer;
    if (!d || !d.balance_wei) return null;
    let matic;
    try { matic = Number(BigInt(d.balance_wei)) / 1e18; } catch { return null; }
    if (matic >= 0.5) return null;
    const banner = document.createElement('div');
    banner.className = `risk-banner ${matic < 0.05 ? 'risk-banner--danger' : ''}`;
    banner.style.margin = '0 0 12px';
    const symbol = (d.network === 'polygon' || d.network === 'polygon_mainnet') ? 'POL' : 'MATIC';
    const sev = matic < 0.05 ? 'Critical' : 'Low';
    banner.innerHTML = `
      <svg class="risk-banner__icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
        <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
        <line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
      </svg>
      <div>
        <div class="risk-banner__title">${sev}: deployer wallet gas balance is ${matic.toFixed(5)} ${symbol}</div>
        <ul class="risk-banner__list">
          <li>Tokenization deploys may revert on insufficient gas. Top up wallet <code style="font-family:'SF Mono',monospace">${d.wallet_address}</code> before continuing.</li>
        </ul>
      </div>`;
    return banner;
  }

  function buildGasBalanceNode(d) {
    const wrap = document.createElement('span');
    if (!d.balance_wei) {
      wrap.textContent = 'No data';
      wrap.title = 'Gas-monitor worker has not sampled this wallet yet.';
      wrap.style.color = 'var(--admin-text-muted)';
      return wrap;
    }
    let matic;
    try {
      matic = Number(BigInt(d.balance_wei)) / 1e18;
    } catch {
      wrap.textContent = '—';
      return wrap;
    }
    const formatted = matic >= 1
      ? matic.toFixed(3)
      : matic.toFixed(5);
    const symbol = (d.network === 'polygon' || d.network === 'polygon_mainnet') ? 'POL' : 'MATIC';

    let cls = 'aging-tag--fresh';
    let warn = '';
    if (matic < 0.05) { cls = 'aging-tag--breach'; warn = ' · CRITICAL'; }
    else if (matic < 0.5) { cls = 'aging-tag--watch'; warn = ' · LOW'; }

    const badge = document.createElement('span');
    badge.className = `aging-tag ${cls}`;
    badge.textContent = `${formatted} ${symbol}${warn}`;
    if (d.balance_checked_at) {
      badge.title = `Last checked: ${new Date(d.balance_checked_at).toLocaleString()}`;
    }
    wrap.appendChild(badge);
    return wrap;
  }

  function stripField(label, valueNode) {
    const f = document.createElement('div');
    f.className = 'deployer-strip__field';
    const l = document.createElement('span');
    l.className = 'deployer-strip__label';
    l.textContent = label;
    const v = document.createElement('span');
    v.className = 'deployer-strip__value';
    if (valueNode instanceof Node) v.appendChild(valueNode);
    else v.textContent = valueNode || '—';
    f.append(l, v);
    return f;
  }

  function addressLink(addr, explorer) {
    const wrap = document.createElement('span');
    if (!addr || !isAddress(addr)) {
      wrap.textContent = addr || 'Not configured';
      return wrap;
    }
    const short = `${addr.slice(0, 6)}…${addr.slice(-4)}`;
    if (explorer) {
      const a = document.createElement('a');
      a.href = `${explorer}/address/${encodeURIComponent(addr)}`;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      a.textContent = short;
      a.title = addr;
      wrap.appendChild(a);
    } else {
      wrap.textContent = short;
      wrap.title = addr;
    }
    const copy = document.createElement('button');
    copy.type = 'button';
    copy.className = 'deployer-strip__copy';
    copy.setAttribute('aria-label', `Copy ${addr}`);
    copy.title = 'Copy address';
    copy.innerHTML = '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>';
    copy.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      navigator.clipboard?.writeText(addr).then(() => {
        copy.style.color = '#10b981';
        setTimeout(() => (copy.style.color = ''), 1200);
      });
    });
    wrap.appendChild(copy);
    return wrap;
  }

  function buildPickerControls() {
    const ctrl = document.createElement('div');
    ctrl.className = 'picker-controls';

    const search = document.createElement('input');
    search.type = 'search';
    search.className = 'picker-search';
    search.placeholder = 'Search assets by title…';
    search.value = pickerState.search;
    search.addEventListener('input', (e) => {
      pickerState.search = e.target.value;
      renderCandidateList();
    });

    const sort = document.createElement('select');
    sort.className = 'picker-select';
    sort.setAttribute('aria-label', 'Sort assets');
    [
      ['updated_desc', 'Recently updated'],
      ['created_desc', 'Newest first'],
      ['price_desc', 'Price: high → low'],
      ['price_asc', 'Price: low → high'],
      ['tokens_desc', 'Most tokens'],
    ].forEach(([val, label]) => {
      const opt = document.createElement('option');
      opt.value = val;
      opt.textContent = label;
      if (val === pickerState.sort) opt.selected = true;
      sort.appendChild(opt);
    });
    sort.addEventListener('change', (e) => {
      pickerState.sort = e.target.value;
      renderCandidateList();
    });

    const chips = document.createElement('div');
    chips.className = 'picker-chips';
    [
      ['tokenizable', 'Tokenizable'],
      ['tokenized', 'Already tokenized'],
      ['all', 'All'],
    ].forEach(([val, label]) => {
      const chip = document.createElement('button');
      chip.type = 'button';
      chip.className = `picker-chip ${pickerState.statusFilter === val ? 'picker-chip--active' : ''}`;
      chip.textContent = label;
      chip.addEventListener('click', () => {
        pickerState.statusFilter = val;
        renderCandidatePicker();
      });
      chips.appendChild(chip);
    });

    const count = document.createElement('span');
    count.className = 'picker-count';
    count.id = 'picker-count';

    const exportBtn = document.createElement('button');
    exportBtn.type = 'button';
    exportBtn.className = 'admin-btn admin-btn--secondary';
    exportBtn.style.minHeight = '32px';
    exportBtn.title = 'Export current view as CSV';
    exportBtn.innerHTML = '<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true" style="margin-right:6px;vertical-align:middle"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>Export CSV';
    exportBtn.addEventListener('click', exportCandidatesCsv);

    ctrl.append(search, sort, chips, count, exportBtn);
    return ctrl;
  }

  function exportCandidatesCsv() {
    const rows = filterAndSort(pickerState.assets);
    const header = ['asset_id', 'title', 'funding_status', 'tokens_total', 'token_price_cents', 'total_value_cents', 'already_tokenized', 'created_at', 'updated_at'];
    const csv = [header.join(',')];
    rows.forEach((a) => {
      csv.push(header.map((k) => csvCell(a[k])).join(','));
    });
    const blob = new Blob([csv.join('\n')], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    const ts = new Date().toISOString().slice(0, 10);
    link.download = `tokenize-candidates-${ts}.csv`;
    document.body.appendChild(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  function csvCell(val) {
    if (val === null || val === undefined) return '';
    const s = String(val);
    if (/[",\n]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
    return s;
  }

  function renderCandidateList() {
    const wrap = document.getElementById('candidate-list-wrap');
    if (!wrap) return;
    wrap.textContent = '';

    const filtered = filterAndSort(pickerState.assets);
    const countEl = document.getElementById('picker-count');
    if (countEl) countEl.textContent = `${filtered.length} of ${pickerState.assets.length} shown`;

    if (!filtered.length) {
      const empty = document.createElement('p');
      empty.className = 'tokenize-status tokenize-status--muted';
      empty.style.padding = '20px';
      empty.textContent = pickerState.search
        ? `No assets match "${pickerState.search}".`
        : 'No assets in this view.';
      wrap.appendChild(empty);
      return;
    }

    const list = document.createElement('div');
    list.className = 'tokenize-candidate-list';
    filtered.forEach((asset) => list.appendChild(buildCandidateRow(asset)));
    wrap.appendChild(list);
  }

  function filterAndSort(assets) {
    const q = pickerState.search.trim().toLowerCase();
    let out = assets.filter((a) => {
      if (pickerState.statusFilter === 'tokenizable' && a.already_tokenized) return false;
      if (pickerState.statusFilter === 'tokenized' && !a.already_tokenized) return false;
      if (q && !a.title.toLowerCase().includes(q)) return false;
      return true;
    });
    const cmp = {
      updated_desc: (a, b) => (b.updated_at || '').localeCompare(a.updated_at || ''),
      created_desc: (a, b) => (b.created_at || '').localeCompare(a.created_at || ''),
      price_desc: (a, b) => (b.token_price_cents || 0) - (a.token_price_cents || 0),
      price_asc: (a, b) => (a.token_price_cents || 0) - (b.token_price_cents || 0),
      tokens_desc: (a, b) => (b.tokens_total || 0) - (a.tokens_total || 0),
    }[pickerState.sort];
    if (cmp) out = out.slice().sort(cmp);
    return out;
  }

  function buildCandidateRow(asset) {
    const row = document.createElement('div');
    row.className = 'tokenize-candidate';
    if (pickerState.selectedId === asset.asset_id) row.classList.add('tokenize-candidate--selected');
    row.setAttribute('role', 'button');
    row.setAttribute('tabindex', '0');
    row.setAttribute('aria-selected', pickerState.selectedId === asset.asset_id ? 'true' : 'false');

    const top = document.createElement('div');
    top.style.display = 'flex';
    top.style.alignItems = 'center';
    top.style.justifyContent = 'space-between';
    top.style.gap = '12px';
    top.style.width = '100%';

    const main = document.createElement('div');
    main.style.minWidth = '0';
    main.style.flex = '1';
    const name = document.createElement('strong');
    name.textContent = asset.title;
    main.appendChild(name);

    const metaRow = document.createElement('div');
    metaRow.className = 'candidate-meta-row';

    const statusBadge = document.createElement('span');
    statusBadge.className = `status-badge status-badge--${asset.funding_status}`;
    statusBadge.textContent = formatStatus(asset.funding_status);
    metaRow.appendChild(statusBadge);

    const tokensSpan = document.createElement('span');
    tokensSpan.textContent = `${Number(asset.tokens_total || 0).toLocaleString()} tokens · ${formatCurrency(asset.token_price_cents)} each`;
    metaRow.appendChild(tokensSpan);

    const valSpan = document.createElement('span');
    valSpan.textContent = `Total ${formatCurrency(asset.total_value_cents)}`;
    metaRow.appendChild(valSpan);

    const aging = buildAgingTag(asset.updated_at);
    if (aging) metaRow.appendChild(aging);

    main.appendChild(metaRow);

    const right = document.createElement('div');
    right.className = 'candidate-right';

    const netPill = document.createElement('span');
    const isMainnet = pickerState.deployer?.network === 'polygon' || pickerState.deployer?.network === 'polygon_mainnet';
    netPill.className = `net-pill ${isMainnet ? '' : 'net-pill--testnet'}`;
    netPill.textContent = isMainnet ? 'Mainnet' : 'Testnet';
    right.appendChild(netPill);

    if (asset.already_tokenized) {
      const badge = document.createElement('a');
      badge.className = 'admin-badge admin-badge--success';
      badge.style.textDecoration = 'none';
      badge.style.minHeight = '22px';
      badge.style.display = 'inline-flex';
      badge.style.alignItems = 'center';
      const explorer = pickerState.deployer?.explorer_url;
      const contract = pickerState.deployer?.contract_address;
      if (explorer && isAddress(contract || '')) {
        badge.href = `${explorer}/address/${encodeURIComponent(contract)}`;
        badge.target = '_blank';
        badge.rel = 'noopener noreferrer';
        badge.title = 'View contract on Polygonscan';
      } else {
        badge.href = `/admin/asset-tokenize?id=${encodeURIComponent(asset.asset_id)}`;
        badge.title = 'View deployment details';
      }
      badge.innerHTML = '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" aria-hidden="true" style="margin-right:4px"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>Tokenized ↗';
      badge.addEventListener('click', (e) => e.stopPropagation());
      right.appendChild(badge);
    } else {
      const badge = document.createElement('span');
      badge.className = 'admin-badge admin-badge--warning';
      badge.style.minHeight = '22px';
      badge.style.display = 'inline-flex';
      badge.style.alignItems = 'center';
      badge.textContent = pickerState.selectedId === asset.asset_id ? 'Selected' : 'Review';
      right.appendChild(badge);
    }

    top.append(main, right);
    row.appendChild(top);

    if (pickerState.selectedId === asset.asset_id) {
      const detail = document.createElement('div');
      detail.className = 'candidate-detail';
      detail.id = `detail-${asset.asset_id}`;
      const cached = pickerState.detailCache[asset.asset_id];
      if (cached) {
        renderInlineDetail(detail, cached);
      } else {
        detail.innerHTML = '<div class="candidate-detail__loading">Loading pre-flight checks…</div>';
        loadInlineDetail(asset.asset_id);
      }
      row.appendChild(detail);
    }

    // Per-row risk banner
    const risks = computeRowRisks(asset);
    if (risks.length) {
      const banner = document.createElement('div');
      banner.className = `risk-banner ${risks.some((r) => r.severity === 'danger') ? 'risk-banner--danger' : ''}`;
      banner.innerHTML = `
        <svg class="risk-banner__icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
          <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
          <line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
        <div>
          <div class="risk-banner__title">${risks.length === 1 ? 'Review needed' : 'Multiple flags'}</div>
          <ul class="risk-banner__list">${risks.map((r) => `<li>${r.message}</li>`).join('')}</ul>
        </div>`;
      row.appendChild(banner);
    }

    const onSelect = (e) => {
      if (e && e.target && e.target.closest('a, button')) return;
      selectCandidate(asset);
    };
    row.addEventListener('click', onSelect);
    row.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        selectCandidate(asset);
      }
    });
    row.addEventListener('mouseenter', () => scheduleHoverPreview(row, asset));
    row.addEventListener('mouseleave', dismissHoverPreview);

    return row;
  }

  function computeRowRisks(asset) {
    const risks = [];
    const days = asset.updated_at
      ? Math.floor((Date.now() - new Date(asset.updated_at).getTime()) / 86400000)
      : null;
    if (days !== null && days > 30 && !asset.already_tokenized) {
      risks.push({ severity: 'danger', message: `Stale: no admin update for ${days} days. Re-verify before deploying.` });
    }
    if (asset.funding_status === 'exited' && !asset.already_tokenized) {
      risks.push({ severity: 'danger', message: 'Asset marked exited but never tokenized — likely shouldn\'t deploy.' });
    }
    if (asset.funding_status === 'funded' && !asset.already_tokenized) {
      risks.push({ severity: 'warn', message: 'Funded off-chain without tokenization — confirm operator intent.' });
    }
    if ((asset.tokens_total || 0) <= 0) {
      risks.push({ severity: 'danger', message: 'Token supply is zero or missing.' });
    }
    if ((asset.token_price_cents || 0) <= 0) {
      risks.push({ severity: 'danger', message: 'Token price is zero or missing.' });
    }
    return risks;
  }

  function scheduleHoverPreview(rowEl, asset) {
    clearTimeout(pickerState.hoverTimer);
    pickerState.hoverTimer = setTimeout(() => showHoverPreview(rowEl, asset), 450);
  }

  function showHoverPreview(rowEl, asset) {
    dismissHoverPreview();
    const rect = rowEl.getBoundingClientRect();
    const popover = document.createElement('div');
    popover.className = 'hover-preview';
    popover.style.top = `${window.scrollY + rect.top}px`;
    popover.style.left = `${window.scrollX + rect.right + 8}px`;
    const created = asset.created_at ? new Date(asset.created_at).toLocaleDateString() : '—';
    const updated = asset.updated_at ? new Date(asset.updated_at).toLocaleString() : '—';
    popover.innerHTML = `
      <div class="hover-preview__title">${asset.title}</div>
      <div class="hover-preview__row"><span>Status</span><strong>${formatStatus(asset.funding_status)}</strong></div>
      <div class="hover-preview__row"><span>Total value</span><strong>${formatCurrency(asset.total_value_cents)}</strong></div>
      <div class="hover-preview__row"><span>Token price</span><strong>${formatCurrency(asset.token_price_cents)}</strong></div>
      <div class="hover-preview__row"><span>Tokens</span><strong>${Number(asset.tokens_total || 0).toLocaleString()}</strong></div>
      <div class="hover-preview__row"><span>Created</span><strong>${created}</strong></div>
      <div class="hover-preview__row"><span>Updated</span><strong>${updated}</strong></div>
      <div class="hover-preview__row"><span>Asset ID</span><strong style="font-family:'SF Mono',monospace;font-size:10px">${asset.asset_id.slice(0, 8)}…</strong></div>`;
    document.body.appendChild(popover);
    pickerState.hoverEl = popover;
    // Reposition if overflows
    const popRect = popover.getBoundingClientRect();
    if (popRect.right > window.innerWidth - 8) {
      popover.style.left = `${window.scrollX + rect.left - popRect.width - 8}px`;
    }
  }

  function dismissHoverPreview() {
    clearTimeout(pickerState.hoverTimer);
    if (pickerState.hoverEl) {
      pickerState.hoverEl.remove();
      pickerState.hoverEl = null;
    }
  }

  function buildAgingTag(updatedAt) {
    if (!updatedAt) return null;
    const days = Math.floor((Date.now() - new Date(updatedAt).getTime()) / 86400000);
    if (Number.isNaN(days)) return null;
    const tag = document.createElement('span');
    let cls = 'aging-tag--fresh';
    let label;
    if (days <= 0) label = 'updated today';
    else if (days < 7) label = `updated ${days}d ago`;
    else if (days < 30) {
      cls = 'aging-tag--watch';
      label = `${days}d ago`;
    } else {
      cls = 'aging-tag--breach';
      label = `stale · ${days}d`;
    }
    tag.className = `aging-tag ${cls}`;
    tag.textContent = label;
    tag.title = `Last updated: ${new Date(updatedAt).toLocaleString()}`;
    return tag;
  }

  function selectCandidate(asset) {
    if (pickerState.selectedId === asset.asset_id) {
      pickerState.selectedId = null;
      clearSummaries();
    } else {
      pickerState.selectedId = asset.asset_id;
      // Optimistic KPI fill from list data
      setEl('summary-valuation', formatCurrency(asset.total_value_cents));
      setEl('summary-price', formatCurrency(asset.token_price_cents));
      setEl('summary-supply', Number(asset.tokens_total || 0).toLocaleString());
      setEl('summary-network', formatNetwork(pickerState.deployer?.network));
    }
    renderCandidateList();
    renderStickyActionBar();
  }

  async function loadInlineDetail(id) {
    try {
      const data = await fetchJSON(TOKENIZE_API_BASE + encodeURIComponent(id));
      pickerState.detailCache[id] = data;
      // KPI refine with authoritative values
      if (pickerState.selectedId === id) {
        setEl('summary-valuation', formatCurrency(data.total_value_cents));
        setEl('summary-price', formatCurrency(data.token_price_cents));
        setEl('summary-supply', Number(data.tokens_total || 0).toLocaleString());
        setEl('summary-network', formatNetwork(data.chain_network));
      }
      const detailEl = document.getElementById(`detail-${id}`);
      if (detailEl) renderInlineDetail(detailEl, data);
      if (pickerState.selectedId === id) renderStickyActionBar();
    } catch (err) {
      const detailEl = document.getElementById(`detail-${id}`);
      if (detailEl) {
        detailEl.innerHTML = '';
        const e = document.createElement('p');
        e.className = 'tokenize-status tokenize-status--danger';
        e.textContent = `Failed to load checks: ${err.message}`;
        detailEl.appendChild(e);
      }
    }
  }

  function renderInlineDetail(container, data) {
    container.textContent = '';

    if (data.already_tokenized) {
      const head = document.createElement('div');
      head.className = 'deploy-result-heading';
      head.innerHTML = '<span aria-hidden="true">✓</span><span>Already deployed on-chain</span>';
      container.appendChild(head);
      const grid = document.createElement('div');
      grid.className = 'deploy-result-grid';
      grid.append(
        resultField('Token ID', data.chain_token_id || '-'),
        resultField('Contract Address', data.chain_contract_address || '-', data.explorer_url)
      );
      container.appendChild(grid);
      return;
    }

    // Mini checklist
    const list = document.createElement('div');
    list.className = 'preflight-checklist';
    const checks = data.checks || {};
    const items = [
      ['Asset Approved & Published', checks.asset_approved],
      ['Token Supply Defined', checks.has_token_supply],
      ['Token Price Set', checks.has_price],
      ['Legal Documents Present', checks.legal_documents_present],
      ['Funding Status Ready', checks.funding_ready],
      ['Metadata URI Ready', checks.metadata_uri_ready],
      ['Chain Configuration Ready', checks.chain_configured],
      ['Operator Permission Verified', checks.operator_can_tokenize],
      ['Not Already Tokenized', checks.not_already_tokenized],
    ];
    items.forEach(([label, pass]) => {
      const item = document.createElement('div');
      item.className = 'preflight-item';
      const icon = document.createElement('div');
      icon.className = `preflight-icon preflight-icon--${pass ? 'pass' : 'fail'}`;
      icon.textContent = pass ? '✓' : '×';
      const copy = document.createElement('div');
      copy.className = 'preflight-copy';
      const t = document.createElement('div');
      t.className = 'preflight-label';
      t.textContent = label;
      copy.appendChild(t);
      const badge = document.createElement('span');
      badge.className = `admin-badge admin-badge--${pass ? 'success' : 'danger'}`;
      badge.textContent = pass ? 'Pass' : 'Fail';
      item.append(icon, copy, badge);
      list.appendChild(item);
    });
    container.appendChild(list);

    const footer = document.createElement('div');
    footer.style.display = 'flex';
    footer.style.justifyContent = 'flex-end';
    footer.style.gap = '8px';
    footer.style.marginTop = '12px';

    const fullPage = document.createElement('a');
    fullPage.href = `/admin/asset-tokenize?id=${encodeURIComponent(data.asset_id)}`;
    fullPage.className = 'admin-btn admin-btn--secondary';
    fullPage.textContent = 'Open full page';

    const deployBtn = document.createElement('a');
    deployBtn.href = `/admin/asset-tokenize?id=${encodeURIComponent(data.asset_id)}`;
    deployBtn.className = 'admin-btn admin-btn--primary';
    deployBtn.textContent = checks.all_passed ? 'Continue to Deploy →' : 'Resolve issues';
    if (!checks.all_passed) deployBtn.style.opacity = '0.7';

    footer.append(fullPage, deployBtn);
    container.appendChild(footer);
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
      tokenizeStep.innerHTML = '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 8l3.5 3.5L13 4"/></svg> Tokenized';
    }
    const liveStep = document.getElementById('timeline-live');
    if (liveStep) {
      if (data.published) {
        liveStep.className = 'timeline-step timeline-step--done';
        liveStep.innerHTML = '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 8l3.5 3.5L13 4"/></svg> Live on Marketplace';
      } else {
        liveStep.className = 'timeline-step timeline-step--active';
        liveStep.textContent = 'Live on Marketplace';
      }
    }
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

  function bindGlobalShortcuts() {
    if (window.__pooolTokenizeShortcuts) return;
    window.__pooolTokenizeShortcuts = true;
    document.addEventListener('keydown', (e) => {
      const inField = ['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement?.tagName);
      // "/" focus search
      if (e.key === '/' && !inField) {
        const search = document.querySelector('.picker-search');
        if (search) {
          e.preventDefault();
          search.focus();
          search.select();
        }
        return;
      }
      // "?" help
      if (e.key === '?' && !inField) {
        e.preventDefault();
        showShortcutsHelp();
        return;
      }
      // Esc closes help / clears selection
      if (e.key === 'Escape') {
        const overlay = document.querySelector('.kb-help-overlay');
        if (overlay) { overlay.remove(); return; }
        if (pickerState.selectedId) {
          pickerState.selectedId = null;
          clearSummaries();
          renderCandidateList();
          renderStickyActionBar();
        }
        return;
      }
      if (inField) return;
      // J/K row navigation
      if (e.key === 'j' || e.key === 'ArrowDown') {
        e.preventDefault();
        moveFocusedRow(1);
      } else if (e.key === 'k' || e.key === 'ArrowUp') {
        e.preventDefault();
        moveFocusedRow(-1);
      } else if (e.key === 'Enter') {
        const filtered = filterAndSort(pickerState.assets);
        const target = filtered[pickerState.focusedIndex];
        if (target) {
          e.preventDefault();
          selectCandidate(target);
        }
      }
    });
  }

  function moveFocusedRow(delta) {
    const filtered = filterAndSort(pickerState.assets);
    if (!filtered.length) return;
    pickerState.focusedIndex = Math.max(0, Math.min(filtered.length - 1, pickerState.focusedIndex + delta));
    if (pickerState.focusedIndex === -1) pickerState.focusedIndex = 0;
    const rows = document.querySelectorAll('#candidate-list-wrap .tokenize-candidate');
    const el = rows[pickerState.focusedIndex];
    if (el) {
      el.focus();
      el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  }

  function showShortcutsHelp() {
    if (document.querySelector('.kb-help-overlay')) return;
    const overlay = document.createElement('div');
    overlay.className = 'kb-help-overlay';
    overlay.innerHTML = `
      <div class="kb-help-modal" role="dialog" aria-modal="true" aria-labelledby="kb-help-title">
        <h3 id="kb-help-title">Keyboard shortcuts</h3>
        <div class="kb-help-modal__row"><span>Focus search</span><span class="kbd">/</span></div>
        <div class="kb-help-modal__row"><span>Next row</span><span><span class="kbd">J</span> / <span class="kbd">↓</span></span></div>
        <div class="kb-help-modal__row"><span>Previous row</span><span><span class="kbd">K</span> / <span class="kbd">↑</span></span></div>
        <div class="kb-help-modal__row"><span>Select / open inline</span><span class="kbd">Enter</span></div>
        <div class="kb-help-modal__row"><span>Deselect / close help</span><span class="kbd">Esc</span></div>
        <div class="kb-help-modal__row"><span>Show this help</span><span class="kbd">?</span></div>
        <div style="margin-top:16px;text-align:right">
          <button type="button" class="admin-btn admin-btn--secondary" id="kb-help-close">Close</button>
        </div>
      </div>`;
    overlay.addEventListener('click', (e) => { if (e.target === overlay) overlay.remove(); });
    document.body.appendChild(overlay);
    document.getElementById('kb-help-close')?.addEventListener('click', () => overlay.remove());
  }

  function setBreadcrumbForPicker() {
    // Drop redundant "Select Asset" segment in picker mode → "Admin › Assets › Tokenize"
    const breadcrumbName = document.getElementById('breadcrumb-asset-name');
    const breadcrumbSep = document.getElementById('breadcrumb-asset-sep');
    if (breadcrumbName) breadcrumbName.style.display = 'none';
    if (breadcrumbSep) breadcrumbSep.style.display = 'none';
    const backLabel = document.getElementById('btn-back-label');
    if (backLabel) backLabel.textContent = 'Back to Assets';
    const backBtn = document.getElementById('btn-back');
    if (backBtn) {
      backBtn.onclick = () => { window.location.href = '/admin/assets'; };
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
