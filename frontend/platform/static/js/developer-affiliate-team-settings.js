/* global window, document */

/** Settings sub-page — display_name + public_slug PATCH form. */
(function () {
  'use strict';

  const DAT = window.DAT;
  if (!DAT) return;

  /// Render a status pill next to the Save button. Replaces any existing
  /// one so repeated saves don't pile up.
  function setStatus(text, kind) {
    const form = DAT.$('#dat-settings-form');
    const actions = form && form.querySelector('.dat-form-actions');
    if (!actions) return;
    let pill = actions.querySelector('.dat-settings-status');
    if (!pill) {
      pill = document.createElement('span');
      pill.className = 'dat-settings-status';
      actions.insertBefore(pill, actions.firstChild);
    }
    pill.textContent = text;
    pill.dataset.kind = kind;
    pill.style.cssText =
      'font-size:13px;align-self:center;margin-right:8px;color:' +
      (kind === 'error' ? '#B42318' : '#137333');
    if (kind === 'success') {
      setTimeout(() => {
        if (pill.dataset.kind === 'success') pill.remove();
      }, 2500);
    }
  }

  async function onSubmit(e) {
    e.preventDefault();
    const form = DAT.$('#dat-settings-form');
    // Trigger native validation (HTML pattern on slug, maxlength on name).
    if (form && !form.checkValidity()) {
      form.reportValidity();
      return;
    }

    const name = DAT.$('#dat-team-display-name').value.trim();
    const slug = DAT.$('#dat-public-slug').value.trim();

    // Defensive client-side guard mirroring server-side validate_slug.
    if (slug && !/^[a-z0-9](?:[a-z0-9-]{1,38}[a-z0-9])?$/.test(slug)) {
      setStatus(
        "Slug must be 3–40 chars, only a–z, 0–9, '-' (not at start/end).",
        'error',
      );
      return;
    }
    if (name && name.length > 120) {
      setStatus('Display name must be 1–120 characters.', 'error');
      return;
    }

    const submitBtn = form && form.querySelector('button[type="submit"]');
    if (submitBtn) submitBtn.disabled = true;
    try {
      await DAT.apiPatch('/api/developer/affiliate/team', {
        display_name: name || null,
        public_slug: slug, // empty string clears
      });
      await DAT.loadTeamInfo();
      setStatus('Saved ✓', 'success');
    } catch (err) {
      setStatus('Save failed: ' + err.message, 'error');
    } finally {
      if (submitBtn) submitBtn.disabled = false;
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    const form = DAT.$('#dat-settings-form');
    if (form) form.addEventListener('submit', onSubmit);
  });
})();
