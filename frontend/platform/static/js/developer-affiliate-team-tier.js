/* global window, document */

/** Tier sub-page — current team tier, full ladder, progress to next,
 *  personal-vs-team comparison, and tier-change history.
 *  Endpoint: GET /api/developer/affiliate/team/tier
 */
(function () {
  'use strict';

  const DAT = window.DAT;
  if (!DAT) return;

  function fmtBps(bps) {
    if (bps == null) return '—';
    return (bps / 100).toFixed(2) + '%';
  }
  function fmtBpsRaw(bps) {
    if (bps == null) return '—';
    return bps + ' bps';
  }

  function renderHero(data) {
    DAT.$('#dat-tier-name').textContent = data.current_tier || '—';
    DAT.$('#dat-tier-rate').textContent = fmtBps(data.current_rate_bps);
    DAT.$('#dat-tier-rate-suffix').textContent = ' · ' + fmtBpsRaw(data.current_rate_bps);
    DAT.$('#dat-tier-volume').textContent = DAT.fmtCents(data.volume_12m_cents);
    if (data.next_tier) {
      DAT.$('#dat-tier-next').textContent = data.next_tier;
      const remaining = (data.next_threshold_cents || 0) - (data.volume_12m_cents || 0);
      DAT.$('#dat-tier-next-hint').textContent =
        DAT.fmtCents(Math.max(0, remaining)) + ' to go (' +
        DAT.fmtCents(data.next_threshold_cents) + ' threshold)';
    } else {
      DAT.$('#dat-tier-next').textContent = 'Max tier ✓';
      DAT.$('#dat-tier-next-hint').textContent = 'You are at the top of the ladder.';
    }

    const pct = Math.round(data.progress_pct || 0);
    DAT.$('#dat-tier-progress-fill').style.width = pct + '%';
    DAT.$('#dat-tier-progress-pct').textContent = pct + '%';
    DAT.$('#dat-tier-progress-from').textContent = data.current_tier || '—';
    DAT.$('#dat-tier-progress-to').textContent = data.next_tier || 'Max';

    // Updated-at pill
    const pill = DAT.$('#dat-tier-updated');
    if (pill && data.tier_updated_at) {
      const d = new Date(data.tier_updated_at);
      pill.textContent = 'Updated ' + d.toLocaleString();
      pill.className = 'dat-status dat-status--active';
    }
  }

  function renderLadder(data) {
    const tbody = DAT.$('#dat-ladder-tbody');
    DAT.clear(tbody);
    const currentMin = (data.ladder.find(t => t.is_current) || {}).min_volume_cents || 0;
    for (const t of data.ladder) {
      const tr = DAT.el('tr');
      const isCurrent = t.is_current;
      const reached = (data.volume_12m_cents || 0) >= t.min_volume_cents;
      const nextTarget = !reached && (!data.ladder.find(x => !x.is_current && (data.volume_12m_cents || 0) >= x.min_volume_cents && x.min_volume_cents > currentMin));
      tr.appendChild(DAT.el(
        'td', null,
        DAT.el('span', { class: 'dat-cell-strong' }, t.name),
      ));
      tr.appendChild(DAT.el('td', { class: 'dat-td--num' }, fmtBps(t.commission_rate_bps)));
      tr.appendChild(DAT.el('td', { class: 'dat-td--num' }, DAT.fmtCents(t.min_volume_cents)));
      const status =
        isCurrent ? DAT.el('span', { class: 'dat-status dat-status--active', title: 'Your current tier' }, 'Current')
        : reached  ? DAT.el('span', { class: 'dat-status dat-status--qualified', title: 'You have surpassed this tier' }, 'Reached')
        : DAT.el('span', { class: 'dat-status dat-status--muted', title: 'Volume needed to unlock' }, 'Locked');
      tr.appendChild(DAT.el('td', null, status));
      if (isCurrent) tr.classList.add('dat-ladder-row--current');
      tbody.appendChild(tr);
    }
  }

  function renderCompare(data) {
    DAT.$('#dat-cmp-personal-tier').textContent = 'Personal';
    DAT.$('#dat-cmp-personal-rate').textContent =
      fmtBps(data.developer_personal_rate_bps) + ' · ' + fmtBpsRaw(data.developer_personal_rate_bps);
    DAT.$('#dat-cmp-team-tier').textContent = 'Team';
    DAT.$('#dat-cmp-team-rate').textContent =
      fmtBps(data.current_rate_bps) + ' · ' + fmtBpsRaw(data.current_rate_bps);
  }

  function renderHistory(data) {
    const tbody = DAT.$('#dat-history-tbody');
    if (!tbody) return; // History card removed from the tier page.
    DAT.clear(tbody);
    if (!data.history || data.history.length === 0) {
      tbody.appendChild(DAT.el(
        'tr', null,
        DAT.el('td', { colspan: 4, class: 'dat-empty dat-empty--cta' },
          DAT.el('strong', { class: 'dat-empty__title' }, 'No tier changes yet'),
          DAT.el('p', { class: 'dat-empty__msg' }, 'When your team crosses the next volume threshold, the promotion will appear here.'),
        ),
      ));
      return;
    }
    for (const h of data.history) {
      const tr = DAT.el('tr');
      tr.appendChild(DAT.el('td', null, h.old_tier || '—'));
      tr.appendChild(DAT.el(
        'td', null,
        DAT.el('span', { class: 'dat-cell-strong' }, h.new_tier),
        ' (' + fmtBps(h.new_bps) + ')',
      ));
      tr.appendChild(DAT.el('td', { class: 'dat-td--num' }, DAT.fmtCents(h.volume_cents)));
      tr.appendChild(DAT.el('td', null, DAT.fmtDate(h.changed_at)));
      tbody.appendChild(tr);
    }
  }

  async function load() {
    try {
      const data = await DAT.apiGet('/api/developer/affiliate/team/tier');
      renderHero(data);
      renderLadder(data);
      renderCompare(data);
      renderHistory(data);
    } catch (e) {
      DAT.toast('Tier', 'Failed to load tier information. Please refresh.', 'error');
      console.error(e);
    }
  }

  document.addEventListener('DOMContentLoaded', load);
})();
