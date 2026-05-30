/* global window, document, fetch */

/**
 * My Circles tab — 2026-05-18 v3 redesign JS.
 *
 * Single-page vision: spotlight hero + horizontal My Circles strip +
 * one unified Discover grid with Phase 2 category filters.
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

  function escAttr(s) {
    return escHtml(s).replace(/"/g, '&quot;').replace(/'/g, '&#39;');
  }

  function $(sel) { return document.querySelector(sel); }

  function initialFor(name) {
    return (name || 'C').trim().charAt(0).toUpperCase();
  }

  function isHolderOnly(circle) {
    return Boolean(circle && (
      circle.token_gate_asset_id ||
      circle.is_holder_only ||
      circle.join_policy === 'holder_only'
    ));
  }

  function isKycGated(circle) {
    return Boolean(circle && (
      circle.is_kyc_gated ||
      circle.kyc_required ||
      circle.join_policy === 'kyc_required'
    ));
  }

  function isOfficial(circle) {
    return Boolean(circle && (circle.is_official || circle.circle_type === 'official'));
  }

  function isAssetCircle(circle) {
    return Boolean(circle && (
      circle.token_gate_asset_id ||
      circle.circle_type === 'asset' ||
      circle.kind === 'asset' ||
      circle.category === 'asset'
    ));
  }

  function visibilityOf(circle) {
    if (!circle) return 'public';
    if (circle.visibility) return circle.visibility;
    return circle.is_public === false ? 'private' : 'public';
  }

  function memberLabel(circle) {
    var count = Number(circle && circle.member_count) || 0;
    return count + ' member' + (count === 1 ? '' : 's');
  }

  function metaParts(circle) {
    var parts = [
      memberLabel(circle),
      visibilityOf(circle) === 'public' ? 'Public' : 'Private'
    ];
    if (isHolderOnly(circle)) parts.push('Holder-only');
    if (isKycGated(circle)) parts.push('KYC required');
    if (isOfficial(circle)) parts.push('Official');
    return parts;
  }

  function metaText(circle) {
    return metaParts(circle).join(' · ');
  }

  function isPrivateCircle(circle) {
    return visibilityOf(circle) === 'private' || circle.is_public === false;
  }

  function getFilterTags(item) {
    var c = item && (item.circle || item);
    var tags = new Set();
    var sources = (item && item._sources) || (item && item._source ? [item._source] : []);
    sources.forEach(function (source) {
      if (source) tags.add(source);
    });
    if (c && c.is_featured) tags.add('featured');
    if (c && isPrivateCircle(c)) tags.add('private');
    if (c && visibilityOf(c) === 'public') tags.add('public');
    if (isAssetCircle(c)) tags.add('asset');
    if (isHolderOnly(c)) tags.add('holder-only');
    if (isKycGated(c)) tags.add('kyc-gated');
    if (isOfficial(c)) tags.add('official');
    return Array.from(tags);
  }

  function hasFilterTag(item, tag) {
    if (tag === 'all') return true;
    var tags = item._filterTags || getFilterTags(item);
    return tags.indexOf(tag) !== -1;
  }

  function circleUrl(circle) {
    return '/community/circle/' + encodeURIComponent(circle.slug || '');
  }

  function canManageRole(role) {
    return ['owner', 'admin', 'moderator', 'platform_admin'].indexOf(String(role || '').toLowerCase()) !== -1;
  }

  function accessChips(circle) {
    var chips = '';
    if (circle && isPrivateCircle(circle)) {
      chips += '<span class="cc-tag cc-tag--private">Private</span>';
    }
    if (isHolderOnly(circle)) {
      chips += '<span class="cc-tag cc-tag--holder">Holder-only</span>';
    }
    if (isKycGated(circle)) {
      chips += '<span class="cc-tag cc-tag--kyc">KYC-gated</span>';
    }
    if (isOfficial(circle)) {
      chips += '<span class="cc-tag cc-tag--official">Official</span>';
    }
    if (circle && circle.private_investor_club) {
      chips += '<span class="cc-tag cc-tag--private">Investor Club</span>';
    }
    return chips;
  }

  function primaryAction(circle, role, extraClass) {
    var url = circleUrl(circle);
    var id = escHtml(circle.id);
    var cls = extraClass || 'cc-card__cta';
    if (_joinedIds.has(circle.id)) {
      return '';
    }
    if (isHolderOnly(circle) || isKycGated(circle)) {
      return '<button class="ds-btn ds-btn--secondary ds-btn--sm ' + cls + ' cc-card__cta--locked" type="button" disabled aria-disabled="true" title="Eligibility required">Locked</button>';
    }
    if (isPrivateCircle(circle)) {
      return '<button class="ds-btn ds-btn--secondary ds-btn--sm ' + cls + '" type="button" data-cc-request="' + id + '">Request Access</button>';
    }
    return '<button class="ds-btn ds-btn--primary ds-btn--sm ' + cls + '" type="button" data-cc-join="' + id + '">Join</button>';
  }

  function actionMenu(circle, role, variant) {
    var url = circleUrl(circle);
    var id = escHtml(circle.id);
    var compact = variant === 'pill';
    var manage = canManageRole(role)
      ? '<a class="cc-card-menu__item" href="' + url + '/settings">Manage</a>'
      : '';
    var leave = _joinedIds.has(circle.id) && String(role || '').toLowerCase() !== 'owner'
      ? '<button type="button" class="cc-card-menu__item" data-cc-leave="' + id + '">Leave</button>'
      : '';
    return (
      '<div class="' + (compact ? 'cc-pill-actions' : 'cc-card-actions') + '">' +
        '<button type="button" class="cc-kebab" aria-label="Circle actions" aria-expanded="false" data-cc-menu-toggle>' +
          '<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="5" cy="12" r="2"/><circle cx="12" cy="12" r="2"/><circle cx="19" cy="12" r="2"/></svg>' +
        '</button>' +
        '<div class="cc-card-menu" role="menu" hidden>' +
          manage +
          '<button type="button" class="cc-card-menu__item" data-cc-copy="' + url + '">Copy Link</button>' +
          leave +
        '</div>' +
      '</div>'
    );
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
    // Fallback: compact member-presence hint when sample faces are unavailable.
    var filled = Math.max(1, Math.min(3, circle.member_count || 1));
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
    var meta = escHtml(metaText(c));
    var ctaHtml = primaryAction(c, null, 'cc-spotlight__cta');
    return (
      '<article class="cc-spotlight__card" data-circle-id="' + escHtml(c.id) + '" data-circle-slug="' + slug + '">' +
        '<span class="cc-spotlight__eyebrow">' +
          (c.is_featured ? '★ Featured by POOOL' : 'Recommended for you') +
        '</span>' +
        '<h2 class="cc-spotlight__name">' + name + '</h2>' +
        '<p class="cc-spotlight__desc">' + desc + '</p>' +
        '<div class="cc-spotlight__meta">' +
          '<span>' + meta + '</span>' +
        '</div>' +
        (ctaHtml ? '<div class="cc-spotlight__actions">' + ctaHtml + '</div>' : '') +
      '</article>'
    );
  }

  // Activity label based on recent_post_count
  function activityLabel(circle) {
    var n = Number(circle && circle.recent_post_count) || 0;
    if (n === 0) return '';
    if (n === 1) return '1 post this week';
    return n + ' posts this week';
  }

  // Minimal source badge — text only, no icon
  function sourceBadge(source) {
    if (source === 'featured') return '<span class="cc-badge cc-badge--featured">Featured</span>';
    if (source === 'trending') return '<span class="cc-badge cc-badge--trending">Trending</span>';
    if (source === 'new')      return '<span class="cc-badge cc-badge--new">New</span>';
    return '';
  }

  // Discover grid card
  function renderCard(item) {
    var c = item.circle || item;
    var source = item._source || null;
    var role = item.role || (_joinedIds.has(c.id) ? 'member' : null);
    var name = escHtml(c.name || 'Untitled');
    var desc = escHtml((c.description || '').slice(0, 120));
    var slug = encodeURIComponent(c.slug || '');
    var initial = escHtml(initialFor(c.name));
    var url = circleUrl(c);

    // Banner strip
    var bannerHtml = '';
    if (c.banner_url) {
      bannerHtml = '<div class="cc-card__banner" style="background-image:url(' + escHtml(c.banner_url) + ')" aria-hidden="true"></div>';
    }

    // Member count + activity
    var memberCount = Number(c.member_count) || 0;
    var memberTxt = memberCount + ' member' + (memberCount === 1 ? '' : 's');
    var activity = activityLabel(c);

    // Member face-stack
    var stack = memberStack(c);

    // Source badge
    var badge = sourceBadge(source);

    var actionBtn = primaryAction(c, role, 'cc-card__cta');
    var footer = (stack || actionBtn) ?
      '<div class="cc-card__footer">' +
        (stack ? '<div class="cc-card__stack">' + stack + '</div>' : '') +
        (actionBtn ? actionBtn : '') +
      '</div>' : '';

    return (
      '<article class="cc-card cc-card--clickable" tabindex="0" role="link" aria-label="Open circle ' + escAttr(c.name || 'Untitled') + '" data-cc-open="' + escAttr(url) + '" data-circle-id="' + escHtml(c.id) + '" data-circle-slug="' + slug + '" data-cc-source="' + escHtml(source || '') + '" data-cc-filter-tags="' + escHtml(getFilterTags(item).join(' ')) + '">' +
        bannerHtml +
        '<div class="cc-card__body">' +
          '<div class="cc-card__top">' +
            '<div class="cc-card__avatar" aria-hidden="true">' + initial + '</div>' +
            '<div class="cc-card__title-block">' +
              '<div class="cc-card__name-row">' +
                '<span class="cc-card__name">' + name + '</span>' +
                (badge ? badge : '') +
              '</div>' +
              '<div class="cc-card__meta">' + escHtml(memberTxt) + '</div>' +
            '</div>' +
          '</div>' +
          (desc ? '<p class="cc-card__desc">' + desc + '</p>' : '') +
          (activity ? '<p class="cc-card__activity">' + activity + '</p>' : '') +
        '</div>' +
        footer +
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
      '<div class="cc-pill-wrap" data-circle-id="' + escHtml(c.id) + '" data-circle-slug="' + slug + '">' +
        '<a class="cc-pill" href="/community/circle/' + slug + '">' +
          '<span class="cc-pill__avatar" aria-hidden="true">' + initial + '</span>' +
          '<span class="cc-pill__body">' +
            '<span class="cc-pill__name">' + name + '</span>' +
            '<span class="cc-pill__meta">' + escHtml(role) + ' · ' + memberStr + '</span>' +
          '</span>' +
        '</a>' +
        actionMenu(c, role, 'pill') +
      '</div>'
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
      items = items.filter(function (it) { return hasFilterTag(it, _activeFilter); });
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
    // Priority: Featured (not already joined) > Trending (not joined) > Public > New.
    // Falls back to the same priority but allowing joined circles when nothing else fits.
    var order = ['featured', 'trending', 'public', 'new', 'private'];
    for (var i = 0; i < order.length; i++) {
      var src = order[i];
      var pick = _discoverItems.find(function (it) {
        var c = it.circle || it;
        return hasFilterTag(it, src) && !_joinedIds.has(c.id) && !isHolderOnly(c) && !isKycGated(c);
      });
      if (pick) return pick.circle || pick;
    }
    for (var j = 0; j < order.length; j++) {
      var src2 = order[j];
      var any = _discoverItems.find(function (it) { return hasFilterTag(it, src2); });
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
      item._sources = [source];
      item._filterTags = getFilterTags(item);
      return item;
    });
  }

  // Dedupe by circle.id while merging all category tags. First source remains
  // the visual badge priority; later sources feed the filter behavior.
  function mergeDiscoverGroups(groups) {
    var seen = new Set();
    var out = [];
    groups.forEach(function (group) {
      group.forEach(function (it) {
        var id = (it.circle || it).id;
        if (!id) return;
        if (!seen.has(id)) {
          seen.add(id);
          out.push(it);
        } else {
          var existing = out.find(function (candidate) {
            return (candidate.circle || candidate).id === id;
          });
          if (existing) {
            var sources = existing._sources || (existing._source ? [existing._source] : []);
            (it._sources || [it._source]).forEach(function (source) {
              if (source && sources.indexOf(source) === -1) sources.push(source);
            });
            existing._sources = sources;
          }
        }
      });
    });
    out.forEach(function (it) { it._filterTags = getFilterTags(it); });
    return out;
  }

  async function loadDiscover() {
    try {
      var res = await fetch('/api/community/circles/discover', { credentials: 'same-origin' });
      if (!res.ok) throw new Error('discover ' + res.status);
      var data = await res.json();
      _discoverItems = mergeDiscoverGroups([
        tagItems(data.featured, 'featured'),
        tagItems(data.trending, 'trending'),
        tagItems(data.new, 'new'),
        tagItems(data.public, 'public'),
        tagItems(data.private, 'private'),
        tagItems(data.asset, 'asset'),
        tagItems(data.holder_only, 'holder-only'),
        tagItems(data.official, 'official'),
        tagItems(data.kyc_gated, 'kyc-gated')
      ]);
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
        var active = b === btn;
        b.classList.toggle('cc-chip--active', active);
        b.setAttribute('aria-pressed', active ? 'true' : 'false');
      });
      renderDiscoverGrid();
    });
  }

  function closeActionMenus(except) {
    document.querySelectorAll('.cc-card-menu').forEach(function (menu) {
      if (menu === except) return;
      menu.hidden = true;
      var toggle = menu.parentElement && menu.parentElement.querySelector('[data-cc-menu-toggle]');
      if (toggle) toggle.setAttribute('aria-expanded', 'false');
    });
  }

  function copyText(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      return navigator.clipboard.writeText(text);
    }
    var ta = document.createElement('textarea');
    ta.value = text;
    ta.setAttribute('readonly', 'readonly');
    ta.style.position = 'fixed';
    ta.style.left = '-9999px';
    document.body.appendChild(ta);
    ta.select();
    var ok = document.execCommand('copy');
    document.body.removeChild(ta);
    return ok ? Promise.resolve() : Promise.reject(new Error('Copy failed'));
  }

  // ─── Optimistic Join handler ────────────────────────────────────────
  // Flow: flip the button + _joinedIds immediately so the user sees an
  // instant state change; reconcile in the background via the real API.
  // On failure: roll back the local state and surface a toast.
  document.addEventListener('click', async function (e) {
    var menuToggle = e.target.closest('[data-cc-menu-toggle]');
    if (menuToggle) {
      var wrap = menuToggle.closest('.cc-card-actions, .cc-pill-actions');
      var menu = wrap && wrap.querySelector('.cc-card-menu');
      if (!menu) return;
      var willOpen = menu.hidden;
      closeActionMenus(menu);
      menu.hidden = !willOpen;
      menuToggle.setAttribute('aria-expanded', willOpen ? 'true' : 'false');
      return;
    }

    var copyBtn = e.target.closest('[data-cc-copy]');
    if (copyBtn) {
      var href = copyBtn.getAttribute('data-cc-copy') || '';
      var absolute = href.indexOf('http') === 0 ? href : window.location.origin + href;
      try {
        await copyText(absolute);
        if (window.showToast) window.showToast('Circle link copied.', 'success');
      } catch (err) {
        if (window.showToast) window.showToast('Could not copy link.', 'error');
      }
      closeActionMenus();
      return;
    }

    var leaveBtn = e.target.closest('[data-cc-leave]');
    if (leaveBtn) {
      var leaveId = leaveBtn.getAttribute('data-cc-leave');
      if (!leaveId || leaveBtn.disabled) return;
      if (!window.confirm('Leave this Circle?')) {
        closeActionMenus();
        return;
      }
      leaveBtn.disabled = true;
      try {
        var leaveRes = await fetch('/api/community/circles/leave', {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ circle_id: leaveId })
        });
        if (!leaveRes.ok) {
          var leaveMsg = 'Leave failed';
          try { var leaveJson = await leaveRes.json(); leaveMsg = leaveJson.error || leaveJson.message || leaveMsg; } catch (_) {}
          throw new Error(leaveMsg);
        }
        _joinedIds.delete(leaveId);
        if (window.showToast) window.showToast('Left Circle.', 'success');
        loadMyCircles();
        loadDiscover();
        if (_searchQuery) runSearch(_searchPage);
      } catch (errLeave) {
        leaveBtn.disabled = false;
        if (window.showToast) window.showToast(errLeave.message || 'Could not leave Circle.', 'error');
      }
      closeActionMenus();
      return;
    }

    if (!e.target.closest('.cc-card-actions, .cc-pill-actions')) {
      closeActionMenus();
    }

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
    var requestBtn = e.target.closest('[data-cc-request]');
    if (requestBtn) {
      var requestId = requestBtn.getAttribute('data-cc-request');
      if (!requestId || requestBtn.disabled) return;
      requestBtn.disabled = true;
      var oldText = requestBtn.textContent;
      requestBtn.textContent = 'Requested';
      try {
        var requestRes = await fetch('/api/community/circles/' + encodeURIComponent(requestId) + '/request', {
          method: 'POST',
          credentials: 'same-origin'
        });
        if (!requestRes.ok) {
          var requestMsg = 'Request failed';
          try { var requestJson = await requestRes.json(); requestMsg = requestJson.error || requestJson.message || requestMsg; } catch (_) {}
          throw new Error(requestMsg);
        }
        if (window.showToast) window.showToast('Access requested.', 'success');
      } catch (errRequest) {
        requestBtn.disabled = false;
        requestBtn.textContent = oldText || 'Request Access';
        if (window.showToast) window.showToast(errRequest.message || 'Could not request access.', 'error');
      }
      return;
    }
    var openCard = e.target.closest('[data-cc-open]');
    if (openCard && !e.target.closest('a, button, input, textarea, select, [role="button"], [data-cc-menu-toggle]')) {
      var openUrl = openCard.getAttribute('data-cc-open');
      if (openUrl) window.location.href = openUrl;
      return;
    }
    var pageBtn = e.target.closest('[data-cc-page]');
    if (pageBtn) {
      runSearch(parseInt(pageBtn.getAttribute('data-cc-page'), 10));
      return;
    }
  });

  document.addEventListener('keydown', function (e) {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    var openCard = e.target.closest && e.target.closest('[data-cc-open]');
    if (!openCard || e.target.closest('a, button, input, textarea, select, [role="button"], [data-cc-menu-toggle]')) return;
    var openUrl = openCard.getAttribute('data-cc-open');
    if (!openUrl) return;
    e.preventDefault();
    window.location.href = openUrl;
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
