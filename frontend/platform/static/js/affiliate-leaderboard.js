/* global window, document, fetch */
/**
 * Phase-5: Public affiliate leaderboard page.
 *
 * Reads `/api/affiliate/leaderboard/public?period=month|lifetime` (no auth).
 * Renders the top 50 opt-in affiliates with rank + avatar + display name +
 * tier badge + amount. Tabs switch period without a hard reload.
 */
(function () {
  'use strict';

  function fmtEur(cents) {
    if (!cents) return '€0';
    return new Intl.NumberFormat('de-DE', {
      style: 'currency', currency: 'EUR', maximumFractionDigits: 0,
    }).format(cents / 100);
  }
  function escape(s) {
    return String(s == null ? '' : s)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;')
      .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  async function load(period) {
    const list = document.getElementById('leaderboard-list');
    const empty = document.getElementById('leaderboard-empty');
    if (!list) return;
    list.innerHTML = '<li class="leaderboard-row leaderboard-row--loading">Loading…</li>';
    empty.hidden = true;
    try {
      const res = await fetch('/api/affiliate/leaderboard/public?period=' + encodeURIComponent(period));
      if (!res.ok) throw new Error('fetch failed');
      const data = await res.json();
      const items = data.items || [];
      if (!items.length) {
        list.innerHTML = '';
        empty.hidden = false;
        return;
      }
      list.innerHTML = items.map(function (it) {
        const amount = period === 'lifetime'
          ? it.lifetime_paid_cents : it.month_paid_cents;
        const tier = it.current_tier
          ? '<span class="lb-tier">' + escape(it.current_tier) + '</span>' : '';
        const avatar = it.public_avatar_url
          ? '<img class="lb-avatar" src="' + escape(it.public_avatar_url) + '" alt=""/>'
          : '<div class="lb-avatar lb-avatar--placeholder">' + escape((it.display_name || '?').charAt(0).toUpperCase()) + '</div>';
        return '<li class="leaderboard-row">'
          + '<span class="lb-rank">#' + it.rank + '</span>'
          + avatar
          + '<div class="lb-meta">'
            + '<div class="lb-name">' + escape(it.display_name) + '</div>'
            + tier
            + '<div class="lb-stat">' + (it.qualified_referrals || 0) + ' qualified referrals</div>'
          + '</div>'
          + '<div class="lb-amount">' + fmtEur(amount) + '</div>'
          + '</li>';
      }).join('');
    } catch (_) {
      list.innerHTML = '<li class="leaderboard-row leaderboard-row--error">Could not load leaderboard. Try refreshing.</li>';
    }
  }

  document.addEventListener('DOMContentLoaded', function () {
    document.body?.classList.remove('fouc-guard');
    const tabs = document.querySelectorAll('.leaderboard-tab');
    tabs.forEach(function (tab) {
      tab.addEventListener('click', function () {
        tabs.forEach(function (t) {
          t.classList.remove('active');
          t.setAttribute('aria-selected', 'false');
        });
        tab.classList.add('active');
        tab.setAttribute('aria-selected', 'true');
        load(tab.getAttribute('data-period'));
      });
    });
    load('month');
  });
})();
