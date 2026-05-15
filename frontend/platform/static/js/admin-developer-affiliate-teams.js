/* global window, document, fetch */

(function () {
  'use strict';

  const $ = (sel, root) => (root || document).querySelector(sel);
  const $$ = (sel, root) => Array.from((root || document).querySelectorAll(sel));

  function fmtCents(c) {
    if (c == null || isNaN(c)) return '—';
    return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 }).format(c / 100);
  }
  function fmtDate(s) {
    if (!s) return '—';
    try { return new Date(s).toLocaleDateString(); } catch (_) { return s; }
  }
  function clear(el) { while (el && el.firstChild) el.removeChild(el.firstChild); }
  function el(tag, attrs, ...children) {
    const n = document.createElement(tag);
    if (attrs) {
      for (const [k, v] of Object.entries(attrs)) {
        if (v == null) continue;
        if (k === 'class') n.className = v;
        else if (k === 'dataset') Object.assign(n.dataset, v);
        else if (k.startsWith('on') && typeof v === 'function') n.addEventListener(k.slice(2), v);
        else n.setAttribute(k, v);
      }
    }
    for (const c of children) {
      if (c == null) continue;
      if (typeof c === 'string' || typeof c === 'number') n.appendChild(document.createTextNode(String(c)));
      else n.appendChild(c);
    }
    return n;
  }

  async function apiGet(p) {
    const r = await fetch(p, { credentials: 'same-origin', headers: { Accept: 'application/json' } });
    if (!r.ok) throw new Error(`GET ${p} ${r.status}`);
    return r.json();
  }
  async function apiPost(p, body) {
    const r = await fetch(p, {
      method: 'POST', credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: body ? JSON.stringify(body) : null,
    });
    const d = await r.json().catch(() => ({}));
    if (!r.ok) throw new Error(d.error || `POST ${p} ${r.status}`);
    return d;
  }

  let allTeams = [];

  async function loadTeams() {
    const tbody = $('#adat-teams-tbody');
    clear(tbody);
    tbody.appendChild(el('tr', null, el('td', { colspan: 9, class: 'adat-empty' }, 'Loading…')));
    try {
      const data = await apiGet('/api/admin/affiliate-teams');
      allTeams = data.teams || [];
      renderTeams();
    } catch (e) {
      clear(tbody);
      tbody.appendChild(el('tr', null, el('td', { colspan: 9, class: 'adat-empty' }, 'Failed to load teams.')));
      console.error(e);
    }
  }

  function renderTeams() {
    const tbody = $('#adat-teams-tbody');
    const filter = $('#adat-status-filter').value;
    clear(tbody);

    const filtered = allTeams.filter((t) => filter === 'all' || t.status === filter);

    // KPI summary
    $('#adat-kpi-count').textContent = String(filtered.length);
    $('#adat-kpi-members').textContent = String(filtered.reduce((s, t) => s + (t.active_members || 0), 0));
    $('#adat-kpi-lifetime').textContent = fmtCents(filtered.reduce((s, t) => s + (t.lifetime_commission_cents || 0), 0));

    if (!filtered.length) {
      tbody.appendChild(el('tr', null, el('td', { colspan: 9, class: 'adat-empty' }, 'No teams match this filter.')));
      return;
    }

    for (const t of filtered) {
      const tr = el('tr');
      tr.appendChild(el('td', null,
        el('strong', null, t.display_name || '—'),
        t.public_slug ? el('div', { class: 'adat-slug' }, `slug: ${t.public_slug}`) : null
      ));
      tr.appendChild(el('td', null, t.developer_email || t.developer_user_id));
      tr.appendChild(el('td', null, el('span', { class: `adat-status adat-status--${t.status}` }, t.status)));
      tr.appendChild(el('td', null, String(t.active_members || 0)));
      tr.appendChild(el('td', null, String(t.pending_members || 0)));
      tr.appendChild(el('td', null, fmtCents(t.payable_commission_cents)));
      tr.appendChild(el('td', null, fmtCents(t.lifetime_commission_cents)));
      tr.appendChild(el('td', null, fmtDate(t.created_at)));
      const actions = el('td', { class: 'adat-actions' });
      actions.appendChild(el('button', {
        class: 'admin-btn admin-btn--ghost adat-btn-small',
        onclick: () => openDetail(t.team_id),
      }, 'View'));
      if (t.status === 'active') {
        actions.appendChild(el('button', {
          class: 'admin-btn admin-btn--warning adat-btn-small',
          onclick: () => statusChange(t.team_id, 'suspend'),
        }, 'Suspend'));
      } else if (t.status === 'paused') {
        actions.appendChild(el('button', {
          class: 'admin-btn admin-btn--success adat-btn-small',
          onclick: () => statusChange(t.team_id, 'resume'),
        }, 'Resume'));
      }
      if (t.status !== 'terminated') {
        actions.appendChild(el('button', {
          class: 'admin-btn admin-btn--danger adat-btn-small',
          onclick: () => statusChange(t.team_id, 'terminate'),
        }, 'Terminate'));
      }
      tr.appendChild(actions);
      tbody.appendChild(tr);
    }
  }

  async function statusChange(teamId, action) {
    const verb = { suspend: 'suspend', resume: 'resume', terminate: 'terminate' }[action];
    let reason = '';
    if (action !== 'resume') {
      reason = window.prompt(`Reason for ${verb}?`) || '';
      if (!reason.trim() && action === 'terminate') {
        alert('Reason required for terminate.');
        return;
      }
    }
    try {
      await apiPost(`/api/admin/affiliate-teams/${teamId}/${action}`, { reason: reason || null });
      await loadTeams();
      if ($('#adat-drawer').hidden === false) openDetail(teamId);
    } catch (e) {
      alert(`${verb} failed: ${e.message}`);
    }
  }

  async function openDetail(teamId) {
    $('#adat-drawer').hidden = false;
    $('#adat-drawer-backdrop').hidden = false;
    const body = $('#adat-drawer-body');
    clear(body);
    body.appendChild(el('p', null, 'Loading…'));
    try {
      const data = await apiGet(`/api/admin/affiliate-teams/${teamId}`);
      renderDetail(data);
    } catch (e) {
      clear(body);
      body.appendChild(el('p', null, 'Failed to load detail.'));
      console.error(e);
    }
  }

  function renderDetail(data) {
    const team = data.team || {};
    const counters = data.counters || {};
    const members = data.members || [];
    const audit = data.audit || [];

    $('#adat-drawer-title').textContent = team.display_name || 'Team';
    const body = $('#adat-drawer-body');
    clear(body);

    body.appendChild(el('section', { class: 'adat-detail-section' },
      el('h4', null, 'Team'),
      el('dl', { class: 'adat-dl' },
        el('dt', null, 'Developer:'), el('dd', null, team.developer_email || team.developer_user_id),
        el('dt', null, 'Status:'), el('dd', null, el('span', { class: `adat-status adat-status--${team.status}` }, team.status)),
        el('dt', null, 'Slug:'), el('dd', null, team.public_slug || '—'),
        el('dt', null, 'Created:'), el('dd', null, fmtDate(team.created_at)),
        team.terminated_at ? el('dt', null, 'Terminated:') : null,
        team.terminated_at ? el('dd', null, `${fmtDate(team.terminated_at)} — ${team.terminated_reason || ''}`) : null,
      )
    ));

    // Counters
    body.appendChild(el('section', { class: 'adat-detail-section' },
      el('h4', null, 'Counters'),
      el('dl', { class: 'adat-dl' },
        el('dt', null, 'Lifetime commission:'), el('dd', null, fmtCents(counters.lifetime_commission_cents)),
        el('dt', null, 'Pending:'), el('dd', null, fmtCents(counters.pending_commission_cents)),
        el('dt', null, 'Payable:'), el('dd', null, fmtCents(counters.payable_commission_cents)),
        el('dt', null, 'Paid out:'), el('dd', null, fmtCents(counters.paid_commission_cents)),
        el('dt', null, 'Clawed back:'), el('dd', null, fmtCents(counters.clawed_back_cents)),
      )
    ));

    // Members
    const memberTable = el('table', { class: 'adat-inline-table' });
    memberTable.appendChild(el('thead', null, el('tr', null,
      el('th', null, 'Member'), el('th', null, 'Status'), el('th', null, 'Link'), el('th', null, 'Joined'), el('th', null, 'Action'),
    )));
    const memberTbody = el('tbody');
    if (!members.length) {
      memberTbody.appendChild(el('tr', null, el('td', { colspan: 5, class: 'adat-empty' }, 'No members.')));
    } else {
      for (const m of members) {
        const tr = el('tr');
        tr.appendChild(el('td', null, m.full_name || m.email || '—'));
        tr.appendChild(el('td', null, el('span', { class: `adat-status adat-status--${m.status}` }, m.status.replace(/_/g, ' '))));
        tr.appendChild(el('td', null, m.link_code ? el('code', null, m.link_code) : '—'));
        tr.appendChild(el('td', null, fmtDate(m.joined_at || m.invited_at)));
        const act = el('td');
        if (m.status !== 'removed') {
          act.appendChild(el('button', {
            class: 'admin-btn admin-btn--ghost adat-btn-small',
            onclick: () => adminMoveMember(m.membership_id, team.team_id),
          }, 'Move'));
          act.appendChild(el('button', {
            class: 'admin-btn admin-btn--danger adat-btn-small',
            onclick: () => adminRemoveMember(m.membership_id, team.team_id),
          }, 'Remove'));
        }
        tr.appendChild(act);
        memberTbody.appendChild(tr);
      }
    }
    memberTable.appendChild(memberTbody);

    body.appendChild(el('section', { class: 'adat-detail-section' },
      el('h4', null, `Members (${members.length})`),
      memberTable,
    ));

    // Audit
    const auditList = el('ul', { class: 'adat-audit' });
    if (!audit.length) {
      auditList.appendChild(el('li', { class: 'adat-empty' }, 'No audit entries.'));
    } else {
      for (const a of audit) {
        auditList.appendChild(el('li', null,
          el('time', null, fmtDate(a.created_at)),
          ' ',
          el('strong', null, a.action),
          a.metadata && Object.keys(a.metadata).length ? el('pre', { class: 'adat-audit-meta' }, JSON.stringify(a.metadata, null, 2)) : null,
        ));
      }
    }
    body.appendChild(el('section', { class: 'adat-detail-section' },
      el('h4', null, 'Audit Trail (last 50)'),
      auditList,
    ));
  }

  async function adminMoveMember(membershipId, currentTeamId) {
    // Build a select-list of all OTHER teams currently in `allTeams`.
    const targets = allTeams.filter((t) => t.team_id !== currentTeamId && t.status === 'active');
    if (!targets.length) {
      alert('No other active teams to move to.');
      return;
    }
    const promptText =
      'Move member to which team? Enter the number:\n\n' +
      targets.map((t, i) => `  ${i + 1}. ${t.display_name} (${t.developer_email || t.developer_user_id})`).join('\n');
    const choice = window.prompt(promptText);
    if (!choice) return;
    const idx = parseInt(choice, 10) - 1;
    if (isNaN(idx) || idx < 0 || idx >= targets.length) {
      alert('Invalid selection.');
      return;
    }
    const reason = window.prompt('Reason for move (required):');
    if (!reason || !reason.trim()) return;
    try {
      await apiPost(`/api/admin/affiliate-teams/members/${membershipId}/move`, {
        target_team_id: targets[idx].team_id,
        reason,
      });
      await loadTeams();
      openDetail(targets[idx].team_id);
    } catch (e) {
      alert('Move failed: ' + e.message);
    }
  }

  async function adminRemoveMember(membershipId, teamId) {
    const reason = window.prompt('Admin-Override remove. Reason:');
    if (!reason || !reason.trim()) return;
    try {
      await apiPost(`/api/admin/affiliate-teams/members/${membershipId}/remove`, { reason });
      await loadTeams();
      openDetail(teamId);
    } catch (e) {
      alert('Remove failed: ' + e.message);
    }
  }

  function closeDrawer() {
    $('#adat-drawer').hidden = true;
    $('#adat-drawer-backdrop').hidden = true;
  }

  document.addEventListener('DOMContentLoaded', () => {
    $('#adat-status-filter').addEventListener('change', renderTeams);
    $$('[data-close-drawer]').forEach((el) => el.addEventListener('click', closeDrawer));
    loadTeams();
  });
})();
