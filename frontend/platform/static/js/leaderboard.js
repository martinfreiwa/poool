/**
 * Leaderboard Page — "Neon Ethos" Institutional Redesign
 * Fetches rankings, populates bento top-3 cards + institutional ledger table,
 * handles metric/timeframe switching, search, filters, visibility preferences.
 */

(function () {
  'use strict';

  // ─── XSS-safe HTML escaper ───────────────────────────────────
  function escHtml(str) {
    if (typeof str !== 'string') return String(str);
    var d = document.createElement('div');
    d.appendChild(document.createTextNode(str));
    return d.innerHTML;
  }

  let currentMetric = 'invested';
  let currentTimeframe = 'alltime';
  let currentPage = 1;
  let currentSearch = '';
  let currentTier = '';
  let currentPerPage = 10;
  let isFetching = false;
  let searchTimeout = null;
  let cachedPrefs = null;
  // Tab-switch coalescer — see scheduleTabRefetch() below. Rapid clicks on
  // metric / timeframe tabs would otherwise fire one GET per click and hit
  // the per-user `lb:get` rate-limit bucket. We collapse bursts to a single
  // request after a short quiet window.
  let tabSwitchTimer = null;
  const TAB_DEBOUNCE_MS = 180;

  // ─── Demo data (audit C2) ───────────────────────────────────────
  // The 20-row sample fixture lives in leaderboard-demo.js and is fetched
  // via dynamic import ONLY when `?demo` is present. Production page loads
  // never download it. The first call lazy-loads + memoizes the module.
  var _demoModulePromise = null;
  function loadDemoModule() {
    if (!_demoModulePromise) {
      _demoModulePromise = import('/static/js/leaderboard-demo.js');
    }
    return _demoModulePromise;
  }

  // ─── Formatters ────────────────────────────────────────────────
  function formatMetric(val, type) {
    if (type === 'invested' || type === 'revenue' || type === 'highest_inv') {
      return new Intl.NumberFormat('de-DE', { style: 'currency', currency: 'EUR' }).format(val / 100);
    } else if (type === 'roi') {
      return (val / 100).toFixed(2) + '%';
    } else {
      return val.toLocaleString();
    }
  }

  // Inputs are integer cents. Convert to euros before applying the compact
  // M/K suffix so the threshold for "€1M" is 100_000_000 cents, not 1_000_000.
  function formatCompact(val) {
    if (val >= 100000000) return '€' + (val / 1e8).toFixed(2) + 'M';
    if (val >= 100000)    return '€' + (val / 1e5).toFixed(1) + 'K';
    return '€' + (val / 100).toFixed(0);
  }

  function getMetricName(type) {
    // Labels match the underlying calculation, not marketing copy:
    //   - roi = weighted *target* yield (annual_yield_bps), not realized return
    //   - revenue = sum of referees' active investments (network volume),
    //     not commission earned
    var map = {
      'invested': 'Total Invested',
      'assets': 'Assets Held',
      'roi': 'Avg Target Yield',
      'affiliates': 'Affiliates',
      'revenue': 'Network Volume',
      'highest_inv': 'Largest Single Investment'
    };
    return map[type] || 'Score';
  }

  function getInitials(name) {
    if (!name) return '??';
    var trimmed = name.trim();
    // Pseudonym pattern: "INVESTOR #B45F03" → use first 2 chars of the hex slug
    // so avatars are unique per investor instead of collapsing to "I#".
    var pseudo = trimmed.match(/^[A-Z]+\s*#([A-F0-9]+)/i);
    if (pseudo) return pseudo[1].substring(0, 2).toUpperCase();
    var parts = trimmed.split(/\s+/).filter(function (p) { return p && !p.startsWith('#'); });
    if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase();
    if (parts.length === 1) return parts[0].substring(0, 2).toUpperCase();
    return trimmed.substring(0, 2).toUpperCase();
  }

  // NOTE: the previous "Asset Mix" donut/bar derived percentages from
  //   invested_cents vs referral_network_value_cents and floored each side to 5%.
  // It rendered the same "100 / 5" ratio for every user without referral
  // activity, which is most users — pure visual noise. Real asset-class
  // breakdown requires per-investment asset_type aggregation; until that
  // ships, the bento donut and table mix column are removed entirely
  // rather than displaying fake data.

  // ─── Init ──────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', init);

  async function init() {
    try {
      var results = await Promise.all([
        fetchRankings(currentMetric, currentTimeframe, currentPage, currentSearch, currentTier),
        fetchPreferences(),
      ]);
      var data = results[0];
      var prefs = results[1];

      cachedPrefs = prefs;
      renderLeaderboardData(data);
      applyPrefs(prefs);
    } catch (err) {
      console.error('Leaderboard init failed:', err);
      showLayer('error');
    }

    // Audit task A2 — unhide the admin-only "Refresh now" control if the
    // viewer holds an admin role. Failures are silent (button stays hidden).
    revealAdminControls();
  }

  async function revealAdminControls() {
    try {
      var res = await fetch('/api/me', { credentials: 'same-origin' });
      if (!res.ok) return;
      var me = await res.json();
      var roles = Array.isArray(me && me.roles) ? me.roles : [];
      if (roles.indexOf('admin') !== -1 || roles.indexOf('super_admin') !== -1) {
        var btn = document.getElementById('lb-refresh-btn');
        if (btn) btn.hidden = false;
      }
    } catch (_) {
      // Network or auth error — leave button hidden.
    }
  }

  function renderLeaderboardData(data) {
    var forceDemo = new URLSearchParams(window.location.search).has('demo');

    if (forceDemo) {
      // Audit C2: load the demo fixture lazily so it doesn't ship to
      // every prod page-load. Until the module resolves we render an
      // empty content layer; once loaded we replace with the demo data.
      loadDemoModule().then(function(mod) {
        var demo = mod.getDemoData(data, currentMetric, currentTimeframe);
        var start = (currentPage - 1) * currentPerPage;
        var sliced = demo.rankings.slice(start, start + currentPerPage);

        renderBentoCards(demo.rankings);
        renderMinorCards(demo.rankings);
        renderMyRank(demo.my_rank);
        renderTable(sliced, false);
        renderMeta(demo);
        renderPagination(demo);
        hideInlineStatus();
        showLayer('content');
      }).catch(function(err) {
        console.error('Demo module failed to load; falling back to real data:', err);
        renderRealData(data);
      });
      return;
    }

    renderRealData(data);
  }

  function renderRealData(data) {
    if (!currentSearch && !currentTier && (!data.rankings || data.rankings.length === 0) && data.total_participants === 0) {
      renderMyRank(data.my_rank);
      renderMeta(data);
      hideInlineStatus();
      showLayer('empty');
      return;
    }

    renderBentoCards(data.rankings || []);
    renderMinorCards(data.rankings || []);
    renderMyRank(data.my_rank);
    renderTable(data.rankings || [], false);
    renderMeta(data);
    renderPagination(data);
    hideInlineStatus();
    showLayer('content');
  }

  // ─── API Calls ─────────────────────────────────────────────────
  async function fetchRankings(metric, timeframe, page, search, tier, perPage) {
    page = page || 1;
    timeframe = timeframe || 'alltime';
    perPage = perPage || currentPerPage;
    var url = '/api/leaderboard?metric=' + metric + '&timeframe=' + timeframe + '&page=' + page + '&per_page=' + perPage;
    if (search) url += '&search=' + encodeURIComponent(search);
    if (tier) url += '&tier_id=' + tier;

    var res = await fetch(url, { credentials: 'same-origin' });
    if (!res.ok) throw new Error('Failed to fetch rankings');
    return res.json();
  }

  // ─── Pagination & Controls ─────────────────────────────────────
  window.changePerPage = async function (val) {
    currentPerPage = parseInt(val, 10);
    currentPage = 1;
    await refetchAndRender();
  };

  window.goToPage = async function (page) {
    if (isFetching || page < 1) return;
    currentPage = page;
    await refetchAndRender();
  };

  function renderPagination(data) {
    var container = document.getElementById('lb-pagination-controls');
    if (!container) return;
    container.innerHTML = '';

    if (!data || data.total_participants <= currentPerPage) {
      return;
    }

    var totalPages = Math.ceil(data.total_participants / currentPerPage);
    
    // Previous Button
    var prev = document.createElement('button');
    prev.className = 'lb-pag-btn ds-btn ds-btn--secondary ds-btn--sm';
    prev.type = 'button';
    prev.textContent = '‹';
    prev.setAttribute('aria-label', 'Previous leaderboard page');
    prev.disabled = currentPage === 1;
    prev.onclick = function() { goToPage(currentPage - 1); };
    container.appendChild(prev);

    // Page Numbers (Simplified logic)
    for (var i = 1; i <= totalPages; i++) {
       // Only show near pages
       if (totalPages > 7 && i > 3 && i < totalPages - 2 && Math.abs(i - currentPage) > 1) {
          if (i === 4 || i === totalPages - 3) {
            var dot = document.createElement('span');
            dot.innerText = '...';
            dot.style.padding = '0 8px';
            container.appendChild(dot);
          }
          continue;
       }
       
       var btn = document.createElement('button');
       btn.className = 'lb-pag-btn ds-btn ds-btn--secondary ds-btn--sm' + (currentPage === i ? ' active' : '');
       btn.type = 'button';
       btn.innerText = i;
       btn.setAttribute('aria-label', 'Go to leaderboard page ' + i);
       if (currentPage === i) {
         btn.setAttribute('aria-current', 'page');
       }
       (function(p) {
         btn.onclick = function() { goToPage(p); };
       })(i);
       container.appendChild(btn);
    }

    // Next Button
    var next = document.createElement('button');
    next.className = 'lb-pag-btn ds-btn ds-btn--secondary ds-btn--sm';
    next.type = 'button';
    next.textContent = '›';
    next.setAttribute('aria-label', 'Next leaderboard page');
    next.disabled = currentPage === totalPages;
    next.onclick = function() { goToPage(currentPage + 1); };
    container.appendChild(next);
  }

  async function fetchPreferences() {
    try {
      var res = await fetch('/api/leaderboard/preferences', { credentials: 'same-origin' });
      if (!res.ok) return { visible: false, show_avatar: false };
      return res.json();
    } catch (e) {
      return { visible: false, show_avatar: false };
    }
  }

  async function updatePreferences(prefs) {
    try {
      var res = await fetch('/api/leaderboard/preferences', {
        method: 'PUT',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(prefs),
      });
      if (!res.ok) throw new Error('Failed to update preferences');
      cachedPrefs = await res.json();
      applyPrefs(cachedPrefs);
      setPreferenceStatus('Preference saved', false);
    } catch (err) {
      console.error('Failed to update preferences:', err);
      setPreferenceStatus('Could not save preference. Refresh and try again.', true);
      throw err;
    }
  }

  function setPreferenceStatus(message, isError) {
    var status = document.getElementById('lb-preference-status');
    if (!status) return;
    status.textContent = message;
    status.style.color = isError ? '#B42318' : '';
  }

  function showInlineStatus(message) {
    var status = document.getElementById('lb-inline-status');
    if (!status) return;
    status.textContent = message;
    status.classList.remove('hidden');
  }

  function hideInlineStatus() {
    var status = document.getElementById('lb-inline-status');
    if (!status) return;
    status.textContent = '';
    status.classList.add('hidden');
  }

  function setControlsBusy(isBusy) {
    // Important: we used to `control.disabled = true` here. Disabling the
    // currently-focused button drops focus to <body>, which broke
    // keyboard navigation inside the tablist (ArrowRight worked once,
    // then the next key fired on body and went nowhere). Now we signal
    // busy state via `aria-busy` + CSS opacity only — focus survives.
    var controls = document.querySelectorAll('.lb-topbar-tab[data-metric], .lb-tf-btn[data-timeframe], #lb-per-page-select, #lb-search-input, #lb-visibility-toggle');
    controls.forEach(function(control) {
      control.setAttribute('aria-busy', isBusy ? 'true' : 'false');
    });
  }

  // ─── Layer Management ──────────────────────────────────────────
  function showLayer(layer) {
    var layers = ['loading', 'error', 'empty', 'content'];
    layers.forEach(function (l) {
      var el = document.getElementById('lb-' + l + '-layer');
      if (!el) return;
      if (l === layer) {
        el.style.display = l === 'content' ? 'flex' : 'flex';
        el.classList.remove('hidden');
      } else {
        el.style.display = 'none';
        el.classList.add('hidden');
      }
    });
  }

  // ─── Bento Cards (Top 3) ──────────────────────────────────────
  function renderBentoCards(rankings) {
    var grid = document.getElementById('lb-bento-grid');
    if (!grid) return;
    grid.innerHTML = '';

    var top3 = rankings.slice(0, 3);
    if (top3.length === 0) return;

    var tierBgColors = {
      'Premium': '#7F56D9',
      'Elite': '#2E90FA',
      'Pro': '#12B76A',
      'Plus': '#F79009',
      'Intro': '#D0D5DD'
    };

    top3.forEach(function (entry, i) {
      var rankNum = String(i + 1).padStart(2, '0');
      var roi = entry.metrics ? entry.metrics.portfolio_roi_bps : 0;
      var roiPct = (roi / 100).toFixed(1);
      var isPositive = roi >= 0;
      var tierColor = entry.tier_badge_color || tierBgColors[entry.tier_name] || '#D0D5DD';
      var assetCount = entry.metrics ? entry.metrics.asset_count : 0;
      // Tier badge styling moved to CSS data-tier attribute selectors so each
      // known tier ships an AA-compliant bg + text pair (≥4.5:1 contrast at
      // 10px). Inline `style="background:..."` only applied when the API
      // explicitly returns a custom `tier_badge_color` — that override path
      // is reserved for admin-managed bespoke tiers.
      var badgeStyle = entry.tier_badge_color
        ? ' style="background:' + escHtml(entry.tier_badge_color) + ';color:#181D27"'
        : '';
      var card = document.createElement('div');
      card.className = 'lb-bento-card';
      card.innerHTML =
        '<div class="lb-bento-watermark">' + rankNum + '</div>' +
        '<div class="lb-bento-header">' +
          '<div class="lb-bento-info">' +
            '<span class="lb-bento-tier-badge" data-tier="' + escHtml(entry.tier_name || '') + '"' + badgeStyle + '>' + escHtml(entry.tier_name) + '</span>' +
            '<div class="lb-bento-name">' + escHtml(entry.display_name) + '</div>' +
            '<div class="lb-bento-subtitle">' + assetCount + ' Asset' + (assetCount !== 1 ? 's' : '') + '</div>' +
          '</div>' +
          renderAssetMixDonut(entry.asset_mix) +
        '</div>' +
        '<div class="lb-bento-values">' +
          '<div>' +
            '<div class="lb-bento-aum-label">' + escHtml(getMetricName(currentMetric)) + '</div>' +
            '<div class="lb-bento-aum-value">' + escHtml(formatMetric(entry.metric_value, currentMetric)) + '</div>' +
          '</div>' +
          '<div style="text-align:right;">' +
            '<span class="lb-bento-yield ' + (isPositive ? 'positive' : 'negative') + '" title="Average annual yield target across this investor’s active portfolio">' +
              '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="' + (isPositive ? '23 6 13.5 15.5 8.5 10.5 1 18' : '23 18 13.5 8.5 8.5 13.5 1 6') + '"></polyline></svg>' +
              (isPositive ? '+' : '') + roiPct + '%' +
            '</span>' +
          '</div>' +
        '</div>' +
        renderAssetMixLegend(entry.asset_mix);

      grid.appendChild(card);
    });
  }

  /// Asset-type display labels + brand-aligned colors. Returned by the
  /// backend as snake_case enum strings (real_estate, commercial_property,
  /// commodity, business, startup, land_plot).
  var ASSET_TYPE_META = {
    'real_estate':         { label: 'Real Estate',  color: '#0000FF' },
    'commercial_property': { label: 'Commercial',   color: '#7F56D9' },
    'commodity':           { label: 'Commodity',    color: '#F79009' },
    'business':            { label: 'Business',     color: '#03FF88' },
    'startup':             { label: 'Startup',      color: '#2E90FA' },
    'land_plot':           { label: 'Land Plot',    color: '#12B76A' },
  };

  /// Render a multi-slice SVG donut for the user's asset-type breakdown.
  /// Falls back to an empty string when no mix data is present so older
  /// API responses keep working.
  function renderAssetMixDonut(mix) {
    if (!mix || !mix.length) return '';
    var total = mix.reduce(function (s, m) { return s + (m.invested_cents || 0); }, 0);
    if (total === 0) return '';
    var r = 28, circ = 2 * Math.PI * r;
    var offset = 0;
    var slices = mix.map(function (m) {
      var meta = ASSET_TYPE_META[m.asset_type] || { label: m.asset_type, color: '#D0D5DD' };
      var pct = m.invested_cents / total;
      var dash = (pct * circ).toFixed(2) + ' ' + (circ - pct * circ).toFixed(2);
      // Rotate each slice's start to where the previous one ended.
      var rot = (offset / circ) * 360 - 90; // -90 = start at 12 o'clock
      offset += pct * circ;
      return '<circle cx="32" cy="32" r="' + r + '" fill="transparent"' +
        ' stroke="' + meta.color + '" stroke-width="8"' +
        ' stroke-dasharray="' + dash + '"' +
        ' transform="rotate(' + rot.toFixed(2) + ' 32 32)"' +
        '><title>' + escHtml(meta.label) + ': ' + Math.round(pct * 100) + '%</title></circle>';
    }).join('');
    var ariaParts = mix.map(function (m) {
      var meta = ASSET_TYPE_META[m.asset_type] || { label: m.asset_type };
      return Math.round(((m.invested_cents || 0) / total) * 100) + '% ' + meta.label;
    }).join(', ');
    return (
      '<div class="lb-bento-donut">' +
        '<svg viewBox="0 0 64 64" role="img" aria-label="Asset mix: ' + escHtml(ariaParts) + '">' +
          '<circle cx="32" cy="32" r="' + r + '" fill="transparent" stroke="#f2f4f7" stroke-width="8"></circle>' +
          slices +
        '</svg>' +
        '<span class="lb-bento-donut-label" aria-hidden="true">MIX</span>' +
      '</div>'
    );
  }

  /// Render the inline legend below the bento values — one row per slice,
  /// up to 4 slices to keep the card compact.
  function renderAssetMixLegend(mix) {
    if (!mix || !mix.length) return '';
    var total = mix.reduce(function (s, m) { return s + (m.invested_cents || 0); }, 0);
    if (total === 0) return '';
    var items = mix.slice(0, 4).map(function (m) {
      var meta = ASSET_TYPE_META[m.asset_type] || { label: m.asset_type, color: '#D0D5DD' };
      var pct = Math.round((m.invested_cents / total) * 100);
      return '<div class="lb-bento-legend-item">' +
        '<span class="lb-bento-legend-dot" style="background:' + meta.color + '"></span>' +
        pct + '% ' + escHtml(meta.label) +
      '</div>';
    }).join('');
    return '<div class="lb-bento-legend">' + items + '</div>';
  }

  // ─── Minor Cards (Ranks 4–9) ──────────────────────────────────
  function renderMinorCards(rankings) {
    var grid = document.getElementById('lb-minor-grid');
    if (!grid) return;
    grid.innerHTML = '';

    var minor = rankings.slice(3, 9);
    if (minor.length === 0) {
      grid.style.display = 'none';
      return;
    }
    grid.style.display = '';

    var tierBgColors = {
      'Premium': '#7F56D9',
      'Elite': '#2E90FA',
      'Pro': '#12B76A',
      'Plus': '#F79009',
      'Intro': '#D0D5DD'
    };

    minor.forEach(function (entry) {
      var rankNum = String(entry.rank).padStart(2, '0');
      var roi = entry.metrics ? entry.metrics.portfolio_roi_bps : 0;
      var roiPct = (roi / 100).toFixed(1);
      var isPositive = roi >= 0;
      var tierColor = entry.tier_badge_color || tierBgColors[entry.tier_name] || '#D0D5DD';
      var initials = getInitials(entry.display_name);

      var card = document.createElement('div');
      card.className = 'lb-minor-card';
      card.innerHTML =
        '<div class="lb-minor-watermark">' + rankNum + '</div>' +
        '<div class="lb-minor-top">' +
          (entry.avatar_url
            ? '<div class="lb-minor-avatar" style="background:' + escHtml(tierColor) + '"><img src="' + escHtml(entry.avatar_url) + '" alt="' + escHtml(entry.display_name) + '"/></div>'
            : '<div class="lb-minor-avatar" style="background:' + escHtml(tierColor) + '">' + escHtml(initials) + '</div>'
          ) +
          '<div class="lb-minor-info">' +
            '<div class="lb-minor-name">' + escHtml(entry.display_name) + '</div>' +
            '<div class="lb-minor-tier">' + escHtml(entry.tier_name) + '</div>' +
          '</div>' +
        '</div>' +
        '<div class="lb-minor-bottom">' +
          '<div class="lb-minor-value">' + escHtml(formatMetric(entry.metric_value, currentMetric)) + '</div>' +
          '<span class="lb-minor-yield ' + (isPositive ? 'positive' : 'negative') + '">' +
            '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="' + (isPositive ? '23 6 13.5 15.5 8.5 10.5 1 18' : '23 18 13.5 8.5 8.5 13.5 1 6') + '"></polyline></svg>' +
            (isPositive ? '+' : '') + roiPct + '%' +
          '</span>' +
        '</div>';

      grid.appendChild(card);
    });
  }

  // ─── My Rank ───────────────────────────────────────────────────
  function renderMyRank(myRank) {
    if (!myRank) return;

    var rankEl = document.getElementById('lb-my-rank');
    var labelEl = document.getElementById('lb-my-metric-label');
    var card = document.getElementById('lb-my-rank-card');

    if (rankEl) {
      rankEl.textContent = myRank.rank ? '#' + myRank.rank : '#—';
    }
    
    // Add dynamic breakdown if we have other ranks
    if (card && myRank.metrics && myRank.rank) {
       // Remove existing breakdown if any
       var existing = card.querySelector('.lb-rank-breakdown');
       if (existing) existing.remove();
       
       var breakdown = document.createElement('div');
       breakdown.className = 'lb-rank-breakdown';
       breakdown.style.marginTop = '16px';
       breakdown.style.display = 'grid';
       breakdown.style.gridTemplateColumns = 'repeat(2, 1fr)';
       breakdown.style.gap = '16px 24px';
       breakdown.style.fontSize = '13px';
       breakdown.style.opacity = '0.9';

       var items = [
         { label: 'Holdings', val: myRank.metrics.total_invested_cents },
         { label: 'Assets', val: myRank.metrics.asset_count },
         { label: 'Yield', val: (myRank.metrics.portfolio_roi_bps/100).toFixed(1) + '%' },
         { label: 'Affiliates', val: myRank.metrics.affiliate_count }
       ];
       
       items.forEach(function(it) {
         var item = document.createElement('div');
         var label = document.createElement('span');
         label.style.fontWeight = '700';
         label.textContent = it.label + ': ';
         item.appendChild(label);
         item.appendChild(document.createTextNode(typeof it.val === 'number' && it.label !== 'Assets' && it.label !== 'Affiliates' ? formatCompact(it.val) : String(it.val)));
         breakdown.appendChild(item);
       });
       
       card.appendChild(breakdown);
    }

    if (labelEl) {
      labelEl.textContent = myRank.rank ? 'You are currently in the top tier of institutional traders.' : 'Start investing to get ranked.';
    }
  }

  // ─── Table (Institutional Ledger) ─────────────────────────────
  function renderTable(rankings, append) {
    var tbody = document.getElementById('lb-rankings-body');
    if (!tbody) return;
    if (!append) {
      tbody.innerHTML = '';
      if (rankings.length === 0) {
        tbody.innerHTML = '<tr><td colspan="4" style="text-align:center;color:#667085;padding:24px;">No investors found matching your filters.</td></tr>';
        return;
      }
    }

    // Update header — show active sort indicator so it's visible the column
    // is being driven by the topbar metric tab selection.
    var headerEl = document.getElementById('lb-table-metric-header');
    if (headerEl) {
      headerEl.innerHTML =
        '<span class="lb-th-label">' + escHtml(getMetricName(currentMetric)) + '</span>' +
        '<span class="lb-th-sort" aria-hidden="true">▼</span>';
      headerEl.setAttribute('aria-sort', 'descending');
      headerEl.classList.add('is-sorted');
    }

    for (var idx = 0; idx < rankings.length; idx++) {
      var entry = rankings[idx];
      var tr = document.createElement('tr');
      if (entry.is_current_user) tr.classList.add('is-me');

      var roi = entry.metrics ? entry.metrics.portfolio_roi_bps : 0;
      var roiPct = (roi / 100).toFixed(1);
      var isPositive = roi >= 0;
      var tierColor = entry.tier_badge_color || '#D0D5DD';
      var initials = getInitials(entry.display_name);

      // Rank
      var tdRank = document.createElement('td');
      tdRank.innerHTML = '<span class="lb-rank-cell">' + String(entry.rank).padStart(2, '0') + '</span>';
      tr.appendChild(tdRank);

      // Entity
      var tdEntity = document.createElement('td');
      var entityHtml = '<div class="lb-entity-cell">';
      if (entry.avatar_url) {
        entityHtml += '<div class="lb-entity-avatar" style="background:' + escHtml(tierColor) + '"><img src="' + escHtml(entry.avatar_url) + '" alt="' + escHtml(entry.display_name) + '"/></div>';
      } else {
        entityHtml += '<div class="lb-entity-avatar" style="background:' + escHtml(tierColor) + '">' + escHtml(initials) + '</div>';
      }
      entityHtml += '<div class="lb-entity-details">';
      entityHtml += '<div class="lb-entity-name">' + escHtml(entry.display_name) + '</div>';
      entityHtml += '<div class="lb-entity-sub">' + escHtml(entry.tier_name) + '</div>';
      entityHtml += '</div>';
      if (entry.is_current_user) {
        entityHtml += '<span class="lb-me-badge">You</span>';
      }
      entityHtml += '</div>';
      tdEntity.innerHTML = entityHtml;
      tr.appendChild(tdEntity);

      // Holdings (with hover/focus tooltip — see CSS .lb-holdings-cell:hover)
      var tdHoldings = document.createElement('td');
      tdHoldings.className = 'text-right lb-holdings-cell';
      tdHoldings.setAttribute('tabindex', '0');
      var holdingsHtml = '<span class="lb-holdings-value">' + escHtml(formatMetric(entry.metric_value, currentMetric)) + '</span>';
      if (entry.metrics) {
        holdingsHtml +=
          '<div class="lb-score-tooltip" role="tooltip">' +
            '<div class="lb-tt-header">Investor Breakdown</div>' +
            '<div class="lb-tt-row"><span>Total Invested:</span> <span>' + escHtml(formatMetric(entry.metrics.total_invested_cents, 'invested')) + '</span></div>' +
            '<div class="lb-tt-row"><span>Assets Held:</span> <span>' + escHtml(String(entry.metrics.asset_count)) + '</span></div>' +
            '<div class="lb-tt-row"><span>Avg Target Yield:</span> <span>' + escHtml(formatMetric(entry.metrics.portfolio_roi_bps, 'roi')) + '</span></div>' +
            '<div class="lb-tt-row"><span>Affiliates:</span> <span>' + escHtml(String(entry.metrics.affiliate_count)) + '</span></div>' +
            '<div class="lb-tt-row"><span>Network Volume:</span> <span>' + escHtml(formatMetric(entry.metrics.referral_network_value_cents, 'revenue')) + '</span></div>' +
            '<div class="lb-tt-row"><span>Highest Single Inv.:</span> <span>' + escHtml(formatMetric(entry.metrics.highest_investment_cents, 'highest_inv')) + '</span></div>' +
          '</div>';
      }
      tdHoldings.innerHTML = holdingsHtml;
      tr.appendChild(tdHoldings);

      // Yield — investment-weighted average TARGET yield (annual_yield_bps of
      // each asset weighted by purchase_value_cents). Not realized return.
      var tdYield = document.createElement('td');
      tdYield.className = 'text-right col-yield';
      tdYield.innerHTML = '<span class="lb-yield-value ' + (isPositive ? 'positive' : 'negative') + '" title="Investment-weighted average target yield (annual)">' + (isPositive ? '+' : '') + roiPct + '%</span>';
      tr.appendChild(tdYield);

      tbody.appendChild(tr);
    }
  }

  // ─── Meta ──────────────────────────────────────────────────────
  function renderMeta(data) {
    var countEl = document.getElementById('lb-total-participants');
    if (countEl) {
      countEl.textContent = data.total_participants.toLocaleString() + ' investors total';
    }

    var updatedEl = document.getElementById('lb-last-updated');
    if (updatedEl) {
      if (data.last_updated) {
        var d = new Date(data.last_updated);
        updatedEl.textContent = 'Updated: ' + d.toLocaleString();
      } else {
        updatedEl.textContent = 'Updated: live timeframe';
      }
    }
  }



  // ─── Preferences ───────────────────────────────────────────────
  function applyPrefs(prefs) {
    var toggle = document.getElementById('lb-visibility-toggle');
    if (toggle) toggle.checked = !!prefs.visible;
    var label = document.getElementById('lb-visibility-label');
    if (label) {
      label.textContent = prefs.visible ? 'Visible in public rankings' : 'Hidden from public rankings';
    }
    var avatarToggle = document.getElementById('lb-show-avatar-toggle');
    if (avatarToggle) {
      avatarToggle.checked = !!prefs.show_avatar;
      avatarToggle.disabled = !prefs.visible;
    }
    var nameInput = document.getElementById('lb-display-name-input');
    if (nameInput && document.activeElement !== nameInput) {
      nameInput.value = prefs.display_name || '';
      nameInput.disabled = !prefs.visible;
    }
    setPreferenceStatus(
      prefs.visible
        ? 'Your profile appears in the public leaderboard.'
        : 'Your profile is hidden — only you can see your rank.',
      false
    );
  }

  // ─── Global Handlers ──────────────────────────────────────────

  // Coalesce rapid tab clicks into a single refetch after TAB_DEBOUNCE_MS
  // of quiet. Visual tab state still updates instantly on every click —
  // only the network request is deferred, so the UI feels responsive while
  // bursts (e.g. tab-cycling with arrow keys) hit the API just once.
  function scheduleTabRefetch() {
    if (tabSwitchTimer) clearTimeout(tabSwitchTimer);
    tabSwitchTimer = setTimeout(function () {
      tabSwitchTimer = null;
      refetchAndRender();
    }, TAB_DEBOUNCE_MS);
  }

  // Top bar metric tabs
  window.switchMetricTab = function (metric, btn) {
    currentMetric = metric;
    currentPage = 1;

    // Update tab active state globally but only for metric components.
    // Also keeps the roving-tabindex in sync when mouse-clicked so the
    // tablist W3C pattern works for both pointer and keyboard users.
    var metricTabs = document.querySelectorAll('.lb-topbar-tab[data-metric], .lb-tf-btn[data-metric]');
    metricTabs.forEach(function (t) {
      var on = t.dataset.metric === currentMetric;
      t.classList.toggle('active', on);
      t.setAttribute('aria-selected', on ? 'true' : 'false');
      t.setAttribute('tabindex', on ? '0' : '-1');
    });

    scheduleTabRefetch();
  };

  window.switchTimeframe = function (tf, btn) {
    currentTimeframe = tf;
    currentPage = 1;
    var buttons = document.querySelectorAll('.lb-tf-btn[data-timeframe]');
    buttons.forEach(function (b) {
      b.classList.remove('active');
      b.setAttribute('aria-selected', 'false');
      b.setAttribute('tabindex', '-1');
    });
    if (btn) {
      btn.classList.add('active');
      btn.setAttribute('aria-selected', 'true');
      btn.setAttribute('tabindex', '0');
    }
    scheduleTabRefetch();
  };

  window.debounceSearch = function(event) {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(async function() {
      currentSearch = event.target.value.trim();
      currentPage = 1;
      await refetchAndRender();
    }, 300);
  };

  window.switchTier = async function (tierId, btn) {
    currentTier = tierId || '';
    currentPage = 1;
    var buttons = document.querySelectorAll('#lb-tier-filter [data-tier-id]');
    buttons.forEach(function (b) {
      b.classList.remove('active');
      b.setAttribute('aria-selected', 'false');
    });
    if (btn) {
      btn.classList.add('active');
      btn.setAttribute('aria-selected', 'true');
    }
    await refetchAndRender();
  };

  async function refetchAndRender() {
    if (isFetching) return;
    isFetching = true;
    var hasFilters = !!(currentSearch || currentTier);
    var bentoGrid = document.getElementById('lb-bento-grid');
    var summaryGrid = document.getElementById('lb-summary-grid');
    var table = document.getElementById('lb-rankings-table');
    var fadables = hasFilters ? [table].filter(Boolean) : [bentoGrid, summaryGrid, table].filter(Boolean);

    fadables.forEach(function (el) {
      el.style.opacity = '0.4';
      el.style.pointerEvents = 'none';
    });
    setControlsBusy(true);

    try {
      var data = await fetchRankings(currentMetric, currentTimeframe, currentPage, currentSearch, currentTier, currentPerPage);
      renderLeaderboardData(data);
    } catch (err) {
      console.error('Refetch failed:', err);
      showInlineStatus('Could not refresh the leaderboard. Previous results are still shown.');
    } finally {
      isFetching = false;
      setControlsBusy(false);
      fadables.forEach(function (el) {
        el.style.opacity = '1';
        el.style.pointerEvents = '';
      });
    }
  }

  // Audit task A2 — admin-only manual refresh. POST /api/leaderboard/refresh
  // with a CSRF token, then refetch the rankings on success so the table
  // updates without a full page reload. Failures surface inline.
  function getCsrfToken() {
    var match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]+)/);
    return match ? decodeURIComponent(match[1]) : '';
  }

  function setRefreshStatus(message, isError) {
    var status = document.getElementById('lb-refresh-status');
    if (!status) return;
    status.textContent = message;
    status.style.color = isError ? '#B42318' : 'var(--btn-primary-bg, #0000FF)';
  }

  window.adminRefreshLeaderboard = async function (btn) {
    if (!btn || btn.disabled) return;
    btn.disabled = true;
    var originalLabel = btn.textContent;
    btn.textContent = 'Refreshing...';
    setRefreshStatus('Recomputing scores...', false);
    try {
      var res = await fetch('/api/leaderboard/refresh', {
        method: 'POST',
        credentials: 'same-origin',
        headers: {
          'Content-Type': 'application/json',
          'X-CSRF-Token': getCsrfToken(),
        },
      });
      if (!res.ok) {
        var detail = '';
        try { detail = (await res.json()).error || ''; } catch (_) {}
        throw new Error(detail || ('HTTP ' + res.status));
      }
      setRefreshStatus('Leaderboard refreshed.', false);
      await refetchAndRender();
    } catch (err) {
      console.error('Admin refresh failed:', err);
      setRefreshStatus('Refresh failed: ' + (err.message || 'unknown error'), true);
    } finally {
      btn.disabled = false;
      btn.textContent = originalLabel;
    }
  };

  // ─── Preference handlers ──────────────────────────────────────
  // All three handlers share the same optimistic-update + rollback shape:
  //   1. Snapshot the previous prefs for rollback on failure.
  //   2. Optimistically mutate cachedPrefs + UI (applyPrefs).
  //   3. Refetch the rankings so visibility changes update the list
  //      (toggling off makes the viewer disappear, toggling on can add
  //      them; display_name edits update their entity row).
  //   4. On error, restore the previous prefs + UI.

  function commitPrefs(nextPrefs) {
    var previousPrefs = cachedPrefs ? Object.assign({}, cachedPrefs) : { visible: false, show_avatar: false, display_name: null };
    cachedPrefs = Object.assign({}, cachedPrefs, nextPrefs);
    applyPrefs(cachedPrefs);
    updatePreferences(cachedPrefs).then(function () {
      refetchAndRender();
    }).catch(function () {
      cachedPrefs = previousPrefs;
      applyPrefs(previousPrefs);
    });
  }

  window.toggleVisibility = function (checkbox) {
    commitPrefs({
      visible: checkbox.checked,
      show_avatar: cachedPrefs ? !!cachedPrefs.show_avatar : false,
      display_name: cachedPrefs ? (cachedPrefs.display_name || null) : null,
    });
  };

  window.toggleShowAvatar = function (checkbox) {
    commitPrefs({
      visible: cachedPrefs ? !!cachedPrefs.visible : false,
      show_avatar: checkbox.checked,
      display_name: cachedPrefs ? (cachedPrefs.display_name || null) : null,
    });
  };

  // Debounce display_name updates so we don't PUT on every keystroke.
  var displayNameTimer = null;
  window.updateDisplayName = function (input) {
    if (displayNameTimer) clearTimeout(displayNameTimer);
    displayNameTimer = setTimeout(function () {
      var trimmed = (input.value || '').trim();
      commitPrefs({
        visible: cachedPrefs ? !!cachedPrefs.visible : false,
        show_avatar: cachedPrefs ? !!cachedPrefs.show_avatar : false,
        display_name: trimmed === '' ? null : trimmed,
      });
    }, 600);
  };

  // ─── Tablist keyboard navigation (W3C ARIA APG pattern) ──────
  // Roving-tabindex: only the active tab is tab-reachable. Inside the
  // tablist, ← / → move and activate, Home / End jump to ends, Enter / Space
  // re-confirm the active tab. Pattern is applied to both the metric tabs
  // (left of topbar) and the timeframe tabs (right of topbar).
  function wireTablistKeyboardNav(root) {
    if (!root) return;
    var tabs = Array.prototype.slice.call(root.querySelectorAll('[role="tab"]'));
    if (!tabs.length) return;

    function setRovingFocus(targetIdx) {
      tabs.forEach(function (t, i) {
        t.setAttribute('tabindex', i === targetIdx ? '0' : '-1');
      });
    }

    function activate(idx) {
      var t = tabs[idx];
      if (!t) return;
      // Existing handlers (switchMetricTab / switchTimeframe / switchTier)
      // are inline onclick bindings — synthesizing a click runs the same
      // path the mouse does, including the async refetchAndRender.
      t.click();
      setRovingFocus(idx);
      t.focus();
    }

    // Seed roving tabindex: 0 on the active tab, -1 on the others.
    var activeIdx = Math.max(0, tabs.findIndex(function (t) {
      return t.classList.contains('active') || t.getAttribute('aria-selected') === 'true';
    }));
    setRovingFocus(activeIdx);

    tabs.forEach(function (tab, idx) {
      tab.addEventListener('keydown', function (e) {
        var next = null;
        switch (e.key) {
          case 'ArrowRight':
          case 'ArrowDown':
            next = (idx + 1) % tabs.length;
            break;
          case 'ArrowLeft':
          case 'ArrowUp':
            next = (idx - 1 + tabs.length) % tabs.length;
            break;
          case 'Home':
            next = 0;
            break;
          case 'End':
            next = tabs.length - 1;
            break;
          case 'Enter':
          case ' ':
            e.preventDefault();
            activate(idx);
            return;
          default:
            return;
        }
        e.preventDefault();
        activate(next);
      });
    });
  }

  // Wire all three tablists on DOMContentLoaded. Defer to a microtask so
  // any inline `aria-selected="true"` set in the template is reflected
  // before the roving-tabindex seed runs.
  document.addEventListener('DOMContentLoaded', function () {
    // Tier filter was removed by product decision but the markup may come
    // back — query defensively for all role=tablist groups.
    document.querySelectorAll('[role="tablist"]').forEach(wireTablistKeyboardNav);
  });
})();
