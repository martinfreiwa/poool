/* global window, document, fetch */

/**
 * Shared shell for /developer/affiliate-team/* sub-pages.
 *
 * Loads on every page; handles:
 *   - KPI tiles (live counters from /api/developer/affiliate/team)
 *   - Page-header title + sub-nav active state
 *   - Invite modal + POST
 *
 * Per-page modules (developer-affiliate-team-{members,customers,products,settings}.js)
 * handle their own section data fetching.
 */

(function () {
  'use strict';

  // ─── Common DOM helpers exposed to per-page modules ───────────────────
  const DAT = (window.DAT = window.DAT || {});

  DAT.$ = (sel, root) => (root || document).querySelector(sel);
  DAT.$$ = (sel, root) => Array.from((root || document).querySelectorAll(sel));

  DAT.fmtCents = function (c) {
    if (c == null || isNaN(c)) return '—';
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      maximumFractionDigits: 0,
    }).format(c / 100);
  };

  DAT.fmtDate = function (s) {
    if (!s) return '—';
    try {
      return new Date(s).toLocaleDateString();
    } catch (_) {
      return s;
    }
  };

  DAT.clear = function (el) {
    while (el && el.firstChild) el.removeChild(el.firstChild);
  };

  DAT.el = function (tag, attrs, ...children) {
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
  };

  DAT.apiGet = async function (path) {
    const res = await fetch(path, {
      credentials: 'same-origin',
      headers: { Accept: 'application/json' },
    });
    if (!res.ok) throw new Error(`GET ${path} failed: ${res.status}`);
    return res.json();
  };

  DAT.apiPost = async function (path, body) {
    const res = await fetch(path, {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: body ? JSON.stringify(body) : null,
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || `POST ${path} failed: ${res.status}`);
    return data;
  };

  DAT.apiPatch = async function (path, body) {
    const res = await fetch(path, {
      method: 'PATCH',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: JSON.stringify(body || {}),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || `PATCH ${path} failed: ${res.status}`);
    return data;
  };

  // ─── Team-info + KPI tiles ────────────────────────────────────────────
  DAT.loadTeamInfo = async function () {
    try {
      const data = await DAT.apiGet('/api/developer/affiliate/team');
      const nameEl = DAT.$('#dat-team-name');
      const metaEl = DAT.$('#dat-team-meta');
      if (nameEl) nameEl.textContent = data.display_name || 'Team';
      if (metaEl) metaEl.textContent = data.public_slug ? `Public slug: ${data.public_slug}` : 'No public slug set';

      const tile = (id, value) => {
        const el = DAT.$(id);
        if (el) el.textContent = value;
      };
      tile('#dat-tile-members', data.active_members != null ? String(data.active_members) : '0');
      const c = data.counters || {};
      tile('#dat-tile-lifetime', DAT.fmtCents(c.lifetime_commission_cents || 0));
      tile('#dat-tile-pending', DAT.fmtCents(c.pending_commission_cents || 0));
      tile('#dat-tile-payable', DAT.fmtCents(c.payable_commission_cents || 0));
      tile('#dat-tile-paid', DAT.fmtCents(c.paid_commission_cents || 0));

      // Settings-page pre-fill (only if those inputs exist on this page)
      const nameInp = DAT.$('#dat-team-display-name');
      const slugInp = DAT.$('#dat-public-slug');
      if (nameInp && data.display_name) nameInp.value = data.display_name;
      if (slugInp && data.public_slug) slugInp.value = data.public_slug;

      DAT.teamData = data; // cached for other modules (e.g. members list)
    } catch (e) {
      const nameEl = DAT.$('#dat-team-name');
      if (nameEl) nameEl.textContent = 'Failed to load team';
      console.error('[affiliate-team] loadTeamInfo failed:', e);
    }
  };

  // ─── Invite modal (with a11y focus trap + ESC) ────────────────────────
  let modalReturnFocusEl = null;
  let modalKeydownHandler = null;

  function focusableEls(root) {
    return Array.from(
      root.querySelectorAll(
        'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    ).filter((el) => !el.hidden && el.offsetParent !== null);
  }

  function openInviteModal() {
    const modal = DAT.$('#dat-invite-modal');
    if (!modal) return;
    modalReturnFocusEl = document.activeElement;
    modal.hidden = false;
    const preview = DAT.$('#dat-invite-preview');
    if (preview) preview.hidden = true;
    const input = DAT.$('#dat-invite-email');
    if (input) {
      input.value = '';
      setTimeout(() => input.focus(), 50);
    }

    // ESC to close + Tab cycle inside the modal panel.
    const panel = modal.querySelector('.dat-modal__panel');
    modalKeydownHandler = (e) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        closeInviteModal();
        return;
      }
      if (e.key !== 'Tab' || !panel) return;
      const f = focusableEls(panel);
      if (!f.length) return;
      const first = f[0];
      const last = f[f.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        last.focus();
        e.preventDefault();
      } else if (!e.shiftKey && document.activeElement === last) {
        first.focus();
        e.preventDefault();
      }
    };
    document.addEventListener('keydown', modalKeydownHandler);
  }

  function closeInviteModal() {
    const modal = DAT.$('#dat-invite-modal');
    if (modal) modal.hidden = true;
    if (modalKeydownHandler) {
      document.removeEventListener('keydown', modalKeydownHandler);
      modalKeydownHandler = null;
    }
    if (modalReturnFocusEl && typeof modalReturnFocusEl.focus === 'function') {
      modalReturnFocusEl.focus();
    }
    modalReturnFocusEl = null;
  }

  async function submitInvite(e) {
    e.preventDefault();
    const email = DAT.$('#dat-invite-email').value.trim();
    if (!email) return;
    try {
      const data = await DAT.apiPost('/api/developer/affiliate/team/invite', { email });
      const preview = DAT.$('#dat-invite-preview');
      if (data.preview_token) {
        DAT.$('#dat-invite-token').textContent = data.preview_token;
        if (preview) preview.hidden = false;
      } else {
        closeInviteModal();
      }
      // Tell the active sub-page module to refresh if it cares about members.
      if (typeof DAT.onInviteSent === 'function') DAT.onInviteSent();
    } catch (err) {
      alert('Invitation failed: ' + err.message);
    }
  }

  function bindShell() {
    // Sidebar active-state handled by sidebar.html itself (parent + child markers).

    // Invite modal triggers
    const inviteBtn = DAT.$('#dat-invite-btn');
    if (inviteBtn) inviteBtn.addEventListener('click', openInviteModal);
    DAT.$$('[data-close="invite"]').forEach((el) => el.addEventListener('click', closeInviteModal));
    const inviteForm = DAT.$('#dat-invite-form');
    if (inviteForm) inviteForm.addEventListener('submit', submitInvite);

    // KPI tiles
    DAT.loadTeamInfo();
  }

  document.addEventListener('DOMContentLoaded', bindShell);
})();
