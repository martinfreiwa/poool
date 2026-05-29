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
  function dateTimeLocalToIso(id) {
    var raw = getInputValue(id).trim();
    if (!raw) return null;
    var parsed = new Date(raw);
    return Number.isNaN(parsed.getTime()) ? null : parsed.toISOString();
  }

  // Extract slug from /community/circle/:slug/settings
  function getSlug() {
    var m = location.pathname.match(/^\/community\/circle\/([^/]+)\/settings/);
    return m ? decodeURIComponent(m[1]) : null;
  }

  var STATE = {
    circle: null,
    myRole: null,
    manage: null,
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
      await loadManageSummary();
      hydrateForm();
      gateCardsByRole();
      await Promise.all([
        loadMembers(),
        loadBans(),
        loadJoinRequests(),
        loadCircleReports(),
        loadOpsAlerts(),
        loadResourceLibrary(),
      ]);
    } catch (e) {
      console.error(e);
      showFatal('Network error loading circle.');
    }
  }

  async function loadManageSummary() {
    if (!STATE.circle || !STATE.circle.id) return;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/manage', {
        credentials: 'same-origin',
      });
      if (!res.ok) {
        STATE.manage = null;
        hideManageCards();
        return;
      }
      var data = await res.json();
      STATE.manage = data;
      if (data.circle) {
        STATE.circle = Object.assign({}, STATE.circle, data.circle);
      }
      renderAnalytics(data.analytics || {}, data.audit_log || []);
    } catch (e) {
      console.error(e);
      STATE.manage = null;
      hideManageCards();
    }
  }

  function hideManageCards() {
    ['ccs-content-card', 'ccs-moderation-card', 'ccs-rules-card', 'ccs-analytics-card', 'ccs-reports-card', 'ccs-resources-card'].forEach(function (id) {
      var el = document.getElementById(id);
      if (el) el.hidden = true;
    });
  }

  function showFatal(msg) {
    var root = $('#ccs-root');
    if (root) root.innerHTML = '<div class="ds-card" style="padding:24px;text-align:center">' + escHtml(msg) + '</div>';
  }

  function getInputValue(id) {
    var el = document.getElementById(id);
    return el ? el.value || '' : '';
  }

  function setInputValue(id, value) {
    var el = document.getElementById(id);
    if (el) el.value = value == null ? '' : String(value);
  }

  function setText(id, value) {
    var el = document.getElementById(id);
    if (el) el.textContent = value == null ? '' : String(value);
  }

  function getChecked(id) {
    var el = document.getElementById(id);
    return !!(el && el.checked);
  }

  function setChecked(id, value) {
    var el = document.getElementById(id);
    if (el) el.checked = !!value;
  }

  function parseList(value) {
    return String(value || '')
      .split(/[\n,]+/)
      .map(function (item) { return item.trim(); })
      .filter(Boolean);
  }

  function joinList(value) {
    return Array.isArray(value) ? value.join('\n') : '';
  }

  function renderAnalytics(analytics, auditLog) {
    setText('ccs-analytics-posts', analytics.posts_7d == null ? '—' : analytics.posts_7d);
    setText('ccs-analytics-comments', analytics.comments_7d == null ? '—' : analytics.comments_7d);
    setText('ccs-analytics-active', analytics.active_members_7d == null ? '—' : analytics.active_members_7d);
    setText('ccs-analytics-reports', analytics.pending_reports == null ? '—' : analytics.pending_reports);

    var log = document.getElementById('ccs-audit-log');
    if (!log) return;
    if (!Array.isArray(auditLog) || auditLog.length === 0) {
      log.innerHTML = '<div class="ccs-empty">No recent Circle audit entries.</div>';
      return;
    }
    log.innerHTML = auditLog.map(function (entry) {
      return '<div class="ccs-audit-log__row">' +
        '<span>' + escHtml(entry.action || 'circle.audit') + '</span>' +
        '<time>' + escHtml(entry.created_at || '') + '</time>' +
      '</div>';
    }).join('');
  }

  async function loadOpsAlerts() {
    var el = document.getElementById('ccs-ops-alerts');
    if (!el || !STATE.circle || !STATE.manage) return;
    el.innerHTML = '<div class="ccs-empty">Loading ops alerts…</div>';
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/ops-alerts', {
        credentials: 'same-origin',
      });
      if (!res.ok) throw new Error('ops alerts ' + res.status);
      var data = await res.json();
      var alerts = Array.isArray(data.alerts) ? data.alerts : [];
      renderOpsAlerts(alerts);
    } catch (e) {
      console.error(e);
      el.innerHTML = '<div class="ccs-empty">Failed to load ops alerts.</div>';
    }
  }

  function renderOpsAlerts(alerts) {
    var el = document.getElementById('ccs-ops-alerts');
    if (!el) return;
    if (!alerts.length) {
      el.innerHTML = '<div class="ccs-empty">No open Circle ops alerts.</div>';
      return;
    }
    el.innerHTML = alerts.map(opsAlertRow).join('');
  }

  function opsAlertRow(alert) {
    var id = escHtml(alert.id);
    var severity = escHtml(alert.severity || 'info');
    var status = escHtml(alert.status || 'open');
    var workflow = escHtml(resourceLabel(alert.workflow_state || 'triage'));
    var type = String(alert.alert_type || 'ops_alert').replace(/_/g, ' ');
    var details = alert.details && typeof alert.details === 'object' ? alert.details : {};
    var detailBits = Object.keys(details).slice(0, 4).map(function (key) {
      return key.replace(/_/g, ' ') + ': ' + details[key];
    });
    return '<article class="ccs-ops-alert-row ccs-ops-alert-row--' + severity + '" data-alert-id="' + id + '">' +
      '<div class="ccs-ops-alert-row__body">' +
        '<div class="ccs-ops-alert-row__meta">' +
          '<span>' + escHtml(severity) + '</span>' +
          '<span>' + escHtml(status) + '</span>' +
          '<span>' + workflow + '</span>' +
          '<span>' + escHtml(type) + '</span>' +
        '</div>' +
        '<p class="ccs-ops-alert-row__summary">' + escHtml(alert.summary || 'Circle ops alert') + '</p>' +
        (detailBits.length ? '<p class="ccs-ops-alert-row__details">' + escHtml(detailBits.join(' · ')) + '</p>' : '') +
      '</div>' +
      '<div class="ccs-ops-alert-row__actions">' +
        (status === 'open'
          ? '<button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-ccs-alert-action="acknowledge" data-alert-id="' + id + '">Acknowledge</button>'
          : '') +
        '<button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-ccs-alert-action="set_workflow_state" data-alert-id="' + id + '">Workflow</button>' +
        '<button type="button" class="ds-btn ds-btn--primary ds-btn--sm" data-ccs-alert-action="resolve" data-alert-id="' + id + '">Resolve</button>' +
      '</div>' +
    '</article>';
  }

  function promptOpsAlertWorkflowState() {
    var value = prompt(
      'Workflow state: triage, investigating, waiting_on_moderator, waiting_on_policy, mitigated, monitoring',
      'investigating'
    );
    if (value == null) return null;
    value = value.trim().toLowerCase().replace(/[\s-]+/g, '_');
    var allowed = ['triage', 'investigating', 'waiting_on_moderator', 'waiting_on_policy', 'mitigated', 'monitoring'];
    if (allowed.indexOf(value) < 0) {
      alert('Invalid workflow state.');
      return null;
    }
    return value;
  }

  function hydrateForm() {
    var c = STATE.circle;
    // Hero avatar — show the circle's name initial (consistent with the
    // new circle-card design). Emoji is still editable below for back-compat.
    $('#ccs-avatar').textContent = (c.name || 'C').trim().charAt(0).toUpperCase();
    $('#ccs-name').textContent = c.name || '—';
    $('#ccs-meta-members').textContent = (c.member_count || 0) + ' members';
    $('#ccs-meta-privacy').textContent = c.is_public ? 'Public' : 'Private';
    var roleEl = $('#ccs-meta-role');
    var roleLabel = STATE.myRole ? STATE.myRole : 'visitor';
    roleEl.textContent = roleLabel.charAt(0).toUpperCase() + roleLabel.slice(1);
    roleEl.setAttribute('role', 'img');
    roleEl.setAttribute('aria-label', 'Your role in this circle: ' + roleLabel);

    $('#ccs-view-feed-link').setAttribute('href', '/community/circle/' + encodeURIComponent(c.slug || ''));

    $('#ccs-input-name').value = c.name || '';
    var emojiEl = document.getElementById('ccs-input-emoji');
    if (emojiEl) emojiEl.value = c.avatar_emoji || '🟢';
    $('#ccs-input-desc').value = c.description || '';
    $('#ccs-input-slug').value = c.slug || '';
    $('#ccs-input-public').checked = !!c.is_public;
    setInputValue('ccs-input-category', c.category || '');
    setInputValue('ccs-input-language', c.language || 'en');
    setInputValue('ccs-input-location', c.location_text || '');
    setInputValue('ccs-input-required-tags', (c.required_post_tags || []).join(', '));
    setChecked('ccs-input-media', c.media_uploads_enabled !== false);
    setChecked('ccs-input-polls', c.polls_enabled !== false);
    setChecked('ccs-input-links', c.link_posting_enabled !== false);
    setChecked('ccs-input-anonymous', !!c.anonymous_posting_enabled);
    setChecked('ccs-input-first-post-approval', !!c.first_post_approval_enabled);
    setChecked('ccs-input-join-approval', !!c.join_approval_required);
    setChecked('ccs-input-auto-approve-verified', !!c.auto_approve_verified_investors);
    setChecked('ccs-input-announcement-comments', c.announcement_comments_enabled !== false);
    setChecked('ccs-input-onboarding', c.onboarding_enabled !== false);
    setInputValue('ccs-input-slow-mode', c.slow_mode_seconds || 0);
    setInputValue('ccs-input-blocked-words', joinList(c.blocked_words || []));
    setInputValue('ccs-input-risk-keywords', joinList(c.investment_risk_keywords || []));
    setInputValue('ccs-input-rules', c.rules_text || '');
    setInputValue('ccs-input-disclaimer', c.investment_disclaimer || '');
    updateSlugPreview();
    setBannerPreview(c.banner_url || '');
    if (typeof updateDescCounter === 'function') updateDescCounter();

    STATE.saved = readForm();
    refreshFooter();
  }

  function readForm() {
    return {
      name: ($('#ccs-input-name').value || '').trim(),
      avatar_emoji: ((document.getElementById('ccs-input-emoji') || {}).value || '').trim(),
      description: ($('#ccs-input-desc').value || '').trim(),
      slug: ($('#ccs-input-slug').value || '').trim().toLowerCase(),
      is_public: $('#ccs-input-public').checked,
      category: getInputValue('ccs-input-category').trim(),
      language: getInputValue('ccs-input-language').trim().toLowerCase(),
      location_text: getInputValue('ccs-input-location').trim(),
      required_post_tags: parseList(getInputValue('ccs-input-required-tags')),
      media_uploads_enabled: getChecked('ccs-input-media'),
      polls_enabled: getChecked('ccs-input-polls'),
      link_posting_enabled: getChecked('ccs-input-links'),
      anonymous_posting_enabled: getChecked('ccs-input-anonymous'),
      first_post_approval_enabled: getChecked('ccs-input-first-post-approval'),
      join_approval_required: getChecked('ccs-input-join-approval'),
      auto_approve_verified_investors: getChecked('ccs-input-auto-approve-verified'),
      announcement_comments_enabled: getChecked('ccs-input-announcement-comments'),
      onboarding_enabled: getChecked('ccs-input-onboarding'),
      slow_mode_seconds: Number.parseInt(getInputValue('ccs-input-slow-mode') || '0', 10) || 0,
      blocked_words: parseList(getInputValue('ccs-input-blocked-words')),
      investment_risk_keywords: parseList(getInputValue('ccs-input-risk-keywords')),
      rules_text: getInputValue('ccs-input-rules').trim(),
      investment_disclaimer: getInputValue('ccs-input-disclaimer').trim(),
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

  // ─── Banner preview + upload (immediate save on file pick) ───────
  function setBannerPreview(url) {
    var wrap = $('#ccs-banner-preview');
    var img = $('#ccs-banner-preview-img');
    if (!wrap || !img) return;
    if (url) {
      img.src = url;
      wrap.hidden = false;
    } else {
      img.src = '';
      wrap.hidden = true;
    }
  }

  async function uploadBanner(file) {
    var fd = new FormData();
    fd.append('file', file);
    var res = await fetch('/api/upload/post-image', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'X-CSRF-Token': csrf() },
      body: fd,
    });
    if (!res.ok) {
      var msg = 'Upload failed';
      try { var j = await res.json(); msg = j.error || j.message || msg; } catch (_) {}
      throw new Error(msg);
    }
    var data = await res.json();
    return data.image_url || data.url || '';
  }

  async function persistBanner(bannerUrl) {
    if (!STATE.circle) return;
    var res = await fetch('/api/community/circles/' + STATE.circle.id, {
      method: 'PUT',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf() },
      body: JSON.stringify({ banner_url: bannerUrl == null ? '' : bannerUrl }),
    });
    if (!res.ok) {
      var msg = 'Save failed';
      try { var j = await res.json(); msg = j.error || j.message || msg; } catch (_) {}
      throw new Error(msg);
    }
    var updated = await res.json();
    if (STATE.circle) STATE.circle.banner_url = updated.banner_url || '';
    return updated.banner_url || '';
  }

  function wireBannerControls() {
    var fileInput = $('#ccs-banner-file');
    var status = $('#ccs-status');
    if (fileInput && !fileInput._wired) {
      fileInput._wired = true;
      fileInput.addEventListener('change', async function (e) {
        var file = e.target.files && e.target.files[0];
        if (!file) return;
        if (file.size > 2 * 1024 * 1024) {
          if (status) status.textContent = 'Banner must be 2 MB or smaller.';
          fileInput.value = '';
          return;
        }
        if (status) status.textContent = 'Uploading banner…';
        try {
          var url = await uploadBanner(file);
          if (!url) throw new Error('No URL returned.');
          var saved = await persistBanner(url);
          setBannerPreview(saved || url);
          if (status) {
            status.textContent = 'Banner saved ✓';
            setTimeout(function () { status.textContent = ''; }, 1800);
          }
        } catch (err) {
          if (status) status.textContent = err.message || 'Upload failed';
        } finally {
          fileInput.value = '';
        }
      });
    }
  }

  // ─── Role gating ──────────────────────────────────────────────────
  function gateCardsByRole() {
    var isOwner = STATE.myRole === 'owner';
    var isAdminOrOwner = STATE.myRole === 'owner' || STATE.myRole === 'admin';
    var isMod = STATE.myRole === 'moderator' || isAdminOrOwner;
    var canResources = canManageResources();

    // Danger Zone: owner only — mirror visibility to the sidebar nav link.
    var danger = $('#ccs-danger-card');
    if (danger) danger.hidden = !isOwner;
    var dangerNav = $('#ccs-nav-danger');
    if (dangerNav) dangerNav.hidden = !isOwner;

    // Privacy / slug edits: admin+
    $('#ccs-input-public').disabled = !isAdminOrOwner;
    $('#ccs-input-slug').disabled = !isOwner; // slug change = owner only

    if (!isAdminOrOwner) {
      [
        'ccs-input-name', 'ccs-input-emoji', 'ccs-input-desc',
        'ccs-input-category', 'ccs-input-language', 'ccs-input-location',
        'ccs-input-required-tags', 'ccs-input-rules', 'ccs-input-disclaimer',
        'ccs-input-join-approval', 'ccs-input-auto-approve-verified',
        'ccs-input-announcement-comments', 'ccs-input-onboarding',
      ].forEach(function (id) {
        var el = document.getElementById(id);
        if (el) el.disabled = true;
      });
      ['ccs-input-media', 'ccs-input-polls', 'ccs-input-links', 'ccs-input-anonymous'].forEach(function (id) {
        var el = document.getElementById(id);
        if (el) el.disabled = true;
      });
    }

    // Read-only mode for non-mods
    if (!isMod) {
      [
        'ccs-input-name', 'ccs-input-emoji', 'ccs-input-desc',
        'ccs-input-category', 'ccs-input-language', 'ccs-input-location',
        'ccs-input-required-tags', 'ccs-input-slow-mode',
        'ccs-input-blocked-words', 'ccs-input-risk-keywords',
        'ccs-input-rules', 'ccs-input-disclaimer',
      ].forEach(function (id) {
        var el = document.getElementById(id);
        if (el) el.disabled = true;
      });
      document.querySelectorAll('.ccs-toggle-stack input').forEach(function (el) {
        el.disabled = true;
      });
      var save = $('#ccs-save-btn'); if (save) save.disabled = true;
    }

    var resourcesCard = $('#ccs-resources-card');
    if (resourcesCard) resourcesCard.hidden = !canResources;
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
    var card = $('#ccs-requests-card');
    var navP = $('#ccs-nav-requests');
    if (!STATE.circle) {
      if (card) card.hidden = true;
      if (navP) navP.hidden = true;
      return;
    }
    if (card) card.hidden = false;
    if (navP) navP.hidden = false;
    var el = $('#ccs-requests-list');
    if (!el) return;
    el.innerHTML = '<div class="ccs-empty" role="status">Loading join requests...</div>';
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/requests', { credentials: 'same-origin' });
      if (!res.ok) throw new Error('join requests ' + res.status);
      var data = await res.json();
      var reqs = Array.isArray(data.requests) ? data.requests : [];
      if (reqs.length === 0) {
        el.innerHTML = '<div class="ccs-empty" role="status">No pending join requests.</div>';
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
    } catch (e) {
      console.error(e);
      el.innerHTML = '<div class="ccs-empty" role="status">Failed to load join requests.</div>';
    }
  }

  async function loadCircleReports() {
    var el = $('#ccs-reports-list');
    if (!el || !STATE.circle || !STATE.manage) return;
    el.innerHTML = '<div class="ccs-empty">Loading reports…</div>';
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/reports', {
        credentials: 'same-origin',
      });
      if (!res.ok) throw new Error('reports ' + res.status);
      var data = await res.json();
      var reports = data.reports || [];
      if (!Array.isArray(reports) || reports.length === 0) {
        el.innerHTML = '<div class="ccs-empty">No pending reports for this Circle.</div>';
        return;
      }
      el.innerHTML = bulkReportToolbar(reports.length) + reports.map(reportRow).join('');
    } catch (e) {
      console.error(e);
      el.innerHTML = '<div class="ccs-empty">Failed to load reports.</div>';
    }
  }

  function bulkReportToolbar(count) {
    return '<div class="ccs-report-bulk" aria-label="Bulk report actions">' +
      '<div class="ccs-report-bulk__copy">' +
        '<strong>' + String(count) + ' pending reports</strong>' +
        '<span>Select reports to triage related posts in one audited action.</span>' +
      '</div>' +
      '<div class="ccs-report-bulk__actions">' +
        '<button type="button" class="ds-btn ds-btn--danger ds-btn--sm" data-ccs-report-bulk-action="hide_posts">Hide selected posts</button>' +
        '<button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-ccs-report-bulk-action="dismiss_reports">Dismiss selected reports</button>' +
      '</div>' +
    '</div>';
  }

  function selectedReportIds() {
    return Array.prototype.slice.call(document.querySelectorAll('#ccs-reports-list [data-ccs-report-select]:checked'))
      .map(function (input) { return input.value; })
      .filter(Boolean);
  }

  function reportRow(report) {
    var id = escHtml(report.id);
    return '<article class="ccs-report-row" data-report-id="' + id + '">' +
      '<label class="ccs-report-row__select">' +
        '<input type="checkbox" data-ccs-report-select value="' + id + '" aria-label="Select report">' +
        '<span>Select</span>' +
      '</label>' +
      '<div class="ccs-report-row__body">' +
        '<div class="ccs-report-row__meta">' +
          '<span>' + escHtml(report.reason || 'reported') + '</span>' +
          '<span>Reporter: ' + escHtml(report.reporter_name || 'Unknown') + '</span>' +
          '<span>Author: ' + escHtml(report.post_author_name || 'Unknown') + '</span>' +
        '</div>' +
        '<p class="ccs-report-row__content">' + escHtml(report.post_content || '') + '</p>' +
        (report.reporter_note ? '<p class="ccs-report-row__note">' + escHtml(report.reporter_note) + '</p>' : '') +
      '</div>' +
      '<div class="ccs-report-row__actions">' +
        '<button type="button" class="ds-btn ds-btn--danger ds-btn--sm" data-ccs-report-action="hide_post" data-report-id="' + id + '">Hide post</button>' +
        '<button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-ccs-report-action="dismiss_report" data-report-id="' + id + '">Dismiss</button>' +
      '</div>' +
    '</article>';
  }

  function canManageResources() {
    return STATE.myRole === 'owner' || STATE.myRole === 'admin' || STATE.myRole === 'platform_admin';
  }

  function resourceLabel(value) {
    return String(value || '')
      .replace(/_/g, ' ')
      .replace(/\b\w/g, function (ch) { return ch.toUpperCase(); });
  }

  async function loadResourceLibrary() {
    var el = $('#ccs-resources-manage-list');
    var card = $('#ccs-resources-card');
    if (!el || !card || !STATE.circle || !STATE.manage) return;
    if (!canManageResources()) {
      card.hidden = true;
      return;
    }
    el.innerHTML = '<div class="ccs-empty">Loading resources…</div>';
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/resources/manage', {
        credentials: 'same-origin',
      });
      if (!res.ok) throw new Error('resources ' + res.status);
      var data = await res.json();
      var resources = Array.isArray(data.resources) ? data.resources : [];
      renderResourceLibrary(resources);
    } catch (e) {
      console.error(e);
      el.innerHTML = '<div class="ccs-empty">Failed to load resources.</div>';
    }
  }

  function renderResourceLibrary(resources) {
    var el = $('#ccs-resources-manage-list');
    if (!el) return;
    if (!resources.length) {
      el.innerHTML = '<div class="ccs-empty">No Circle resources yet.</div>';
      return;
    }
    el.innerHTML = resources.map(resourceRow).join('');
  }

  function resourceRow(resource) {
    var id = escHtml(resource.id);
    var active = resource.is_active !== false;
    var versionCount = Number(resource.version_count || 0);
    var source = resource.has_private_file ? 'Private file' : (resource.external_url || 'External link');
    var lifecycle = [
      resource.upload_status ? 'Upload: ' + resourceLabel(resource.upload_status) : null,
      resource.retention_policy ? 'Retention: ' + resourceLabel(resource.retention_policy) : null,
      resource.legal_hold ? 'Legal hold' : null,
      resource.reviewed_at ? 'Reviewed ' + resource.reviewed_at : (resource.review_required_at ? 'Review due ' + resource.review_required_at : null),
      resource.retention_until ? 'Retain until ' + resource.retention_until : null,
    ].filter(Boolean);
    var restoreAction = ['deleted', 'expired', 'rejected'].indexOf(resource.upload_status) >= 0 ? 'restore_lifecycle' : 'restore';
    return '<article class="ccs-resource-row' + (active ? '' : ' ccs-resource-row--inactive') + '" data-resource-id="' + id + '">' +
      '<div class="ccs-resource-row__body">' +
        '<div class="ccs-resource-row__meta">' +
          '<span>' + escHtml(resourceLabel(resource.resource_type)) + '</span>' +
          '<span>' + escHtml(resourceLabel(resource.access_scope)) + '</span>' +
          '<span>' + (active ? 'Active' : 'Archived') + '</span>' +
          (resource.is_official ? '<span>Official</span>' : '') +
        '</div>' +
        '<p class="ccs-resource-row__title">' + escHtml(resource.title || 'Untitled resource') + '</p>' +
        '<p class="ccs-resource-row__source">' + escHtml(source) + '</p>' +
        '<p class="ccs-resource-row__version">Current version: ' + escHtml(resource.version_label || 'v1') + ' · ' + versionCount + ' version' + (versionCount === 1 ? '' : 's') + '</p>' +
        (lifecycle.length ? '<p class="ccs-resource-row__lifecycle">' + lifecycle.map(escHtml).join(' · ') + '</p>' : '') +
        (resource.document_lifecycle_notes ? '<p class="ccs-resource-row__note">' + escHtml(resource.document_lifecycle_notes) + '</p>' : '') +
        '<div class="ccs-resource-row__versions" id="ccs-resource-versions-' + id + '" hidden></div>' +
      '</div>' +
      '<div class="ccs-resource-row__actions">' +
        '<a class="ds-btn ds-btn--ghost ds-btn--sm" href="' + escHtml(resource.delivery_url || '#') + '" target="_blank" rel="noopener noreferrer">Open</a>' +
        '<button type="button" class="ds-btn ds-btn--secondary ds-btn--sm" data-ccs-resource-action="versions" data-resource-id="' + id + '">Versions</button>' +
        '<button type="button" class="ds-btn ds-btn--secondary ds-btn--sm" data-ccs-resource-action="add_version" data-resource-id="' + id + '">Add version</button>' +
        '<button type="button" class="ds-btn ds-btn--secondary ds-btn--sm" data-ccs-resource-action="upload_version" data-resource-id="' + id + '">Replace file</button>' +
        '<button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-ccs-resource-action="mark_reviewed" data-resource-id="' + id + '">Reviewed</button>' +
        '<button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-ccs-resource-action="legal_hold" data-resource-id="' + id + '">Legal hold</button>' +
        '<button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-ccs-resource-action="expire" data-resource-id="' + id + '">Expire</button>' +
        '<button type="button" class="ds-btn ds-btn--ghost ds-btn--sm" data-ccs-resource-action="' + (active ? 'archive' : restoreAction) + '" data-resource-id="' + id + '">' + (active ? 'Archive' : 'Restore') + '</button>' +
        '<button type="button" class="ds-btn ds-btn--danger ds-btn--sm" data-ccs-resource-action="soft_delete" data-resource-id="' + id + '">Soft delete</button>' +
      '</div>' +
    '</article>';
  }

  async function createResourceFromForm(event) {
    if (event) event.preventDefault();
    if (!STATE.circle || !canManageResources()) return;
    var form = $('#ccs-resource-form');
    var title = getInputValue('ccs-resource-title').trim();
    var url = getInputValue('ccs-resource-url').trim();
    var storagePath = getInputValue('ccs-resource-storage-path').trim();
    var fileInput = $('#ccs-resource-file');
    var uploadFile = fileInput && fileInput.files && fileInput.files.length ? fileInput.files[0] : null;
    var sourceCount = (url ? 1 : 0) + (storagePath ? 1 : 0) + (uploadFile ? 1 : 0);
    if ((!title && !uploadFile) || sourceCount !== 1) {
      $('#ccs-status').textContent = 'Provide a title and exactly one source, or upload a file.';
      return;
    }
    var retentionUntil = dateTimeLocalToIso('ccs-resource-retention-until');
    var reviewRequiredAt = dateTimeLocalToIso('ccs-resource-review-required-at');

    var btn = form ? form.querySelector('button[type="submit"]') : null;
    if (btn) btn.disabled = true;
    try {
      if (uploadFile) {
        var uploadData = new FormData();
        uploadData.append('file', uploadFile);
        if (title) uploadData.append('title', title);
        uploadData.append('resource_type', getInputValue('ccs-resource-type') || 'community_resource');
        uploadData.append('access_scope', getInputValue('ccs-resource-access') || 'member');
        uploadData.append('version_label', getInputValue('ccs-resource-version-label').trim() || 'v1');
        uploadData.append('is_official', getChecked('ccs-resource-official') ? 'true' : 'false');
        uploadData.append('retention_policy', getInputValue('ccs-resource-retention-policy') || 'standard');
        if (retentionUntil) uploadData.append('retention_until', retentionUntil);
        if (reviewRequiredAt) uploadData.append('review_required_at', reviewRequiredAt);
        var lifecycleNotes = getInputValue('ccs-resource-lifecycle-notes').trim();
        if (lifecycleNotes) uploadData.append('document_lifecycle_notes', lifecycleNotes);
        var uploadRes = await fetch('/api/community/circles/' + STATE.circle.id + '/resources/upload', {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'X-CSRF-Token': csrf() },
          body: uploadData,
        });
        if (!uploadRes.ok) {
          var uploadJson = await uploadRes.json().catch(function () { return {}; });
          throw new Error(uploadJson.error || uploadJson.message || 'Resource upload failed');
        }
        if (form) form.reset();
        $('#ccs-status').textContent = 'Resource uploaded ✓';
        setTimeout(function () { $('#ccs-status').textContent = ''; }, 1800);
        await loadResourceLibrary();
        return;
      }
      var payload = {
        title: title,
        resource_type: getInputValue('ccs-resource-type') || 'community_resource',
        access_scope: getInputValue('ccs-resource-access') || 'member',
        version_label: getInputValue('ccs-resource-version-label').trim() || 'v1',
        is_official: getChecked('ccs-resource-official'),
        requires_download: Boolean(storagePath),
        upload_status: getInputValue('ccs-resource-upload-status') || null,
        retention_policy: getInputValue('ccs-resource-retention-policy') || 'standard',
        retention_until: retentionUntil,
        review_required_at: reviewRequiredAt,
        document_lifecycle_notes: getInputValue('ccs-resource-lifecycle-notes').trim() || null,
      };
      if (url) payload.url = url;
      if (storagePath) payload.storage_object_path = storagePath;
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/resources/manage', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf() },
        body: JSON.stringify(payload),
      });
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Resource create failed');
      }
      if (form) form.reset();
      $('#ccs-status').textContent = 'Resource added ✓';
      setTimeout(function () { $('#ccs-status').textContent = ''; }, 1800);
      await loadResourceLibrary();
    } catch (e) {
      $('#ccs-status').textContent = e.message || 'Resource create failed';
    } finally {
      if (btn) btn.disabled = false;
    }
  }

  async function toggleResourceActive(resourceId, active, btn) {
    if (!resourceId) return;
    btn.disabled = true;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/resources/' + resourceId + '/manage', {
        method: 'PUT',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf() },
        body: JSON.stringify({ is_active: active }),
      });
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Resource update failed');
      }
      await loadResourceLibrary();
    } catch (e) {
      alert(e.message);
    } finally {
      btn.disabled = false;
    }
  }

  async function runResourceLifecycleAction(resourceId, action, btn) {
    if (!resourceId || !action) return;
    var payload = { action: action };
    if (action === 'restore_lifecycle') payload.action = 'restore';
    if (action === 'soft_delete' || action === 'expire' || action === 'legal_hold' || action === 'mark_reviewed') {
      var note = prompt('Lifecycle note (optional)', '') || '';
      if (note.trim()) payload.note = note.trim();
    }
    if (action === 'standard_retention') {
      payload.retention_policy = 'standard';
    }
    btn.disabled = true;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/resources/' + resourceId + '/lifecycle', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf() },
        body: JSON.stringify(payload),
      });
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Resource lifecycle update failed');
      }
      await loadResourceLibrary();
    } catch (e) {
      alert(e.message);
    } finally {
      btn.disabled = false;
    }
  }

  async function addResourceVersion(resourceId, btn) {
    if (!resourceId) return;
    var url = prompt('New version external URL');
    if (url == null) return;
    url = url.trim();
    if (!url) {
      alert('External URL is required for this version.');
      return;
    }
    var label = (prompt('Version label', 'v2') || 'v2').trim();
    var note = prompt('Change note (optional)', '') || '';
    btn.disabled = true;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/resources/' + resourceId + '/versions', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf() },
        body: JSON.stringify({
          url: url,
          version_label: label,
          change_note: note.trim(),
          requires_download: false,
        }),
      });
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Version create failed');
      }
      await loadResourceLibrary();
    } catch (e) {
      alert(e.message);
    } finally {
      btn.disabled = false;
    }
  }

  function pickResourceVersionFile() {
    return new Promise(function (resolve) {
      var input = document.createElement('input');
      input.type = 'file';
      input.accept = '.pdf,.doc,.docx,.zip,image/jpeg,image/png,image/webp,application/pdf,application/msword,application/vnd.openxmlformats-officedocument.wordprocessingml.document,application/zip';
      input.addEventListener('change', function () {
        resolve(input.files && input.files.length ? input.files[0] : null);
      }, { once: true });
      input.click();
    });
  }

  async function uploadResourceVersionFile(resourceId, btn) {
    if (!resourceId) return;
    var file = await pickResourceVersionFile();
    if (!file) return;
    var label = (prompt('Version label', 'replacement') || 'replacement').trim();
    var note = prompt('Change note (optional)', 'Binary replacement upload') || '';
    var retentionUntil = dateTimeLocalToIso('ccs-resource-retention-until');
    var reviewRequiredAt = dateTimeLocalToIso('ccs-resource-review-required-at');
    var lifecycleNotes = getInputValue('ccs-resource-lifecycle-notes').trim();
    var data = new FormData();
    data.append('file', file);
    data.append('version_label', label || 'replacement');
    if (note.trim()) data.append('change_note', note.trim());
    data.append('retention_policy', getInputValue('ccs-resource-retention-policy') || 'standard');
    if (retentionUntil) data.append('retention_until', retentionUntil);
    if (reviewRequiredAt) data.append('review_required_at', reviewRequiredAt);
    if (lifecycleNotes) data.append('document_lifecycle_notes', lifecycleNotes);
    btn.disabled = true;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/resources/' + resourceId + '/versions/upload', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'X-CSRF-Token': csrf() },
        body: data,
      });
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Version upload failed');
      }
      $('#ccs-status').textContent = 'Replacement uploaded ✓';
      setTimeout(function () { $('#ccs-status').textContent = ''; }, 1800);
      await loadResourceLibrary();
    } catch (e) {
      alert(e.message || 'Version upload failed');
    } finally {
      btn.disabled = false;
    }
  }

  async function toggleResourceVersions(resourceId, btn) {
    var target = document.getElementById('ccs-resource-versions-' + resourceId);
    if (!target) return;
    if (!target.hidden) {
      target.hidden = true;
      return;
    }
    target.hidden = false;
    target.innerHTML = '<div class="ccs-empty">Loading versions…</div>';
    btn.disabled = true;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/resources/' + resourceId + '/versions', {
        credentials: 'same-origin',
      });
      if (!res.ok) throw new Error('versions ' + res.status);
      var data = await res.json();
      var versions = Array.isArray(data.versions) ? data.versions : [];
      target.innerHTML = versions.length
        ? versionComparisonSummary(data.comparison) + versions.map(resourceVersionRow).join('')
        : '<div class="ccs-empty">No version history.</div>';
    } catch (e) {
      target.innerHTML = '<div class="ccs-empty">Failed to load versions.</div>';
    } finally {
      btn.disabled = false;
    }
  }

  function formatVersionValue(value) {
    if (value === null || typeof value === 'undefined' || value === '') return 'None';
    if (typeof value === 'boolean') return value ? 'Yes' : 'No';
    if (Array.isArray(value)) return value.join(', ');
    if (typeof value === 'object') {
      try { return JSON.stringify(value); } catch (_) { return 'Changed'; }
    }
    return String(value);
  }

  function versionComparisonSummary(comparison) {
    if (!comparison || !Array.isArray(comparison.changed_fields)) return '';
    var fields = comparison.changed_fields.slice(0, 6);
    var remaining = Math.max(0, comparison.changed_fields.length - fields.length);
    return '<section class="ccs-resource-version-compare" aria-label="Version comparison">' +
      '<div class="ccs-resource-version-compare__header">' +
        '<strong>Current vs ' + escHtml(formatVersionValue(comparison.candidate_label || 'previous version')) + '</strong>' +
        '<span>' + escHtml(String(comparison.change_count || 0)) + ' metadata change' + (Number(comparison.change_count || 0) === 1 ? '' : 's') + '</span>' +
      '</div>' +
      (fields.length ? '<dl>' + fields.map(function (field) {
        return '<div>' +
          '<dt>' + escHtml(field.label || field.field || 'Field') + '</dt>' +
          '<dd><span>Current: ' + escHtml(formatVersionValue(field.current)) + '</span><span>Candidate: ' + escHtml(formatVersionValue(field.candidate)) + '</span></dd>' +
        '</div>';
      }).join('') + '</dl>' : '<p>No metadata differences detected.</p>') +
      (remaining ? '<p class="ccs-resource-version-compare__more">+' + escHtml(String(remaining)) + ' more changes</p>' : '') +
    '</section>';
  }

  function resourceVersionRow(version) {
    var resourceId = escHtml(version.resource_id);
    var versionId = escHtml(version.id);
    var deliveryUrl = escHtml(version.delivery_url || '#');
    var reviewStatus = String(version.review_status || 'pending').replace(/[^a-z0-9_-]/gi, '').toLowerCase() || 'pending';
    var reviewCopy = resourceLabel(version.review_status || 'pending');
    var reviewNote = version.review_note ? ' · ' + version.review_note : '';
    return '<div class="ccs-resource-version-row ccs-resource-version-row--' + escHtml(reviewStatus) + '">' +
      '<span>' + escHtml(version.version_label || 'v1') + (version.is_current ? ' · current' : '') + '</span>' +
      '<span>' + escHtml(version.has_private_file ? 'Private file' : (version.external_url || 'External link')) + '</span>' +
      '<span>' + escHtml(resourceLabel(version.upload_status || 'external')) + '</span>' +
      '<span class="ccs-resource-version-row__review">' + escHtml(reviewCopy + reviewNote) + '</span>' +
      '<time>' + escHtml(version.created_at || '') + '</time>' +
      '<span class="ccs-resource-version-row__actions">' +
        '<a href="' + deliveryUrl + '" target="_blank" rel="noopener noreferrer">Open</a>' +
        (reviewStatus === 'approved' ? '' : '<button type="button" data-ccs-resource-version-action="approve" data-resource-id="' + resourceId + '" data-version-id="' + versionId + '">Approve</button>') +
        (reviewStatus === 'rejected' ? '' : '<button type="button" data-ccs-resource-version-action="reject" data-resource-id="' + resourceId + '" data-version-id="' + versionId + '">Reject</button>') +
        (version.is_current ? '' : '<button type="button" data-ccs-resource-version-action="restore" data-resource-id="' + resourceId + '" data-version-id="' + versionId + '">Restore</button>') +
      '</span>' +
    '</div>';
  }

  async function reviewResourceVersion(resourceId, versionId, action, btn) {
    if (!resourceId || !versionId || !action) return;
    var note = prompt(action === 'reject' ? 'Rejection note (required)' : 'Review note (optional)', '') || '';
    if (action === 'reject' && !note.trim()) {
      alert('A rejection note is required.');
      return;
    }
    btn.disabled = true;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/resources/' + resourceId + '/versions/' + versionId + '/review', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf() },
        body: JSON.stringify({ action: action, note: note.trim() || null }),
      });
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Version review failed');
      }
      $('#ccs-status').textContent = 'Version reviewed ✓';
      setTimeout(function () { $('#ccs-status').textContent = ''; }, 1800);
      await loadResourceLibrary();
    } catch (e) {
      alert(e.message || 'Version review failed');
    } finally {
      btn.disabled = false;
    }
  }

  async function restoreResourceVersion(resourceId, versionId, btn) {
    if (!resourceId || !versionId) return;
    if (!window.confirm('Restore this version as the current resource?')) return;
    btn.disabled = true;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/resources/' + resourceId + '/versions/' + versionId + '/restore', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'X-CSRF-Token': csrf() },
      });
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Version restore failed');
      }
      $('#ccs-status').textContent = 'Version restored ✓';
      setTimeout(function () { $('#ccs-status').textContent = ''; }, 1800);
      await loadResourceLibrary();
    } catch (e) {
      alert(e.message || 'Version restore failed');
    } finally {
      btn.disabled = false;
    }
  }

  // ─── Save / Discard ───────────────────────────────────────────────
  async function save() {
    var cur = readForm();
    var prev = STATE.saved;
    var payload = {};
    Object.keys(cur).forEach(function (key) {
      if (JSON.stringify(cur[key]) !== JSON.stringify(prev[key])) {
        payload[key] = cur[key];
      }
    });
    if (Object.keys(payload).length === 0) return;

    var btn = $('#ccs-save-btn');
    btn.disabled = true; btn.textContent = 'Saving…';
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/manage', {
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
    var reportBtn = e.target.closest('[data-ccs-report-action]');
    if (reportBtn && STATE.circle) {
      await runReportAction(
        reportBtn.getAttribute('data-ccs-report-action'),
        reportBtn.getAttribute('data-report-id'),
        reportBtn
      );
      return;
    }
    var bulkReportBtn = e.target.closest('[data-ccs-report-bulk-action]');
    if (bulkReportBtn && STATE.circle) {
      await runBulkReportAction(
        bulkReportBtn.getAttribute('data-ccs-report-bulk-action'),
        bulkReportBtn
      );
      return;
    }
    var alertBtn = e.target.closest('[data-ccs-alert-action]');
    if (alertBtn && STATE.circle) {
      await runOpsAlertAction(
        alertBtn.getAttribute('data-ccs-alert-action'),
        alertBtn.getAttribute('data-alert-id'),
        alertBtn
      );
      return;
    }
    var resourceBtn = e.target.closest('[data-ccs-resource-action]');
    if (resourceBtn && STATE.circle) {
      var resourceAction = resourceBtn.getAttribute('data-ccs-resource-action');
      var resourceId = resourceBtn.getAttribute('data-resource-id');
      if (resourceAction === 'archive') {
        await toggleResourceActive(resourceId, false, resourceBtn);
      } else if (resourceAction === 'restore') {
        await toggleResourceActive(resourceId, true, resourceBtn);
      } else if (resourceAction === 'add_version') {
        await addResourceVersion(resourceId, resourceBtn);
      } else if (resourceAction === 'upload_version') {
        await uploadResourceVersionFile(resourceId, resourceBtn);
      } else if (resourceAction === 'versions') {
        await toggleResourceVersions(resourceId, resourceBtn);
      } else if (
        resourceAction === 'mark_reviewed' ||
        resourceAction === 'legal_hold' ||
        resourceAction === 'expire' ||
        resourceAction === 'soft_delete' ||
        resourceAction === 'restore_lifecycle'
      ) {
        await runResourceLifecycleAction(resourceId, resourceAction, resourceBtn);
      }
      return;
    }
    var versionBtn = e.target.closest('[data-ccs-resource-version-action]');
    if (versionBtn && STATE.circle) {
      var versionAction = versionBtn.getAttribute('data-ccs-resource-version-action');
      if (versionAction === 'restore') {
        await restoreResourceVersion(
          versionBtn.getAttribute('data-resource-id'),
          versionBtn.getAttribute('data-version-id'),
          versionBtn
        );
      } else if (versionAction === 'approve' || versionAction === 'reject') {
        await reviewResourceVersion(
          versionBtn.getAttribute('data-resource-id'),
          versionBtn.getAttribute('data-version-id'),
          versionAction,
          versionBtn
        );
      }
      return;
    }
    if (e.target.id === 'ccs-save-btn') save();
    if (e.target.id === 'ccs-discard-btn') discard();
    if (e.target.id === 'ccs-delete-btn') openDeleteCircleConfirm();
    if (e.target.id === 'ccs-delete-confirm-cancel' || e.target.id === 'ccs-delete-confirm-close') closeDeleteCircleConfirm();
    if (e.target && e.target.getAttribute && e.target.getAttribute('data-ccs-delete-modal') === '') closeDeleteCircleConfirm();
    if (e.target.id === 'ccs-delete-confirm-submit') deleteCircle();
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

  async function runReportAction(action, reportId, btn) {
    if (!reportId || !action) return;
    var note = prompt(action === 'hide_post' ? 'Why should this post be hidden?' : 'Why is this report dismissed?');
    if (note == null) return;
    note = note.trim();
    if (!note) {
      alert('Moderation notes are required.');
      return;
    }

    btn.disabled = true;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/reports/' + reportId + '/action', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf() },
        body: JSON.stringify({ action: action, admin_notes: note }),
      });
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Report action failed');
      }
      await Promise.all([loadCircleReports(), loadManageSummary()]);
      if (window.communitySync) {
        window.communitySync.emit('circle.report_moderated', {
          circle_id: STATE.circle.id,
          report_id: reportId,
          action: action,
        });
      }
    } catch (e) {
      alert(e.message);
    } finally {
      btn.disabled = false;
    }
  }

  async function runBulkReportAction(action, btn) {
    if (!action) return;
    var ids = selectedReportIds();
    if (ids.length === 0) {
      alert('Select at least one report first.');
      return;
    }
    var note = prompt(
      action === 'hide_posts'
        ? 'Why should these posts be hidden?'
        : 'Why are these reports dismissed?'
    );
    if (note == null) return;
    note = note.trim();
    if (!note) {
      alert('Moderation notes are required.');
      return;
    }

    btn.disabled = true;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/reports/bulk-action', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf() },
        body: JSON.stringify({ action: action, report_ids: ids, admin_notes: note }),
      });
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Bulk report action failed');
      }
      await Promise.all([loadCircleReports(), loadManageSummary()]);
      if (window.communitySync) {
        window.communitySync.emit('circle.reports_bulk_moderated', {
          circle_id: STATE.circle.id,
          report_ids: ids,
          action: action,
        });
      }
    } catch (e) {
      alert(e.message);
    } finally {
      btn.disabled = false;
    }
  }

  async function runOpsAlertAction(action, alertId, btn) {
    if (!alertId || !action) return;
    var workflowState = null;
    if (action === 'set_workflow_state') {
      workflowState = promptOpsAlertWorkflowState();
      if (!workflowState) return;
    }
    var note = prompt(action === 'resolve' ? 'Resolution note (optional):' : (action === 'set_workflow_state' ? 'Workflow note (optional):' : 'Acknowledgement note (optional):'));
    if (note == null) return;
    var payload = { action: action, note: note.trim() };
    if (workflowState) payload.workflow_state = workflowState;
    btn.disabled = true;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id + '/ops-alerts/' + alertId + '/action', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf() },
        body: JSON.stringify(payload),
      });
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Ops alert action failed');
      }
      await Promise.all([loadOpsAlerts(), loadManageSummary()]);
      if (window.communitySync) {
        window.communitySync.emit('circle.ops_alert_action', {
          circle_id: STATE.circle.id,
          alert_id: alertId,
          action: action,
        });
      }
    } catch (e) {
      alert(e.message);
    } finally {
      btn.disabled = false;
    }
  }

  function openDeleteCircleConfirm() {
    var modal = $('#ccs-delete-confirm-modal');
    if (!modal || !STATE.circle) return;
    var name = $('#ccs-delete-confirm-name');
    var error = $('#ccs-delete-confirm-error');
    if (name) name.textContent = STATE.circle.name || 'This circle';
    if (error) {
      error.hidden = true;
      error.textContent = '';
    }
    modal.style.display = 'flex';
    modal.setAttribute('aria-hidden', 'false');
    var submit = $('#ccs-delete-confirm-submit');
    if (submit) submit.focus();
  }

  function closeDeleteCircleConfirm() {
    var modal = $('#ccs-delete-confirm-modal');
    if (!modal) return;
    modal.style.display = 'none';
    modal.setAttribute('aria-hidden', 'true');
    var error = $('#ccs-delete-confirm-error');
    if (error) {
      error.hidden = true;
      error.textContent = '';
    }
    var trigger = $('#ccs-delete-btn');
    if (trigger) trigger.focus();
  }

  function setDeleteError(message) {
    var error = $('#ccs-delete-confirm-error');
    if (!error) {
      alert(message);
      return;
    }
    error.textContent = message;
    error.hidden = false;
  }

  async function deleteCircle() {
    if (!STATE.circle || !STATE.circle.id) return;
    var submit = $('#ccs-delete-confirm-submit');
    if (submit) submit.disabled = true;
    try {
      var res = await fetch('/api/community/circles/' + STATE.circle.id, {
        method: 'DELETE', credentials: 'same-origin',
        headers: { 'X-CSRF-Token': csrf() },
      });
      if (!res.ok) {
        var j = await res.json().catch(function () { return {}; });
        throw new Error(j.error || j.message || 'Delete failed');
      }
      if (window.communitySync) {
        window.communitySync.emit('circle.deleted', { circle_id: STATE.circle.id });
      }
      location.href = '/community/circles';
    } catch (e) {
      setDeleteError(e.message || 'Delete failed');
      if (submit) submit.disabled = false;
    }
  }

  // ─── Boot ─────────────────────────────────────────────────────────
  function updateDescCounter() {
    var el = document.getElementById('ccs-input-desc');
    var out = document.getElementById('ccs-desc-count');
    if (el && out) out.textContent = String((el.value || '').length);
  }

  document.addEventListener('DOMContentLoaded', function () {
    loadCircle();
    wireBannerControls();
    [
      'ccs-input-name', 'ccs-input-emoji', 'ccs-input-desc', 'ccs-input-public',
      'ccs-input-category', 'ccs-input-language', 'ccs-input-location',
      'ccs-input-required-tags', 'ccs-input-media', 'ccs-input-polls',
      'ccs-input-links', 'ccs-input-anonymous', 'ccs-input-first-post-approval',
      'ccs-input-join-approval', 'ccs-input-auto-approve-verified',
      'ccs-input-announcement-comments', 'ccs-input-onboarding',
      'ccs-input-slow-mode', 'ccs-input-blocked-words', 'ccs-input-risk-keywords',
      'ccs-input-rules', 'ccs-input-disclaimer',
    ].forEach(function (id) {
      var el = document.getElementById(id);
      if (el) el.addEventListener('input', refreshFooter);
      if (el && el.type === 'checkbox') el.addEventListener('change', refreshFooter);
    });
    var slug = $('#ccs-input-slug');
    if (slug) slug.addEventListener('input', function () { updateSlugPreview(); refreshFooter(); });
    var desc = document.getElementById('ccs-input-desc');
    if (desc) {
      desc.addEventListener('input', updateDescCounter);
      updateDescCounter();
    }
    var resourceForm = document.getElementById('ccs-resource-form');
    if (resourceForm) {
      resourceForm.addEventListener('submit', createResourceFromForm);
    }
    window.addEventListener('beforeunload', function (e) {
      if (isDirty()) { e.preventDefault(); e.returnValue = ''; }
    });
  });
})();
