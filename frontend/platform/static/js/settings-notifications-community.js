/**
 * Community notification preferences sub-page (14.8.15).
 *
 * Backend:
 *   GET /api/community/notification-prefs  → 8 booleans
 *   PUT /api/community/notification-prefs  body { key?: boolean, ... }
 *
 * Each toggle PUTs only the changed key so partial-update via COALESCE
 * doesn't clobber the others.
 */
(function () {
  'use strict';

  const KEYS = [
    { key: 'post_like',    name: 'Likes',         desc: 'Someone likes one of your posts.' },
    { key: 'post_comment', name: 'Comments',      desc: 'Someone comments on one of your posts.' },
    { key: 'mention',      name: 'Mentions',      desc: 'Someone @-mentions you in a post or comment.' },
    { key: 'follow',       name: 'New followers', desc: 'Someone starts following you.' },
    { key: 'announcement', name: 'Announcements', desc: 'A platform-wide community announcement is posted.' },
    { key: 'ama',          name: 'AMAs',          desc: 'An AMA you follow has a new question or answer.' },
    { key: 'challenge',    name: 'Challenges',    desc: 'A community challenge starts or you progress.' },
    { key: 'reward',       name: 'Rewards',       desc: 'You earn a badge, XP milestone, or other reward.' },
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

  function row(def, value) {
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
    input.checked = !!value;
    input.setAttribute('aria-label', def.name);
    const slider = document.createElement('span');
    slider.className = 'notif-prefs-toggle__slider';
    toggle.append(input, slider);

    input.addEventListener('change', async () => {
      setStatus('Saving...');
      try {
        const body = {};
        body[def.key] = input.checked;
        const res = await fetch('/api/community/notification-prefs', {
          method: 'PUT',
          credentials: 'same-origin',
          headers: csrfHeaders({ 'Content-Type': 'application/json' }),
          body: JSON.stringify(body),
        });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
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
      const res = await fetch('/api/community/notification-prefs', { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      grid.replaceChildren();
      KEYS.forEach((def) => grid.appendChild(row(def, data[def.key])));
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
