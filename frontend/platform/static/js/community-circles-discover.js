/* global window, document, fetch */

/**
 * MyCircle tab — 2026-05-18 v3 redesign JS.
 *
 * Single-page vision: spotlight hero + horizontal My Circles strip +
 * one unified Discover grid (Featured/Trending/New merged with tag chips).
 *
 * IDs read by this script:
 *   • cc-spotlight, cc-spotlight-section  (hero tile)
 *   • cc-my-circles-list, cc-my-circles-count  (horizontal strip)
 *   • cc-discover-list, cc-discover-filters    (unified grid + filter chips)
 *   • cc-search-input, cc-search-results-section, cc-search-results,
 *     cc-search-pagination
 */
(function () {
  'use strict';

  // Circle IDs the current user is already a member/owner/admin of.
  var _joinedIds = new Set();
  // Merged discover dataset (with each item tagged by source).
  var _discoverItems = [];
  // Active discover filter chip.
  var _activeFilter = 'all';

  function escHtml(s) {
    if (typeof s !== 'string') return String(s == null ? '' : s);
    var d = document.createElement('div');
    d.appendChild(document.createTextNode(s));
    return d.innerHTML;
  }

  function $(sel) { return document.querySelector(sel); }

  function initialFor(name) {
    return (name || 'C').trim().charAt(0).toUpperCase();
  }

  // Activity copy from `recent_post_count` (already populated by the backend
  // trending refresh job; safe to treat as 0 when missing).
  function activityLabel(circle) {
    var n = Number(circle && circle.recent_post_count) || 0;
    if (n === 0) return { text: 'Quiet this week', state: 'quiet' };
    if (n === 1) return { text: '1 post this week', state: 'low' };
    if (n < 5)   return { text: n + ' posts this week', state: 'low' };
    return { text: n + ' posts this week', state: 'active' };
  }

  function activityPill(circle) {
    var a = activityLabel(circle);
    return '<span class="cc-activity cc-activity--' + a.state + '">' +
      '<span class="cc-activity__dot" aria-hidden="true"></span>' +
      a.text + '</span>';
  }

  // Member face-avatar stack — uses the `member_preview` slice returned
  // by /api/community/circles/discover (up to 5 most-recent members
  // hydrated with display_name + avatar_url). Falls back to the simple
  // fill-ratio dots when the API has no preview (legacy responses or
  // empty circles).
  function memberStack(circle) {
    var preview = Array.isArray(circle.member_preview) ? circle.member_preview : [];
    if (preview.length > 0) {
      var faces = preview.slice(0, 4);
      var extra = Math.max(0, (circle.member_count || 0) - faces.length);
      var html = '<span class="cc-faces" aria-hidden="true">';
      for (var i = 0; i < faces.length; i++) {
        var f = faces[i];
        if (f.avatar_url) {
          html += '<span class="cc-faces__face">' +
            '<img src="' + escHtml(f.avatar_url) + '" alt="" loading="lazy" />' +
            '</span>';
        } else {
          html += '<span class="cc-faces__face cc-faces__face--initial">' +
            escHtml(initialFor(f.display_name)) +
            '</span>';
        }
      }
      if (extra > 0) {
        html += '<span class="cc-faces__more">+' + extra + '</span>';
      }
      html += '</span>';
      return html;
    }
    // Fallback: 3-dot capacity hint
    var filled = Math.max(1, Math.min(3, Math.ceil((circle.member_count || 0) / Math.max(1, circle.max_members || 50) * 3)));
    var out = '<span class="cc-stack" aria-hidden="true">';
    for (var j = 0; j < 3; j++) {
      var on = j < filled;
      out += '<span class="cc-stack__dot' + (on ? ' cc-stack__dot--on' : '') + '"></span>';
    }
    out += '</span>';
    return out;
  }

  // ─── Source-tag chip (Featured / Trending / New) ─────────────────────
  function sourceChip(source) {
    if (source === 'featured') {
      return '<span class="cc-tag cc-tag--featured" title="Featured by POOOL">' +
        '<svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>' +
        'Featured</span>';
    }
    if (source === 'trending') {
      return '<span class="cc-tag cc-tag--trending" title="Active in the last 7 days">' +
        '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 6 13.5 15.5 8.5 10.5 1 18"/><polyline points="17 6 23 6 23 12"/></svg>' +
        'Trending</span>';
    }
    if (source === 'new') {
      return '<span class="cc-tag cc-tag--new" title="Created in the last 30 days">' +
        '<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="16"/><line x1="8" y1="12" x2="16" y2="12"/></svg>' +
        'New</span>';
    }
    return '';
  }

  // ─── Card renderers ─────────────────────────────────────────────────

  // Spotlight hero — bold cobalt card for ONE recommendation
  function renderSpotlight(circle) {
    var c = circle;
    var slug = encodeURIComponent(c.slug || '');
    var name = escHtml(c.name || 'Untitled');
    var desc = escHtml((c.description || 'Join the conversation with active investors.').slice(0, 140));
    var memberStr = (c.member_count || 0) + ' member' + ((c.member_count === 1) ? '' : 's');
    var privacy = c.is_public ? 'Public' : 'Private';
    var alreadyIn = _joinedIds.has(c.id);
    var ctaHtml = alreadyIn
      ? '<a class="ds-btn cc-spotlight__cta" href="/community/circle/' + slug + '">Open Circle</a>'
      : '<button class="ds-btn cc-spotlight__cta" type="button" data-cc-join="' + escHtml(c.id) + '">Join Circle</button>';
    return (
      '<article class="cc-spotlight__card" data-circle-id="' + escHtml(c.id) + '" data-circle-slug="' + slug + '">' +
        '<span class="cc-spotlight__eyebrow">' +
          (c.is_featured ? '★ Featured by POOOL' : 'Recommended for you') +
        '</span>' +
        '<h2 class="cc-spotlight__name">' + name + '</h2>' +
        '<p class="cc-spotlight__desc">' + desc + '</p>' +
        '<div class="cc-spotlight__meta">' +
          '<span>' + memberStr + '</span>' +
          '<span class="cc-spotlight__dot">·</span>' +
          '<span>' + privacy + '</span>' +
          '<span class="cc-spotlight__dot">·</span>' +
          '<span>' + activityLabel(c).text + '</span>' +
        '</div>' +
        '<div class="cc-spotlight__actions">' + ctaHtml + '</div>' +
      '</article>'
    );
  }

  // Discover grid card — uniform white tile with brand-gradient top bar
  function renderCard(item) {
    var c = item.circle || item;
    var source = item._source || null;
    var role = _joinedIds.has(c.id) ? 'member' : null;
    var name = escHtml(c.name || 'Untitled');
    var desc = escHtml((c.description || '').slice(0, 110));
    var memberStr = (c.member_count || 0) + ' / ' + (c.max_members || 0) + ' members';
    var privacy = c.is_public ? 'Public' : 'Private';
    var slug = encodeURIComponent(c.slug || '');
    var initial = escHtml(initialFor(c.name));

    var actionBtn = role
      ? '<a class="ds-btn ds-btn--secondary ds-btn--sm cc-card__cta" href="/community/circle/' + slug + '">Open</a>'
      : '<button class="ds-btn ds-btn--primary ds-btn--sm cc-card__cta" type="button" data-cc-join="' + escHtml(c.id) + '">Join</button>';

    return (
      '<article class="cc-card" data-circle-id="' + escHtml(c.id) + '" data-circle-slug="' + slug + '" data-cc-source="' + escHtml(source || '') + '">' +
        '<div class="cc-card__top">' +
          '<div class="cc-card__avatar" aria-hidden="true">' + initial + '</div>' +
          '<div class="cc-card__title-block">' +
            '<div class="cc-card__name-row">' +
              '<span class="cc-card__name">' + name + '</span>' +
              sourceChip(source) +
            '</div>' +
            '<div class="cc-card__meta">' +
              memberStack(c) +
              '<span>' + memberStr + '</span>' +
              '<span class="cc-card__sep">·</span>' +
              '<span>' + privacy + '</span>' +
            '</div>' +
          '</div>' +
        '</div>' +
        (desc ? '<p class="cc-card__desc">' + desc + '</p>' : '<div class="cc-card__desc cc-card__desc--placeholder"></div>') +
        '<div class="cc-card__footer">' +
          activityPill(c) +
          actionBtn +
        '</div>' +
      '</article>'
    );
  }

  // My Circles pill — compact horizontal entry
  function renderPill(item) {
    var c = item.circle || item;
    var role = item.role || 'member';
    var slug = encodeURIComponent(c.slug || '');
    var name = escHtml(c.name || 'Untitled');
    var initial = escHtml(initialFor(c.name));
    var memberStr = (c.member_count || 0) + ' members';
    return (
      '<a class="cc-pill" href="/community/circle/' + slug + '" data-circle-id="' + escHtml(c.id) + '">' +
        '<span class="cc-pill__avatar" aria-hidden="true">' + initial + '</span>' +
        '<span class="cc-pill__body">' +
          '<span class="cc-pill__name">' + name + '</span>' +
          '<span class="cc-pill__meta">' + escHtml(role) + ' · ' + memberStr + '</span>' +
        '</span>' +
      '</a>'
    );
  }

  // ─── Skeleton loaders ───────────────────────────────────────────────
  function skeletonSpotlight() {
    return (
      '<div class="cc-spotlight__card cc-skel" aria-hidden="true">' +
        '<span class="cc-skel__line cc-skel__line--xs"></span>' +
        '<span class="cc-skel__line cc-skel__line--xl"></span>' +
        '<span class="cc-skel__line cc-skel__line--lg"></span>' +
        '<span class="cc-skel__line cc-skel__line--md"></span>' +
        '<span class="cc-skel__btn"></span>' +
      '</div>'
    );
  }
  function skeletonPill() {
    return (
      '<div class="cc-pill cc-skel" aria-hidden="true">' +
        '<span class="cc-skel__avatar"></span>' +
        '<span class="cc-skel__col">' +
          '<span class="cc-skel__line cc-skel__line--sm"></span>' +
          '<span class="cc-skel__line cc-skel__line--xs"></span>' +
        '</span>' +
      '</div>'
    );
  }
  function skeletonCard() {
    return (
      '<div class="cc-card cc-skel" aria-hidden="true">' +
        '<div class="cc-skel__row">' +
          '<span class="cc-skel__avatar"></span>' +
          '<span class="cc-skel__col">' +
            '<span class="cc-skel__line cc-skel__line--md"></span>' +
            '<span class="cc-skel__line cc-skel__line--xs"></span>' +
          '</span>' +
        '</div>' +
        '<span class="cc-skel__line cc-skel__line--lg"></span>' +
        '<span class="cc-skel__line cc-skel__line--md"></span>' +
        '<div class="cc-skel__footer">' +
          '<span class="cc-skel__line cc-skel__line--xs"></span>' +
          '<span class="cc-skel__btn"></span>' +
        '</div>' +
      '</div>'
    );
  }
  function repeat(html, n) { var out = ''; for (var i = 0; i < n; i++) out += html; return out; }

  function paintSkeletons() {
    var spot = $('#cc-spotlight');
    if (spot && !spot.dataset.loaded) spot.innerHTML = skeletonSpotlight();
    var pills = $('#cc-my-circles-list');
    if (pills && !pills.dataset.loaded) pills.innerHTML = repeat(skeletonPill(), 3);
    var grid = $('#cc-discover-list');
    if (grid && !grid.dataset.loaded) grid.innerHTML = repeat(skeletonCard(), 6);
  }

  // ─── Render orchestration ────────────────────────────────────────────

  function renderDiscoverGrid() {
    var el = $('#cc-discover-list');
    if (!el) return;
    el.dataset.loaded = '1';
    var items = _discoverItems;
    if (_activeFilter !== 'all') {
      items = items.filter(function (it) { return it._source === _activeFilter; });
    }
    if (items.length === 0) {
      el.innerHTML = '<div class="cc-empty">' +
        (_activeFilter === 'all'
          ? 'No circles yet — create the first one!'
          : 'Nothing in this category right now.') +
        '</div>';
      return;
    }
    el.innerHTML = items.map(renderCard).join('');
  }

  function pickSpotlight() {
    // Priority: Featured (not already joined) > Trending (not joined) > New (not joined)
    // Falls back to the same priority but allowing joined circles when nothing else fits.
    var order = ['featured', 'trending', 'new'];
    for (var i = 0; i < order.length; i++) {
      var src = order[i];
      var pick = _discoverItems.find(function (it) {
        return it._source === src && !_joinedIds.has((it.circle || it).id);
      });
      if (pick) return pick.circle || pick;
    }
    for (var j = 0; j < order.length; j++) {
      var src2 = order[j];
      var any = _discoverItems.find(function (it) { return it._source === src2; });
      if (any) return any.circle || any;
    }
    return null;
  }

  function renderSpotlightFromDiscover() {
    var section = $('#cc-spotlight-section');
    var holder = $('#cc-spotlight');
    if (!holder || !section) return;
    holder.dataset.loaded = '1';
    var circle = pickSpotlight();
    if (!circle) {
      section.hidden = true;
      holder.innerHTML = '';
      return;
    }
    section.hidden = false;
    holder.innerHTML = renderSpotlight(circle);
  }

  // ─── Data loaders ────────────────────────────────────────────────────

  function tagItems(arr, source) {
    if (!Array.isArray(arr)) return [];
    return arr.map(function (raw) {
      var item = raw && raw.circle ? raw : { circle: raw };
      item._source = source;
      return item;
    });
  }

  // Dedupe by circle.id, preferring first-seen (featured before trending before new).
  function mergeDiscover(featured, trending, fresh) {
    var seen = new Set();
    var out = [];
    [featured, trending, fresh].forEach(function (group) {
      group.forEach(function (it) {
        var id = (it.circle || it).id;
        if (id && !seen.has(id)) {
          seen.add(id);
          out.push(it);
        }
      });
    });
    return out;
  }

  async function loadDiscover() {
    try {
      var res = await fetch('/api/community/circles/discover', { credentials: 'same-origin' });
      if (!res.ok) throw new Error('discover ' + res.status);
      var data = await res.json();
      _discoverItems = mergeDiscover(
        tagItems(data.featured, 'featured'),
        tagItems(data.trending, 'trending'),
        tagItems(data.new, 'new')
      );
      renderSpotlightFromDiscover();
      renderDiscoverGrid();
    } catch (e) {
      console.error('[circles] discover failed', e);
      var el = $('#cc-discover-list');
      if (el) el.innerHTML = '<div class="cc-empty">Could not load circles. Try again later.</div>';
    }
  }

  async function loadMyCircles() {
    try {
      var res = await fetch('/api/community/me/circles', { credentials: 'same-origin' });
      if (!res.ok) throw new Error('my-circles ' + res.status);
      var data = await res.json();
      var list = (data && data.circles) || [];
      _joinedIds = new Set(list.map(function (it) { return it.circle && it.circle.id; }).filter(Boolean));
      var countEl = $('#cc-my-circles-count');
      if (countEl) countEl.textContent = list.length === 0 ? '' : '(' + list.length + ')';
      var el = $('#cc-my-circles-list');
      if (!el) return;
      el.dataset.loaded = '1';
      if (list.length === 0) {
        el.innerHTML = '<div class="cc-myrow__empty">You haven\'t joined any circles yet. Browse Discover below or create one.</div>';
        return;
      }
      el.innerHTML = list.map(renderPill).join('');
    } catch (e) {
      console.error('[circles] my circles failed', e);
    }
  }

  // ─── Search (debounced) ──────────────────────────────────────────────
  var _searchTimer = null;
  var _searchPage = 1;
  var _searchQuery = '';

  function ccSearchDebounced(q) {
    _searchQuery = (q || '').trim();
    if (_searchTimer) clearTimeout(_searchTimer);
    _searchTimer = setTimeout(function () { runSearch(1); }, 250);
  }

  function setDiscoverVisibility(showDiscover) {
    var disc = $('#cc-discover-section');
    var spot = $('#cc-spotlight-section');
    if (disc) disc.hidden = !showDiscover;
    if (spot && !showDiscover) spot.hidden = true;
    else if (spot && showDiscover && _discoverItems.length) spot.hidden = false;
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
      setDiscoverVisibility(true);
      return;
    }
    if (section) section.hidden = false;
    setDiscoverVisibility(false);
    resultsEl.innerHTML = '<div class="cc-empty">Searching…</div>';
    try {
      var url = '/api/community/circles/search?q=' + encodeURIComponent(_searchQuery) +
        '&page=' + _searchPage + '&per_page=12';
      var res = await fetch(url, { credentials: 'same-origin' });
      if (!res.ok) throw new Error('search ' + res.status);
      var data = await res.json();
      var list = data.results || [];
      if (list.length === 0) {
        resultsEl.innerHTML = '<div class="cc-empty">No circles match "' + escHtml(_searchQuery) + '".</div>';
        if (pagEl) pagEl.innerHTML = '';
        return;
      }
      resultsEl.innerHTML = list.map(function (c) { return renderCard({ circle: c }); }).join('');
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

  // ─── Filter chip handler ─────────────────────────────────────────────
  function bindFilterChips() {
    var chipBar = $('#cc-discover-filters');
    if (!chipBar) return;
    chipBar.addEventListener('click', function (e) {
      var btn = e.target.closest('[data-cc-filter]');
      if (!btn) return;
      var filter = btn.getAttribute('data-cc-filter');
      _activeFilter = filter;
      chipBar.querySelectorAll('.cc-chip').forEach(function (b) {
        b.classList.toggle('cc-chip--active', b === btn);
      });
      renderDiscoverGrid();
    });
  }

  // ─── Optimistic Join handler ────────────────────────────────────────
  // Flow: flip the button + _joinedIds immediately so the user sees an
  // instant state change; reconcile in the background via the real API.
  // On failure: roll back the local state and surface a toast.
  document.addEventListener('click', async function (e) {
    var joinBtn = e.target.closest('[data-cc-join]');
    if (joinBtn) {
      var id = joinBtn.getAttribute('data-cc-join');
      if (!id || joinBtn.disabled) return;

      // ── 1. Optimistic UI flip ────────────────────────────────────────
      var card = joinBtn.closest('[data-circle-id]');
      var wasJoined = _joinedIds.has(id);
      _joinedIds.add(id);
      joinBtn.disabled = true;
      joinBtn.textContent = 'Joined ✓';
      joinBtn.classList.add('cc-card__cta--just-joined');
      if (card) card.classList.add('cc-card--joined');

      try {
        var res = await fetch('/api/community/circles/' + encodeURIComponent(id) + '/join', {
          method: 'POST', credentials: 'same-origin',
        });
        if (!res.ok) {
          var msg = 'Join failed';
          try { var j = await res.json(); msg = j.error || j.message || msg; } catch (_) {}
          throw new Error(msg);
        }

        // ── 2. Background reconcile (no UI flicker) ────────────────────
        if (window.showToast) window.showToast('Joined! See the latest posts.', 'success');
        // Quietly refresh both rails so member counts + My Circles strip
        // pick up the new membership.
        loadMyCircles();
        loadDiscover();
        if (_searchQuery) runSearch(_searchPage);
        if (window.communitySync) {
          window.communitySync.emit('circle.joined', { circle_id: id });
        }
      } catch (err) {
        // ── 3. Roll back optimistic flip ───────────────────────────────
        if (!wasJoined) _joinedIds.delete(id);
        joinBtn.disabled = false;
        joinBtn.textContent = 'Join';
        joinBtn.classList.remove('cc-card__cta--just-joined');
        if (card) card.classList.remove('cc-card--joined');
        if (window.showToast) window.showToast(err.message || 'Could not join circle', 'error');
      }
      return;
    }
    var pageBtn = e.target.closest('[data-cc-page]');
    if (pageBtn) {
      runSearch(parseInt(pageBtn.getAttribute('data-cc-page'), 10));
      return;
    }
  });

  // ─── Boot ────────────────────────────────────────────────────────────
  function init() {
    if (!document.getElementById('cc-my-circles-list')) return;
    bindFilterChips();
    paintSkeletons();
    // My Circles first so _joinedIds is hydrated before Discover renders.
    loadMyCircles().finally(loadDiscover);
  }

  function wireBodyListener() {
    if (!document.body) return;
    document.body.addEventListener('htmx:afterSwap', function (evt) {
      if (evt.detail && evt.detail.target && evt.detail.target.id === 'community-content-area') {
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

  // Cross-tab refresh
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
