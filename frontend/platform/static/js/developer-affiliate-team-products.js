/* global window, document */

/** Products sub-page — aggregate of assets sold via team-business links. */
(function () {
  'use strict';

  const DAT = window.DAT;
  if (!DAT) return;

  async function loadProducts() {
    const tbody = DAT.$('#dat-products-tbody');
    if (!tbody) return;
    DAT.clear(tbody);
    tbody.appendChild(DAT.el('tr', null, DAT.el('td', { colspan: 4, class: 'dat-empty' }, 'Loading…')));
    try {
      const data = await DAT.apiGet('/api/developer/affiliate/team/products');
      const rows = data.rows || [];
      DAT.clear(tbody);
      if (!rows.length) {
        tbody.appendChild(DAT.el('tr', null, DAT.el('td', { colspan: 4, class: 'dat-empty' }, 'No product sales yet via team-business links.')));
        return;
      }
      for (const p of rows) {
        const tr = DAT.el('tr');
        tr.appendChild(DAT.el('td', null, p.asset_name || p.asset_id));
        tr.appendChild(DAT.el('td', null, String(p.units_sold || 0)));
        tr.appendChild(DAT.el('td', null, DAT.fmtCents(p.gross_revenue_cents)));
        tr.appendChild(DAT.el('td', null, DAT.fmtCents(p.commission_cents)));
        tbody.appendChild(tr);
      }
    } catch (e) {
      DAT.clear(tbody);
      tbody.appendChild(DAT.el('tr', null, DAT.el('td', { colspan: 4, class: 'dat-empty' }, 'Failed to load products.')));
      console.error(e);
    }
  }

  document.addEventListener('DOMContentLoaded', loadProducts);
})();
