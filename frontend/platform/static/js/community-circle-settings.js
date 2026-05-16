/* global window, document, fetch, location */

/**
 * Circle Settings sub-page (replaces the old in-page modal).
 *
 * URL: /community/circle/:slug/settings
 *
 * Per-card visibility is driven by `my_role` from
 * `/api/community/circles/by-slug/:slug`:
 *   - member            → read-only header + members list (display only)
 *   - moderator         → can kick/ban members (not other mods/admins)
 *   - admin             → moderator powers + privacy + token-gate
 *   - owner             → all of the above + transfer + delete + slug
 */
(function () {
  'use strict';

  function escHtml(s) {
    if (typeof s !== 'string') return String(s == null ? '' : s);
    var d = document.createElement('div');
    d.appendChild(document.createTextNode(s));
    return d.innerHTML;
  }
  function $(sel) { return document.querySelector(sel); }
  function csrf() {
    var v = '; ' + document.cookie;
    var p = v.split('; csrf_token=');
    if (p.length === 2) return decodeURIComponent(p.pop().split(';').shift());
    return '';
  }

  // Extract slug from /community/circle/:slug/settings
  function getSlug() {
    var m = location.pathname.match(/^\/community\/circle\/([^/]+)\/settings/);
    return m ? decodeURIComponent(m[1]) : null;
  }

  var STATE = {
    circle: null,
    myRole: null,
    saved: null, // snapshot of last-saved form values for dirty-detection
  };

  // ─── Hydrate ──────────────────────────────────────────────────────
  async function loadCircle() {
    var slug = getSlug();
    if (!slug) return;
    try {
      var res = await fetch('/api/community/circles/by-slug/' + encodeURIComponent(slug), {
        credentials: 'same-origin',
      });
      if (!res.ok) {
        if (res.status === 404) {
          showFatal('Circle not found.');
        } else {
          showFatal('Could not load circle.');
        }
        return;
      }
      var data = await res.json();
      STATE.circle = data.circle;
      STATE.myRole = data.my_role;
      hydrateForm();
      gateCardsByRole();
      await Promise.all([loadMembers(), loadBans(), loadJoinRequests()]);
    } catch (e) {
      console.error(e);
      showFatal('Network error loading circle.');
    }
  }

  function showFatal(msg) {
    var root = $('#ccs-root');
    if (root) root.innerHTML = '<div class="ds-card" style="padding:24px;text-align:center">' + escHtml(msg) + '</div>';
  }

  function hydrateForm() {
    var c = STATE.circle;
    $('#ccs-avatar').textContent = c.avatar_emoji || '🟢';
    $('#ccs-name').textContent = c.name || '—';
    $('#ccs-meta-members').textContent = (c.member_count || 0) + ' / ' + (c.max_members || 0) + ' members';
    $('#ccs-meta-privacy').textContent = c.is_public ? 'Public' : 'Private';
    var roleEl = $('#ccs-meta-role');
    var roleLabel = STATE.myRole ? STATE.myRole : 'visitor';
    roleEl.textContent = roleLabel;
    roleEl.className = 'ccs-header__role-badge ccs-header__role-badge--' + roleLabel;
    roleEl.setAttribute('role', 'img');
    roleEl.setAttribute('aria-label', 'Your role in this circle: ' + roleLabel);

    $('#ccs-view-feed-link').setAttribute('href', '/community?tab=circle');

    $('#ccs-input-name').value = c.name || '';
    $('#ccs-input-emoji').value = c.avatar_emoji || '🟢';
    $('#ccs-input-desc').value = c.description || '';
    $('#ccs-input-slug').value = c.slug || '';
    $('#ccs-input-public').checked = !!c.is_public;
    updateSlugPreview();

    STATE.saved = readForm();
    refreshFooter();
  }

  function readForm() {
    return {
      name: ($('#ccs-input-name').value || '').trim(),
      emoji: ($('#ccs-input-emoji').value || '').trim(),
      description: ($('#ccs-input-desc').value || '').trim(),
      slug: ($('#ccs-input-slug').value || '').trim().toLowerCase(),
      is_public: $('#ccs-input-public').checked,
    };
  }

  function isDirty() {
    if (!STATE.saved) return false;
    var cur = readForm();
    return JSON.stringify(cur) !== JSON.stringify(STATE.saved);
  }

  function refreshFooter() {
    var dirty = isDirty();
    var footer = $('#ccs-footer');
    if (footer) footer.dataset.state = dirty ? 'dirty' : 'pristine';
    $('#ccs-save-btn').disabled = !dirty;
    $('#ccs-discard-btn').disabled = !dirty;
  }

  function updateSlugPreview() {
    var s = ($('#ccs-input-slug').value || '').trim().toLowerCase() || 'your-slug';
    $('#ccs-slug-preview').textContent = '/community/circle/' + s;
  }

  // ─── Role gating ──────────────────────────────────────────────────
  function gateCardsByRole() {
    var isOwner = STATE.myRole === 'owner';
    var isAdminOrOwner = STATE.myRole === 'owner' || STATE.myRole === 'admin';
    var isMod = STATE.myRole === 'moderator' || isAdminOrOwner;

    // Danger Zone: owner only
    var danger = $('#ccs-danger-card');
    if (danger) danger.hidden = !isOwner;

    // Privacy / slug edits: admin+
    $('#ccs-input-public').disabled = !isAdminOrOwner;
    $('#ccs-input-slug').disabled = !isOwner; // slug change = owner only

    // Read-only mode for non-mods
    if (!isMod) {
      ['ccs-input-name', 'ccs-input-emoji', 'ccs-input-desc'].forEach(function (id) {
        var el = document.getElementById(id);
        if (el) el.disabled = true;
      });
      var save = $('#ccs-save-btn'); if (save) save.disabled = true;
    }
  }

  // ─── Members list ─────────────────────────────────────────────────
  async function loadMembers() {
    var listEl = $('#ccs-members-list');
    if (!listEl) return;
    listEl.innerHTML = '<div class="ccs-empty">Loading members…</div>';
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/members', { credentials: 'same-origin' });
      if (!res.ok) throw new Error('members ' + res.status);
      var data = await res.json();
      var members = data.members || data || [];
      if (!Array.isArray(members) || members.length === 0) {
        listEl.innerHTML = '<div class="ccs-empty">No members.</div>';
        return;
      }
      listEl.innerHTML = members.map(memberRow).join('');
    } catch (e) {
      console.error(e);
      listEl.innerHTML = '<div class="ccs-empty">Failed to load members.</div>';
    }
  }

  function memberRow(m) {
    var uid = escHtml(m.user_id);
    var role = escHtml(m.role || 'member');
    var label = escHtml(m.display_name || m.email || m.user_id);
    var isOwner = STATE.myRole === 'owner';
    var isAdminPlus = isOwner || STATE.myRole === 'admin';
    var canMod = isOwner || STATE.myRole === 'admin' || STATE.myRole === 'moderator';

    var promote = '';
    if (isOwner && role !== 'owner' && role !== 'moderator') {
      promote = '<button class="ds-btn ds-btn--secondary ds-btn--sm" data-ccs-action="promote" data-uid="' + uid + '">Make Moderator</button>';
    } else if (isOwner && role === 'moderator') {
      promote = '<button class="ds-btn ds-btn--ghost ds-btn--sm" data-ccs-action="demote" data-uid="' + uid + '">Demote</button>';
    }
    var kick = canMod && role !== 'owner'
      ? '<button class="ds-btn ds-btn--ghost ds-btn--sm" data-ccs-action="kick" data-uid="' + uid + '">Kick</button>'
      : '';
    var ban = canMod && role !== 'owner'
      ? '<button class="ds-btn ds-btn--danger ds-btn--sm" data-ccs-action="ban" data-uid="' + uid + '">Ban</button>'
      : '';

    return (
      '<div class="ccs-member-row" data-uid="' + uid + '">' +
        '<div class="ccs-member-row__info">' +
          '<span class="ccs-member-row__name">' + label + '</span>' +
          '<span class="ccs-member-row__role ccs-member-row__role--' + role + '" ' +
                'role="img" aria-label="Circle role: ' + role + '">' + role + '</span>' +
        '</div>' +
        '<div class="ccs-member-row__actions">' + promote + kick + ban + '</div>' +
      '</div>'
    );
  }

  // ─── Bans + Join Requests ─────────────────────────────────────────
  async function loadBans() {
    // No public endpoint yet; render placeholder. Future: GET /api/community/circles/:id/bans.
    var el = $('#ccs-bans-list');
    if (el) el.innerHTML = '<div class="ccs-empty">No banned users.</div>';
  }
  async function loadJoinRequests() {
    if (!STATE.circle || STATE.circle.is_public) {
      var card = $('#ccs-requests-card'); if (card) card.hidden = true; return;
    }
    var el = $('#ccs-requests-list');
    if (!el) return;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/requests', { credentials: 'same-origin' });
      if (!res.ok) return;
      var data = await res.json();
      var reqs = data.requests || [];
      var card = $('#ccs-requests-card');
      if (card) card.hidden = reqs.length === 0;
      if (reqs.length === 0) {
        el.innerHTML = '<div class="ccs-empty">No pending requests.</div>';
      } else {
        el.innerHTML = reqs.map(function (r) {
          return '<div class="ccs-member-row">' +
            '<div class="ccs-member-row__info"><span class="ccs-member-row__name">' + escHtml(r.display_name || r.user_id) + '</span></div>' +
            '<div class="ccs-member-row__actions">' +
              '<button class="ds-btn ds-btn--primary ds-btn--sm" data-ccs-req="approve" data-req-id="' + escHtml(r.id) + '">Approve</button>' +
              '<button class="ds-btn ds-btn--ghost ds-btn--sm" data-ccs-req="decline" data-req-id="' + escHtml(r.id) + '">Decline</button>' +
            '</div>' +
          '</div>';
        }).join('');
      }
    } catch (e) { console.error(e); }
  }

  // ─── Save / Discard ───────────────────────────────────────────────
  async function save() {
    var cur = readForm();
    var prev = STATE.saved;
    var payload = {};
    if (cur.name !== prev.name)               payload.name = cur.name;
    if (cur.emoji !== prev.emoji)             payload.avatar_emoji = cur.emoji;
    if (cur.description !== prev.description) payload.description = cur.description;
    if (cur.slug !== prev.slug)               payload.slug = cur.slug;
    if (cur.is_public !== prev.is_public)     payload.is_public = cur.is_public;
    if (Object.keys(payload).length === 0) return;

    var btn = $('#ccs-save-btn');
    btn.disabled = true; btn.textContent = 'Saving…';
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id, {
        method: 'PUT',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf() },
        body: JSON.stringify(payload),
      });
      if (!res.ok) {
        var msg = 'Save failed'; try { var j = await res.json(); msg = j.error || j.message || msg; } catch (_) {}
        throw new Error(msg);
      }
      // If slug changed, redirect to new URL.
      if (payload.slug && payload.slug !== prev.slug) {
        location.href = '/community/circle/' + encodeURIComponent(payload.slug) + '/settings';
        return;
      }
      STATE.saved = cur;
      $('#ccs-status').textContent = 'Saved ✓';
      setTimeout(function () { $('#ccs-status').textContent = ''; }, 1800);
      refreshFooter();
    } catch (e) {
      $('#ccs-status').textContent = e.message || 'Save failed';
    } finally {
      btn.textContent = 'Save changes';
      refreshFooter();
    }
  }

  function discard() {
    hydrateForm();
    $('#ccs-status').textContent = 'Changes discarded';
    setTimeout(function () { $('#ccs-status').textContent = ''; }, 1500);
  }

  // ─── Member actions (delegate) ────────────────────────────────────
  document.addEventListener('click', async function (e) {
    var actBtn = e.target.closest('[data-ccs-action]');
    if (actBtn && STATE.circle) {
      var action = actBtn.getAttribute('data-ccs-action');
      var uid = actBtn.getAttribute('data-uid');
      await runMemberAction(action, uid, actBtn);
      return;
    }
    var reqBtn = e.target.closest('[data-ccs-req]');
    if (reqBtn && STATE.circle) {
      var verb = reqBtn.getAttribute('data-ccs-req');
      var rid = reqBtn.getAttribute('data-req-id');
      await fetch('/api/community/circles/requests/' + rid + '/' + verb, {
        method: 'POST', credentials: 'same-origin',
        headers: { 'X-CSRF-Token': csrf() },
      });
      loadJoinRequests();
      return;
    }
    if (e.target.id === 'ccs-save-btn') save();
    if (e.target.id === 'ccs-discard-btn') discard();
    if (e.target.id === 'ccs-delete-btn') deleteCircle();
  });

  async function runMemberAction(action, uid, btn) {
    var cid = STATE.circle.id;
    var url, opts;
    if (action === 'promote') {
      url = '/api/community/circles/' + cid + '/moderator/' + uid;
      opts = { method: 'POST', body: JSON.stringify({ moderator: true }) };
    } else if (action === 'demote') {
      url = '/api/community/circles/' + cid + '/moderator/' + uid;
      opts = { method: 'POST', body: JSON.stringify({ moderator: false }) };
    } else if (action === 'kick') {
      if (!confirm('Kick this member?')) return;
      url = '/api/community/circles/' + cid + '/kick/' + uid;
      opts = { method: 'POST' };
    } else if (action === 'ban') {
      var reason = prompt('Reason for ban (optional):') || null;
      if (reason === undefined) return;
      url = '/api/community/circles/' + cid + '/bans';
      opts = { method: 'POST', body: JSON.stringify({ user_id: uid, reason: reason }) };
    } else { return; }

    btn.disabled = true;
    try {
      var res = await fetch(url, Object.assign({
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf() },
      }, opts));
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Action failed');
      }
      await loadMembers();
      // Notify other tabs — they may show the affected user or circle
      // and need to refresh role/member-count.
      if (window.communitySync) {
        var evt = action === 'ban'    ? 'circle.banned' :
                  action === 'kick'   ? 'circle.kicked' :
                  action === 'promote'? 'circle.role_changed' :
                  action === 'demote' ? 'circle.role_changed' : 'circle.updated';
        window.communitySync.emit(evt, { circle_id: cid, user_id: uid });
      }
    } catch (e) {
      alert(e.message);
    } finally {
      btn.disabled = false;
    }
  }

  async function deleteCircle() {
    if (!confirm('Delete this circle permanently? All members will be removed and this cannot be undone.')) return;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id, {
        method: 'DELETE', credentials: 'same-origin',
        headers: { 'X-CSRF-Token': csrf() },
      });
      if (!res.ok) throw new Error('Delete failed');
      if (window.communitySync) {
        window.communitySync.emit('circle.deleted', { circle_id: STATE.circle.id });
      }
      location.href = '/community?tab=circle';
    } catch (e) {
      alert(e.message);
    }
  }

  // ─── Boot ─────────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', function () {
    loadCircle();
    ['ccs-input-name', 'ccs-input-emoji', 'ccs-input-desc', 'ccs-input-public'].forEach(function (id) {
      var el = document.getElementById(id);
      if (el) el.addEventListener('input', refreshFooter);
      if (el && el.type === 'checkbox') el.addEventListener('change', refreshFooter);
    });
    var slug = $('#ccs-input-slug');
    if (slug) slug.addEventListener('input', function () { updateSlugPreview(); refreshFooter(); });
    window.addEventListener('beforeunload', function (e) {
      if (isDirty()) { e.preventDefault(); e.returnValue = ''; }
    });
  });
})();
