/**
 * Admin community settings (14.8.24).
 * Renders the key/value rows returned by GET /api/admin/community/settings
 * and submits changes back via PUT.
 */
(function () {
  'use strict';

  const STATUS = document.getElementById('admin-settings-status');
  const ROWS = document.getElementById('admin-settings-rows');
  const FORM = document.getElementById('admin-settings-form');
  let originalValues = {};

  function csrfHeaders(extra = {}) {
    const m = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]+)/);
    const token = m ? decodeURIComponent(m[1]) : '';
    return token ? { ...extra, 'X-CSRF-Token': token } : extra;
  }

  function setStatus(msg, isError = false) {
    if (!STATUS) return;
    STATUS.textContent = msg || '';
    STATUS.style.color = isError ? 'var(--btn-danger-bg, #D92D20)' : '';
  }

  function looksBoolean(value) {
    return value === 'true' || value === 'false';
  }

  function looksNumber(value) {
    return /^\d+$/.test(value);
  }

  function rowFor(setting) {
    const row = document.createElement('div');
    row.className = 'ds-form-group admin-settings-row';

    const label = document.createElement('label');
    label.className = 'ds-label';
    label.setAttribute('for', `setting-${setting.key}`);
    label.textContent = setting.key;
    row.appendChild(label);

    if (setting.description) {
      const desc = document.createElement('div');
      desc.className = 'ds-helper-text';
      desc.textContent = setting.description;
      row.appendChild(desc);
    }

    let input;
    if (looksBoolean(setting.value)) {
      input = document.createElement('select');
      input.className = 'ds-input';
      ['true', 'false'].forEach((v) => {
        const opt = document.createElement('option');
        opt.value = v;
        opt.textContent = v;
        if (v === setting.value) opt.selected = true;
        input.appendChild(opt);
      });
    } else {
      input = document.createElement('input');
      input.className = 'ds-input';
      input.type = looksNumber(setting.value) ? 'number' : 'text';
      input.value = setting.value;
    }
    input.id = `setting-${setting.key}`;
    input.name = setting.key;
    row.appendChild(input);

    originalValues[setting.key] = setting.value;
    return row;
  }

  async function load() {
    try {
      const res = await fetch('/api/admin/community/settings', {
        credentials: 'same-origin',
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      ROWS.replaceChildren();
      originalValues = {};
      (data.settings || []).forEach((s) => ROWS.appendChild(rowFor(s)));
      if ((data.settings || []).length === 0) {
        ROWS.textContent = 'No settings defined.';
      }
    } catch (e) {
      setStatus(`Failed to load settings: ${e.message}`, true);
    }
  }

  FORM.addEventListener('submit', async (e) => {
    e.preventDefault();
    const updates = [];
    Object.keys(originalValues).forEach((key) => {
      const input = document.getElementById(`setting-${key}`);
      if (!input) return;
      const next = input.value.trim();
      if (next !== originalValues[key]) {
        updates.push({ key, value: next });
      }
    });
    if (updates.length === 0) {
      setStatus('No changes.');
      return;
    }
    setStatus('Saving…');
    try {
      const res = await fetch('/api/admin/community/settings', {
        method: 'PUT',
        credentials: 'same-origin',
        headers: csrfHeaders({ 'Content-Type': 'application/json' }),
        body: JSON.stringify({ updates }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      setStatus(`Saved ${data.updated} setting${data.updated === 1 ? '' : 's'}.`);
      load();
    } catch (e) {
      setStatus(`Save failed: ${e.message}`, true);
    }
  });

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', load);
  } else {
    load();
  }
})();
