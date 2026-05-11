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

  // ─── Sample / Demo Data ─────────────────────────────────────────
  var DEMO_RANKINGS = [
    { rank: 1,  display_name: 'Alexander K.',  avatar_url: null, tier_name: 'Premium', tier_badge_color: '#7F56D9', metric_value: 4825000, is_current_user: false, metrics: { total_invested_cents: 4825000, asset_count: 12, portfolio_roi_bps: 1450, affiliate_count: 8,  referral_revenue_cents: 920000,  highest_investment_cents: 1500000 }},
    { rank: 2,  display_name: 'Sophia M.',     avatar_url: null, tier_name: 'Elite',   tier_badge_color: '#2E90FA', metric_value: 3690000, is_current_user: false, metrics: { total_invested_cents: 3690000, asset_count: 9,  portfolio_roi_bps: 1280, affiliate_count: 5,  referral_revenue_cents: 410000,  highest_investment_cents: 1200000 }},
    { rank: 3,  display_name: 'Maximilian R.', avatar_url: null, tier_name: 'Elite',   tier_badge_color: '#2E90FA', metric_value: 2850000, is_current_user: false, metrics: { total_invested_cents: 2850000, asset_count: 7,  portfolio_roi_bps: 1120, affiliate_count: 3,  referral_revenue_cents: 180000,  highest_investment_cents: 950000  }},
    { rank: 4,  display_name: 'Emma L.',       avatar_url: null, tier_name: 'Pro',     tier_badge_color: '#12B76A', metric_value: 2150000, is_current_user: false, metrics: { total_invested_cents: 2150000, asset_count: 6,  portfolio_roi_bps: 980,  affiliate_count: 11, referral_revenue_cents: 750000,  highest_investment_cents: 800000  }},
    { rank: 5,  display_name: 'Lukas W.',      avatar_url: null, tier_name: 'Pro',     tier_badge_color: '#12B76A', metric_value: 1780000, is_current_user: false, metrics: { total_invested_cents: 1780000, asset_count: 5,  portfolio_roi_bps: 1340, affiliate_count: 2,  referral_revenue_cents: 95000,   highest_investment_cents: 750000  }},
    { rank: 6,  display_name: 'Hannah B.',     avatar_url: null, tier_name: 'Pro',     tier_badge_color: '#12B76A', metric_value: 1320000, is_current_user: false, metrics: { total_invested_cents: 1320000, asset_count: 4,  portfolio_roi_bps: 870,  affiliate_count: 6,  referral_revenue_cents: 320000,  highest_investment_cents: 600000  }},
    { rank: 7,  display_name: 'Noah S.',       avatar_url: null, tier_name: 'Plus',    tier_badge_color: '#F79009', metric_value: 980000,  is_current_user: false, metrics: { total_invested_cents: 980000,  asset_count: 3,  portfolio_roi_bps: 760,  affiliate_count: 1,  referral_revenue_cents: 45000,   highest_investment_cents: 500000  }},
    { rank: 8,  display_name: 'Mia F.',        avatar_url: null, tier_name: 'Plus',    tier_badge_color: '#F79009', metric_value: 750000,  is_current_user: false, metrics: { total_invested_cents: 750000,  asset_count: 3,  portfolio_roi_bps: 920,  affiliate_count: 4,  referral_revenue_cents: 210000,  highest_investment_cents: 350000  }},
    { rank: 9,  display_name: 'Julian D.',     avatar_url: null, tier_name: 'Plus',    tier_badge_color: '#F79009', metric_value: 520000,  is_current_user: false, metrics: { total_invested_cents: 520000,  asset_count: 2,  portfolio_roi_bps: 650,  affiliate_count: 0,  referral_revenue_cents: 0,       highest_investment_cents: 300000  }},
    { rank: 10, display_name: 'Lena V.',       avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 350000,  is_current_user: false, metrics: { total_invested_cents: 350000,  asset_count: 1,  portfolio_roi_bps: 480,  affiliate_count: 0,  referral_revenue_cents: 0,       highest_investment_cents: 350000  }},
    { rank: 11, display_name: 'Oliver P.',     avatar_url: null, tier_name: 'Plus',    tier_badge_color: '#F79009', metric_value: 310000,  is_current_user: false, metrics: { total_invested_cents: 310000,  asset_count: 2,  portfolio_roi_bps: 540,  affiliate_count: 3,  referral_revenue_cents: 62000,   highest_investment_cents: 200000  }},
    { rank: 12, display_name: 'Isabelle T.',   avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 275000,  is_current_user: false, metrics: { total_invested_cents: 275000,  asset_count: 1,  portfolio_roi_bps: 390,  affiliate_count: 1,  referral_revenue_cents: 18000,   highest_investment_cents: 275000  }},
    { rank: 13, display_name: 'Marcus H.',     avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 240000,  is_current_user: false, metrics: { total_invested_cents: 240000,  asset_count: 1,  portfolio_roi_bps: 310,  affiliate_count: 0,  referral_revenue_cents: 0,       highest_investment_cents: 240000  }},
    { rank: 14, display_name: 'Clara N.',      avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 195000,  is_current_user: false, metrics: { total_invested_cents: 195000,  asset_count: 1,  portfolio_roi_bps: 280,  affiliate_count: 2,  referral_revenue_cents: 9500,    highest_investment_cents: 195000  }},
    { rank: 15, display_name: 'Felix A.',      avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 160000,  is_current_user: false, metrics: { total_invested_cents: 160000,  asset_count: 1,  portfolio_roi_bps: 220,  affiliate_count: 0,  referral_revenue_cents: 0,       highest_investment_cents: 160000  }},
    { rank: 16, display_name: 'Anna C.',       avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 132000,  is_current_user: false, metrics: { total_invested_cents: 132000,  asset_count: 1,  portfolio_roi_bps: 190,  affiliate_count: 1,  referral_revenue_cents: 5000,    highest_investment_cents: 132000  }},
    { rank: 17, display_name: 'David R.',      avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 110000,  is_current_user: false, metrics: { total_invested_cents: 110000,  asset_count: 1,  portfolio_roi_bps: 160,  affiliate_count: 0,  referral_revenue_cents: 0,       highest_investment_cents: 110000  }},
    { rank: 18, display_name: 'Yuki T.',       avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 85000,   is_current_user: false, metrics: { total_invested_cents: 85000,   asset_count: 1,  portfolio_roi_bps: 120,  affiliate_count: 0,  referral_revenue_cents: 0,       highest_investment_cents: 85000   }},
    { rank: 19, display_name: 'Sara K.',       avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 60000,   is_current_user: false, metrics: { total_invested_cents: 60000,   asset_count: 1,  portfolio_roi_bps: 90,   affiliate_count: 0,  referral_revenue_cents: 0,       highest_investment_cents: 60000   }},
    { rank: 20, display_name: 'Tom B.',        avatar_url: null, tier_name: 'Intro',   tier_badge_color: '#D0D5DD', metric_value: 40000,   is_current_user: true,  metrics: { total_invested_cents: 40000,   asset_count: 1,  portfolio_roi_bps: 60,   affiliate_count: 0,  referral_revenue_cents: 0,       highest_investment_cents: 40000   }},
  ];

  function getDemoData(realData) {
    var metricKey = {
      'invested':    'total_invested_cents',
      'assets':      'asset_count',
      'roi':         'portfolio_roi_bps',
      'affiliates':  'affiliate_count',
      'revenue':     'referral_revenue_cents',
      'highest_inv': 'highest_investment_cents',
    }[currentMetric] || 'total_invested_cents';

    var timeframeMultiplier = currentTimeframe === 'weekly' ? 0.05 : (currentTimeframe === 'monthly' ? 0.2 : 1);
    var copied = JSON.parse(JSON.stringify(DEMO_RANKINGS));
    var sorted = copied.sort(function (a, b) {
      return b.metrics[metricKey] - a.metrics[metricKey];
    });

    sorted.forEach(function (entry, i) {
      entry.rank = i + 1;
      if (metricKey !== 'asset_count' && metricKey !== 'affiliate_count' && metricKey !== 'portfolio_roi_bps') {
         entry.metrics[metricKey] = Math.round(entry.metrics[metricKey] * timeframeMultiplier);
      }
      entry.metric_value = entry.metrics[metricKey];
    });

    var mockMe = sorted.find(function(r) { return r.is_current_user; }) || sorted[sorted.length-1];
    return {
      rankings: sorted,
      my_rank: (realData && realData.my_rank && realData.my_rank.rank) ? realData.my_rank : mockMe,
      total_participants: 20,
      metric_type: currentMetric,
      timeframe: currentTimeframe,
      last_updated: new Date().toISOString(),
      has_more: false,
    };
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
    var map = {
      'invested': 'Total Investment',
      'assets': 'Number of Assets',
      'roi': 'Portfolio ROI',
      'affiliates': 'Affiliates Count',
      'revenue': 'Affiliate Revenue',
      'highest_inv': 'Highest Single Investment'
    };
    return map[type] || 'Score';
  }

  function getInitials(name) {
    if (!name) return '??';
    var parts = name.trim().split(/\s+/);
    if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase();
    return name.substring(0, 2).toUpperCase();
  }

  // Compute percentages for asset mix donut/bar
  function getAssetMixPct(entry) {
    var inv = entry.metrics ? (entry.metrics.total_invested_cents || 0) : 0;
    var ref = entry.metrics ? (entry.metrics.referral_revenue_cents || 0) : 0;
    var total = inv + ref;
    if (total === 0) return { primary: 100, secondary: 0 };
    var p = Math.round((inv / total) * 100);
    return { primary: Math.max(p, 5), secondary: Math.max(100 - p, 5) };
  }

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
  }

  function renderLeaderboardData(data) {
    var forceDemo = new URLSearchParams(window.location.search).has('demo');

    if (forceDemo) {
      var demo = getDemoData(data);
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
      return;
    }

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
    status.style.color = isError ? '#B42318' : 'var(--btn-primary-bg, #0000FF)';
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
    var controls = document.querySelectorAll('.lb-topbar-tab[data-metric], .lb-tf-btn[data-timeframe], #lb-per-page-select, #lb-search-input, #lb-visibility-toggle');
    controls.forEach(function(control) {
      control.disabled = isBusy;
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
      var mix = getAssetMixPct(entry);
      var roi = entry.metrics ? entry.metrics.portfolio_roi_bps : 0;
      var roiPct = (roi / 100).toFixed(1);
      var isPositive = roi >= 0;
      var tierColor = entry.tier_badge_color || tierBgColors[entry.tier_name] || '#D0D5DD';
      var assetCount = entry.metrics ? entry.metrics.asset_count : 0;

      // Donut SVG values
      var circumference = 2 * Math.PI * 28; // r=28 → ~175.9
      var primaryArc = (mix.primary / 100) * circumference;
      var secondaryArc = (mix.secondary / 100) * circumference;
      var primaryOffset = circumference - primaryArc;
      var secondaryOffset = circumference - secondaryArc;
      // Rotate the secondary arc to start after primary
      var secondaryRotation = (mix.primary / 100) * 360;

      var card = document.createElement('div');
      card.className = 'lb-bento-card';
      card.innerHTML =
        '<div class="lb-bento-watermark">' + rankNum + '</div>' +
        '<div class="lb-bento-header">' +
          '<div class="lb-bento-info">' +
            '<span class="lb-bento-tier-badge" style="background:' + escHtml(tierColor) + '">' + escHtml(entry.tier_name) + '</span>' +
            '<div class="lb-bento-name">' + escHtml(entry.display_name) + '</div>' +
            '<div class="lb-bento-subtitle">' + assetCount + ' Asset' + (assetCount !== 1 ? 's' : '') + '</div>' +
          '</div>' +
          '<div class="lb-bento-donut">' +
            '<svg viewBox="0 0 64 64">' +
              '<circle cx="32" cy="32" r="28" fill="transparent" stroke="#f2f4f7" stroke-width="7"></circle>' +
              '<circle cx="32" cy="32" r="28" fill="transparent" stroke="#0000FF" stroke-width="7" stroke-dasharray="' + circumference + '" stroke-dashoffset="' + primaryOffset + '"></circle>' +
              '<circle cx="32" cy="32" r="28" fill="transparent" stroke="#03FF88" stroke-width="7" stroke-dasharray="' + circumference + '" stroke-dashoffset="' + secondaryOffset + '" style="transform: rotate(' + secondaryRotation + 'deg); transform-origin: center;"></circle>' +
            '</svg>' +
            '<span class="lb-bento-donut-label">MIX</span>' +
          '</div>' +
        '</div>' +
        '<div class="lb-bento-values">' +
          '<div>' +
            '<div class="lb-bento-aum-label">' + escHtml(getMetricName(currentMetric)) + '</div>' +
            '<div class="lb-bento-aum-value">' + escHtml(formatMetric(entry.metric_value, currentMetric)) + '</div>' +
          '</div>' +
          '<div style="text-align:right;">' +
            '<span class="lb-bento-yield ' + (isPositive ? 'positive' : 'negative') + '">' +
              '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="' + (isPositive ? '23 6 13.5 15.5 8.5 10.5 1 18' : '23 18 13.5 8.5 8.5 13.5 1 6') + '"></polyline></svg>' +
              (isPositive ? '+' : '') + roiPct + '%' +
            '</span>' +
          '</div>' +
        '</div>' +
        '<div class="lb-bento-legend">' +
          '<div class="lb-bento-legend-item"><span class="lb-bento-legend-dot primary"></span>' + mix.primary + '% Direct</div>' +
          '<div class="lb-bento-legend-item"><span class="lb-bento-legend-dot green"></span>' + mix.secondary + '% Referral</div>' +
        '</div>';

      grid.appendChild(card);
    });
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
        tbody.innerHTML = '<tr><td colspan="6" style="text-align:center;color:#667085;padding:24px;">No investors found matching your filters.</td></tr>';
        return;
      }
    }

    // Update header
    var headerEl = document.getElementById('lb-table-metric-header');
    if (headerEl) headerEl.textContent = getMetricName(currentMetric);

    for (var idx = 0; idx < rankings.length; idx++) {
      var entry = rankings[idx];
      var tr = document.createElement('tr');
      if (entry.is_current_user) tr.classList.add('is-me');

      var mix = getAssetMixPct(entry);
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

      // Asset Mix
      var tdMix = document.createElement('td');
      tdMix.className = 'col-mix';
      tdMix.innerHTML =
        '<div class="lb-mix-bar-cell">' +
          '<div class="lb-mix-bar">' +
            '<div class="lb-mix-bar-primary" style="width:' + mix.primary + '%"></div>' +
            '<div class="lb-mix-bar-green" style="width:' + mix.secondary + '%"></div>' +
          '</div>' +
          '<span class="lb-mix-ratio">' + mix.primary + '/' + mix.secondary + '</span>' +
        '</div>';
      tr.appendChild(tdMix);

      // Holdings (with tooltip)
      var tdHoldings = document.createElement('td');
      tdHoldings.className = 'text-right lb-holdings-cell';
      var holdingsHtml = '<span class="lb-holdings-value">' + escHtml(formatMetric(entry.metric_value, currentMetric)) + '</span>';
      if (entry.metrics) {
        holdingsHtml +=
          '<div class="lb-score-tooltip">' +
            '<div class="lb-tt-header">Investor Details</div>' +
            '<div class="lb-tt-row"><span>Total Investment:</span> <span>' + escHtml(formatMetric(entry.metrics.total_invested_cents, 'invested')) + '</span></div>' +
            '<div class="lb-tt-row"><span>Assets:</span> <span>' + escHtml(String(entry.metrics.asset_count)) + '</span></div>' +
            '<div class="lb-tt-row"><span>Portfolio ROI:</span> <span>' + escHtml(formatMetric(entry.metrics.portfolio_roi_bps, 'roi')) + '</span></div>' +
            '<div class="lb-tt-row"><span>Affiliates:</span> <span>' + escHtml(String(entry.metrics.affiliate_count)) + '</span></div>' +
            '<div class="lb-tt-row"><span>Ref Revenue:</span> <span>' + escHtml(formatMetric(entry.metrics.referral_revenue_cents, 'revenue')) + '</span></div>' +
            '<div class="lb-tt-row"><span>Highest Inv:</span> <span>' + escHtml(formatMetric(entry.metrics.highest_investment_cents, 'highest_inv')) + '</span></div>' +
          '</div>';
      }
      tdHoldings.innerHTML = holdingsHtml;
      tr.appendChild(tdHoldings);

      // Yield
      var tdYield = document.createElement('td');
      tdYield.className = 'text-right col-yield';
      tdYield.innerHTML = '<span class="lb-yield-value ' + (isPositive ? 'positive' : 'negative') + '">' + (isPositive ? '+' : '') + roiPct + '%</span>';
      tr.appendChild(tdYield);

      // Status
      var tdStatus = document.createElement('td');
      tdStatus.className = 'text-right col-status';
      var statusClass = (entry.metrics && entry.metrics.portfolio_roi_bps < 0) ? 'lb-status-hedged' : 'lb-status-active';
      var statusText = (entry.metrics && entry.metrics.portfolio_roi_bps < 0) ? 'HEDGED' : 'ACTIVE';
      tdStatus.innerHTML = '<span class="lb-status-pill ' + statusClass + '">' + statusText + '</span>';
      tr.appendChild(tdStatus);

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
    if (toggle) toggle.checked = prefs.visible;
    var label = document.getElementById('lb-visibility-label');
    if (label) {
      label.textContent = prefs.visible ? 'Visible in public rankings' : 'Hidden from public rankings';
    }
    setPreferenceStatus(prefs.visible ? 'Your public leaderboard profile is visible.' : 'Your public leaderboard profile is hidden.', false);
  }

  // ─── Global Handlers ──────────────────────────────────────────

  // Top bar metric tabs
  window.switchMetricTab = async function (metric, btn) {
    currentMetric = metric;
    currentPage = 1;

    // Update tab active state globally but only for metric components
    var metricTabs = document.querySelectorAll('.lb-topbar-tab[data-metric], .lb-tf-btn[data-metric]');
    metricTabs.forEach(function (t) {
      if (t.dataset.metric === currentMetric) {
        t.classList.add('active');
      } else {
        t.classList.remove('active');
      }
    });

    await refetchAndRender();
  };

  window.switchTimeframe = async function (tf, btn) {
    currentTimeframe = tf;
    currentPage = 1;
    var buttons = document.querySelectorAll('.lb-tf-btn[data-timeframe]');
    buttons.forEach(function (b) { b.classList.remove('active'); });
    if (btn) btn.classList.add('active');
    await refetchAndRender();
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
    buttons.forEach(function (b) { b.classList.remove('active'); });
    if (btn) btn.classList.add('active');
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

  window.toggleVisibility = function (checkbox) {
    var previousPrefs = cachedPrefs ? Object.assign({}, cachedPrefs) : { visible: !checkbox.checked, show_avatar: false, display_name: null };
    var prefs = {
      visible: checkbox.checked,
      show_avatar: cachedPrefs ? (cachedPrefs.show_avatar || false) : false,
      display_name: cachedPrefs ? (cachedPrefs.display_name || null) : null,
    };
    cachedPrefs = Object.assign({}, cachedPrefs, { visible: checkbox.checked });
    updatePreferences(prefs).catch(function() {
      cachedPrefs = previousPrefs;
      applyPrefs(previousPrefs);
    });
  };
})();
