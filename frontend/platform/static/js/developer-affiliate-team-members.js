/* global window, document */

/** Members sub-page — lists active + pending team members with approve/remove actions. */
(function () {
  'use strict';

  const DAT = window.DAT;
  if (!DAT) return;

  async function loadMembers() {
    const tbody = DAT.$('#dat-members-tbody');
    if (!tbody) return;
    DAT.clear(tbody);
    tbody.appendChild(DAT.el('tr', null, DAT.el('td', { colspan: 6, class: 'dat-empty' }, 'Loading…')));
    try {
      const data = await DAT.apiGet('/api/developer/affiliate/team/members');
      const rows = data.members || [];
      DAT.clear(tbody);
      if (!rows.length) {
        tbody.appendChild(DAT.el('tr', null, DAT.el('td', { colspan: 6, class: 'dat-empty' }, 'No members yet. Invite someone to get started.')));
        return;
      }
      for (const m of rows) {
        const tr = DAT.el('tr');
        tr.appendChild(DAT.el('td', null, m.full_name || '—'));
        tr.appendChild(DAT.el('td', null, m.email || '—'));
        tr.appendChild(DAT.el('td', null, DAT.el('span', { class: `dat-status dat-status--${m.status}` }, m.status.replace(/_/g, ' '))));
        tr.appendChild(DAT.el('td', null, m.link_code ? DAT.el('code', { class: 'dat-code' }, m.link_code) : '—'));
        tr.appendChild(DAT.el('td', null, DAT.fmtDate(m.joined_at || m.invited_at)));
        const actions = DAT.el('td', { class: 'dat-actions' });
        if (m.status === 'pending_developer_approval') {
          actions.appendChild(DAT.el('button', {
            class: 'ds-btn ds-btn--small ds-btn--primary',
            onclick: () => approveMember(m.membership_id),
          }, 'Approve'));
        }
        if (m.status !== 'removed') {
          actions.appendChild(DAT.el('button', {
            class: 'ds-btn ds-btn--small ds-btn--danger',
            onclick: () => removeMember(m.membership_id, m.full_name || m.email),
          }, 'Remove'));
        }
        tr.appendChild(actions);
        tbody.appendChild(tr);
      }
    } catch (e) {
      DAT.clear(tbody);
      tbody.appendChild(DAT.el('tr', null, DAT.el('td', { colspan: 6, class: 'dat-empty' }, 'Failed to load members.')));
      console.error(e);
    }
  }

  async function approveMember(id) {
    try {
      await DAT.apiPost(`/api/developer/affiliate/team/members/${id}/approve`);
      await DAT.loadTeamInfo();
      await loadMembers();
    } catch (e) {
      alert('Approve failed: ' + e.message);
    }
  }

  async function removeMember(id, label) {
    if (!confirm(`Remove ${label || 'this member'}? Business links will be deactivated.`)) return;
    try {
      await DAT.apiPost(`/api/developer/affiliate/team/members/${id}/remove`, { reason: 'removed_by_developer' });
      await DAT.loadTeamInfo();
      await loadMembers();
    } catch (e) {
      alert('Remove failed: ' + e.message);
    }
  }

  DAT.onInviteSent = loadMembers;

  document.addEventListener('DOMContentLoaded', loadMembers);
})();
