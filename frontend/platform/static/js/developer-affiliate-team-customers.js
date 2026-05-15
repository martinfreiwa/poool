/* global window, document */

/** Customers sub-page — list customers acquired through team-business links. */
(function () {
  'use strict';

  const DAT = window.DAT;
  if (!DAT) return;

  async function loadCustomers() {
    const tbody = DAT.$('#dat-customers-tbody');
    if (!tbody) return;
    DAT.clear(tbody);
    tbody.appendChild(DAT.el('tr', null, DAT.el('td', { colspan: 6, class: 'dat-empty' }, 'Loading…')));

    const filter = DAT.$('#dat-cust-filter');
    const member = filter && filter.value ? `?attribution_user_id=${encodeURIComponent(filter.value)}` : '';
    try {
      const data = await DAT.apiGet(`/api/developer/affiliate/team/customers${member}`);
      const rows = data.rows || [];
      DAT.clear(tbody);
      if (!rows.length) {
        tbody.appendChild(DAT.el('tr', null, DAT.el('td', { colspan: 6, class: 'dat-empty' }, 'No customers yet via team-business links.')));
        return;
      }
      for (const c of rows) {
        const tr = DAT.el('tr');
        tr.appendChild(DAT.el('td', null, c.full_name || c.email || '—'));
        tr.appendChild(DAT.el('td', null, c.attribution_user_name || '—'));
        tr.appendChild(DAT.el('td', null, DAT.el('span', { class: `dat-status dat-status--${c.referral_status}` }, c.referral_status.replace(/_/g, ' '))));
        tr.appendChild(DAT.el('td', null, DAT.fmtCents(c.gross_invested_cents)));
        tr.appendChild(DAT.el('td', null, DAT.fmtCents(c.commission_earned_cents)));
        tr.appendChild(DAT.el('td', null, DAT.fmtDate(c.created_at)));
        tbody.appendChild(tr);
      }
    } catch (e) {
      DAT.clear(tbody);
      tbody.appendChild(DAT.el('tr', null, DAT.el('td', { colspan: 6, class: 'dat-empty' }, 'Failed to load customers.')));
      console.error(e);
    }
  }

  async function populateMemberFilter() {
    const filter = DAT.$('#dat-cust-filter');
    if (!filter) return;
    try {
      const data = await DAT.apiGet('/api/developer/affiliate/team/members');
      const members = (data.members || []).filter((m) => m.status === 'active');
      DAT.clear(filter);
      filter.appendChild(DAT.el('option', { value: '' }, 'All members'));
      for (const m of members) {
        filter.appendChild(DAT.el('option', { value: m.user_id }, m.full_name || m.email || m.user_id));
      }
    } catch (e) {
      console.error('member filter populate failed:', e);
    }
  }

  document.addEventListener('DOMContentLoaded', async () => {
    await populateMemberFilter();
    await loadCustomers();
    const filter = DAT.$('#dat-cust-filter');
    if (filter) filter.addEventListener('change', loadCustomers);
  });
})();
