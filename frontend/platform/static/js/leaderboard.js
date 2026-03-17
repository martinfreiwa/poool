/**
 * Leaderboard Page — Client-side logic
 * Fetches rankings, populates podium + table, handles timeframe toggle & visibility preferences.
 */

(function () {
  'use strict';

  let currentTimeframe = 'alltime';
  let currentPage = 1;
  let currentSearch = '';
  let currentTier = '';
  let hasMore = true;
  let isFetching = false;
  let searchTimeout = null;
  let cachedPrefs = null;

  // ─── Init ──────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', init);

  async function init() {
    try {
      const [data, prefs] = await Promise.all([
        fetchRankings(currentTimeframe, currentPage, currentSearch, currentTier),
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
  async function fetchRankings(timeframe, page, search, tier) {
    page = page || 1;
    let url = `/api/leaderboard?timeframe=${timeframe}&page=${page}`;
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
      if (scoreEl) scoreEl.textContent = entry.total_score;
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
    var deltaEl = document.getElementById('lb-my-delta');

    if (rankEl) {
      rankEl.textContent = myRank.rank ? '#' + myRank.rank : '#—';
    }
    if (scoreEl) {
      scoreEl.textContent = myRank.total_score;
    }

    if (deltaEl) {
      var d = myRank.delta_weekly;
      if (d > 0) {
        deltaEl.textContent = '↑ ' + d;
        deltaEl.className = 'lb-yr-delta positive';
      } else if (d < 0) {
        deltaEl.textContent = '↓ ' + Math.abs(d);
        deltaEl.className = 'lb-yr-delta negative';
      } else {
        deltaEl.textContent = '— 0';
        deltaEl.className = 'lb-yr-delta neutral';
      }
    }

    // Score breakdown bars
    var bd = myRank.score_breakdown;
    setBar('lb-bar-invest', bd.invest_score);
    setBar('lb-bar-referral', bd.referral_score);
    setBar('lb-bar-tier', bd.tier_score);
    setBar('lb-bar-diversity', bd.diversity_score);
  }

  function setBar(id, score) {
    var el = document.getElementById(id);
    if (el) {
      el.style.width = (score / 10) + '%';
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
      avatarImg.src = entry.avatar_url || '/images/Image.webp';
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

      // Score cell with bar
      var scoreTd = document.createElement('td');
      var scoreDiv = document.createElement('div');
      scoreDiv.className = 'lb-score-bar-cell';

      var scoreNum = document.createElement('span');
      scoreNum.className = 'score-number';
      scoreNum.textContent = entry.total_score;
      scoreDiv.appendChild(scoreNum);

      var barOuter = document.createElement('div');
      barOuter.className = 'score-bar ds-progress ds-progress--sm';
      var barInner = document.createElement('div');
      barInner.className = 'score-bar-fill ds-progress__fill';
      barInner.style.width = (entry.total_score / 10) + '%';
      barOuter.appendChild(barInner);
      scoreDiv.appendChild(barOuter);

      // --- Add hover tooltip for score breakdown ---
      if (entry.score_breakdown) {
        var tt = document.createElement('div');
        tt.className = 'lb-score-tooltip';
        tt.innerHTML =
          '<div class="lb-tt-header">Score Breakdown</div>' +
          '<div class="lb-tt-row"><span>Investment:</span> <span>' + entry.score_breakdown.invest_score + '</span></div>' +
          '<div class="lb-tt-row"><span>Referrals:</span> <span>' + entry.score_breakdown.referral_score + '</span></div>' +
          '<div class="lb-tt-row"><span>Tier:</span> <span>' + entry.score_breakdown.tier_score + '</span></div>' +
          '<div class="lb-tt-row"><span>Diversity:</span> <span>' + entry.score_breakdown.diversity_score + '</span></div>';
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
  window.switchTimeframe = async function (btn) {
    // Update active button state
    document.querySelectorAll('.lb-tf-btn').forEach(function (b) {
      b.classList.remove('active', 'ds-btn--primary');
      b.classList.add('ds-btn--secondary');
    });
    btn.classList.add('active', 'ds-btn--primary');
    btn.classList.remove('ds-btn--secondary');

    var tf = btn.dataset.tf;
    currentTimeframe = tf;
    currentPage = 1;
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
      var data = await fetchRankings(currentTimeframe, currentPage, currentSearch, currentTier);
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
      var data = await fetchRankings(currentTimeframe, currentPage, currentSearch, currentTier);

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
