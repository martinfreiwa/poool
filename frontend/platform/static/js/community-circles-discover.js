/* global window, document, fetch */

/**
 * MyCircle tab — 2026-05-16 rework JS.
 *
 * Loads My Circles + Discover (Featured/Trending/New) + Search.
 * Card renderer + Join button are co-located here so the new partial
 * (`partials/community_circle.html`) has zero dependency on the legacy
 * `community-circles.js` flow (XP summary, stats, recent activity, etc.).
 *
 * The legacy file still loads — its init code checks for the OLD DOM IDs
 * (`#xp-summary-card`, `#circle-content`, `#xp-history-list`, …) which the
 * rewritten partial doesn't have, so it no-ops. Pending invites + join
 * requests sections kept compatible IDs so existing handlers continue to
 * work (#pending-invites-section, #invite-list, etc.).
 */
(function () {
  'use strict';

  function escHtml(s) {
    if (typeof s !== 'string') return String(s == null ? '' : s);
    var d = document.createElement('div');
    d.appendChild(document.createTextNode(s));
    return d.innerHTML;
  }

  function $(sel) { return document.querySelector(sel); }
  function $$(sel) { return Array.from(document.querySelectorAll(sel)); }

  // ─── Card renderer ────────────────────────────────────────────────
  // Compact `.cc-card-compact` chrome — matches the investor-dashboard
  // ds-stat-card density so MyCircle visually aligns with the rest of
  // the platform.
  function renderCard(circle, opts) {
    opts = opts || {};
    var role = opts.role || null;
    var c = circle;
    var name = escHtml(c.name || 'Untitled');
    var emoji = escHtml(c.avatar_emoji || '🟢');
    var desc = escHtml((c.description || '').slice(0, 80));
    var memberStr = (c.member_count || 0) + '/' + (c.max_members || 0) + ' members';
    var privacy = c.is_public ? 'Public' : 'Private';
    var featured = c.is_featured ? '<span class="cc-card-badge cc-card-badge--featured" title="Featured by POOOL">★</span>' : '';
    var roleBadge = role ? '<span class="cc-card-badge cc-card-badge--role">' + escHtml(role) + '</span>' : '';
    var slug = encodeURIComponent(c.slug || '');

    var actionBtn;
    if (role) {
      actionBtn = '<a class="ds-btn ds-btn--secondary ds-btn--sm" href="/community/circle/' + slug + '">Open</a>';
    } else {
      actionBtn = '<button class="ds-btn ds-btn--primary ds-btn--sm" type="button" data-cc-join="' + escHtml(c.id) + '">Join</button>';
    }

    return (
      '<article class="ds-card cc-card-compact" data-circle-id="' + escHtml(c.id) + '" data-circle-slug="' + slug + '">' +
        '<div class="cc-card-compact__head">' +
          '<div class="cc-card-compact__avatar" aria-hidden="true">' + emoji + '</div>' +
          '<div class="cc-card-compact__info">' +
            '<div class="cc-card-compact__name-row">' +
              '<span class="cc-card-compact__name">' + name + '</span>' +
              featured + roleBadge +
            '</div>' +
            '<div class="cc-card-compact__meta">' + memberStr + ' · ' + privacy + '</div>' +
          '</div>' +
        '</div>' +
        (desc ? '<p class="cc-card-compact__desc">' + desc + '</p>' : '') +
        '<div class="cc-card-compact__actions">' + actionBtn + '</div>' +
      '</article>'
    );
  }

  function renderRail(elId, items, opts) {
    var el = $('#' + elId);
    if (!el) return;
    if (!items || items.length === 0) {
      el.innerHTML = '<div class="cc-empty">' + escHtml((opts && opts.emptyText) || 'Nothing here yet.') + '</div>';
      return;
    }
    el.innerHTML = items.map(function (it) {
      return renderCard(it.circle || it, { role: it.role || null });
    }).join('');
  }

  // ─── Data loaders ─────────────────────────────────────────────────
  async function loadDiscover() {
    try {
      var res = await fetch('/api/community/circles/discover', { credentials: 'same-origin' });
      if (!res.ok) throw new Error('discover ' + res.status);
      var data = await res.json();
      // Featured: only show section if non-empty
      var fs = $('#cc-featured-section');
      if (data.featured && data.featured.length > 0) {
        if (fs) fs.hidden = false;
        renderRail('cc-featured-list', data.featured);
      } else if (fs) {
        fs.hidden = true;
      }
      renderRail('cc-trending-list', data.trending, { emptyText: 'No trending circles yet — be the first to post in one.' });
      renderRail('cc-new-list',      data.new,      { emptyText: 'No new circles in the last 30 days.' });
    } catch (e) {
      console.error('[circles] discover failed', e);
    }
  }

  async function loadMyCircles() {
    try {
      var res = await fetch('/api/community/me/circles', { credentials: 'same-origin' });
      if (!res.ok) throw new Error('my-circles ' + res.status);
      var data = await res.json();
      var list = (data && data.circles) || [];
      var countEl = $('#cc-my-circles-count');
      if (countEl) countEl.textContent = list.length === 0 ? '' : '(' + list.length + ')';
      var el = $('#cc-my-circles-list');
      if (!el) return;
      if (list.length === 0) {
        el.innerHTML = '<div class="cc-empty">You haven\'t joined any circles yet. Browse below or create one.</div>';
        return;
      }
      el.innerHTML = list.map(function (it) { return renderCard(it.circle, { role: it.role }); }).join('');
    } catch (e) {
      console.error('[circles] my circles failed', e);
    }
  }

  // ─── Search (debounced) ───────────────────────────────────────────
  var _searchTimer = null;
  var _searchPage = 1;
  var _searchQuery = '';

  function ccSearchDebounced(q) {
    _searchQuery = (q || '').trim();
    if (_searchTimer) clearTimeout(_searchTimer);
    _searchTimer = setTimeout(function () { runSearch(1); }, 250);
  }

  async function runSearch(page) {
    _searchPage = page || 1;
    var section = $('#cc-search-results-section');
    var resultsEl = $('#cc-search-results');
    var pagEl = $('#cc-search-pagination');
    if (!resultsEl) return;
    if (!_searchQuery) {
      if (section) section.hidden = true;
      resultsEl.innerHTML = '';
      if (pagEl) pagEl.innerHTML = '';
      return;
    }
    if (section) section.hidden = false;
    resultsEl.innerHTML = '<div class="cc-empty">Searching…</div>';
    try {
      var url = '/api/community/circles/search?q=' + encodeURIComponent(_searchQuery) +
        '&page=' + _searchPage + '&per_page=10';
      var res = await fetch(url, { credentials: 'same-origin' });
      if (!res.ok) throw new Error('search ' + res.status);
      var data = await res.json();
      var list = data.results || [];
      if (list.length === 0) {
        resultsEl.innerHTML = '<div class="cc-empty">No circles match "' + escHtml(_searchQuery) + '".</div>';
        if (pagEl) pagEl.innerHTML = '';
        return;
      }
      resultsEl.innerHTML = list.map(function (c) { return renderCard(c); }).join('');
      if (pagEl) {
        var totalPages = data.total_pages || 1;
        if (totalPages > 1) {
          var btns = '';
          for (var p = 1; p <= Math.min(totalPages, 10); p++) {
            btns += '<button type="button" class="cc-page-btn' +
              (p === _searchPage ? ' cc-page-btn--active' : '') +
              '" data-cc-page="' + p + '">' + p + '</button>';
          }
          pagEl.innerHTML = btns;
        } else {
          pagEl.innerHTML = '';
        }
      }
    } catch (e) {
      console.error('[circles] search failed', e);
      resultsEl.innerHTML = '<div class="cc-empty">Search failed. Try again.</div>';
    }
  }

  function ccClearSearch() {
    var inp = $('#cc-search-input');
    if (inp) inp.value = '';
    _searchQuery = '';
    runSearch(1);
  }

  // ─── Join button delegation ──────────────────────────────────────
  document.addEventListener('click', async function (e) {
    var joinBtn = e.target.closest('[data-cc-join]');
    if (joinBtn) {
      var id = joinBtn.getAttribute('data-cc-join');
      joinBtn.disabled = true;
      joinBtn.textContent = 'Joining…';
      try {
        var res = await fetch('/api/community/circles/' + encodeURIComponent(id) + '/join', {
          method: 'POST', credentials: 'same-origin',
        });
        if (!res.ok) {
          var msg = 'Join failed';
          try { var j = await res.json(); msg = j.error || j.message || msg; } catch (_) {}
          throw new Error(msg);
        }
        // Reload my-circles + the rail this card was in (cheap full reload).
        await Promise.all([loadMyCircles(), loadDiscover()]);
        if (_searchQuery) await runSearch(_searchPage);
        // Broadcast to other tabs so their My Circles list refreshes too.
        if (window.communitySync) {
          window.communitySync.emit('circle.joined', { circle_id: id });
        }
      } catch (err) {
        if (window.showToast) window.showToast(err.message || 'Could not join circle', 'error');
        joinBtn.disabled = false;
        joinBtn.textContent = 'Join';
      }
      return;
    }
    var pageBtn = e.target.closest('[data-cc-page]');
    if (pageBtn) {
      runSearch(parseInt(pageBtn.getAttribute('data-cc-page'), 10));
      return;
    }
  });

  // ─── Boot on first display of the circle tab ──────────────────────
  // The community page lazy-loads the partial via HTMX; we listen for the
  // swap event and (re-)initialise.
  function init() {
    if (!document.getElementById('cc-my-circles-list')) return; // partial not present
    loadMyCircles();
    loadDiscover();
  }

  // Script may load BEFORE <body> exists (some pages put scripts in <head>).
  // Defer body-level listener wiring until DOM is parsed.
  function wireBodyListener() {
    if (!document.body) return; // defensive — already guarded by readyState below
    document.body.addEventListener('htmx:afterSwap', function (evt) {
      if (evt.detail && evt.detail.target && evt.detail.target.id === 'community-content-area') {
        // Defer to next tick so the swapped DOM is mounted.
        setTimeout(init, 0);
      }
    });
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', function () {
      init();
      wireBodyListener();
    });
  } else {
    init();
    wireBodyListener();
  }

  // Cross-tab refresh: if join/leave/role-change happens in another tab,
  // mirror the state change here. Debounced so a burst of events only
  // triggers one network round-trip.
  if (window.communitySync) {
    var _syncTimer = null;
    var queueRefresh = function () {
      if (_syncTimer) clearTimeout(_syncTimer);
      _syncTimer = setTimeout(function () {
        _syncTimer = null;
        if (document.getElementById('cc-my-circles-list')) {
          loadMyCircles();
          loadDiscover();
        }
      }, 200);
    };
    window.communitySync.on('circle.*', queueRefresh);
  }

  // Public surface
  window.ccSearchDebounced = ccSearchDebounced;
  window.ccClearSearch = ccClearSearch;
  window.ccLoadMyCircles = loadMyCircles;
  window.ccLoadDiscover = loadDiscover;
})();
