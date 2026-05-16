/* global window, document, fetch, location, setTimeout, clearTimeout, setInterval */

/**
 * Phase-3 P1: In-app inbox bell.
 *
 * Polls `/api/inbox/unread-count` every 60 s and renders the badge.
 * On click, opens a dropdown of the 10 latest notifications (cursor-paginated).
 * Anonymous users get the bell silently removed on the first 401 / count=0
 * cycle — we don't want a useless bell on the public marketing pages.
 *
 * Backend: `crate::inbox::router()` in /backend/src/inbox.rs.
 */
(function () {
  'use strict';

  const POLL_MS = 60_000;
  const PAGE_SIZE = 10;

  const $ = (sel, root) => (root || document).querySelector(sel);

  function escapeText(s) {
    return String(s == null ? '' : s)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;')
      .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }
  function relTime(iso) {
    if (!iso) return '';
    const t = new Date(iso).getTime();
    if (Number.isNaN(t)) return '';
    const delta = Math.max(0, Date.now() - t);
    const min = Math.floor(delta / 60_000);
    if (min < 1) return 'just now';
    if (min < 60) return min + 'm';
    const hr = Math.floor(min / 60);
    if (hr < 24) return hr + 'h';
    const d = Math.floor(hr / 24);
    if (d < 30) return d + 'd';
    return new Date(iso).toLocaleDateString();
  }

  async function fetchJson(url, init) {
    try {
      const res = await fetch(url, { credentials: 'same-origin', ...(init || {}) });
      if (!res.ok) return { __status: res.status };
      return await res.json();
    } catch (_e) {
      return { __error: true };
    }
  }

  function hideBellIfAnonymous(status) {
    if (status === 401 || status === 403) {
      const host = $('#inbox-bell');
      if (host) host.style.display = 'none';
      return true;
    }
    return false;
  }

  function setBadge(count) {
    const badge = $('#inbox-bell-badge');
    if (!badge) return;
    if (count > 0) {
      badge.textContent = count > 99 ? '99+' : String(count);
      badge.hidden = false;
    } else {
      badge.hidden = true;
    }
  }

  async function refreshBadge() {
    const data = await fetchJson('/api/inbox/unread-count');
    if (hideBellIfAnonymous(data.__status)) return;
    setBadge((data && typeof data.count === 'number') ? data.count : 0);
  }

  // ─── Dropdown panel ────────────────────────────────────────────────────
  let _panel = null;
  let _open = false;

  function ensurePanel() {
    if (_panel) return _panel;
    _panel = document.createElement('div');
    _panel.className = 'inbox-bell__panel';
    _panel.setAttribute('role', 'menu');
    _panel.hidden = true;
    _panel.innerHTML = `
      <div class="inbox-bell__panel-header">
        <span class="inbox-bell__panel-title">Notifications</span>
        <button type="button" class="inbox-bell__mark-all" id="inbox-mark-all">Mark all read</button>
      </div>
      <div class="inbox-bell__panel-body" id="inbox-panel-body">
        <div class="inbox-bell__loading">Loading…</div>
      </div>
      <a class="inbox-bell__panel-footer" href="/notifications" hidden>View all</a>`;
    $('#inbox-bell').appendChild(_panel);

    $('#inbox-mark-all').addEventListener('click', async function (e) {
      e.preventDefault();
      e.stopPropagation();
      await fetchJson('/api/inbox/read-all', { method: 'POST' });
      await Promise.all([loadPanel(), refreshBadge()]);
    });
    return _panel;
  }

  function renderRow(n) {
    const linkAttrs = n.link_url
      ? `href="${escapeText(n.link_url)}"`
      : 'href="#" tabindex="-1"';
    const unreadClass = n.is_read ? '' : ' inbox-row--unread';
    const body = n.body
      ? `<div class="inbox-row__body">${escapeText(n.body)}</div>`
      : '';
    return `
      <a class="inbox-row${unreadClass}" ${linkAttrs} data-id="${escapeText(n.id)}">
        <div class="inbox-row__main">
          <div class="inbox-row__title">${escapeText(n.title)}</div>
          ${body}
        </div>
        <div class="inbox-row__time" title="${escapeText(n.created_at)}">${escapeText(relTime(n.created_at))}</div>
      </a>`;
  }

  async function loadPanel() {
    ensurePanel();
    const body = $('#inbox-panel-body');
    body.innerHTML = '<div class="inbox-bell__loading">Loading…</div>';
    const data = await fetchJson('/api/inbox?limit=' + PAGE_SIZE);
    if (hideBellIfAnonymous(data.__status)) { _panel.hidden = true; return; }
    const items = (data && data.items) || [];
    if (!items.length) {
      body.innerHTML = '<div class="inbox-bell__empty">No notifications yet.</div>';
      return;
    }
    body.innerHTML = items.map(renderRow).join('');
    // Click→mark-as-read on each row (preserves navigation).
    body.querySelectorAll('.inbox-row').forEach(function (row) {
      row.addEventListener('click', function () {
        const id = row.getAttribute('data-id');
        // Fire-and-forget; navigation happens via the <a> default action.
        fetch('/api/inbox/' + encodeURIComponent(id) + '/read', {
          method: 'POST', credentials: 'same-origin',
        }).catch(function () {});
        row.classList.remove('inbox-row--unread');
      });
    });
  }

  async function openPanel() {
    ensurePanel();
    _panel.hidden = false;
    _open = true;
    const btn = $('#inbox-bell-btn');
    if (btn) btn.setAttribute('aria-expanded', 'true');
    await loadPanel();
  }
  function closePanel() {
    if (_panel) _panel.hidden = true;
    _open = false;
    const btn = $('#inbox-bell-btn');
    if (btn) btn.setAttribute('aria-expanded', 'false');
  }
  function togglePanel() { _open ? closePanel() : openPanel(); }

  // ─── Init ───────────────────────────────────────────────────────────────
  function init() {
    const btn = $('#inbox-bell-btn');
    if (!btn) return;
    btn.addEventListener('click', function (e) {
      e.preventDefault();
      e.stopPropagation();
      togglePanel();
    });
    // Click outside closes.
    document.addEventListener('click', function (e) {
      if (!_open) return;
      const host = $('#inbox-bell');
      if (host && !host.contains(e.target)) closePanel();
    });
    // ESC closes.
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape' && _open) closePanel();
    });

    refreshBadge();
    setInterval(refreshBadge, POLL_MS);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
