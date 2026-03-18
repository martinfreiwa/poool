/**
 * Leaderboard Page — Client-side logic
 * Fetches rankings, populates podium + table, handles metric toggle & visibility preferences.
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
  let hasMore = true;
  let isFetching = false;
  let searchTimeout = null;
  let cachedPrefs = null;

  function formatMetric(val, type) {
    if (type === 'invested' || type === 'revenue' || type === 'highest_inv') {
      return new Intl.NumberFormat('de-DE', { style: 'currency', currency: 'EUR' }).format(val / 100);
    } else if (type === 'roi') {
      return (val / 100).toFixed(2) + '%';
    } else {
      return val.toLocaleString();
    }
  }

  function getMetricName(type) {
    const map = {
      'invested': 'Total Investment',
      'assets': 'Number of Assets',
      'roi': 'Portfolio ROI',
      'affiliates': 'Affiliates Count',
      'revenue': 'Affiliate Revenue',
      'highest_inv': 'Highest Single Investment'
    };
    return map[type] || 'Score';
  }

  // ─── Init ──────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', init);

  async function init() {
    try {
      const [data, prefs] = await Promise.all([
        fetchRankings(currentMetric, currentTimeframe, currentPage, currentSearch, currentTier),
        fetchPreferences(),
      ]);

      cachedPrefs = prefs;

      // Initialize the custom Poool Dropdown
      var tierSelect = document.getElementById('lb-tier-filter');
      if (tierSelect && window.PooolDropdown) {
        window.PooolDropdown.fromSelect(tierSelect);
      }

      if (!data || data.total_participants === 0) {
        showLayer('empty');
        return;
      }

      renderPodium(data.rankings);
      renderMyRank(data.my_rank);
      renderTable(data.rankings);
      renderMeta(data);
      renderLoadMore(data);
      applyPrefs(prefs);
      showLayer('content');
    } catch (err) {
      console.error('Leaderboard init failed:', err);
      showLayer('error');
    }
  }

  // ─── API Calls ─────────────────────────────────────────────────
  async function fetchRankings(metric, timeframe, page, search, tier) {
    page = page || 1;
    timeframe = timeframe || 'alltime';
    let url = `/api/leaderboard?metric=${metric}&timeframe=${timeframe}&page=${page}`;
    if (search) url += `&search=${encodeURIComponent(search)}`;
    if (tier) url += `&tier_id=${tier}`;

    const res = await fetch(url, { credentials: 'same-origin' });
    if (!res.ok) throw new Error('Failed to fetch rankings');
    return res.json();
  }

  async function fetchPreferences() {
    try {
      const res = await fetch('/api/leaderboard/preferences', {
        credentials: 'same-origin',
      });
      if (!res.ok) return { visible: false, show_avatar: false };
      return res.json();
    } catch {
      return { visible: false, show_avatar: false };
    }
  }

  async function updatePreferences(prefs) {
    try {
      await fetch('/api/leaderboard/preferences', {
        method: 'PUT',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(prefs),
      });
    } catch (err) {
      console.error('Failed to update preferences:', err);
    }
  }

  // ─── Layer Management ──────────────────────────────────────────
  function showLayer(layer) {
    var layers = ['loading', 'error', 'empty', 'content'];
    layers.forEach(function (l) {
      var el = document.getElementById('lb-' + l + '-layer');
      if (!el) return;
      if (l === layer) {
        el.style.display = l === 'content' ? 'block' : 'flex';
        el.classList.remove('hidden');
      } else {
        el.style.display = 'none';
        el.classList.add('hidden');
      }
    });
  }

  // ─── Podium ────────────────────────────────────────────────────
  function renderPodium(rankings) {
    var positions = [
      { slot: 1, index: 0 },
      { slot: 2, index: 1 },
      { slot: 3, index: 2 },
    ];

    positions.forEach(function (pos) {
      var entry = rankings[pos.index];
      if (!entry) return;

      var nameEl = document.getElementById('lb-podium-' + pos.slot + '-name');
      var scoreEl = document.getElementById('lb-podium-' + pos.slot + '-score');
      var avatarEl = document.getElementById('lb-podium-' + pos.slot + '-avatar');
      var tierEl = document.getElementById('lb-podium-' + pos.slot + '-tier');

      if (nameEl) nameEl.textContent = entry.display_name;
      if (scoreEl) scoreEl.textContent = formatMetric(entry.metric_value, currentMetric);
      if (avatarEl && entry.avatar_url) avatarEl.src = entry.avatar_url;
      if (tierEl) {
        tierEl.textContent = entry.tier_name;
        tierEl.style.background = entry.tier_badge_color || '#D0D5DD';
      }
    });
  }

  // ─── My Rank ───────────────────────────────────────────────────
  function renderMyRank(myRank) {
    var rankEl = document.getElementById('lb-my-rank');
    var scoreEl = document.getElementById('lb-my-score');
    var labelEl = document.getElementById('lb-my-metric-label');

    if (rankEl) {
      rankEl.textContent = myRank.rank ? '#' + myRank.rank : '#—';
    }
    if (scoreEl) {
      scoreEl.textContent = formatMetric(myRank.metric_value, currentMetric);
    }
    if (labelEl) {
      labelEl.textContent = getMetricName(currentMetric);
    }
  }

  // ─── Table ─────────────────────────────────────────────────────
  function renderTable(rankings, append = false) {
    var tbody = document.getElementById('lb-rankings-body');
    if (!tbody) return;
    if (!append) {
      tbody.innerHTML = '';
      if (rankings.length === 0) {
        tbody.innerHTML = '<tr><td colspan="4" style="text-align:center;color:#667085;padding:24px;">No investors found matching your filters.</td></tr>';
        return;
      }
    }

    // Update table header to dynamic metric name
    var headerEl = document.getElementById('lb-table-metric-header');
    if (headerEl) {
      headerEl.textContent = getMetricName(currentMetric);
    }

    rankings.forEach(function (entry) {
      var tr = document.createElement('tr');
      if (entry.is_current_user) tr.classList.add('is-me');

      // Rank cell
      var rankTd = document.createElement('td');
      rankTd.className = 'rank-cell' + (entry.rank <= 3 ? ' top-3' : '');
      rankTd.textContent = entry.rank;
      tr.appendChild(rankTd);

      // Investor cell
      var investorTd = document.createElement('td');
      var investorDiv = document.createElement('div');
      investorDiv.className = 'lb-investor-cell';

      var avatarImg = document.createElement('img');
      avatarImg.className = 'lb-row-avatar';
      avatarImg.src = entry.avatar_url || '/static/images/Image.webp';
      avatarImg.alt = entry.display_name;
      investorDiv.appendChild(avatarImg);

      var nameSpan = document.createElement('span');
      nameSpan.className = 'lb-row-name';
      nameSpan.textContent = entry.display_name;
      investorDiv.appendChild(nameSpan);

      if (entry.is_current_user) {
        var meB = document.createElement('span');
        meB.className = 'lb-row-me-badge';
        meB.textContent = 'You';
        investorDiv.appendChild(meB);
      }

      investorTd.appendChild(investorDiv);
      tr.appendChild(investorTd);

      // Score cell
      var scoreTd = document.createElement('td');
      var scoreDiv = document.createElement('div');
      scoreDiv.className = 'lb-score-bar-cell';
      scoreDiv.style.flexDirection = 'row';
      scoreDiv.style.alignItems = 'center';

      var scoreNum = document.createElement('span');
      scoreNum.className = 'score-number';
      scoreNum.textContent = formatMetric(entry.metric_value, currentMetric);
      scoreDiv.appendChild(scoreNum);

      // --- Add hover tooltip for raw metrics ---
      if (entry.metrics) {
        var tt = document.createElement('div');
        tt.className = 'lb-score-tooltip';
        tt.innerHTML =
          '<div class="lb-tt-header">Investor Details</div>' +
          '<div class="lb-tt-row"><span>Total Investment:</span> <span>' + escHtml(formatMetric(entry.metrics.total_invested_cents, 'invested')) + '</span></div>' +
          '<div class="lb-tt-row"><span>Assets:</span> <span>' + escHtml(String(entry.metrics.asset_count)) + '</span></div>' +
          '<div class="lb-tt-row"><span>Portfolio ROI:</span> <span>' + escHtml(formatMetric(entry.metrics.portfolio_roi_bps, 'roi')) + '</span></div>' +
          '<div class="lb-tt-row"><span>Affiliates:</span> <span>' + escHtml(String(entry.metrics.affiliate_count)) + '</span></div>' +
          '<div class="lb-tt-row"><span>Ref Revenue:</span> <span>' + escHtml(formatMetric(entry.metrics.referral_revenue_cents, 'revenue')) + '</span></div>' +
          '<div class="lb-tt-row"><span>Highest Inv:</span> <span>' + escHtml(formatMetric(entry.metrics.highest_investment_cents, 'highest_inv')) + '</span></div>';
        scoreDiv.appendChild(tt);
      }

      scoreTd.appendChild(scoreDiv);
      tr.appendChild(scoreTd);

      // Tier cell
      var tierTd = document.createElement('td');
      var pill = document.createElement('span');
      pill.className = 'lb-tier-pill';
      pill.textContent = entry.tier_name;
      pill.style.background = entry.tier_badge_color || '#D0D5DD';
      tierTd.appendChild(pill);
      tr.appendChild(tierTd);

      tbody.appendChild(tr);
    });
  }

  // ─── Meta ──────────────────────────────────────────────────────
  function renderMeta(data) {
    var countEl = document.getElementById('lb-total-participants');
    if (countEl) {
      countEl.textContent = data.total_participants.toLocaleString() + ' investors total';
    }

    var updatedEl = document.getElementById('lb-last-updated');
    if (updatedEl && data.last_updated) {
      var d = new Date(data.last_updated);
      updatedEl.textContent = 'Last updated: ' + d.toLocaleString();
    }
  }

  // ─── Load More ─────────────────────────────────────────────────
  function renderLoadMore(data) {
    hasMore = data.has_more;
    var btn = document.getElementById('lb-load-more-btn');
    var container = document.getElementById('lb-load-more-container');
    if (btn && container) {
      if (hasMore) {
        container.classList.remove('hidden');
        container.style.display = 'block';
        btn.disabled = false;
        btn.textContent = 'Load More Rankings';
      } else {
        container.classList.add('hidden');
        container.style.display = 'none';
      }
    }
  }

  // ─── Preferences ───────────────────────────────────────────────
  function applyPrefs(prefs) {
    var toggle = document.getElementById('lb-visibility-toggle');
    if (toggle) {
      toggle.checked = prefs.visible;
    }
  }

  // ─── Global Handlers ──────────────────────────────────────────
  window.switchMetric = async function () {
    var select = document.getElementById('lb-metric-select');
    if (!select) return;
    
    currentMetric = select.value;
    currentPage = 1;
    await refetchAndRender();
  };

  window.switchTimeframe = async function (tf, btn) {
    currentTimeframe = tf;
    currentPage = 1;
    // Update active button styling
    var buttons = document.querySelectorAll('.lb-tf-btn[data-timeframe]');
    buttons.forEach(function (b) { b.classList.remove('active'); });
    if (btn) btn.classList.add('active');
    await refetchAndRender();
  };

  window.applyFilters = async function() {
    var tierSelect = document.getElementById('lb-tier-filter');
    currentTier = tierSelect ? tierSelect.value : '';
    currentPage = 1;
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

  window.loadMore = async function() {
    if (!hasMore || isFetching) return;
    isFetching = true;
    var btn = document.getElementById('lb-load-more-btn');
    if (btn) btn.textContent = 'Loading...';

    currentPage++;
    try {
      var data = await fetchRankings(currentMetric, currentTimeframe, currentPage, currentSearch, currentTier);
      renderTable(data.rankings, true);
      renderLoadMore(data);
    } catch (e) {
      console.error(e);
      currentPage--;
    } finally {
      isFetching = false;
    }
  };

  async function refetchAndRender() {
    // When filters are active, only fade the table (podium stays as the global top 3).
    var hasFilters = !!(currentSearch || currentTier);
    var podium = document.getElementById('lb-podium');
    var rankCard = document.getElementById('lb-my-rank-card');
    var table = document.getElementById('lb-rankings-table');
    var fadables = hasFilters ? [table].filter(Boolean) : [podium, rankCard, table].filter(Boolean);

    fadables.forEach(function (el) {
      el.style.transition = 'opacity 0.2s ease';
      el.style.opacity = '0.4';
      el.style.pointerEvents = 'none';
    });

    try {
      var data = await fetchRankings(currentMetric, currentTimeframe, currentPage, currentSearch, currentTier);

      // Only go to full-page empty state when there are zero investors on the leaderboard at all (no filters active)
      if (!data || (data.total_participants === 0 && !hasFilters)) {
        showLayer('empty');
        return;
      }

      showLayer('content');

      // Re-render podium only when there's no active filter (so podium always shows real global top 3)
      if (!hasFilters) {
        renderPodium(data.rankings);
        renderMyRank(data.my_rank);
      }

      renderTable(data.rankings, false);
      renderMeta(data);
      renderLoadMore(data);
    } catch (err) {
      console.error('Refetch failed:', err);
    } finally {
      fadables.forEach(function (el) {
        el.style.opacity = '1';
        el.style.pointerEvents = '';
      });
    }
  }

  window.toggleVisibility = function (checkbox) {
    // Preserve avatar preference — only update visibility
    var prefs = {
      visible: checkbox.checked,
      show_avatar: cachedPrefs ? (cachedPrefs.show_avatar || false) : false,
      display_name: cachedPrefs ? (cachedPrefs.display_name || null) : null,
    };
    cachedPrefs = Object.assign({}, cachedPrefs, { visible: checkbox.checked });
    updatePreferences(prefs);
  };
})();
