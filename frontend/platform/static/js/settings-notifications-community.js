/**
 * Community notification preferences sub-page.
 *
 * Backend (canonical):
 *   GET /api/community/notifications/preferences  → { prefs: { key: bool, ... } }
 *   PUT /api/community/notifications/preferences  body { prefs: { key: bool, ... } }
 *
 * Keys match the type_codes that `notify_user` actually emits. Missing keys
 * default to enabled server-side, so toggling stores only `false` flips.
 *
 * The legacy `/api/community/notification-prefs` endpoint (per-column on
 * community_profiles) is no longer consulted by the delivery filter and is
 * scheduled for removal — do not call it from new code.
 */
(function () {
  'use strict';

  const KEYS = [
    { key: 'mention',             name: 'Mentions',          desc: 'Someone @-mentions you in a post or comment.' },
    { key: 'new_follower',        name: 'New followers',     desc: 'Someone starts following you.' },
    { key: 'post_like',           name: 'Reactions',         desc: 'Someone reacts to one of your posts.' },
    { key: 'comment_reply',       name: 'Comments / replies', desc: 'Someone comments on or replies to your post.' },
    { key: 'ama_answer',          name: 'AMA answers',       desc: 'A question you submitted gets answered.' },
    { key: 'challenge_completed', name: 'Challenge milestones', desc: 'You complete a community challenge.' },
    { key: 'level_up',            name: 'Level-ups',         desc: 'You reach a new community level.' },
  ];

  function csrfHeaders(extra = {}) {
    const m = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]+)/);
    const token = m ? decodeURIComponent(m[1]) : '';
    return token ? { ...extra, 'X-CSRF-Token': token } : extra;
  }

  function setStatus(msg, isError) {
    const el = document.getElementById('notif-prefs-status');
    if (!el) return;
    el.textContent = msg || '';
    el.style.color = isError ? '#B42318' : '';
  }

  // In-memory mirror of the JSONB blob — needed so each toggle PUTs the full
  // object back (system A is whole-object, not per-key partial-update).
  const state = { prefs: {} };

  function isEnabled(key) {
    if (!Object.prototype.hasOwnProperty.call(state.prefs, key)) return true;
    return Boolean(state.prefs[key]);
  }

  function row(def) {
    const wrap = document.createElement('div');
    wrap.className = 'notif-prefs-row';

    const labelCol = document.createElement('div');
    labelCol.className = 'notif-prefs-row__label';
    const name = document.createElement('div');
    name.className = 'notif-prefs-row__name';
    name.textContent = def.name;
    const desc = document.createElement('div');
    desc.className = 'notif-prefs-row__desc';
    desc.textContent = def.desc;
    labelCol.append(name, desc);

    const toggle = document.createElement('label');
    toggle.className = 'notif-prefs-toggle';
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.checked = isEnabled(def.key);
    input.setAttribute('aria-label', def.name);
    const slider = document.createElement('span');
    slider.className = 'notif-prefs-toggle__slider';
    toggle.append(input, slider);

    input.addEventListener('change', async () => {
      setStatus('Saving…');
      const next = { ...state.prefs, [def.key]: input.checked };
      try {
        const res = await fetch('/api/community/notifications/preferences', {
          method: 'PUT',
          credentials: 'same-origin',
          headers: csrfHeaders({ 'Content-Type': 'application/json' }),
          body: JSON.stringify({ prefs: next }),
        });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        state.prefs = next;
        setStatus(`${def.name} ${input.checked ? 'enabled' : 'disabled'}.`);
      } catch (err) {
        input.checked = !input.checked;
        setStatus(`Save failed: ${err.message}`, true);
      }
    });

    wrap.append(labelCol, toggle);
    return wrap;
  }

  async function load() {
    const grid = document.getElementById('notif-prefs-grid');
    try {
      const res = await fetch('/api/community/notifications/preferences', { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      state.prefs = (data.prefs && typeof data.prefs === 'object') ? data.prefs : {};
      grid.replaceChildren();
      KEYS.forEach((def) => grid.appendChild(row(def)));
    } catch (err) {
      grid.textContent = '';
      setStatus(`Failed to load preferences: ${err.message}`, true);
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', load);
  } else {
    load();
  }
})();
