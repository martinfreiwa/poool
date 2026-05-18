/* global window, document */

/** Members sub-page — team-member roster with sort/search/pagination.
 *  Built on `DAT.dataTable` (developer-affiliate-team-shell.js).
 */
(function () {
  'use strict';

  const DAT = window.DAT;
  if (!DAT) return;

  let _lastRows = [];

  // Universal badge system (.ubadge) — map domain statuses to semantic variants.
  const UBADGE_BY_STATUS = {
    active: 'success',
    invited: 'info',
    pending_developer_approval: 'warning',
    removed: 'danger',
    qualified: 'success',
    paid: 'success',
    under_holdback: 'info',
    disqualified: 'danger',
    expired: 'neutral',
  };
  function statusPill(status) {
    const variant = UBADGE_BY_STATUS[status] || 'neutral';
    return DAT.el(
      'span',
      { class: `ubadge ubadge--sm ubadge--${variant}`, title: DAT.humanize(status) },
      DAT.humanize(status),
    );
  }

  function memberCell(row) {
    const wrap = DAT.el('div', { class: 'dat-cell-stack' });
    wrap.appendChild(DAT.el('span', { class: 'dat-cell-strong' }, row.full_name || '—'));
    if (row.email) {
      wrap.appendChild(DAT.el('span', { class: 'dat-cell-sub' }, row.email));
    }
    return wrap;
  }

  // Inline SVG factory — keeps JS self-contained.
  function svg(paths, attrs) {
    const a = Object.assign({
      width: '16', height: '16', viewBox: '0 0 24 24', fill: 'none',
      stroke: 'currentColor', 'stroke-width': '2',
      'stroke-linecap': 'round', 'stroke-linejoin': 'round',
      'aria-hidden': 'true',
    }, attrs || {});
    const xmlns = 'http://www.w3.org/2000/svg';
    const el = document.createElementNS(xmlns, 'svg');
    Object.entries(a).forEach(([k, v]) => el.setAttribute(k, v));
    el.innerHTML = paths;
    return el;
  }

  function actionsCell(row) {
    const wrap = DAT.el('div', { class: 'dat-actions' });
    const label = row.full_name || row.email || 'this member';
    if (row.status === 'pending_developer_approval') {
      const btn = DAT.el('button', {
        type: 'button',
        class: 'dat-icon-btn dat-icon-btn--approve',
        'aria-label': `Approve ${label}`,
        title: `Approve ${label}`,
        onclick: () => approve(row.membership_id, label),
      });
      btn.appendChild(svg('<polyline points="20 6 9 17 4 12"/>'));
      wrap.appendChild(btn);
    }
    // Phase-1: resend-invitation button on invited rows.
    if (row.status === 'invited') {
      wrap.appendChild(DAT.el('button', {
        type: 'button',
        class: 'ds-btn ds-btn--sm ds-btn--secondary',
        'aria-label': `Resend invitation to ${label}`,
        title: `Resend invitation to ${label}`,
        onclick: () => resend(row.membership_id, label),
      }, 'Resend'));
    }
    if (row.status !== 'removed') {
      const btn = DAT.el('button', {
        type: 'button',
        class: 'dat-icon-btn dat-icon-btn--danger',
        'aria-label': `Remove ${label}`,
        title: `Remove ${label}`,
        onclick: () => remove(row.membership_id, label),
      });
      btn.appendChild(svg(
        '<polyline points="3 6 5 6 21 6"/>' +
        '<path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6"/>' +
        '<path d="M10 11v6"/>' +
        '<path d="M14 11v6"/>' +
        '<path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2"/>'
      ));
      wrap.appendChild(btn);
    }
    return wrap;
  }

  async function resend(id, label) {
    try {
      await DAT.apiPost(`/api/developer/affiliate/team/members/${id}/resend-invitation`);
      DAT.toast('Invitation resent', `A fresh invitation email is on its way to ${label}.`, 'success');
      _table?.reload();
    } catch (e) {
      DAT.toast('Resend failed', e.message || 'Could not resend invitation.', 'error');
    }
  }

  async function approve(id, label) {
    const ok = await DAT.confirm({
      title: `Approve ${label}?`,
      message: 'They will get an active business affiliate link and can start earning commissions immediately.',
      confirmText: 'Approve member',
      cancelText: 'Keep pending',
    });
    if (!ok) return;
    try {
      await DAT.apiPost(`/api/developer/affiliate/team/members/${id}/approve`);
      await DAT.loadTeamInfo();
      DAT.toast('Member approved', `${label} is now active on your team.`, 'success');
      _table?.reload();
    } catch (e) {
      DAT.toast('Approve failed', e.message || 'Could not approve.', 'error');
    }
  }

  async function remove(id, label) {
    const ok = await DAT.confirm({
      title: `Remove ${label}?`,
      message: 'Their business affiliate link will be deactivated. Existing customers stay attributed.',
      confirmText: 'Remove member',
      cancelText: 'Keep member',
      danger: true,
    });
    if (!ok) return;
    try {
      await DAT.apiPost(`/api/developer/affiliate/team/members/${id}/remove`, { reason: 'removed_by_developer' });
      await DAT.loadTeamInfo();
      DAT.toast('Member removed', `${label} has been removed.`, 'success');
      _table?.reload();
    } catch (e) {
      DAT.toast('Remove failed', e.message || 'Could not remove.', 'error');
    }
  }

  function exportCsv() {
    if (!_lastRows.length) {
      DAT.toast('Nothing to export', 'No members in the current view.', 'info');
      return;
    }
    const header = ['Member', 'Email', 'Status', 'Joined', 'Customers', 'Commission (EUR)', 'Last sale', 'Business link'];
    const body = _lastRows.map((m) => [
      m.full_name || '',
      m.email || '',
      DAT.humanize(m.status),
      m.joined_at || m.invited_at || '',
      m.customer_count,
      ((m.commission_cents || 0) / 100).toFixed(2),
      m.last_sale_at || '',
      m.link_code || '',
    ]);
    const stamp = new Date().toISOString().slice(0, 10);
    DAT.downloadCsv(`team-members-${stamp}.csv`, [header, ...body]);
  }

  let _table = null;

  document.addEventListener('DOMContentLoaded', () => {
    const tbody = DAT.$('#dat-members-tbody');
    const theadRow = DAT.$('#dat-members-thead-row');
    const pagerHost = DAT.$('#dat-members-pager-host');
    const pagerFooterHost = DAT.$('#dat-members-pager-footer');
    const searchHost = DAT.$('#dat-members-search-host');
    if (!tbody || !theadRow || !pagerHost) return;

    _table = DAT.dataTable({
      pageKey: 'members',
      endpoint: '/api/developer/affiliate/team/members',
      tbody, theadRow, pagerHost, pagerFooterHost, searchHost,
      extraParams: () => ({}),
      emptyText: 'No team members match your filter. Invite someone via the button above.',
      columns: [
        { key: 'full_name', label: 'Member', sortable: true, render: memberCell },
        { key: 'status', label: 'Status', sortable: true, render: (r) => statusPill(r.status) },
        { key: 'joined_at', label: 'Joined', sortable: true, defaultDir: 'desc',
          render: (r) => DAT.fmtDate(r.joined_at || r.invited_at) },
        { key: 'customers', label: 'Customers', sortable: true, numeric: true, defaultDir: 'desc',
          render: (r) => (r.customer_count || 0).toLocaleString() },
        { key: 'commission', label: 'Commission', sortable: true, numeric: true, defaultDir: 'desc',
          render: (r) => DAT.fmtCents(r.commission_cents) },
        { key: 'last_sale', label: 'Last sale', sortable: true, defaultDir: 'desc',
          render: (r) => r.last_sale_at ? DAT.fmtDate(r.last_sale_at) : DAT.el('span', { class: 'dat-muted' }, '—') },
        { key: 'link', label: 'Business link', sortable: false,
          render: (r) => r.link_code
            ? DAT.el('code', { class: 'dat-code', title: r.link_code }, r.link_code)
            : DAT.el('span', { class: 'dat-muted', title: 'Generated after acceptance' }, 'Not generated') },
        { key: 'actions', label: 'Actions', sortable: false, render: actionsCell },
      ],
      onRowsLoaded: (rows) => { _lastRows = rows; },
    });

    const exp = DAT.$('#dat-members-export');
    if (exp) exp.addEventListener('click', exportCsv);
  });

  DAT.onInviteSent = () => _table?.reload();
})();
