/**
 * MP Settings — mp-settings.js
 * Loads, validates, diffs, and saves marketplace settings with audit history.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/settings';
  const HISTORY_API = '/api/admin/marketplace/settings/history';
  const CONTEXT_API = '/api/admin/marketplace/settings/context';
  const SCHEDULE_API = '/api/admin/marketplace/settings/schedule';
  // Permission check via probe (no /me endpoint available)

  const HIGH_RISK_FIELDS = new Set(['settlement_mode', 'matching_algorithm']);

  const FIELD_LABELS = {
    matching_algorithm: 'Matching Algorithm',
    tick_size_cents: 'Tick Size (USD)',
    min_order_size: 'Min Order Size',
    max_order_size: 'Max Order Size',
    settlement_mode: 'Settlement Mode',
    max_gas_gwei: 'Max Gas Price (Gwei)',
    settlement_batch_size: 'Settlement Batch Size',
    trading_enabled: '24/7 Trading',
    maintenance_window: 'Maintenance Window',
    weekend_trading: 'Weekend Trading',
  };

  const DEFAULT_SETTINGS = {
    matching_algorithm: 'price-time',
    tick_size_cents: 5,
    min_order_size: 1,
    max_order_size: 10000,
    settlement_mode: 'instant',
    max_gas_gwei: 5,
    settlement_batch_size: 50,
    trading_enabled: true,
    maintenance_window: false,
    weekend_trading: false,
  };

  const tradingHours = [
    { enabled: true, label: '24/7 Trading', desc: 'Marketplace is open continuously with no scheduled downtime', key: 'trading_enabled' },
    { enabled: false, label: 'Maintenance Window', desc: 'Daily maintenance from 04:00-04:30 UTC (matching engine offline)', key: 'maintenance_window' },
    { enabled: false, label: 'Weekend Trading', desc: 'Allow trading on Saturday and Sunday (disabled = weekday only)', key: 'weekend_trading' },
  ];

  // Mutable state
  let baseline = { ...DEFAULT_SETTINGS };
  let canManage = false;
  let envBeforeUnload = false;
  let lastContext = null;
  let historyEntries = [];
  let historyFilter = '';
  let historySelected = new Set();
  let lastSeenUpdate = null;
  let anomalyDismissed = false;
  let perFieldHistory = {}; // { field_key: { actor, created_at } }

  // ─── helpers ─────────────────────────────────────────────
  function csrfToken() {
    if (typeof window.getCsrfToken === 'function') return window.getCsrfToken();
    const value = `; ${document.cookie}`;
    const parts = value.split('; csrf_token=');
    return parts.length === 2 ? parts.pop().split(';').shift() : '';
  }
  function csrfHeaders(headers = {}) {
    const token = csrfToken();
    return token ? { ...headers, 'X-CSRF-Token': token } : headers;
  }
  function $(id) { return document.getElementById(id); }
  function setFieldValue(id, value) { const f = $(id); if (f) f.value = value; }

  function formatCents(cents) {
    const safe = Number.isSafeInteger(Number(cents)) ? Number(cents) : DEFAULT_SETTINGS.tick_size_cents;
    return `${Math.floor(safe / 100)}.${String(safe % 100).padStart(2, '0')}`;
  }
  function formatValue(key, value) {
    if (key === 'tick_size_cents') return `$${formatCents(value)}`;
    if (typeof value === 'boolean') return value ? 'On' : 'Off';
    return String(value);
  }

  // ─── parsing & validation ───────────────────────────────
  function parsePositiveInt(id, label) {
    const raw = ($(id)?.value || '').trim();
    if (!/^\d+$/.test(raw)) throw new Error(`${label} must be a positive whole number.`);
    const v = Number(raw);
    if (!Number.isSafeInteger(v) || v < 1) throw new Error(`${label} must be a positive whole number.`);
    return v;
  }
  function parseUsdToCents(id, label) {
    const raw = ($(id)?.value || '').trim();
    const m = raw.match(/^(\d+)(?:\.(\d{1,2}))?$/);
    if (!m) throw new Error(`${label} must be a dollar amount with at most two decimals.`);
    const dollars = Number(m[1]);
    const cents = Number((m[2] || '').padEnd(2, '0'));
    const total = dollars * 100 + cents;
    if (!Number.isSafeInteger(total) || total < 1) throw new Error(`${label} must be at least $0.01.`);
    return total;
  }

  function setFieldError(fieldId, errId, msg) {
    const f = $(fieldId);
    const e = $(errId);
    if (msg) {
      f?.classList.add('mp-invalid');
      if (e) { e.textContent = msg; e.hidden = false; }
    } else {
      f?.classList.remove('mp-invalid');
      if (e) { e.hidden = true; e.textContent = ''; }
    }
  }
  function clearAllErrors() {
    ['err-tick', 'err-min-order', 'err-max-order', 'err-batch'].forEach((id) => {
      const e = $(id); if (e) { e.hidden = true; e.textContent = ''; }
    });
    document.querySelectorAll('.mp-invalid').forEach((el) => el.classList.remove('mp-invalid'));
  }

  function validateLive() {
    let ok = true;
    // tick
    try { parseUsdToCents('setting-tick', 'Tick size'); setFieldError('setting-tick', 'err-tick', null); }
    catch (e) { setFieldError('setting-tick', 'err-tick', e.message); ok = false; }
    // min/max order
    let min = null, max = null;
    try { min = parsePositiveInt('setting-min-order', 'Min order size'); setFieldError('setting-min-order', 'err-min-order', null); }
    catch (e) { setFieldError('setting-min-order', 'err-min-order', e.message); ok = false; }
    try { max = parsePositiveInt('setting-max-order', 'Max order size'); setFieldError('setting-max-order', 'err-max-order', null); }
    catch (e) { setFieldError('setting-max-order', 'err-max-order', e.message); ok = false; }
    if (min !== null && max !== null && min > max) {
      setFieldError('setting-min-order', 'err-min-order', 'Min must be ≤ Max.');
      setFieldError('setting-max-order', 'err-max-order', 'Max must be ≥ Min.');
      ok = false;
    }
    // batch
    try { parsePositiveInt('setting-batch', 'Settlement batch size'); setFieldError('setting-batch', 'err-batch', null); }
    catch (e) { setFieldError('setting-batch', 'err-batch', e.message); ok = false; }
    return ok;
  }

  // ─── collect / apply ───────────────────────────────────
  function collectSettings() {
    if (!validateLive()) throw new Error('Fix the highlighted fields before saving.');
    return {
      matching_algorithm: $('setting-algo')?.value || DEFAULT_SETTINGS.matching_algorithm,
      tick_size_cents: parseUsdToCents('setting-tick', 'Tick size'),
      min_order_size: parsePositiveInt('setting-min-order', 'Min order size'),
      max_order_size: parsePositiveInt('setting-max-order', 'Max order size'),
      settlement_mode: $('setting-settlement')?.value || DEFAULT_SETTINGS.settlement_mode,
      max_gas_gwei: parsePositiveInt('setting-gas', 'Max gas price'),
      settlement_batch_size: parsePositiveInt('setting-batch', 'Settlement batch size'),
      trading_enabled: tradingHours[0].enabled,
      maintenance_window: tradingHours[1].enabled,
      weekend_trading: tradingHours[2].enabled,
    };
  }

  function applySettings(data) {
    const s = { ...DEFAULT_SETTINGS, ...(data || {}) };
    setFieldValue('setting-algo', s.matching_algorithm);
    setFieldValue('setting-tick', formatCents(s.tick_size_cents));
    setFieldValue('setting-min-order', s.min_order_size);
    setFieldValue('setting-max-order', s.max_order_size);
    setFieldValue('setting-settlement', s.settlement_mode);
    setFieldValue('setting-gas', s.max_gas_gwei);
    setFieldValue('setting-batch', s.settlement_batch_size);
    tradingHours[0].enabled = s.trading_enabled !== false;
    tradingHours[1].enabled = !!s.maintenance_window;
    tradingHours[2].enabled = !!s.weekend_trading;
  }

  // ─── toggles ───────────────────────────────────────────
  function renderToggles() {
    const container = $('trading-hours-body');
    if (!container) return;
    container.replaceChildren();
    const isContinuous = tradingHours[0].enabled;
    tradingHours.forEach((item, index) => {
      const row = document.createElement('div');
      row.className = 'mp-settings-toggle-row mp-th-row';
      if (index < tradingHours.length - 1) row.style.borderBottom = '1px solid var(--admin-border)';
      const lockedByContinuous = isContinuous && index > 0;
      if (lockedByContinuous) row.setAttribute('data-disabled', 'true');

      const textWrap = document.createElement('div');
      textWrap.style.flex = '1';
      const labelText = document.createElement('div');
      labelText.style.cssText = 'font-size:14px;font-weight:600;color:var(--admin-text-primary);';
      if (lockedByContinuous) {
        const tag = document.createElement('span');
        tag.className = 'mp-th-state-tag';
        tag.textContent = 'Locked by 24/7';
        labelText.append(tag);
      }
      labelText.append(item.label);
      const desc = document.createElement('div');
      desc.style.cssText = 'font-size:12px;color:var(--admin-text-muted);margin-top:2px;';
      desc.textContent = item.desc;
      textWrap.append(labelText, desc);

      const stateLabel = document.createElement('span');
      stateLabel.style.cssText = 'font-size:11px;color:var(--admin-text-muted);text-transform:uppercase;letter-spacing:.5px;margin-right:10px;';
      stateLabel.textContent = item.enabled ? 'Enabled' : 'Disabled';

      const label = document.createElement('label');
      label.className = 'mp-toggle';
      label.style.flexShrink = '0';
      const input = document.createElement('input');
      input.type = 'checkbox';
      input.checked = item.enabled;
      input.dataset.idx = String(index);
      input.setAttribute('aria-label', item.label);
      if (lockedByContinuous) input.disabled = true;
      const slider = document.createElement('span');
      slider.className = 'mp-toggle-slider';

      input.addEventListener('change', function () {
        const idx = Number(this.dataset.idx);
        tradingHours[idx].enabled = this.checked;
        stateLabel.textContent = this.checked ? 'Enabled' : 'Disabled';
        // 24/7 implies others off
        if (idx === 0 && this.checked) {
          tradingHours[1].enabled = false;
          tradingHours[2].enabled = false;
          renderToggles();
        }
        updateDirty();
      });

      label.append(input, slider);
      row.append(textWrap, stateLabel, label);
      container.appendChild(row);
    });
  }

  // ─── dirty-state ───────────────────────────────────────
  function diffAgainstBaseline() {
    let current;
    try { current = collectSettings(); } catch { return null; }
    const diffs = [];
    Object.keys(current).forEach((key) => {
      if (current[key] !== baseline[key]) {
        diffs.push({ key, before: baseline[key], after: current[key], highRisk: HIGH_RISK_FIELDS.has(key) });
      }
    });
    return diffs;
  }
  function updateDirty() {
    const diffs = diffAgainstBaseline();
    const bar = $('mp-sticky-save');
    if (!bar) return;
    if (!diffs || diffs.length === 0) {
      bar.hidden = true;
      envBeforeUnload = false;
      markChangedFields([]);
      return;
    }
    bar.hidden = false;
    envBeforeUnload = true;
    $('mp-dirty-count').textContent = String(diffs.length);
    $('mp-dirty-plural').textContent = diffs.length === 1 ? '' : 's';
    markChangedFields(diffs.map((d) => d.key));
  }

  const FIELD_TO_INPUT = {
    matching_algorithm: 'setting-algo',
    tick_size_cents: 'setting-tick',
    min_order_size: 'setting-min-order',
    max_order_size: 'setting-max-order',
    settlement_mode: 'setting-settlement',
    max_gas_gwei: 'setting-gas',
    settlement_batch_size: 'setting-batch',
  };

  function markChangedFields(keys) {
    Object.entries(FIELD_TO_INPUT).forEach(([k, id]) => {
      const input = $(id);
      const group = input?.closest('.admin-form-group');
      if (!group) return;
      const isChanged = keys.includes(k);
      group.dataset.changed = isChanged ? 'true' : 'false';
      // Manage revert button
      let revertBtn = group.querySelector('.mp-field-revert');
      if (isChanged && !revertBtn) {
        revertBtn = document.createElement('button');
        revertBtn.type = 'button';
        revertBtn.className = 'mp-field-revert';
        revertBtn.title = `Revert to ${formatValue(k, baseline[k])}`;
        revertBtn.textContent = '↺ revert';
        revertBtn.addEventListener('click', () => revertField(k));
        let meta = group.querySelector('.mp-field-meta');
        if (!meta) {
          meta = document.createElement('div');
          meta.className = 'mp-field-meta';
          group.appendChild(meta);
        }
        meta.appendChild(revertBtn);
      } else if (!isChanged && revertBtn) {
        revertBtn.remove();
      }
    });
  }

  function revertField(key) {
    const id = FIELD_TO_INPUT[key];
    if (!id) return;
    const input = $(id);
    if (!input) return;
    if (key === 'tick_size_cents') input.value = formatCents(baseline[key]);
    else input.value = baseline[key];
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }

  // ─── env + permissions ─────────────────────────────────
  function detectEnv() {
    const host = location.hostname;
    if (host === 'localhost' || host === '127.0.0.1' || host.endsWith('.local')) return 'dev';
    if (host.includes('staging') || host.includes('stg') || host.startsWith('dev.')) return 'staging';
    return 'prod';
  }
  function showEnvBanner() {
    const env = detectEnv();
    const el = $('mp-env-banner');
    if (!el) return;
    el.classList.remove('mp-env-banner--prod', 'mp-env-banner--staging', 'mp-env-banner--dev');
    el.classList.add(`mp-env-banner--${env}`);
    const label = $('mp-env-banner__label');
    if (label) label.textContent = env === 'prod' ? 'PRODUCTION' : env === 'staging' ? 'STAGING' : 'DEVELOPMENT';
    const msg = el.querySelector('.mp-env-banner__msg');
    if (msg) {
      msg.textContent = env === 'prod'
        ? 'Changes affect live trading. Saves are audit-logged.'
        : env === 'staging'
          ? 'Staging environment. Safe to experiment.'
          : 'Local development.';
    }
    el.hidden = false;
  }

  function lockUI() {
    document.querySelectorAll('.mp-settings-card').forEach((c) => c.setAttribute('data-locked', 'true'));
    document.querySelectorAll('#settings-content input, #settings-content select').forEach((el) => { el.disabled = true; });
    const save = $('btn-save-settings'); if (save) save.disabled = true;
    const reset = $('btn-reset-settings'); if (reset) reset.disabled = true;
    const notice = $('mp-permission-notice'); if (notice) notice.hidden = false;
  }

  // ─── modal helpers ─────────────────────────────────────
  let lastFocus = null;
  let trapHandler = null;
  function openModal(id) {
    const m = $(id);
    if (!m) return;
    lastFocus = document.activeElement;
    m.hidden = false;
    document.body.style.overflow = 'hidden';
    m.querySelectorAll('[data-close]').forEach((el) => {
      el.addEventListener('click', () => closeModal(id), { once: true });
    });
    // Focus trap
    const focusables = m.querySelectorAll('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])');
    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    setTimeout(() => first?.focus(), 0);
    trapHandler = (e) => {
      if (e.key !== 'Tab') return;
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault(); last?.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault(); first?.focus();
      }
    };
    m.addEventListener('keydown', trapHandler);
  }
  function closeModal(id) {
    const m = $(id);
    if (!m) return;
    m.hidden = true;
    document.body.style.overflow = '';
    if (trapHandler) m.removeEventListener('keydown', trapHandler);
    trapHandler = null;
    if (lastFocus && lastFocus.focus) lastFocus.focus();
  }

  function buildDiffTable(diffs) {
    const tbody = $('mp-diff-table').querySelector('tbody');
    tbody.replaceChildren();
    diffs.forEach((d) => {
      const tr = document.createElement('tr');
      if (d.highRisk) tr.className = 'mp-diff-high';
      tr.innerHTML = `
        <td>${FIELD_LABELS[d.key] || d.key}${d.highRisk ? ' <span title="High risk">⚠</span>' : ''}</td>
        <td class="mp-diff-old">${escapeHtml(formatValue(d.key, d.before))}</td>
        <td>→</td>
        <td class="mp-diff-new">${escapeHtml(formatValue(d.key, d.after))}</td>
      `;
      tbody.appendChild(tr);
    });
  }
  function escapeHtml(s) { return String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c])); }

  function buildDryRun(diffs) {
    const wrap = $('mp-dryrun');
    const list = $('mp-dryrun-list');
    if (!wrap || !list) return;
    if (!lastContext || !diffs.length) { wrap.hidden = true; return; }
    const items = [];
    const trades = lastContext.trades_24h || 0;
    const volumeUsd = (lastContext.volume_24h_cents || 0) / 100;
    const openOrders = lastContext.open_orders || 0;
    const pending = lastContext.pending_settlements || 0;

    diffs.forEach((d) => {
      if (d.key === 'settlement_mode') {
        items.push(`<li><strong>${pending}</strong> pending settlement(s) will switch to <strong>${formatValue(d.key, d.after)}</strong> handling.</li>`);
        items.push(`<li>~<strong>${trades.toLocaleString()}</strong> trades / <strong>$${volumeUsd.toLocaleString()}</strong> volume affected (24h baseline).</li>`);
      } else if (d.key === 'tick_size_cents') {
        items.push(`<li>Existing <strong>${openOrders.toLocaleString()}</strong> open orders may need re-pricing to new tick.</li>`);
      } else if (d.key === 'min_order_size' || d.key === 'max_order_size') {
        items.push(`<li>Orders outside [${diffs.find((x) => x.key === 'min_order_size')?.after ?? baseline.min_order_size}, ${diffs.find((x) => x.key === 'max_order_size')?.after ?? baseline.max_order_size}] will be rejected on submit.</li>`);
      } else if (d.key === 'settlement_batch_size') {
        const avg = lastContext.avg_batch_size_24h || 0;
        if (avg > Number(d.after)) items.push(`<li>Avg batch size 24h is <strong>${avg}</strong> — new limit <strong>${d.after}</strong> may force more frequent batches.</li>`);
        else items.push(`<li>Current avg batch size <strong>${avg}</strong> fits new limit <strong>${d.after}</strong>.</li>`);
      } else if (d.key === 'matching_algorithm') {
        items.push(`<li>Reorders priority for <strong>${openOrders.toLocaleString()}</strong> open orders.</li>`);
      } else if (d.key === 'trading_enabled' && !d.after) {
        items.push(`<li>Marketplace pause: <strong>${openOrders.toLocaleString()}</strong> open orders frozen.</li>`);
      }
    });

    if (!items.length) { wrap.hidden = true; return; }
    list.innerHTML = items.join('');
    wrap.hidden = false;
  }

  // ─── API ───────────────────────────────────────────────
  async function parseJsonError(res) {
    try { const d = await res.json(); return d.error || d.message || `HTTP ${res.status}`; }
    catch (_) { return `HTTP ${res.status}`; }
  }

  async function loadSettings() {
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(await parseJsonError(res));
      const data = await res.json();
      applySettings(data);
      baseline = collectSettings();
    } catch (err) {
      console.warn('[mp-settings] Load failed:', err);
      if (typeof mpToast === 'function') mpToast('Settings could not be loaded. Showing defaults only.', 'warning');
      applySettings(DEFAULT_SETTINGS);
      try { baseline = collectSettings(); } catch (_) { baseline = { ...DEFAULT_SETTINGS }; }
    }
    renderToggles();
    updateDirty();
  }

  async function saveSettings(payload, successMsg) {
    const res = await fetch(API, {
      method: 'POST', credentials: 'same-origin',
      headers: csrfHeaders({ 'Content-Type': 'application/json' }),
      body: JSON.stringify(payload),
    });
    if (!res.ok) {
      const msg = await parseJsonError(res);
      if (res.status === 403) { lockUI(); throw new Error(`Forbidden: ${msg}`); }
      throw new Error(msg);
    }
    if (typeof mpToast === 'function') mpToast(successMsg, 'success');
    baseline = { ...payload };
    updateLastSaved(new Date().toISOString(), 'you');
    updateDirty();
  }

  async function loadHistory() {
    const list = $('mp-history-list');
    list.textContent = 'Loading…';
    try {
      const res = await fetch(HISTORY_API, { credentials: 'same-origin' });
      if (!res.ok) throw new Error(await parseJsonError(res));
      const data = await res.json();
      historyEntries = data.entries || [];
      if (historyEntries.length === 0) { list.textContent = 'No changes recorded yet.'; return; }
      const top = historyEntries[0];
      updateLastSaved(top.created_at, top.actor);
      lastSeenUpdate = top.created_at;
      computePerFieldHistory();
      annotateChangedFields();
      maybeShowAnomaly(top);
      renderHistoryList();
    } catch (err) {
      list.textContent = `Failed to load history: ${err.message}`;
    }
  }

  // Quiet reload (initial + after concurrent edit)
  async function scheduleSave(payload, applyAtIso, note) {
    const res = await fetch(SCHEDULE_API, {
      method: 'POST', credentials: 'same-origin',
      headers: csrfHeaders({ 'Content-Type': 'application/json' }),
      body: JSON.stringify({ state: payload, apply_at: applyAtIso, note }),
    });
    if (!res.ok) throw new Error(await parseJsonError(res));
    return res.json();
  }

  async function loadScheduled() {
    const section = $('mp-scheduled-section');
    const list = $('mp-scheduled-list');
    if (!section || !list) return;
    try {
      const res = await fetch(SCHEDULE_API, { credentials: 'same-origin' });
      if (!res.ok) return;
      const data = await res.json();
      const pending = (data.entries || []).filter((e) => e.status === 'pending' || e.status === 'failed');
      if (!pending.length) { section.hidden = true; return; }
      list.replaceChildren();
      pending.forEach((e) => list.appendChild(renderScheduledRow(e)));
      section.hidden = false;
    } catch (_) { section.hidden = true; }
  }

  function renderScheduledRow(entry) {
    const row = document.createElement('div');
    row.className = 'mp-scheduled-row';
    const left = document.createElement('div');
    left.innerHTML = `
      <div class="mp-scheduled-row__when">${formatTimestamp(entry.apply_at)} · ${relativeTime(entry.apply_at)}</div>
      <div class="mp-scheduled-row__meta">by ${escapeHtml(entry.actor)} · status: ${entry.status}${entry.note ? ' · ' + escapeHtml(entry.note) : ''}${entry.error_message ? ' · ⚠ ' + escapeHtml(entry.error_message) : ''}</div>
    `;
    row.appendChild(left);
    if (entry.status === 'pending' && canManage) {
      const cancel = document.createElement('button');
      cancel.type = 'button';
      cancel.className = 'mp-scheduled-cancel';
      cancel.textContent = 'Cancel';
      cancel.addEventListener('click', () => cancelScheduled(entry.id));
      row.appendChild(cancel);
    }
    return row;
  }

  async function cancelScheduled(id) {
    if (!window.confirm('Cancel this scheduled change?')) return;
    try {
      const res = await fetch(`${SCHEDULE_API}/${id}`, {
        method: 'DELETE', credentials: 'same-origin',
        headers: csrfHeaders(),
      });
      if (!res.ok) throw new Error(await parseJsonError(res));
      if (typeof mpToast === 'function') mpToast('Scheduled change cancelled', 'success');
      loadScheduled();
    } catch (err) {
      if (typeof mpToast === 'function') mpToast(err.message || 'Cancel failed', 'error');
    }
  }

  async function loadHistoryQuiet() {
    try {
      const res = await fetch(HISTORY_API, { credentials: 'same-origin' });
      if (!res.ok) return;
      const data = await res.json();
      historyEntries = data.entries || [];
      if (historyEntries.length) {
        const top = historyEntries[0];
        lastSeenUpdate = top.created_at;
        updateLastSaved(top.created_at, top.actor);
        computePerFieldHistory();
        annotateChangedFields();
        maybeShowAnomaly(top);
      }
    } catch (_) {}
  }

  function computePerFieldHistory() {
    perFieldHistory = {};
    historyEntries.forEach((entry) => {
      const prev = entry.previous_state || {};
      const next = entry.new_state || {};
      Object.keys(FIELD_LABELS).forEach((k) => {
        if (prev[k] !== next[k] && !perFieldHistory[k]) {
          perFieldHistory[k] = { actor: entry.actor, created_at: entry.created_at };
        }
      });
    });
  }

  function annotateChangedFields() {
    const fieldToInputId = {
      matching_algorithm: 'setting-algo',
      tick_size_cents: 'setting-tick',
      min_order_size: 'setting-min-order',
      max_order_size: 'setting-max-order',
      settlement_mode: 'setting-settlement',
      max_gas_gwei: 'setting-gas',
      settlement_batch_size: 'setting-batch',
    };
    Object.entries(fieldToInputId).forEach(([fieldKey, inputId]) => {
      const input = $(inputId);
      if (!input) return;
      const group = input.closest('.admin-form-group');
      if (!group) return;
      // remove old meta
      group.querySelectorAll('.mp-field-meta').forEach((el) => el.remove());
      const h = perFieldHistory[fieldKey];
      if (!h) return;
      const meta = document.createElement('div');
      meta.className = 'mp-field-meta';
      const ago = relativeTime(h.created_at);
      meta.innerHTML = `<span class="mp-field-meta__history">Last changed ${ago} by ${escapeHtml(h.actor)}</span>`;
      group.appendChild(meta);
    });
  }

  function maybeShowAnomaly(top) {
    if (anomalyDismissed) return;
    if (!top || !top.previous_state || !top.new_state) return;
    const ageHours = (Date.now() - new Date(top.created_at.replace(' ', 'T').replace(/\+\d{2}$/, '+00:00')).getTime()) / 3600000;
    if (ageHours > 24) return;
    const changedHigh = [...HIGH_RISK_FIELDS].filter((k) => top.previous_state[k] !== top.new_state[k]);
    if (!changedHigh.length) return;
    const banner = $('mp-anomaly-banner');
    const msg = $('mp-anomaly-banner__msg');
    const title = $('mp-anomaly-banner__title');
    if (title) title.textContent = `High-risk change ${relativeTime(top.created_at)}`;
    if (msg) msg.textContent = `${changedHigh.map((k) => FIELD_LABELS[k]).join(', ')} modified by ${top.actor}.`;
    if (banner) banner.hidden = false;
  }

  function relativeTime(iso) {
    if (!iso) return '';
    const d = new Date(iso.replace(' ', 'T').replace(/\+\d{2}$/, '+00:00'));
    if (Number.isNaN(d.getTime())) return iso;
    const sec = Math.max(1, (Date.now() - d.getTime()) / 1000);
    if (sec < 60) return `${Math.round(sec)}s ago`;
    if (sec < 3600) return `${Math.round(sec / 60)}m ago`;
    if (sec < 86400) return `${Math.round(sec / 3600)}h ago`;
    return `${Math.round(sec / 86400)}d ago`;
  }

  function renderHistoryList() {
    const list = $('mp-history-list');
    list.replaceChildren();
    const filter = historyFilter.toLowerCase();
    const visible = historyEntries.filter((e) => {
      if (!filter) return true;
      if (e.actor?.toLowerCase().includes(filter)) return true;
      const prev = e.previous_state || {};
      const next = e.new_state || {};
      return Object.keys(FIELD_LABELS).some((k) => prev[k] !== next[k] && (FIELD_LABELS[k].toLowerCase().includes(filter) || String(next[k]).toLowerCase().includes(filter)));
    });
    if (!visible.length) { list.innerHTML = '<em>No matches.</em>'; return; }
    visible.forEach((entry) => list.appendChild(renderHistoryEntry(entry)));
    updateCompareButton();
  }

  function renderHistoryEntry(entry) {
    const wrap = document.createElement('div');
    wrap.className = 'mp-history-entry';
    wrap.dataset.id = entry.id;

    const head = document.createElement('div');
    head.className = 'mp-history-entry__head';
    const left = document.createElement('div');
    left.innerHTML = `<div class="mp-history-entry__actor">${escapeHtml(entry.actor || 'unknown')}</div>`;
    const pickWrap = document.createElement('label');
    pickWrap.className = 'mp-history-pick';
    const pick = document.createElement('input');
    pick.type = 'checkbox';
    pick.checked = historySelected.has(entry.id);
    pick.addEventListener('change', () => {
      if (pick.checked) {
        if (historySelected.size >= 2) {
          pick.checked = false; return;
        }
        historySelected.add(entry.id);
      } else {
        historySelected.delete(entry.id);
      }
      updateCompareButton();
    });
    pickWrap.append(pick, document.createTextNode('compare'));
    left.appendChild(pickWrap);

    const right = document.createElement('div');
    right.className = 'mp-history-entry__time';
    right.textContent = `${formatTimestamp(entry.created_at)} · ${relativeTime(entry.created_at)}`;
    head.append(left, right);

    const diffs = document.createElement('div');
    diffs.className = 'mp-history-entry__diffs';
    const prev = entry.previous_state || {};
    const next = entry.new_state || {};
    const changed = [];
    Object.keys(FIELD_LABELS).forEach((k) => {
      if (prev[k] !== next[k]) {
        changed.push(`<div><span class="k">${FIELD_LABELS[k] || k}:</span> <span class="o">${escapeHtml(formatValue(k, prev[k]))}</span> → <span class="n">${escapeHtml(formatValue(k, next[k]))}</span></div>`);
      }
    });
    diffs.innerHTML = changed.join('') || '<em>No diff recorded.</em>';

    wrap.append(head, diffs);

    if (canManage && entry.previous_state) {
      const rb = document.createElement('button');
      rb.className = 'admin-btn admin-btn--secondary mp-history-rollback';
      rb.type = 'button';
      rb.textContent = 'Roll back to this state';
      rb.addEventListener('click', () => rollbackTo(entry.previous_state));
      wrap.appendChild(rb);
    }
    return wrap;
  }

  function updateCompareButton() {
    const btn = $('btn-history-compare');
    if (!btn) return;
    btn.textContent = `Compare selected (${historySelected.size})`;
    btn.disabled = historySelected.size !== 2;
  }

  function renderCompare() {
    const ids = [...historySelected];
    if (ids.length !== 2) return;
    const [a, b] = ids.map((id) => historyEntries.find((e) => e.id === id)).filter(Boolean);
    if (!a || !b) return;
    const stateA = a.new_state || {};
    const stateB = b.new_state || {};
    const rows = Object.keys(FIELD_LABELS).map((k) => {
      const same = stateA[k] === stateB[k];
      return `<tr${same ? '' : ' style="background:rgba(220,38,38,0.05)"'}><td>${FIELD_LABELS[k]}</td><td>${escapeHtml(formatValue(k, stateA[k]))}</td><td>${escapeHtml(formatValue(k, stateB[k]))}</td></tr>`;
    }).join('');
    const cmp = $('mp-history-compare');
    cmp.innerHTML = `
      <div class="mp-history-compare__title">Comparing ${a.actor} (${formatTimestamp(a.created_at)}) vs ${b.actor} (${formatTimestamp(b.created_at)})</div>
      <table class="mp-diff-table"><thead><tr><th>Field</th><th>A</th><th>B</th></tr></thead><tbody>${rows}</tbody></table>
    `;
    cmp.hidden = false;
  }

  async function rollbackTo(state) {
    const ok = window.confirm('Roll back to the previous state? This will be saved as a new audit entry.');
    if (!ok) return;
    const merged = { ...DEFAULT_SETTINGS, ...state };
    try {
      await saveSettings(merged, 'Rolled back to previous state');
      applySettings(merged);
      renderToggles();
      closeModal('mp-history-modal');
    } catch (err) {
      if (typeof mpToast === 'function') mpToast(err.message || 'Rollback failed.', 'error');
    }
  }

  function formatTimestamp(s) {
    if (!s) return '';
    const d = new Date(s.replace(' ', 'T').replace(/\+\d{2}$/, '+00:00'));
    if (Number.isNaN(d.getTime())) return s;
    return d.toLocaleString();
  }
  function updateLastSaved(timestamp, actor) {
    const el = $('mp-last-saved');
    if (!el) return;
    el.textContent = `Last saved ${formatTimestamp(timestamp)} by ${actor}`;
    el.hidden = false;
  }

  // ─── save flow with diff modal ─────────────────────────
  async function attemptSave() {
    let payload;
    try { payload = collectSettings(); }
    catch (err) {
      if (typeof mpToast === 'function') mpToast(err.message, 'error');
      return;
    }
    const diffs = diffAgainstBaseline();
    if (!diffs || diffs.length === 0) {
      if (typeof mpToast === 'function') mpToast('No changes to save.', 'info');
      return;
    }
    buildDiffTable(diffs);
    buildDryRun(diffs);
    const hasHighRisk = diffs.some((d) => d.highRisk);
    const warn = $('mp-confirm-warn');
    const confirmInput = $('mp-confirm-text');
    const btn = $('btn-confirm-save');
    if (hasHighRisk) {
      warn.hidden = false;
      btn.disabled = true;
      confirmInput.value = '';
      confirmInput.oninput = () => { btn.disabled = confirmInput.value.trim() !== 'CONFIRM'; };
    } else {
      warn.hidden = true;
      btn.disabled = false;
    }
    btn.onclick = async () => {
      btn.disabled = true;
      const scheduleOn = $('mp-schedule-enable')?.checked;
      btn.textContent = scheduleOn ? 'Scheduling…' : 'Saving…';
      try {
        if (scheduleOn) {
          const whenLocal = $('mp-schedule-when')?.value;
          if (!whenLocal) throw new Error('Pick a date/time.');
          const applyAt = new Date(whenLocal).toISOString();
          const note = $('mp-schedule-note')?.value || null;
          await scheduleSave(payload, applyAt, note);
          if (typeof mpToast === 'function') mpToast(`Change scheduled for ${formatTimestamp(applyAt)}`, 'success');
        } else {
          await saveSettings(payload, 'Marketplace settings saved');
        }
        closeModal('mp-confirm-modal');
      } catch (err) {
        if (typeof mpToast === 'function') mpToast(err.message || 'Save failed.', 'error');
      }
      btn.disabled = false;
      btn.textContent = scheduleOn ? 'Schedule' : 'Save';
    };
    openModal('mp-confirm-modal');
  }

  function discardChanges() {
    applySettings(baseline);
    renderToggles();
    clearAllErrors();
    updateDirty();
  }

  function exportConfig() {
    let payload;
    try { payload = collectSettings(); } catch (e) { payload = baseline; }
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `marketplace-settings-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  // ─── scroll-spy ───────────────────────────────────────
  function initScrollSpy() {
    const links = document.querySelectorAll('.mp-section-nav a[data-spy]');
    if (!links.length || !('IntersectionObserver' in window)) return;
    const map = new Map();
    links.forEach((l) => {
      const id = l.getAttribute('href').slice(1);
      const sec = document.getElementById(id);
      if (sec) map.set(sec, l);
    });
    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            links.forEach((l) => l.classList.remove('active'));
            map.get(e.target)?.classList.add('active');
          }
        });
      },
      { rootMargin: '-30% 0px -60% 0px', threshold: 0 }
    );
    map.forEach((_, sec) => io.observe(sec));
  }

  // ─── live context (gas, batch utilization) ───────────
  async function fetchLiveContext() {
    try {
      const res = await fetch(CONTEXT_API, { credentials: 'same-origin' });
      if (!res.ok) return;
      const data = await res.json();
      applyLiveContext(data);
    } catch (_) { /* silent */ }
  }
  function applyLiveContext(ctx) {
    if (!ctx) return;
    lastContext = ctx;
    const gasEl = document.querySelector('[data-live="gas"]');
    if (gasEl && ctx.network_gas_gwei != null) {
      gasEl.textContent = `${ctx.network_gas_gwei} gwei`;
      const limit = Number($('setting-gas')?.value) || 0;
      const liveWrap = $('live-gas');
      if (liveWrap) liveWrap.classList.toggle('mp-live-warn', ctx.network_gas_gwei > limit);
    }
    const utilEl = document.querySelector('[data-live="batch_util"]');
    if (utilEl && ctx.avg_batch_size_24h != null && ctx.batch_size_limit != null) {
      const pct = Math.min(100, (ctx.avg_batch_size_24h / ctx.batch_size_limit) * 100);
      const level = pct > 80 ? 'high' : pct > 40 ? 'med' : 'low';
      utilEl.innerHTML = `${ctx.avg_batch_size_24h}/${ctx.batch_size_limit}` +
        `<span class="mp-util-bar"><span class="mp-util-bar__fill" data-level="${level}" style="width:${pct.toFixed(0)}%"></span></span>`;
    }
    // Concurrent-edit detection
    if (lastSeenUpdate && ctx.last_settings_update && ctx.last_settings_update !== lastSeenUpdate) {
      showConcurrentBanner(ctx.last_settings_update);
    }
    if (!lastSeenUpdate && ctx.last_settings_update) {
      lastSeenUpdate = ctx.last_settings_update;
    }
  }

  function showConcurrentBanner(timestamp) {
    const el = $('mp-concurrent-banner');
    if (!el) return;
    const actorEl = $('mp-concurrent-actor');
    if (actorEl) actorEl.textContent = `Latest save at ${formatTimestamp(timestamp)}.`;
    el.hidden = false;
  }
  function reloadAfterConcurrent() {
    $('mp-concurrent-banner').hidden = true;
    lastSeenUpdate = null;
    loadSettings();
    loadHistoryQuiet();
  }

  // ─── init ──────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', async () => {
    showEnvBanner();
    canManage = true;
    await loadSettings();
    // Try to fetch latest history entry for "last saved" line
    fetch(HISTORY_API, { credentials: 'same-origin' })
      .then((r) => r.ok ? r.json() : null)
      .then((d) => { if (d?.entries?.[0]) updateLastSaved(d.entries[0].created_at, d.entries[0].actor); })
      .catch(() => {});

    // Live validation + dirty tracking
    document.querySelectorAll('#settings-content input, #settings-content select').forEach((el) => {
      el.addEventListener('input', () => { validateLive(); updateDirty(); });
      el.addEventListener('change', () => { validateLive(); updateDirty(); });
    });

    // Steppers (#25)
    document.querySelectorAll('.mp-stepper__btn').forEach((btn) => {
      btn.addEventListener('click', () => {
        const target = $(btn.dataset.target);
        if (!target || target.disabled) return;
        const step = Number(btn.dataset.step) || 1;
        const min = Number(target.min) || 0;
        const cur = Number(target.value) || 0;
        const next = Math.max(min, cur + step);
        target.value = String(next);
        target.dispatchEvent(new Event('input', { bubbles: true }));
        target.dispatchEvent(new Event('change', { bubbles: true }));
      });
    });

    // Scroll-spy nav (#19)
    initScrollSpy();

    // Live context fetch (#14, #15)
    fetchLiveContext();
    setInterval(fetchLiveContext, 30000);

    $('btn-save-settings')?.addEventListener('click', attemptSave);
    $('btn-save-sticky')?.addEventListener('click', attemptSave);
    $('btn-discard')?.addEventListener('click', discardChanges);
    $('btn-export-config')?.addEventListener('click', exportConfig);

    $('btn-show-history')?.addEventListener('click', () => {
      historySelected.clear();
      historyFilter = '';
      const search = $('mp-history-search'); if (search) search.value = '';
      const cmp = $('mp-history-compare'); if (cmp) cmp.hidden = true;
      openModal('mp-history-modal');
      loadHistory();
      loadScheduled();
    });

    $('mp-schedule-enable')?.addEventListener('change', (e) => {
      const fields = $('mp-schedule-fields');
      const btn = $('btn-confirm-save');
      if (fields) fields.hidden = !e.target.checked;
      if (btn) btn.textContent = e.target.checked ? 'Schedule' : 'Save';
      if (e.target.checked) {
        const w = $('mp-schedule-when');
        if (w && !w.value) {
          const t = new Date(Date.now() + 5 * 60000);
          w.value = t.toISOString().slice(0, 16);
        }
      }
    });

    $('mp-history-search')?.addEventListener('input', (e) => {
      historyFilter = e.target.value || '';
      renderHistoryList();
    });

    $('btn-history-compare')?.addEventListener('click', renderCompare);

    document.querySelectorAll('[data-dismiss-anomaly]').forEach((b) => {
      b.addEventListener('click', () => {
        anomalyDismissed = true;
        $('mp-anomaly-banner').hidden = true;
      });
    });

    $('btn-reload-concurrent')?.addEventListener('click', reloadAfterConcurrent);

    // Initial history load to populate field-level meta + anomaly
    loadHistoryQuiet().then(() => annotateChangedFields());

    $('btn-reset-settings')?.addEventListener('click', async () => {
      const confirmed = typeof window.pooolConfirm === 'function'
        ? await window.pooolConfirm({
            title: 'Reset marketplace settings',
            message: 'Restore defaults and persist as a new audit entry.',
            confirmText: 'Reset',
            cancelText: 'Cancel',
            type: 'warning',
          })
        : window.confirm('Reset marketplace settings to defaults?');
      if (!confirmed) return;
      try {
        await saveSettings(DEFAULT_SETTINGS, 'Settings reset to defaults');
        applySettings(DEFAULT_SETTINGS);
        renderToggles();
      } catch (err) {
        if (typeof mpToast === 'function') mpToast(err.message || 'Reset failed.', 'error');
      }
    });

    // Keyboard: Cmd/Ctrl+S
    document.addEventListener('keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        attemptSave();
      }
      if (e.key === 'Escape') {
        ['mp-confirm-modal', 'mp-history-modal'].forEach((id) => {
          if (!$(id)?.hidden) closeModal(id);
        });
      }
    });

    // beforeunload guard
    window.addEventListener('beforeunload', (e) => {
      if (envBeforeUnload) {
        e.preventDefault();
        e.returnValue = '';
      }
    });
  });
})();
