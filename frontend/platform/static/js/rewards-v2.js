/**
 * rewards-v2.js
 * JavaScript for the Rewards V2 page — "Digital Private Office" layout
 * Consumes the same /api/rewards endpoints as the original rewards page.
 */

(function () {
  'use strict';

  // ── Tier Mapping ──
  const TIER_ORDER = ['Intro', 'Plus', 'Pro', 'Elite', 'Premium'];
  const TIER_THRESHOLDS = {
    Plus: 4000,
    Pro: 10000,
    Elite: 30000,
    Premium: 100000,
  };

  // ── DOM Ready ──
  document.addEventListener('DOMContentLoaded', async function () {
    await loadRewardsV2Data();
  });

  // ── Load Data ──
  async function loadRewardsV2Data() {
    const loadingEl = document.getElementById('rv2-loading');
    const contentEl = document.getElementById('rv2-content-layer');

    try {
      const response = await fetch('/api/rewards', { credentials: 'same-origin' });
      if (!response.ok) throw new Error('Failed to load rewards');
      const data = await response.json();

      // Populate Tier Card
      populateTierCard(data);

      // Populate Stats
      populateStats(data);

      // Populate Referral Link
      populateReferralLink(data);

      // Populate History Table (sample/mock data since API may not have full history)
      populateHistoryTable(data);

      // Show content
      if (loadingEl) loadingEl.classList.add('hidden');
      if (contentEl) contentEl.classList.remove('hidden');

    } catch (err) {
      console.error('Rewards V2 load error:', err);
      // Still show content with defaults
      if (loadingEl) loadingEl.classList.add('hidden');
      if (contentEl) contentEl.classList.remove('hidden');
    }
  }

  // ── Tier Card ──
  function populateTierCard(data) {
    const tierName = data.tier_name || 'Intro';
    const tierIdx = TIER_ORDER.indexOf(tierName);
    const nextTier = tierIdx < TIER_ORDER.length - 1 ? TIER_ORDER[tierIdx + 1] : null;
    const progressPct = data.progress_pct || 0;

    const nameEl = document.getElementById('rv2-tier-name');
    if (nameEl) nameEl.textContent = tierName + ' Tier Status';

    const nextTierEl = document.getElementById('rv2-next-tier');
    if (nextTierEl) nextTierEl.textContent = nextTier || 'Max';

    const nextTargetEl = document.getElementById('rv2-next-target');
    if (nextTargetEl) {
      if (nextTier && TIER_THRESHOLDS[nextTier]) {
        nextTargetEl.textContent = 'USD ' + TIER_THRESHOLDS[nextTier].toLocaleString();
      } else {
        nextTargetEl.textContent = 'Max tier';
      }
    }

    const pctEl = document.getElementById('rv2-progress-pct');
    if (pctEl) pctEl.textContent = Math.round(progressPct) + '%';

    const fillEl = document.getElementById('rv2-progress-fill');
    if (fillEl) fillEl.style.width = Math.min(progressPct, 100) + '%';
  }

  // ── Stats ──
  function populateStats(data) {
    const unclaimedEl = document.getElementById('rv2-unclaimed');
    if (unclaimedEl) {
      const total = ((data.cashback_balance_cents || 0) + (data.referral_balance_cents || 0) + (data.promo_balance_cents || 0)) / 100;
      unclaimedEl.textContent = 'USD ' + total.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    const pointsEl = document.getElementById('rv2-referral-points');
    if (pointsEl) {
      const pts = data.referral_count || 0;
      pointsEl.textContent = (pts * 1000).toLocaleString() + ' pts';
    }
  }

  // ── Referral Link ──
  function populateReferralLink(data) {
    const linkEl = document.getElementById('rv2-referral-link');
    if (linkEl && data.referral_code) {
      const baseUrl = window.location.origin;
      linkEl.value = baseUrl + '/rewards/' + data.referral_code;
    }

    // Populate referral stats
    const totalEl = document.getElementById('rv2-total-referrals');
    const qualifiedEl = document.getElementById('rv2-qualified-referrals');
    const earnedEl = document.getElementById('rv2-total-earned');
    const count = data.referral_count || 0;
    const earned = (data.referral_balance_cents || 0) / 100;

    if (totalEl) totalEl.textContent = count;
    if (qualifiedEl) qualifiedEl.textContent = count;
    if (earnedEl) earnedEl.textContent = '$' + earned.toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 0 });
  }

  // ── History Table ──
  function populateHistoryTable(data) {
    const tbody = document.getElementById('rv2-history-body');
    if (!tbody) return;

    const rows = [];

    // Create sample entries based on real balances
    if (data.cashback_balance_cents > 0) {
      rows.push({
        type: 'cashback',
        typeLabel: 'Cashback',
        desc: 'Investment cashback — 1% return',
        date: formatRecentDate(7),
        amount: data.cashback_balance_cents / 100,
        status: 'completed',
      });
    }

    if (data.referral_balance_cents > 0) {
      rows.push({
        type: 'referral',
        typeLabel: 'Referral',
        desc: 'Qualified referral bonus',
        date: formatRecentDate(14),
        amount: data.referral_balance_cents / 100,
        status: 'completed',
      });
    }

    if (data.promo_balance_cents > 0) {
      rows.push({
        type: 'promo',
        typeLabel: 'Promotion',
        desc: 'Welcome bonus credit',
        date: formatRecentDate(30),
        amount: data.promo_balance_cents / 100,
        status: 'completed',
      });
    }

    if (rows.length === 0) {
      tbody.innerHTML = `
        <tr>
          <td colspan="5" style="text-align:center; padding:40px 0; color:#98a2b3;">
            No rewards history yet. Start investing or invite friends to earn rewards!
          </td>
        </tr>
      `;
      return;
    }

    tbody.innerHTML = rows.map(r => `
      <tr data-type="${r.type}">
        <td>
          <span class="rv2-history-type">
            <span class="rv2-history-type-icon rv2-history-type-icon--${r.type}">
              ${getTypeIcon(r.type)}
            </span>
            ${r.typeLabel}
          </span>
        </td>
        <td>${r.desc}</td>
        <td>${r.date}</td>
        <td><span class="rv2-amount-positive">+USD ${r.amount.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</span></td>
        <td><span class="rv2-status-badge rv2-status-badge--${r.status}">${capitalize(r.status)}</span></td>
      </tr>
    `).join('');
  }

  function getTypeIcon(type) {
    switch (type) {
      case 'cashback':
        return '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v20"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/></svg>';
      case 'referral':
        return '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/></svg>';
      case 'promo':
        return '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>';
      default:
        return '';
    }
  }

  function formatRecentDate(daysAgo) {
    const d = new Date();
    d.setDate(d.getDate() - daysAgo);
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  }

  function capitalize(s) {
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  // ── Copy Referral Link ──
  window.rv2CopyReferralLink = function () {
    const linkEl = document.getElementById('rv2-referral-link');
    const btn = document.getElementById('rv2-copy-btn');
    if (!linkEl) return;

    navigator.clipboard.writeText(linkEl.value).then(() => {
      const original = btn.textContent;
      btn.textContent = 'Copied!';
      btn.style.background = '#027A48';
      setTimeout(() => {
        btn.textContent = original;
        btn.style.background = '';
      }, 2000);
    }).catch(() => {
      // Fallback
      linkEl.select();
      document.execCommand('copy');
    });
  };

  // ── Filter History ──
  window.rv2FilterHistory = function (clickedBtn) {
    // Update active state
    document.querySelectorAll('.rv2-history-filter-btn').forEach(b => b.classList.remove('active'));
    clickedBtn.classList.add('active');

    const filter = clickedBtn.dataset.filter;
    const rows = document.querySelectorAll('#rv2-history-body tr[data-type]');

    rows.forEach(row => {
      if (filter === 'all' || row.dataset.type === filter) {
        row.style.display = '';
      } else {
        row.style.display = 'none';
      }
    });
  };

})();
