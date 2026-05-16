/* global window, document */

/** Members sub-page — team-member roster with sort/search/pagination.
 *  Built on `DAT.dataTable` (developer-affiliate-team-shell.js).
 */
(function () {
  'use strict';

  const DAT = window.DAT;
  if (!DAT) return;

  let _lastRows = [];

  function statusPill(status) {
    return DAT.el(
      'span',
      { class: `dat-status dat-status--${status}`, title: DAT.humanize(status) },
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

  function actionsCell(row) {
    const wrap = DAT.el('div', { class: 'dat-actions' });
    const label = row.full_name || row.email || 'this member';
    if (row.status === 'pending_developer_approval') {
      wrap.appendChild(DAT.el('button', {
        type: 'button',
        class: 'ds-btn ds-btn--sm ds-btn--primary',
        'aria-label': `Approve ${label}`,
        title: `Approve ${label}`,
        onclick: () => approve(row.membership_id, label),
      }, 'Approve'));
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
      wrap.appendChild(DAT.el('button', {
        type: 'button',
        class: 'ds-btn ds-btn--sm ds-btn--danger',
        'aria-label': `Remove ${label}`,
        title: `Remove ${label}`,
        onclick: () => remove(row.membership_id, label),
      }, 'Remove'));
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
    const chipsHost = DAT.$('#dat-members-chips');
    if (!tbody || !theadRow || !pagerHost) return;

    let chipBar = null;
    if (chipsHost) {
      chipBar = DAT.chipBar({
        host: chipsHost,
        pageKey: 'members',
        chips: [
          { value: 'active', label: 'Active' },
          { value: 'invited', label: 'Invited' },
          { value: 'pending_developer_approval', label: 'Pending approval' },
          { value: 'removed', label: 'Removed' },
        ],
        onChange: () => _table?.reload(),
      });
    }

    _table = DAT.dataTable({
      pageKey: 'members',
      endpoint: '/api/developer/affiliate/team/members',
      tbody, theadRow, pagerHost,
      extraParams: () => ({ status: chipBar ? chipBar.value() : '' }),
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
