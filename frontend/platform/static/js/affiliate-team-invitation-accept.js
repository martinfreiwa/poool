/* global window, document, fetch, location, sessionStorage */

/**
 * Invite-acceptance landing page (Phase-2 P0).
 *
 * Flow:
 *  1. Read `?token=` from URL (or fall back to `sessionStorage` — set on
 *     login-link click so the resume-after-auth path can recover it).
 *  2. GET `/api/affiliate/team/invitation-preview?token=…` (public).
 *     • 200 → render team + inviter, branch on auth state.
 *     • 4xx → error state.
 *  3. Check auth via `/api/affiliate/team/my-membership` (401 = anonymous).
 *  4. If logged in → "Accept" button POSTs `/api/affiliate/team/accept-invitation`.
 *     If anonymous → "Log in / Sign up" buttons stash the token in
 *     sessionStorage; the post-login resume happens on dashboard or here
 *     if the user comes back to this URL.
 *
 *  Defensive choices:
 *    - All fetches abort after 15 s.
 *    - Token clamped to 200 chars, only safe printable ASCII shown.
 *    - sessionStorage (not localStorage) so the token doesn't outlive the
 *      browser session if the user abandons.
 */
(function () {
  'use strict';

  const SS_KEY = 'poool:pendingInviteToken';
  const TIMEOUT_MS = 15_000;

  // ── Tiny DOM helpers (no DAT shell on this minimal page) ────────────────
  const $ = (sel) => document.querySelector(sel);
  function showOnly(stateId) {
    ['#invite-state-loading', '#invite-state-error', '#invite-state-preview', '#invite-state-success']
      .forEach((id) => { const el = $(id); if (el) el.hidden = id !== stateId; });
  }
  function safeText(s) {
    if (s == null) return '';
    return String(s).replace(/[^\x20-\x7E -￿]/g, '').slice(0, 200);
  }

  async function timedFetch(url, init) {
    const ctrl = new AbortController();
    const t = setTimeout(() => ctrl.abort(), TIMEOUT_MS);
    try {
      return await fetch(url, { ...init, signal: ctrl.signal, credentials: 'same-origin' });
    } finally {
      clearTimeout(t);
    }
  }

  function getToken() {
    const url = new URL(location.href);
    const fromUrl = url.searchParams.get('token');
    if (fromUrl && fromUrl.trim()) return fromUrl.trim().slice(0, 200);
    try {
      const fromSs = sessionStorage.getItem(SS_KEY);
      if (fromSs) return fromSs.slice(0, 200);
    } catch { /* sessionStorage disabled — fine */ }
    return null;
  }

  function showError(msg) {
    const el = $('#invite-error-msg');
    if (el && msg) el.textContent = msg;
    showOnly('#invite-state-error');
  }

  async function fetchPreview(token) {
    const res = await timedFetch(
      `/api/affiliate/team/invitation-preview?token=${encodeURIComponent(token)}`,
      { headers: { Accept: 'application/json' } },
    );
    if (res.status === 404) throw new Error('expired');
    if (res.status === 429) throw new Error('rate_limited');
    if (!res.ok) throw new Error('unknown');
    return res.json();
  }

  async function checkAuth() {
    // `my-membership` returns 401 when anonymous and 200 (with status='none'
    // or with a membership row) when logged in. Cheapest auth check we have.
    try {
      const res = await timedFetch('/api/affiliate/team/my-membership',
        { headers: { Accept: 'application/json' } });
      return res.ok;
    } catch { return false; }
  }

  async function doAccept(token) {
    const res = await timedFetch('/api/affiliate/team/accept-invitation', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: JSON.stringify({ token }),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || `Accept failed: ${res.status}`);
    }
    return res.json().catch(() => ({}));
  }

  function persistTokenForResume(token) {
    try { sessionStorage.setItem(SS_KEY, token); } catch { /* ignore */ }
  }
  function clearPersistedToken() {
    try { sessionStorage.removeItem(SS_KEY); } catch { /* ignore */ }
  }

  function renderPreview(data, token, isLoggedIn) {
    const teamEl = $('#invite-team-name');
    const inviterEl = $('#invite-inviter');
    const expiresEl = $('#invite-expires');
    const tokenEl = $('#invite-token-display');

    if (teamEl) teamEl.textContent = safeText(data.team_name) || 'a POOOL Affiliate Team';
    if (inviterEl) inviterEl.textContent = safeText(data.inviter_name) || 'The team owner';
    if (tokenEl) tokenEl.textContent = safeText(token);

    if (expiresEl && data.expires_at) {
      try {
        const d = new Date(data.expires_at);
        if (!Number.isNaN(d.getTime())) {
          expiresEl.textContent = `This invitation expires ${d.toLocaleString()}.`;
          expiresEl.hidden = false;
        }
      } catch { /* swallow */ }
    }

    const loggedInBlock = $('#invite-auth-loggedin');
    const anonBlock = $('#invite-auth-anon');
    if (isLoggedIn) {
      if (loggedInBlock) loggedInBlock.hidden = false;
      if (anonBlock) anonBlock.hidden = true;
    } else {
      if (loggedInBlock) loggedInBlock.hidden = true;
      if (anonBlock) anonBlock.hidden = false;
      // Login/Signup buttons stash the token first so we can resume after auth.
      const login = $('#invite-login-link');
      const signup = $('#invite-signup-link');
      const stash = () => persistTokenForResume(token);
      if (login) login.addEventListener('click', stash);
      if (signup) signup.addEventListener('click', stash);
    }

    showOnly('#invite-state-preview');
  }

  async function handleAcceptClick(token) {
    const btn = $('#invite-accept-btn');
    if (btn) { btn.disabled = true; btn.textContent = 'Accepting…'; }
    try {
      await doAccept(token);
      clearPersistedToken();
      showOnly('#invite-state-success');
    } catch (e) {
      // Reset the button so the user can retry; surface message inline.
      if (btn) { btn.disabled = false; btn.textContent = 'Accept invitation'; }
      const msg = String(e && e.message || 'Accept failed. Please try again.');
      // Common errors: 401 (lost session), 404 (expired between preview and accept).
      if (/401/.test(msg) || /unauthorized/i.test(msg)) {
        persistTokenForResume(token);
        showError('Your session ended. Please log in to accept this invitation.');
      } else if (/expired|not found/i.test(msg) || /404/.test(msg)) {
        showError('This invitation has expired or was already used.');
      } else {
        showError(msg);
      }
    }
  }

  function bindAccept(token) {
    const btn = $('#invite-accept-btn');
    if (btn) btn.addEventListener('click', () => handleAcceptClick(token));
    const decline = $('#invite-decline-btn');
    if (decline) decline.addEventListener('click', () => {
      clearPersistedToken();
      location.href = '/marketplace';
    });
  }

  async function boot() {
    const token = getToken();
    if (!token) {
      showError('No invitation token was provided.');
      return;
    }

    let preview;
    try {
      preview = await fetchPreview(token);
    } catch (e) {
      if (e.message === 'rate_limited') {
        showError('Too many lookups from your IP. Please wait a moment and try again.');
      } else if (e.message === 'expired') {
        showError('This invitation has expired, was already used, or never existed.');
      } else {
        showError('Could not load this invitation. Please try again later.');
      }
      return;
    }

    const isLoggedIn = await checkAuth();
    renderPreview(preview, token, isLoggedIn);
    if (isLoggedIn) bindAccept(token);
  }

  document.addEventListener('DOMContentLoaded', boot);
})();
