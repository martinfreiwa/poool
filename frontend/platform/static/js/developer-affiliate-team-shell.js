/* global window, document, fetch */

/**
 * Shared shell for /developer/affiliate-team/* sub-pages.
 *
 * Loads on every page; handles:
 *   - KPI tiles (live counters from /api/developer/affiliate/team)
 *   - Page-header title + sub-nav active state
 *   - Invite modal + POST
 *
 * Per-page modules (developer-affiliate-team-{members,customers,products,settings}.js)
 * handle their own section data fetching.
 */

(function () {
  'use strict';

  // ─── Common DOM helpers exposed to per-page modules ───────────────────
  const DAT = (window.DAT = window.DAT || {});

  DAT.$ = (sel, root) => (root || document).querySelector(sel);
  DAT.$$ = (sel, root) => Array.from((root || document).querySelectorAll(sel));

  // Single currency contract across the platform: EUR. Matches the
  // leaderboard formatter (Intl.NumberFormat de-DE EUR). Keep this in sync
  // if a multi-currency layer ships later.
  DAT.fmtCents = function (c) {
    if (c == null || isNaN(c)) return '—';
    return new Intl.NumberFormat('de-DE', {
      style: 'currency',
      currency: 'EUR',
      maximumFractionDigits: 0,
    }).format(c / 100);
  };

  DAT.fmtDate = function (s) {
    if (!s) return '—';
    try {
      return new Date(s).toLocaleDateString();
    } catch (_) {
      return s;
    }
  };

  DAT.clear = function (el) {
    while (el && el.firstChild) el.removeChild(el.firstChild);
  };

  DAT.el = function (tag, attrs, ...children) {
    const node = document.createElement(tag);
    if (attrs) {
      for (const [k, v] of Object.entries(attrs)) {
        if (v == null) continue;
        if (k === 'class') node.className = v;
        else if (k === 'dataset') Object.assign(node.dataset, v);
        else if (k.startsWith('on') && typeof v === 'function') node.addEventListener(k.slice(2), v);
        else node.setAttribute(k, v);
      }
    }
    for (const c of children) {
      if (c == null) continue;
      if (typeof c === 'string' || typeof c === 'number') node.appendChild(document.createTextNode(String(c)));
      else node.appendChild(c);
    }
    return node;
  };

  // FC1 fix: every fetch now has a 15s AbortController timeout AND idempotent
  // GETs retry once on network error or HTTP 5xx (POST/PATCH are NOT retried
  // — even if they look safe, the server may have committed before the
  // network broke, so retrying could double-create).
  //
  // Internal helper. All three apiGet/apiPost/apiPatch route through it.
  const DEFAULT_TIMEOUT_MS = 15_000;
  async function fetchWithTimeout(path, init, timeoutMs) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), timeoutMs || DEFAULT_TIMEOUT_MS);
    try {
      return await fetch(path, { ...init, signal: controller.signal });
    } catch (err) {
      if (err && err.name === 'AbortError') {
        throw new Error(`Request to ${path} timed out after ${timeoutMs || DEFAULT_TIMEOUT_MS}ms`);
      }
      throw err;
    } finally {
      clearTimeout(timeout);
    }
  }

  DAT.apiGet = async function (path) {
    const opts = { credentials: 'same-origin', headers: { Accept: 'application/json' } };
    let res;
    try {
      res = await fetchWithTimeout(path, opts);
    } catch (networkErr) {
      // FC1: single retry for network failures on GET (idempotent).
      // Don't retry timeouts immediately — back off 500ms first.
      await new Promise((r) => setTimeout(r, 500));
      res = await fetchWithTimeout(path, opts);
    }
    if (!res.ok) {
      // FC1: retry once on 5xx (transient backend issues). 4xx is user error
      // — no retry.
      if (res.status >= 500 && res.status < 600) {
        await new Promise((r) => setTimeout(r, 500));
        res = await fetchWithTimeout(path, opts);
      }
      if (!res.ok) throw new Error(`GET ${path} failed: ${res.status}`);
    }
    return res.json();
  };

  DAT.apiPost = async function (path, body) {
    // POST: NO retry. The server may have committed before the network broke,
    // so retrying could create duplicates (e.g. double-invite, double-charge).
    const res = await fetchWithTimeout(path, {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: body ? JSON.stringify(body) : null,
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || `POST ${path} failed: ${res.status}`);
    return data;
  };

  DAT.apiPatch = async function (path, body) {
    // PATCH: NO retry, same reasoning as POST. PATCH is often idempotent in
    // theory but the server can still commit before the network fails.
    const res = await fetchWithTimeout(path, {
      method: 'PATCH',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: JSON.stringify(body || {}),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || `PATCH ${path} failed: ${res.status}`);
    return data;
  };

  /// Toast wrapper — uses global showPooolToast if available, falls back to
  /// console + (last-resort) alert so behaviour degrades gracefully.
  DAT.toast = function (title, message, type) {
    if (typeof window.showPooolToast === 'function') {
      window.showPooolToast(title, message, type || 'info');
      return;
    }
    if (type === 'error') console.error('[toast]', title, message);
    else console.log('[toast]', title, message);
  };

  /// Modal confirm dialog — returns Promise<boolean>. Built ad-hoc so we
  /// can replace native `confirm()` without depending on a heavier modal
  /// library. Trap focus + ESC to cancel, Enter to confirm.
  DAT.confirm = function (opts) {
    const o = opts || {};
    const title = o.title || 'Are you sure?';
    const message = o.message || '';
    const confirmText = o.confirmText || 'Confirm';
    const cancelText = o.cancelText || 'Cancel';
    const danger = !!o.danger;

    return new Promise((resolve) => {
      const backdrop = document.createElement('div');
      backdrop.className = 'dat-confirm-backdrop';
      backdrop.setAttribute('role', 'presentation');

      const dialog = document.createElement('div');
      dialog.className = 'dat-confirm-dialog';
      dialog.setAttribute('role', 'alertdialog');
      dialog.setAttribute('aria-modal', 'true');
      dialog.setAttribute('aria-labelledby', 'dat-confirm-title');
      dialog.setAttribute('aria-describedby', 'dat-confirm-msg');

      const h = document.createElement('h2');
      h.id = 'dat-confirm-title';
      h.className = 'dat-confirm-dialog__title';
      h.textContent = title;

      const p = document.createElement('p');
      p.id = 'dat-confirm-msg';
      p.className = 'dat-confirm-dialog__msg';
      p.textContent = message;

      const actions = document.createElement('div');
      actions.className = 'dat-confirm-dialog__actions';

      const cancelBtn = document.createElement('button');
      cancelBtn.type = 'button';
      cancelBtn.className = 'ds-btn ds-btn--secondary';
      cancelBtn.textContent = cancelText;

      const okBtn = document.createElement('button');
      okBtn.type = 'button';
      okBtn.className = 'ds-btn ' + (danger ? 'ds-btn--danger' : 'ds-btn--primary');
      okBtn.textContent = confirmText;

      actions.appendChild(cancelBtn);
      actions.appendChild(okBtn);
      dialog.appendChild(h);
      if (message) dialog.appendChild(p);
      dialog.appendChild(actions);
      backdrop.appendChild(dialog);

      const prevFocus = document.activeElement;
      const close = (value) => {
        document.removeEventListener('keydown', onKey, true);
        backdrop.remove();
        if (prevFocus && typeof prevFocus.focus === 'function') prevFocus.focus();
        resolve(value);
      };
      const onKey = (e) => {
        if (e.key === 'Escape') { e.preventDefault(); close(false); }
        else if (e.key === 'Enter') { e.preventDefault(); close(true); }
        else if (e.key === 'Tab') {
          // Trap focus between the two buttons
          const focusables = [cancelBtn, okBtn];
          const cur = document.activeElement;
          const idx = focusables.indexOf(cur);
          e.preventDefault();
          const next = (idx + (e.shiftKey ? -1 : 1) + focusables.length) % focusables.length;
          focusables[next].focus();
        }
      };
      cancelBtn.addEventListener('click', () => close(false));
      okBtn.addEventListener('click', () => close(true));
      backdrop.addEventListener('click', (e) => { if (e.target === backdrop) close(false); });
      document.addEventListener('keydown', onKey, true);

      document.body.appendChild(backdrop);
      // Focus the confirm button for danger actions (matches macOS pattern
      // where the destructive default still requires explicit Enter), and
      // the cancel button for non-danger (safer default).
      (danger ? okBtn : okBtn).focus();
    });
  };

  /// Humanize a snake_case / kebab-case status into Title Case. Used by
  /// per-page tables so status pills read "Pending approval" not
  /// "pending_developer_approval".
  DAT.humanize = function (raw) {
    if (!raw) return '';
    return String(raw)
      .replace(/[_-]+/g, ' ')
      .trim()
      .replace(/\w\S*/g, (w) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase());
  };

  /// CSV download — builds a blob from a 2-D array (header row + body rows)
  /// and triggers a synthetic <a download>. Values containing commas, quotes,
  /// or newlines are RFC 4180-quoted. Anchor is released after click.
  DAT.downloadCsv = function (filename, rows) {
    const escape = (v) => {
      if (v == null) return '';
      const s = String(v);
      if (/[",\n\r]/.test(s)) return '"' + s.replace(/"/g, '""') + '"';
      return s;
    };
    const csv = rows.map((r) => r.map(escape).join(',')).join('\r\n');
    const blob = new Blob(['﻿' + csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.style.display = 'none';
    document.body.appendChild(a);
    a.click();
    setTimeout(() => {
      a.remove();
      URL.revokeObjectURL(url);
    }, 100);
  };

  // ───────────────────────────────────────────────────────────────────
  // Phase-5: native XLSX download
  //
  // Builds the minimum 5-file OOXML structure ("Office Open XML
  // Spreadsheet" / Excel 2007+ format) Excel + Google Sheets accept:
  //   [Content_Types].xml
  //   _rels/.rels
  //   xl/workbook.xml
  //   xl/_rels/workbook.xml.rels
  //   xl/worksheets/sheet1.xml
  // Each entry is DEFLATE-compressed via the native
  // `CompressionStream('deflate-raw')` (no JS lib needed; supported in
  // every modern browser since 2024).
  //
  // Numeric cells use Excel's `n` type; everything else falls back to
  // inline string `<is><t>…</t></is>` so we don't need a shared-strings
  // table. Header row is bolded via a single style ref ("1") defined in
  // a tiny inline styles file.
  //
  // API mirrors `DAT.downloadCsv(filename, rows[]:any[][])`.
  // ───────────────────────────────────────────────────────────────────

  function _xmlEsc(s) {
    return String(s == null ? '' : s)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&apos;');
  }
  function _colLetters(idx) {
    let s = '';
    let n = idx;
    while (n >= 0) { s = String.fromCharCode(65 + (n % 26)) + s; n = Math.floor(n / 26) - 1; }
    return s;
  }

  // CRC-32 table (lazy-init).
  let _CRC_TABLE = null;
  function _crc32(bytes) {
    if (!_CRC_TABLE) {
      _CRC_TABLE = new Uint32Array(256);
      for (let i = 0; i < 256; i++) {
        let c = i;
        for (let k = 0; k < 8; k++) c = (c & 1) ? (0xEDB88320 ^ (c >>> 1)) : (c >>> 1);
        _CRC_TABLE[i] = c >>> 0;
      }
    }
    let crc = 0xFFFFFFFF;
    for (let i = 0; i < bytes.length; i++) {
      crc = (_CRC_TABLE[(crc ^ bytes[i]) & 0xFF] ^ (crc >>> 8)) >>> 0;
    }
    return (crc ^ 0xFFFFFFFF) >>> 0;
  }
  async function _deflateRaw(bytes) {
    const stream = new Blob([bytes]).stream()
      .pipeThrough(new CompressionStream('deflate-raw'));
    const buf = await new Response(stream).arrayBuffer();
    return new Uint8Array(buf);
  }

  /// Build a minimal ZIP container from a list of
  /// `{ name: string, bytes: Uint8Array }` entries.
  async function _buildZip(entries) {
    const records = [];
    let offset = 0;
    const central = [];
    for (const e of entries) {
      const nameBytes = new TextEncoder().encode(e.name);
      const compressed = await _deflateRaw(e.bytes);
      const crc = _crc32(e.bytes);
      const lfh = new DataView(new ArrayBuffer(30));
      lfh.setUint32(0, 0x04034b50, true); // local file header signature
      lfh.setUint16(4, 20, true);          // version needed
      lfh.setUint16(6, 0, true);           // gp flag
      lfh.setUint16(8, 8, true);           // method = deflate
      lfh.setUint16(10, 0, true);          // mod time
      lfh.setUint16(12, 0, true);          // mod date
      lfh.setUint32(14, crc, true);
      lfh.setUint32(18, compressed.length, true);
      lfh.setUint32(22, e.bytes.length, true);
      lfh.setUint16(26, nameBytes.length, true);
      lfh.setUint16(28, 0, true);
      records.push(new Uint8Array(lfh.buffer), nameBytes, compressed);

      const cdfh = new DataView(new ArrayBuffer(46));
      cdfh.setUint32(0, 0x02014b50, true);
      cdfh.setUint16(4, 20, true);  // version made by
      cdfh.setUint16(6, 20, true);  // version needed
      cdfh.setUint16(8, 0, true);
      cdfh.setUint16(10, 8, true);
      cdfh.setUint16(12, 0, true);
      cdfh.setUint16(14, 0, true);
      cdfh.setUint32(16, crc, true);
      cdfh.setUint32(20, compressed.length, true);
      cdfh.setUint32(24, e.bytes.length, true);
      cdfh.setUint16(28, nameBytes.length, true);
      cdfh.setUint16(30, 0, true);
      cdfh.setUint16(32, 0, true);
      cdfh.setUint16(34, 0, true);
      cdfh.setUint16(36, 0, true);
      cdfh.setUint32(38, 0, true);
      cdfh.setUint32(42, offset, true);
      central.push(new Uint8Array(cdfh.buffer), nameBytes);

      offset += 30 + nameBytes.length + compressed.length;
    }
    const centralStart = offset;
    const centralBlob = new Blob(central);
    const centralSize = centralBlob.size;
    const eocd = new DataView(new ArrayBuffer(22));
    eocd.setUint32(0, 0x06054b50, true);
    eocd.setUint16(4, 0, true);
    eocd.setUint16(6, 0, true);
    eocd.setUint16(8, entries.length, true);
    eocd.setUint16(10, entries.length, true);
    eocd.setUint32(12, centralSize, true);
    eocd.setUint32(16, centralStart, true);
    eocd.setUint16(20, 0, true);
    return new Blob([
      ...records,
      centralBlob,
      new Uint8Array(eocd.buffer),
    ], { type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' });
  }

  DAT.downloadXlsx = async function (filename, rows, sheetName) {
    if (!rows || !rows.length) return;
    const enc = new TextEncoder();
    const sheet = sheetName || 'Sheet1';

    let sheetXml = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
      + '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>';
    rows.forEach((row, rIdx) => {
      const rNum = rIdx + 1;
      sheetXml += '<row r="' + rNum + '">';
      row.forEach((cell, cIdx) => {
        const ref = _colLetters(cIdx) + rNum;
        const isHeader = rIdx === 0;
        const styleAttr = isHeader ? ' s="1"' : '';
        if (cell == null || cell === '') {
          sheetXml += '<c r="' + ref + '"' + styleAttr + '/>';
        } else if (typeof cell === 'number' && Number.isFinite(cell)) {
          sheetXml += '<c r="' + ref + '"' + styleAttr + ' t="n"><v>' + cell + '</v></c>';
        } else {
          sheetXml += '<c r="' + ref + '"' + styleAttr + ' t="inlineStr"><is><t xml:space="preserve">'
            + _xmlEsc(cell) + '</t></is></c>';
        }
      });
      sheetXml += '</row>';
    });
    sheetXml += '</sheetData></worksheet>';

    const contentTypes = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
      + '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
      + '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
      + '<Default Extension="xml" ContentType="application/xml"/>'
      + '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
      + '<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>'
      + '<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>'
      + '</Types>';
    const rootRels = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
      + '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
      + '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>'
      + '</Relationships>';
    const workbookXml = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
      + '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
      + ' xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
      + '<sheets><sheet name="' + _xmlEsc(sheet) + '" sheetId="1" r:id="rId1"/></sheets>'
      + '</workbook>';
    const workbookRels = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
      + '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
      + '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>'
      + '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>'
      + '</Relationships>';
    // Minimal styles file with one bold style at index 1 (used by headers).
    const stylesXml = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
      + '<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
      + '<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font>'
      + '<font><b/><sz val="11"/><name val="Calibri"/></font></fonts>'
      + '<fills count="1"><fill><patternFill patternType="none"/></fill></fills>'
      + '<borders count="1"><border/></borders>'
      + '<cellStyleXfs count="1"><xf/></cellStyleXfs>'
      + '<cellXfs count="2"><xf/><xf fontId="1" applyFont="1"/></cellXfs>'
      + '</styleSheet>';

    const entries = [
      { name: '[Content_Types].xml',         bytes: enc.encode(contentTypes) },
      { name: '_rels/.rels',                  bytes: enc.encode(rootRels) },
      { name: 'xl/workbook.xml',              bytes: enc.encode(workbookXml) },
      { name: 'xl/_rels/workbook.xml.rels',   bytes: enc.encode(workbookRels) },
      { name: 'xl/styles.xml',                bytes: enc.encode(stylesXml) },
      { name: 'xl/worksheets/sheet1.xml',     bytes: enc.encode(sheetXml) },
    ];

    try {
      const blob = await _buildZip(entries);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      a.style.display = 'none';
      document.body.appendChild(a);
      a.click();
      setTimeout(() => { a.remove(); URL.revokeObjectURL(url); }, 100);
    } catch (e) {
      console.error('XLSX export failed:', e);
      // Graceful fallback: CSV with same filename + .csv suffix.
      DAT.downloadCsv(filename.replace(/\.xlsx$/, '.csv'), rows);
    }
  };

  /// FG1 fix: Shared skeleton-rows helper. Was previously duplicated in
  /// members.js / customers.js / products.js with identical 10-line bodies.
  /// Renders `count` placeholder rows × `cols` cells so the table reserves
  /// vertical space and signals loading without layout-shift when real rows
  /// arrive. Call before any fetch that fills #<tbody>.
  DAT.skeletonRows = function (tbody, cols, count) {
    if (!tbody) return;
    DAT.clear(tbody);
    for (let i = 0; i < count; i++) {
      const tr = DAT.el('tr', { class: 'dat-row--skeleton' });
      for (let c = 0; c < cols; c++) {
        tr.appendChild(
          DAT.el('td', null, DAT.el('span', { class: 'dat-skeleton dat-skeleton--cell' })),
        );
      }
      tbody.appendChild(tr);
    }
  };

  /// ───────────────────────────────────────────────────────────────────────
  /// DAT.dataTable — shared table widget for /members /customers /products.
  ///
  /// Manages: sort, search (debounced), page-size selector (persisted to
  /// localStorage per pageKey), pagination, URL-state sync, loading/empty
  /// states, error states.
  ///
  /// Config:
  ///   pageKey      — unique id used for localStorage + URL state ('members'…)
  ///   endpoint     — base URL; helper appends ?q=&sort=&dir=&limit=&offset=
  ///   extraParams  — () => object — merged into every request (e.g. filter)
  ///   tbody        — <tbody> element
  ///   theadRow     — <tr> in <thead> that holds the sortable column <th>s.
  ///                  Each <th> with `data-col="key"` becomes clickable.
  ///   columns      — [{key, label, sortable?, numeric?, render(row)→Node|str}]
  ///   pagerHost    — container element for pagination + page-size + search
  ///                  controls. Helper builds the toolbar there.
  ///   onRowsLoaded — (rows, total) → void (e.g. cache for CSV export)
  ///   emptyText    — message when rows.length === 0
  /// ───────────────────────────────────────────────────────────────────────
  /// ───────────────────────────────────────────────────────────────────────
  /// DAT.chipBar — multi-select toggle chips for status filters.
  /// Phase-1 fix: gives Members/Customers a "Status: Active | Invited | …"
  /// chip row that toggles which rows are visible.
  ///
  /// Config:
  ///   host        — container element (chips append here)
  ///   pageKey     — namespacing key for localStorage persistence
  ///   chips       — [{value, label, count?}] — value is sent to backend as
  ///                  comma-separated list; label is shown to user
  ///   defaultAll  — boolean, true = empty selection = all (default true)
  ///   onChange    — (csv) => void — called when selection changes; receives
  ///                  comma-separated active values (or '' when all)
  ///   defaultActive — array of values active on first render (if no
  ///                   persisted state)
  /// Returns: { value() → csv, set(csv) → void }
  /// ───────────────────────────────────────────────────────────────────────
  DAT.chipBar = function (config) {
    const { host, pageKey, chips, onChange, defaultAll = true, defaultActive = null } = config;
    if (!host) return { value: () => '', set: () => {} };
    const LS_KEY = `dat:chipBar:${pageKey}`;
    const url = new URL(location.href);
    const fromUrl = url.searchParams.get('status');
    let persisted = null;
    try { persisted = JSON.parse(localStorage.getItem(LS_KEY) || 'null'); } catch {}
    // Precedence: URL > localStorage > defaultActive > all-empty (= all).
    const initial = fromUrl != null
      ? fromUrl.split(',').filter(Boolean)
      : Array.isArray(persisted) ? persisted
      : defaultActive || (defaultAll ? [] : chips.map((c) => c.value));

    const active = new Set(initial);

    DAT.clear(host);
    const bar = DAT.el('div', { class: 'dat-chip-bar', role: 'group', 'aria-label': 'Filter by status' });
    const allChip = DAT.el('button', {
      type: 'button', class: 'dat-chip', 'data-value': '__all',
      'aria-pressed': active.size === 0 ? 'true' : 'false',
    }, 'All');
    bar.appendChild(allChip);
    const chipEls = chips.map((c) => {
      const el = DAT.el('button', {
        type: 'button', class: 'dat-chip', 'data-value': c.value,
        'aria-pressed': active.has(c.value) ? 'true' : 'false',
      }, c.label);
      bar.appendChild(el);
      return { config: c, el };
    });
    host.appendChild(bar);

    function paint() {
      allChip.setAttribute('aria-pressed', active.size === 0 ? 'true' : 'false');
      allChip.classList.toggle('dat-chip--active', active.size === 0);
      for (const { config: c, el } of chipEls) {
        const on = active.has(c.value);
        el.setAttribute('aria-pressed', on ? 'true' : 'false');
        el.classList.toggle('dat-chip--active', on);
      }
    }
    function save() {
      try { localStorage.setItem(LS_KEY, JSON.stringify([...active])); } catch {}
      // URL sync handled by the consumer's onChange via dataTable.
    }
    function csv() { return [...active].join(','); }
    function fireChange() { if (typeof onChange === 'function') onChange(csv()); }

    allChip.addEventListener('click', () => {
      active.clear();
      paint(); save(); fireChange();
    });
    for (const { config: c, el } of chipEls) {
      el.addEventListener('click', () => {
        if (active.has(c.value)) active.delete(c.value); else active.add(c.value);
        paint(); save(); fireChange();
      });
    }
    paint();

    return {
      value: csv,
      set: (newCsv) => {
        active.clear();
        (newCsv || '').split(',').filter(Boolean).forEach((v) => active.add(v));
        paint(); save();
      },
    };
  };

  /* ──────────────────────────────────────────────────────────────────────
     DAT.topbarDateRange — shared topbar preset picker.
     Pages opt in by passing `dev_nav_show_date_range=true` to the topbar
     include, then call:

         DAT.topbarDateRange({ onChange: (r) => _table.reload() });

     The shell owns: preset → from/to translation, popover open/close,
     trigger label, URL persistence (`?preset=…&from=…&to=…`).
     Callers read the active window with DAT.currentRange() inside their
     dataTable extraParams.
     ────────────────────────────────────────────────────────────────────── */
  function _isoOf(d) {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }
  function _today() { return new Date(); }
  function _daysAgo(n) { const d = _today(); d.setDate(d.getDate() - n); return d; }
  function _startOfYear(d) { return new Date(d.getFullYear(), 0, 1); }
  function _startOfMonth(d) { return new Date(d.getFullYear(), d.getMonth(), 1); }

  DAT.presetRange = function (preset) {
    const t = _today();
    switch (preset) {
      case '7d':         return { from: _isoOf(_daysAgo(6)),  to: _isoOf(t) };
      case '14d':        return { from: _isoOf(_daysAgo(13)), to: _isoOf(t) };
      case '30d':        return { from: _isoOf(_daysAgo(29)), to: _isoOf(t) };
      case '90d':        return { from: _isoOf(_daysAgo(89)), to: _isoOf(t) };
      case 'this-month': return { from: _isoOf(_startOfMonth(t)), to: _isoOf(t) };
      case 'ytd':        return { from: _isoOf(_startOfYear(t)),  to: _isoOf(t) };
      case 'all':        return { from: '2000-01-01',              to: _isoOf(t) };
      default:           return { from: _isoOf(_daysAgo(29)), to: _isoOf(t) };
    }
  };

  DAT.presetLabel = function (preset) {
    switch (preset) {
      case '7d':         return 'Last 7 days';
      case '14d':        return 'Last 14 days';
      case '30d':        return 'Last 30 days';
      case '90d':        return 'Last 90 days';
      case 'this-month': return 'This month';
      case 'ytd':        return 'This year';
      case 'all':        return 'All time';
      case 'custom':     return 'Custom range';
      default:           return 'Last 30 days';
    }
  };

  DAT.currentRange = function () {
    const url = new URL(window.location.href);
    const preset = url.searchParams.get('preset');
    const from = url.searchParams.get('from');
    const to = url.searchParams.get('to');
    if (from && to) return { from, to, preset: preset || 'custom' };
    return Object.assign({ preset: preset || '30d' }, DAT.presetRange(preset || '30d'));
  };

  DAT.persistRange = function (from, to, preset) {
    const url = new URL(window.location.href);
    url.searchParams.set('from', from);
    url.searchParams.set('to', to);
    if (preset) url.searchParams.set('preset', preset);
    window.history.replaceState({}, '', url);
  };

  DAT.topbarDateRange = function (opts) {
    const trigger = DAT.$('#dat-topbar-range-trigger');
    const popover = DAT.$('#dat-topbar-range-popover');
    const label   = DAT.$('#dat-topbar-range-label');
    const fromInp = DAT.$('#dat-an-from');
    const toInp   = DAT.$('#dat-an-to');
    const onChange = (opts && opts.onChange) || function () {};
    if (!trigger || !popover) return null;

    function paint(range) {
      if (fromInp) fromInp.value = range.from;
      if (toInp)   toInp.value   = range.to;
      const customBtn = popover.querySelector('.dat-preset--custom');
      if (customBtn) customBtn.hidden = range.preset !== 'custom';
      popover.querySelectorAll('.dat-preset').forEach((b) => {
        b.classList.toggle('dat-preset--active', b.dataset.preset === range.preset);
      });
      if (label) {
        label.textContent = (range.preset === 'custom' && range.from && range.to)
          ? `${range.from} → ${range.to}`
          : DAT.presetLabel(range.preset);
      }
    }
    function open()  { popover.removeAttribute('hidden'); trigger.setAttribute('aria-expanded', 'true');  }
    function close() { popover.setAttribute('hidden', ''); trigger.setAttribute('aria-expanded', 'false'); }

    // Initial paint from URL/default.
    paint(DAT.currentRange());

    // Preset clicks
    popover.querySelectorAll('.dat-preset').forEach((btn) => {
      btn.addEventListener('click', () => {
        const p = btn.dataset.preset;
        if (p === 'custom') return; // informational pill — shown only when active
        const r = Object.assign({ preset: p }, DAT.presetRange(p));
        DAT.persistRange(r.from, r.to, p);
        paint(r);
        close();
        onChange(r);
      });
    });

    // Custom date inputs (debounced)
    let timer = null;
    function customApply() {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        const from = fromInp && fromInp.value;
        const to   = toInp   && toInp.value;
        if (!from || !to || from > to) return;
        const r = { from, to, preset: 'custom' };
        DAT.persistRange(from, to, 'custom');
        paint(r);
        onChange(r);
      }, 350);
    }
    if (fromInp) fromInp.addEventListener('change', customApply);
    if (toInp)   toInp.addEventListener('change', customApply);

    // Trigger open/close + outside-click + Escape
    trigger.addEventListener('click', (e) => {
      e.stopPropagation();
      if (popover.hasAttribute('hidden')) open(); else close();
    });
    document.addEventListener('click', (e) => {
      if (popover.hasAttribute('hidden')) return;
      if (popover.contains(e.target) || trigger.contains(e.target)) return;
      close();
    });
    document.addEventListener('keydown', (e) => { if (e.key === 'Escape') close(); });

    return { current: DAT.currentRange, paint, open, close };
  };

  DAT.dataTable = function (config) {
    const {
      pageKey, endpoint, extraParams = () => ({}), tbody, theadRow,
      columns, pagerHost, onRowsLoaded, emptyText = 'No results.',
      // Optional separate host for the pagination nav. When provided, the
      // <nav class="dat-pager"> renders into this element instead of the
      // top toolbar host — used when callers want the pager at the
      // bottom of the card (after the table).
      pagerFooterHost = null,
      // Optional separate host for the search input. When provided, the
      // search field renders into this element (typically inside the card
      // header) instead of the top toolbar host above the table.
      searchHost = null,
      bulkActions = null,
      rowIdKey = 'id',
      savedViews = false,
    } = config;
    if (!tbody || !theadRow || !pagerHost) {
      console.error('[DAT.dataTable] missing required element');
      return { reload: () => {}, currentState: () => ({}) };
    }

    const PAGE_SIZES = [5, 10, 25, 50, 100, 500];
    const LS_KEY = `dat:dataTable:${pageKey}`;
    const URL_PARAMS = ['q', 'sort', 'dir', 'limit', 'offset'];

    // ── State (loaded from URL first, then localStorage, then defaults) ──
    const url = new URL(location.href);
    function loadPersist() {
      try {
        return JSON.parse(localStorage.getItem(LS_KEY) || '{}');
      } catch { return {}; }
    }
    const persisted = loadPersist();
    const state = {
      q:      url.searchParams.get('q')      || '',
      sort:   url.searchParams.get('sort')   || persisted.sort   || '',
      dir:    url.searchParams.get('dir')    || persisted.dir    || 'desc',
      limit:  Number(url.searchParams.get('limit')  || persisted.limit  || 50),
      offset: Number(url.searchParams.get('offset') || 0),
      total:  0,
    };
    function savePersist() {
      try {
        localStorage.setItem(LS_KEY, JSON.stringify({
          sort: state.sort, dir: state.dir, limit: state.limit,
        }));
      } catch { /* quota or disabled — ignore */ }
    }
    function syncUrl() {
      const u = new URL(location.href);
      URL_PARAMS.forEach((k) => {
        const v = state[k];
        if (v === '' || v === 0 || v == null) u.searchParams.delete(k);
        else u.searchParams.set(k, String(v));
      });
      history.replaceState({}, '', u);
    }

    // ── Build toolbar (just search up top) + footer (rows + summary + pager)
    DAT.clear(pagerHost);
    const toolbar = DAT.el('div', { class: 'dat-table-toolbar dat-table-toolbar--compact' });
    const searchWrap = DAT.el('div', { class: 'dat-table-toolbar__search' });
    if (!searchHost) {
      // Toolbar search fields keep their icon; card-header search fields stay text-only.
      searchWrap.innerHTML = '<svg class="dat-table-toolbar__search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.3-4.3"/></svg>';
    }
    const searchInput = DAT.el('input', {
      type: 'search',
      class: 'dat-table-toolbar__search-input',
      placeholder: 'Search',
      'aria-label': 'Search this table',
      value: state.q,
    });
    searchWrap.appendChild(searchInput);
    // Search lives in the card header when searchHost is provided; otherwise
    // it stays in the top toolbar above the table.
    if (searchHost) {
      DAT.clear(searchHost);
      searchHost.appendChild(searchWrap);
    } else {
      toolbar.appendChild(searchWrap);
    }

    const summary = DAT.el('div', { class: 'dat-table-toolbar__summary', 'aria-live': 'polite' });
    // Only mount the toolbar if it has something to render — when search is
    // in the header AND pager/rows is in the footer, the top toolbar is empty.
    if (!searchHost || !pagerFooterHost) {
      pagerHost.appendChild(toolbar);
    }

    // Footer pieces — render into pagerFooterHost when provided, else fall back
    // to placing them in the top host (keeps backwards compat).
    const sizeWrap = DAT.el('label', { class: 'dat-table-footer__pagesize' });
    sizeWrap.appendChild(DAT.el('span', { class: 'dat-table-footer__pagesize-label' }, 'Rows'));
    const sizeSelect = DAT.el('select', { class: 'dat-select dat-select--compact', 'aria-label': 'Rows per page' });
    for (const n of PAGE_SIZES) {
      const opt = DAT.el('option', { value: String(n) }, String(n));
      if (n === state.limit) opt.selected = true;
      sizeSelect.appendChild(opt);
    }
    sizeWrap.appendChild(sizeSelect);

    const pager = DAT.el('nav', { class: 'dat-pager dat-pager--compact', 'aria-label': 'Pagination' });

    if (pagerFooterHost) {
      DAT.clear(pagerFooterHost);
      pagerFooterHost.classList.add('dat-table-footer');
      pagerFooterHost.appendChild(sizeWrap);
      pagerFooterHost.appendChild(summary);
      pagerFooterHost.appendChild(pager);
    } else {
      toolbar.appendChild(sizeWrap);
      toolbar.appendChild(summary);
      pagerHost.appendChild(pager);
    }

    // ── Decorate sortable column headers ────────────────────────────────
    const ths = Array.from(theadRow.querySelectorAll('th[data-col]'));
    ths.forEach((th) => {
      const col = columns.find((c) => c.key === th.dataset.col);
      if (!col || col.sortable === false) return;
      th.classList.add('dat-th--sortable');
      th.setAttribute('role', 'button');
      th.setAttribute('tabindex', '0');
      th.setAttribute('aria-sort', 'none');
      // Append sort icon
      const icon = DAT.el('span', { class: 'dat-sort-arrow', 'aria-hidden': 'true' });
      th.appendChild(document.createTextNode(' '));
      th.appendChild(icon);
      const handler = () => {
        if (state.sort === col.key) {
          state.dir = state.dir === 'asc' ? 'desc' : 'asc';
        } else {
          state.sort = col.key;
          state.dir = col.defaultDir || (col.numeric ? 'desc' : 'asc');
        }
        state.offset = 0;
        savePersist();
        load();
      };
      th.addEventListener('click', handler);
      th.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handler(); }
      });
    });
    function paintSortIcons() {
      ths.forEach((th) => {
        const k = th.dataset.col;
        const arrow = th.querySelector('.dat-sort-arrow');
        if (!arrow) return;
        if (k === state.sort) {
          th.setAttribute('aria-sort', state.dir === 'asc' ? 'ascending' : 'descending');
          arrow.textContent = state.dir === 'asc' ? '▲' : '▼';
        } else {
          th.setAttribute('aria-sort', 'none');
          arrow.textContent = '↕';
        }
      });
    }

    // ── Debounced search input ──────────────────────────────────────────
    let searchTimer = 0;
    searchInput.addEventListener('input', () => {
      clearTimeout(searchTimer);
      searchTimer = setTimeout(() => {
        state.q = searchInput.value;
        state.offset = 0;
        load();
      }, 250);
    });
    // Page-size change
    sizeSelect.addEventListener('change', () => {
      state.limit = Number(sizeSelect.value);
      state.offset = 0;
      savePersist();
      load();
    });

    // ── Render pager ────────────────────────────────────────────────────
    function renderPager() {
      DAT.clear(pager);
      const totalPages = Math.max(1, Math.ceil(state.total / state.limit));
      const curPage = Math.floor(state.offset / state.limit) + 1;
      if (totalPages <= 1) return;

      function btn(label, ariaLabel, page, disabled, active) {
        const b = DAT.el('button', {
          type: 'button',
          class: 'dat-pager__btn'
                 + (active ? ' dat-pager__btn--active' : '')
                 + (disabled ? ' dat-pager__btn--disabled' : ''),
          'aria-label': ariaLabel,
        }, label);
        if (disabled) b.disabled = true;
        if (active) b.setAttribute('aria-current', 'page');
        if (!disabled && !active && page != null) {
          b.addEventListener('click', () => {
            state.offset = (page - 1) * state.limit;
            load();
            // Scroll into view for keyboard pager-jumpers
            tbody.parentElement?.parentElement?.scrollIntoView({ behavior: 'smooth', block: 'start' });
          });
        }
        return b;
      }
      pager.appendChild(btn('‹ Prev', 'Previous page', curPage - 1, curPage === 1));
      // Page numbers: show 1, …, curPage-1, curPage, curPage+1, …, totalPages
      const pages = new Set([1, totalPages, curPage - 1, curPage, curPage + 1]);
      const sorted = [...pages].filter((p) => p >= 1 && p <= totalPages).sort((a, b) => a - b);
      let prev = 0;
      for (const p of sorted) {
        if (p - prev > 1) pager.appendChild(DAT.el('span', { class: 'dat-pager__ellipsis' }, '…'));
        pager.appendChild(btn(String(p), `Go to page ${p}`, p, false, p === curPage));
        prev = p;
      }
      pager.appendChild(btn('Next ›', 'Next page', curPage + 1, curPage === totalPages));
    }
    function renderSummary() {
      const from = state.total === 0 ? 0 : state.offset + 1;
      const to = Math.min(state.offset + state.limit, state.total);
      summary.textContent = state.total === 0
        ? '0 results'
        : `Showing ${from.toLocaleString()}–${to.toLocaleString()} of ${state.total.toLocaleString()}`;
    }

    // ── Render rows ─────────────────────────────────────────────────────
    // Phase-4: bulk-selection state. Only relevant when `bulkActions` is
    // set; otherwise everything below is a no-op (no extra <th>/<td>,
    // no selection bar). Selection is per-page — clearing on reload is
    // intentional (user expects re-fetched rows to start fresh).
    const selection = new Set();
    let selectionBar = null;
    let bulkColumnSize = 0; // 1 if checkbox column is rendered, else 0
    if (Array.isArray(bulkActions) && bulkActions.length > 0) {
      bulkColumnSize = 1;
      // Insert the select-all <th> at the start of the thead row.
      const allTh = document.createElement('th');
      allTh.className = 'dat-th--checkbox';
      allTh.setAttribute('scope', 'col');
      const allBox = DAT.el('input', { type: 'checkbox',
        'aria-label': 'Select all rows on this page' });
      allBox.addEventListener('change', () => {
        const rowBoxes = tbody.querySelectorAll('.dat-row-checkbox');
        rowBoxes.forEach((b) => {
          b.checked = allBox.checked;
          const id = b.dataset.rowId;
          if (allBox.checked) selection.add(id);
          else selection.delete(id);
        });
        paintSelectionBar();
      });
      allTh.appendChild(allBox);
      theadRow.insertBefore(allTh, theadRow.firstChild);

      // Selection bar — appended once, hidden while empty.
      selectionBar = DAT.el('div', { class: 'dat-selection-bar', role: 'toolbar', hidden: '' });
      const summarySpan = DAT.el('span', { class: 'dat-selection-bar__summary' }, '0 selected');
      selectionBar._summary = summarySpan;
      selectionBar.appendChild(summarySpan);
      for (const action of bulkActions) {
        const btn = DAT.el('button', { type: 'button', class: 'ds-btn ds-btn--sm ds-btn--secondary' }, action.label);
        btn.addEventListener('click', async () => {
          if (!selection.size) return;
          const ids = [...selection];
          try {
            await action.handler(ids);
          } catch (e) {
            DAT.toast('Bulk action', e.message || 'Failed', 'error');
          }
          selection.clear();
          paintSelectionBar();
          load();
        });
        selectionBar.appendChild(btn);
      }
      const clearBtn = DAT.el('button', { type: 'button', class: 'ds-btn ds-btn--sm ds-btn--ghost' }, 'Clear');
      clearBtn.addEventListener('click', () => {
        selection.clear();
        tbody.querySelectorAll('.dat-row-checkbox').forEach((b) => { b.checked = false; });
        const allBoxInner = theadRow.querySelector('.dat-th--checkbox input[type="checkbox"]');
        if (allBoxInner) allBoxInner.checked = false;
        paintSelectionBar();
      });
      selectionBar.appendChild(clearBtn);
      pagerHost.parentElement?.insertBefore(selectionBar, pagerHost);
    }
    function paintSelectionBar() {
      if (!selectionBar) return;
      const n = selection.size;
      selectionBar._summary.textContent = `${n} selected`;
      selectionBar.hidden = n === 0;
    }

    function renderRows(rows) {
      DAT.clear(tbody);
      const totalCols = columns.length + bulkColumnSize;
      if (!rows.length) {
        tbody.appendChild(DAT.el('tr', null,
          DAT.el('td', { colspan: totalCols, class: 'dat-empty dat-empty--cta' },
            DAT.el('span', { class: 'dat-empty__logo', 'aria-hidden': 'true' }),
            DAT.el('strong', { class: 'dat-empty__title' }, 'No matches'),
            DAT.el('p', { class: 'dat-empty__msg' }, emptyText)),
        ));
        return;
      }
      for (const row of rows) {
        const tr = DAT.el('tr');
        // Phase-4: checkbox column (if bulk actions enabled).
        if (bulkColumnSize) {
          const rid = String(row[rowIdKey] ?? '');
          const cell = DAT.el('td', { class: 'dat-td--checkbox' });
          const box = DAT.el('input', {
            type: 'checkbox',
            class: 'dat-row-checkbox',
            'data-row-id': rid,
            'aria-label': 'Select row',
          });
          if (selection.has(rid)) box.checked = true;
          box.addEventListener('change', () => {
            if (box.checked) selection.add(rid); else selection.delete(rid);
            paintSelectionBar();
          });
          cell.appendChild(box);
          tr.appendChild(cell);
        }
        for (const col of columns) {
          const cell = DAT.el('td', col.numeric ? { class: 'dat-td--num' } : null);
          const out = typeof col.render === 'function' ? col.render(row) : row[col.key];
          if (out == null) cell.textContent = '';
          else if (typeof out === 'string' || typeof out === 'number') cell.textContent = String(out);
          else cell.appendChild(out);
          tr.appendChild(cell);
        }
        tbody.appendChild(tr);
      }
    }

    // ── Phase-4: saved views (per-table preset switcher) ────────────────
    // Each view = JSON snapshot of { q, sort, dir, limit, extras }. Stored
    // under `dat:savedViews:<pageKey>` as an array of { name, view }.
    // UI: a small <select> in the toolbar. "+ Save current view" prompts
    // for a name; selecting a view restores its state and triggers load.
    let viewsSelect = null;
    if (savedViews) {
      const VIEWS_KEY = `dat:savedViews:${pageKey}`;
      function readViews() {
        try { return JSON.parse(localStorage.getItem(VIEWS_KEY) || '[]'); } catch { return []; }
      }
      function writeViews(v) {
        try { localStorage.setItem(VIEWS_KEY, JSON.stringify(v)); } catch {}
      }
      const wrap = DAT.el('label', { class: 'dat-table-toolbar__views' });
      wrap.appendChild(DAT.el('span', null, 'View: '));
      viewsSelect = DAT.el('select', { class: 'dat-select', 'aria-label': 'Saved views' });
      function refreshOptions() {
        DAT.clear(viewsSelect);
        viewsSelect.appendChild(DAT.el('option', { value: '' }, '— current —'));
        for (const v of readViews()) {
          viewsSelect.appendChild(DAT.el('option', { value: v.name }, v.name));
        }
        viewsSelect.appendChild(DAT.el('option', { value: '__save__' }, '+ Save current'));
        viewsSelect.appendChild(DAT.el('option', { value: '__delete__' }, '× Delete a view…'));
      }
      refreshOptions();
      wrap.appendChild(viewsSelect);
      toolbar.insertBefore(wrap, summary);
      viewsSelect.addEventListener('change', () => {
        const choice = viewsSelect.value;
        if (choice === '__save__') {
          const name = window.prompt('Name for this view?');
          if (name && name.trim()) {
            const list = readViews().filter((x) => x.name !== name.trim());
            list.push({
              name: name.trim(),
              view: {
                q: state.q, sort: state.sort, dir: state.dir, limit: state.limit,
              },
            });
            writeViews(list);
            refreshOptions();
          } else {
            viewsSelect.value = '';
          }
          return;
        }
        if (choice === '__delete__') {
          const names = readViews().map((v) => v.name);
          if (!names.length) { viewsSelect.value = ''; return; }
          const which = window.prompt(`Delete which view? (${names.join(', ')})`);
          if (which && names.includes(which)) {
            writeViews(readViews().filter((v) => v.name !== which));
            refreshOptions();
          }
          viewsSelect.value = '';
          return;
        }
        if (choice === '') return;
        const found = readViews().find((v) => v.name === choice);
        if (found) {
          state.q = found.view.q || '';
          state.sort = found.view.sort || '';
          state.dir = found.view.dir || 'desc';
          state.limit = Number(found.view.limit || 50);
          state.offset = 0;
          searchInput.value = state.q;
          for (const o of sizeSelect.options) o.selected = Number(o.value) === state.limit;
          savePersist();
          load();
        }
      });
    }

    // ── Loader ──────────────────────────────────────────────────────────
    let loadSeq = 0;
    async function load() {
      const mySeq = ++loadSeq;
      paintSortIcons();
      syncUrl();
      DAT.skeletonRows(tbody, columns.length, Math.min(8, state.limit));
      summary.textContent = 'Loading…';
      DAT.clear(pager);

      const params = new URLSearchParams();
      if (state.q) params.set('q', state.q);
      if (state.sort) params.set('sort', state.sort);
      if (state.dir) params.set('dir', state.dir);
      params.set('limit', String(state.limit));
      params.set('offset', String(state.offset));
      // Merge any extra params (e.g. attribution filter on customers)
      const extras = extraParams() || {};
      for (const [k, v] of Object.entries(extras)) {
        if (v != null && v !== '') params.set(k, String(v));
      }
      const url = `${endpoint}?${params}`;
      try {
        const data = await DAT.apiGet(url);
        if (mySeq !== loadSeq) return; // stale response — newer load in flight
        const rows = data.rows || data.members || [];
        state.total = Number(data.total ?? rows.length);
        renderRows(rows);
        renderSummary();
        renderPager();
        if (typeof onRowsLoaded === 'function') onRowsLoaded(rows, state.total);
      } catch (e) {
        if (mySeq !== loadSeq) return;
        DAT.clear(tbody);
        tbody.appendChild(DAT.el('tr', null,
          DAT.el('td', { colspan: columns.length, class: 'dat-empty dat-empty--error' },
            'Failed to load data. Please try again.'),
        ));
        summary.textContent = '';
        DAT.toast('Table', e.message || 'Failed to load data.', 'error');
        console.error(e);
      }
    }

    paintSortIcons();
    load();
    return {
      reload: () => { state.offset = 0; load(); },
      currentState: () => ({ ...state }),
    };
  };

  // ─── Team-info + KPI tiles ────────────────────────────────────────────
  DAT.loadTeamInfo = async function () {
    try {
      const data = await DAT.apiGet('/api/developer/affiliate/team');
      const nameEl = DAT.$('#dat-team-name');
      const metaEl = DAT.$('#dat-team-meta');
      if (nameEl) nameEl.textContent = data.display_name || 'Team';
      if (metaEl) metaEl.textContent = data.public_slug ? `Public slug: ${data.public_slug}` : 'No public slug set';

      // Shell tiles use the LEGACY pattern where the article element holds the
      // value as its own textContent. The new analytics page reuses these
      // article ids for tone styling but renders the value via an inner
      // `<strong class="dat-kpi__value">`. If that inner node exists, leave
      // the tile alone — the analytics module owns it and will populate the
      // correct period-scoped value.
      const tile = (id, value) => {
        const el = DAT.$(id);
        if (!el) return;
        if (el.querySelector('.dat-kpi__value')) return; // analytics page — skip
        el.textContent = value;
      };
      tile('#dat-tile-members', data.active_members != null ? String(data.active_members) : '0');
      const c = data.counters || {};
      tile('#dat-tile-lifetime', DAT.fmtCents(c.lifetime_commission_cents || 0));
      tile('#dat-tile-pending', DAT.fmtCents(c.pending_commission_cents || 0));
      tile('#dat-tile-payable', DAT.fmtCents(c.payable_commission_cents || 0));
      tile('#dat-tile-paid', DAT.fmtCents(c.paid_commission_cents || 0));

      // Settings-page pre-fill (only if those inputs exist on this page)
      const nameInp = DAT.$('#dat-team-display-name');
      const slugInp = DAT.$('#dat-public-slug');
      if (nameInp && data.display_name) nameInp.value = data.display_name;
      if (slugInp && data.public_slug) slugInp.value = data.public_slug;

      DAT.teamData = data; // cached for other modules (e.g. members list)
    } catch (e) {
      // FC2 fix: previously a `loadTeamInfo` failure only set a hidden
      // mount's textContent and logged to console — sub-pages saw "—"
      // forever and had no idea why. Surface the failure via toast +
      // expose a sentinel so per-page modules can react if they want.
      const nameEl = DAT.$('#dat-team-name');
      if (nameEl) nameEl.textContent = 'Failed to load team';
      console.error('[affiliate-team] loadTeamInfo failed:', e);
      DAT.teamDataError = e; // sentinel for any module checking
      DAT.toast(
        'Team data',
        'Could not load team info — KPI tiles may be stale. Try refreshing.',
        'error'
      );
    }
  };

  // ─── Invite modal (with a11y focus trap + ESC) ────────────────────────
  let modalReturnFocusEl = null;
  let modalKeydownHandler = null;

  function focusableEls(root) {
    return Array.from(
      root.querySelectorAll(
        'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    ).filter((el) => !el.hidden && el.offsetParent !== null);
  }

  function openInviteModal() {
    const modal = DAT.$('#dat-invite-modal');
    if (!modal) return;
    modalReturnFocusEl = document.activeElement;
    modal.hidden = false;
    const preview = DAT.$('#dat-invite-preview');
    if (preview) preview.hidden = true;
    const input = DAT.$('#dat-invite-email');
    if (input) {
      input.value = '';
      setTimeout(() => input.focus(), 50);
    }

    // FA5 fix: mark the rest of the page `inert` while the modal is open.
    // Previously the focus trap only forced Tab to cycle inside the panel,
    // but click/touch could still hit background elements + assistive tech
    // could still navigate them. `inert` + `aria-hidden="true"` on every
    // top-level sibling of the modal isolates the modal correctly.
    const main = DAT.$('#developer-affiliate-team-main');
    if (main) {
      main.setAttribute('inert', '');
      main.setAttribute('aria-hidden', 'true');
    }
    const sidebar = document.querySelector('.developer-dashboard-sidebar');
    if (sidebar) {
      sidebar.setAttribute('inert', '');
      sidebar.setAttribute('aria-hidden', 'true');
    }
    // Move the modal element out of the inert subtree if it was nested
    // inside `main` (it usually IS — affiliate-team templates render the
    // modal inside `<main>`). Re-parent to <body> for the duration of
    // its visibility; restore on close to keep the DOM tidy.
    if (modal.parentNode && modal.parentNode !== document.body) {
      modal._originalParent = modal.parentNode;
      modal._originalNextSibling = modal.nextSibling;
      document.body.appendChild(modal);
    }

    // ESC to close + Tab cycle inside the modal panel.
    const panel = modal.querySelector('.dat-modal__panel');
    modalKeydownHandler = (e) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        closeInviteModal();
        return;
      }
      if (e.key !== 'Tab' || !panel) return;
      const f = focusableEls(panel);
      if (!f.length) return;
      const first = f[0];
      const last = f[f.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        last.focus();
        e.preventDefault();
      } else if (!e.shiftKey && document.activeElement === last) {
        first.focus();
        e.preventDefault();
      }
    };
    document.addEventListener('keydown', modalKeydownHandler);
  }

  function closeInviteModal() {
    const modal = DAT.$('#dat-invite-modal');
    if (modal) {
      modal.hidden = true;
      // FA5 fix: restore the modal to its original DOM position if we moved
      // it for inert-isolation. Preserves any onclick listeners attached
      // via the modal's original parent context.
      if (modal._originalParent && modal.parentNode === document.body) {
        if (modal._originalNextSibling && modal._originalNextSibling.parentNode === modal._originalParent) {
          modal._originalParent.insertBefore(modal, modal._originalNextSibling);
        } else {
          modal._originalParent.appendChild(modal);
        }
        delete modal._originalParent;
        delete modal._originalNextSibling;
      }
    }
    // FA5 fix: lift inert + aria-hidden from background regions.
    const main = DAT.$('#developer-affiliate-team-main');
    if (main) {
      main.removeAttribute('inert');
      main.removeAttribute('aria-hidden');
    }
    const sidebar = document.querySelector('.developer-dashboard-sidebar');
    if (sidebar) {
      sidebar.removeAttribute('inert');
      sidebar.removeAttribute('aria-hidden');
    }
    if (modalKeydownHandler) {
      document.removeEventListener('keydown', modalKeydownHandler);
      modalKeydownHandler = null;
    }
    if (modalReturnFocusEl && typeof modalReturnFocusEl.focus === 'function') {
      modalReturnFocusEl.focus();
    }
    modalReturnFocusEl = null;
  }

  async function submitInvite(e) {
    e.preventDefault();
    const email = DAT.$('#dat-invite-email').value.trim();
    if (!email) return;
    try {
      const data = await DAT.apiPost('/api/developer/affiliate/team/invite', { email });
      const preview = DAT.$('#dat-invite-preview');
      if (data.preview_token) {
        DAT.$('#dat-invite-token').textContent = data.preview_token;
        if (preview) preview.hidden = false;
      } else {
        // F11 + FA4: response now uses generic "queued" wording even if the
        // backend silently skipped the invite. Close the modal and confirm
        // via toast (works for screen-reader users too via the live region
        // in showPooolToast).
        closeInviteModal();
        DAT.toast(
          'Invitation queued',
          data.message || 'If the email matches a POOOL user without an existing team membership, they will receive it.',
          'success'
        );
      }
      // Tell the active sub-page module to refresh if it cares about members.
      if (typeof DAT.onInviteSent === 'function') DAT.onInviteSent();
    } catch (err) {
      // FA4 fix: replace blocking alert() with non-modal toast.
      // alert() steals focus, isn't styled, and bypasses our design system.
      DAT.toast('Invitation failed', err.message || 'Could not send invitation.', 'error');
    }
  }

  function bindShell() {
    // Sidebar active-state handled by sidebar.html itself (parent + child markers).

    // Invite modal triggers
    const inviteBtn = DAT.$('#dat-invite-btn');
    if (inviteBtn) inviteBtn.addEventListener('click', openInviteModal);
    DAT.$$('[data-close="invite"]').forEach((el) => el.addEventListener('click', closeInviteModal));
    const inviteForm = DAT.$('#dat-invite-form');
    if (inviteForm) inviteForm.addEventListener('submit', submitInvite);

    // KPI tiles
    DAT.loadTeamInfo();
  }

  document.addEventListener('DOMContentLoaded', bindShell);
})();
