/* global window, document, fetch */

(function () {
  'use strict';

  // ─── DOM helpers ─────────────────────────────────────────────────────────
  const $ = (sel, root) => (root || document).querySelector(sel);
  const $$ = (sel, root) => Array.from((root || document).querySelectorAll(sel));

  function fmtCents(c) {
    if (c == null || isNaN(c)) return '—';
    const dollars = c / 100;
    return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 0 }).format(dollars);
  }
  function fmtDate(s) {
    if (!s) return '—';
    try { return new Date(s).toLocaleDateString(); } catch (e) { return s; }
  }
  function clear(el) { while (el && el.firstChild) el.removeChild(el.firstChild); }
  function el(tag, attrs, ...children) {
    const node = document.createElement(tag);
    if (attrs) {
      for (const [k, v] of Object.entries(attrs)) {
        if (v == null) continue;
        if (k === 'class') node.className = v;
        else if (k === 'dataset') Object.assign(node.dataset, v);
        else if (k.startsWith('on') && typeof v === 'function') node.addEventListener(k.slice(2), v);
        else node.setAttribute(k, v);
      }
    }
    for (const c of children) {
      if (c == null) continue;
      if (typeof c === 'string' || typeof c === 'number') node.appendChild(document.createTextNode(String(c)));
      else node.appendChild(c);
    }
    return node;
  }

  // ─── API helpers ─────────────────────────────────────────────────────────
  async function apiGet(path) {
    const res = await fetch(path, { credentials: 'same-origin', headers: { Accept: 'application/json' } });
    if (!res.ok) throw new Error(`GET ${path} failed: ${res.status}`);
    return res.json();
  }
  async function apiPost(path, body) {
    const headers = { 'Content-Type': 'application/json', Accept: 'application/json' };
    const res = await fetch(path, {
      method: 'POST',
      credentials: 'same-origin',
      headers,
      body: body ? JSON.stringify(body) : null,
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || `POST ${path} failed: ${res.status}`);
    return data;
  }
  async function apiPatch(path, body) {
    const headers = { 'Content-Type': 'application/json', Accept: 'application/json' };
    const res = await fetch(path, {
      method: 'PATCH',
      credentials: 'same-origin',
      headers,
      body: JSON.stringify(body || {}),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || `PATCH ${path} failed: ${res.status}`);
    return data;
  }

  // ─── Tile + Tab logic ────────────────────────────────────────────────────
  async function loadTeamInfo() {
    try {
      const data = await apiGet('/api/developer/affiliate/team');
      $('#dat-team-name').textContent = data.display_name || 'Team';
      $('#dat-team-meta').textContent = data.public_slug ? `Public slug: ${data.public_slug}` : 'No public slug set';
      $('#dat-tile-members').textContent = data.active_members != null ? String(data.active_members) : '0';
      const c = data.counters || {};
      $('#dat-tile-lifetime').textContent = fmtCents(c.lifetime_commission_cents || 0);
      $('#dat-tile-pending').textContent = fmtCents(c.pending_commission_cents || 0);
      $('#dat-tile-payable').textContent = fmtCents(c.payable_commission_cents || 0);
      $('#dat-tile-paid').textContent = fmtCents(c.paid_commission_cents || 0);
      // Pre-fill settings form
      const nameInp = $('#dat-team-display-name');
      const slugInp = $('#dat-public-slug');
      if (nameInp && data.display_name) nameInp.value = data.display_name;
      if (slugInp && data.public_slug) slugInp.value = data.public_slug;
    } catch (e) {
      $('#dat-team-name').textContent = 'Failed to load team';
      console.error(e);
    }
  }

  function switchSection(name) {
    $$('.dat-tab').forEach((b) => {
      const active = b.dataset.section === name;
      b.classList.toggle('dat-tab--active', active);
      b.setAttribute('aria-selected', String(active));
    });
    $$('.dat-section').forEach((s) => {
      const isMatch = s.id === `dat-section-${name}`;
      s.classList.toggle('dat-section--active', isMatch);
      s.hidden = !isMatch;
    });
    // Lazy-load section data
    if (name === 'customers' && !customersLoaded) loadCustomers();
    if (name === 'products' && !productsLoaded) loadProducts();
  }

  // ─── Members section ─────────────────────────────────────────────────────
  let membersByUser = new Map();
  async function loadMembers() {
    const tbody = $('#dat-members-tbody');
    clear(tbody);
    tbody.appendChild(el('tr', null, el('td', { colspan: 6, class: 'dat-empty' }, 'Loading…')));
    try {
      const data = await apiGet('/api/developer/affiliate/team/members');
      const rows = data.members || [];
      membersByUser = new Map(rows.filter((m) => m.status === 'active').map((m) => [m.user_id, m.full_name || m.email || m.user_id]));
      clear(tbody);
      if (!rows.length) {
        tbody.appendChild(el('tr', null, el('td', { colspan: 6, class: 'dat-empty' }, 'No members yet. Invite someone to get started.')));
        return;
      }
      for (const m of rows) {
        const tr = el('tr');
        tr.appendChild(el('td', null, m.full_name || '—'));
        tr.appendChild(el('td', null, m.email || '—'));
        tr.appendChild(el('td', null, el('span', { class: `dat-status dat-status--${m.status}` }, m.status.replace(/_/g, ' '))));
        tr.appendChild(el('td', null, m.link_code ? el('code', { class: 'dat-code' }, m.link_code) : '—'));
        tr.appendChild(el('td', null, fmtDate(m.joined_at || m.invited_at)));
        const actions = el('td', { class: 'dat-actions' });
        if (m.status === 'pending_developer_approval') {
          actions.appendChild(el('button', {
            class: 'ds-btn ds-btn--small ds-btn--primary',
            onclick: async () => approveMember(m.membership_id),
          }, 'Approve'));
        }
        if (m.status !== 'removed') {
          actions.appendChild(el('button', {
            class: 'ds-btn ds-btn--small ds-btn--danger',
            onclick: async () => removeMember(m.membership_id, m.full_name || m.email),
          }, 'Remove'));
        }
        tr.appendChild(actions);
        tbody.appendChild(tr);
      }
      // Populate customer filter dropdown
      const filter = $('#dat-cust-filter');
      if (filter) {
        const current = filter.value;
        clear(filter);
        filter.appendChild(el('option', { value: '' }, 'All members'));
        for (const [uid, name] of membersByUser.entries()) {
          filter.appendChild(el('option', { value: uid }, name));
        }
        filter.value = current;
      }
    } catch (e) {
      clear(tbody);
      tbody.appendChild(el('tr', null, el('td', { colspan: 6, class: 'dat-empty' }, 'Failed to load members.')));
      console.error(e);
    }
  }

  async function approveMember(id) {
    try {
      await apiPost(`/api/developer/affiliate/team/members/${id}/approve`);
      await loadMembers();
      await loadTeamInfo();
    } catch (e) {
      alert('Approve failed: ' + e.message);
    }
  }
  async function removeMember(id, label) {
    if (!confirm(`Remove ${label || 'this member'} from the team? Business-Links werden deaktiviert.`)) return;
    try {
      await apiPost(`/api/developer/affiliate/team/members/${id}/remove`, { reason: 'removed_by_developer' });
      await loadMembers();
      await loadTeamInfo();
    } catch (e) {
      alert('Remove failed: ' + e.message);
    }
  }

  // ─── Customers section ───────────────────────────────────────────────────
  let customersLoaded = false;
  async function loadCustomers() {
    const tbody = $('#dat-customers-tbody');
    clear(tbody);
    tbody.appendChild(el('tr', null, el('td', { colspan: 6, class: 'dat-empty' }, 'Loading…')));
    const filter = $('#dat-cust-filter');
    const member = filter && filter.value ? `?attribution_user_id=${encodeURIComponent(filter.value)}` : '';
    try {
      const data = await apiGet(`/api/developer/affiliate/team/customers${member}`);
      const rows = data.rows || [];
      clear(tbody);
      if (!rows.length) {
        tbody.appendChild(el('tr', null, el('td', { colspan: 6, class: 'dat-empty' }, 'No customers yet via team-business links.')));
        customersLoaded = true;
        return;
      }
      for (const c of rows) {
        const tr = el('tr');
        tr.appendChild(el('td', null, c.full_name || c.email || '—'));
        tr.appendChild(el('td', null, c.attribution_user_name || '—'));
        tr.appendChild(el('td', null, el('span', { class: `dat-status dat-status--${c.referral_status}` }, c.referral_status.replace(/_/g, ' '))));
        tr.appendChild(el('td', null, fmtCents(c.gross_invested_cents)));
        tr.appendChild(el('td', null, fmtCents(c.commission_earned_cents)));
        tr.appendChild(el('td', null, fmtDate(c.created_at)));
        tbody.appendChild(tr);
      }
      customersLoaded = true;
    } catch (e) {
      clear(tbody);
      tbody.appendChild(el('tr', null, el('td', { colspan: 6, class: 'dat-empty' }, 'Failed to load customers.')));
      console.error(e);
    }
  }

  // ─── Products section ────────────────────────────────────────────────────
  let productsLoaded = false;
  async function loadProducts() {
    const tbody = $('#dat-products-tbody');
    clear(tbody);
    tbody.appendChild(el('tr', null, el('td', { colspan: 4, class: 'dat-empty' }, 'Loading…')));
    try {
      const data = await apiGet('/api/developer/affiliate/team/products');
      const rows = data.rows || [];
      clear(tbody);
      if (!rows.length) {
        tbody.appendChild(el('tr', null, el('td', { colspan: 4, class: 'dat-empty' }, 'No product sales yet via team-business links.')));
        productsLoaded = true;
        return;
      }
      for (const p of rows) {
        const tr = el('tr');
        tr.appendChild(el('td', null, p.asset_name || p.asset_id));
        tr.appendChild(el('td', null, String(p.units_sold || 0)));
        tr.appendChild(el('td', null, fmtCents(p.gross_revenue_cents)));
        tr.appendChild(el('td', null, fmtCents(p.commission_cents)));
        tbody.appendChild(tr);
      }
      productsLoaded = true;
    } catch (e) {
      clear(tbody);
      tbody.appendChild(el('tr', null, el('td', { colspan: 4, class: 'dat-empty' }, 'Failed to load products.')));
      console.error(e);
    }
  }

  // ─── Invite modal ────────────────────────────────────────────────────────
  function openInviteModal() {
    const modal = $('#dat-invite-modal');
    if (!modal) return;
    modal.hidden = false;
    $('#dat-invite-preview').hidden = true;
    $('#dat-invite-email').value = '';
    setTimeout(() => $('#dat-invite-email').focus(), 50);
  }
  function closeInviteModal() {
    const modal = $('#dat-invite-modal');
    if (modal) modal.hidden = true;
  }
  async function submitInvite(e) {
    e.preventDefault();
    const email = $('#dat-invite-email').value.trim();
    if (!email) return;
    try {
      const data = await apiPost('/api/developer/affiliate/team/invite', { email });
      const preview = $('#dat-invite-preview');
      if (data.preview_token) {
        $('#dat-invite-token').textContent = data.preview_token;
        preview.hidden = false;
      } else {
        closeInviteModal();
      }
      await loadMembers();
    } catch (e) {
      alert('Invitation failed: ' + e.message);
    }
  }

  // ─── Settings save ───────────────────────────────────────────────────────
  function bindSettingsForm() {
    const form = $('#dat-settings-form');
    if (!form) return;
    form.addEventListener('submit', async (e) => {
      e.preventDefault();
      const name = $('#dat-team-display-name').value.trim();
      const slug = $('#dat-public-slug').value.trim();
      try {
        await apiPatch('/api/developer/affiliate/team', {
          display_name: name || null,
          public_slug: slug, // empty string = clear
        });
        await loadTeamInfo();
        const actions = form.querySelector('.dat-form-actions');
        if (actions) {
          const ok = document.createElement('span');
          ok.textContent = 'Saved ✓';
          ok.style.cssText = 'color:#137333;font-size:13px;align-self:center;margin-right:8px;';
          actions.insertBefore(ok, actions.firstChild);
          setTimeout(() => ok.remove(), 2500);
        }
      } catch (err) {
        alert('Save failed: ' + err.message);
      }
    });
  }

  // ─── Init ────────────────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', () => {
    // Active sidebar nav highlight
    const nav = document.getElementById('nav-item-affiliate-team');
    if (nav) nav.classList.add('sidebar__nav-item--active');

    // Bind tab buttons
    $$('.dat-tab').forEach((b) => b.addEventListener('click', () => switchSection(b.dataset.section)));

    // Invite modal triggers
    $('#dat-invite-btn').addEventListener('click', openInviteModal);
    $$('[data-close="invite"]').forEach((el) => el.addEventListener('click', closeInviteModal));
    $('#dat-invite-form').addEventListener('submit', submitInvite);

    // Customer filter
    const filter = $('#dat-cust-filter');
    if (filter) filter.addEventListener('change', () => { customersLoaded = false; loadCustomers(); });

    bindSettingsForm();

    // Initial load
    loadTeamInfo();
    loadMembers();
  });
})();
