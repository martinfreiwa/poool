/* global window, document, navigator */

/** Settings sub-page — Team Identity + Public Join Page + Team Overview.
 *  3-card layout with sticky action bar that detects dirty-state and
 *  enables/disables Save & Discard.
 */
(function () {
  'use strict';

  const DAT = window.DAT;
  if (!DAT) return;

  // Mirror the server's validate_slug regex: must start and end with
  // [a-z0-9], 1-38 inner chars from [a-z0-9-], total length 1-40.
  const SLUG_RE = /^[a-z0-9](?:[a-z0-9-]{1,38}[a-z0-9])?$/;

  /// Snapshot of the last saved values; used to compute dirty-state and
  /// to support the Discard-changes button. Bank IBAN is tracked as the
  /// MASKED form returned by the API (e.g. "DE89 **** **** 0130") because
  /// the plaintext never round-trips back from the server.
  let _lastSaved = {
    display_name: '',
    public_slug: '',
    bank_account_holder: '',
    bank_iban: '',
    bank_bic: '',
    bank_name: '',
    bank_country: '',
    // Phase-4 branding fields. NULL on the wire = '' here so the
    // dirty-detector treats "leave empty" as clean.
    logo_url: '',
    accent_color: '',
    email_from_display: '',
  };

  /// Compact an IBAN input the same way the backend does: strip whitespace /
  /// dashes / dots and uppercase. Used for dirty-detection so re-typing the
  /// same masked value reads as "clean".
  function compactIban(v) {
    return (v || '').replace(/[\s\-.]/g, '').toUpperCase();
  }

  // ─── Dirty-state + footer wiring ─────────────────────────────────
  function currentValues() {
    return {
      display_name: (DAT.$('#dat-team-display-name')?.value || '').trim(),
      public_slug:  (DAT.$('#dat-public-slug')?.value || '').trim().toLowerCase(),
      bank_account_holder: (DAT.$('#dat-bank-account-holder')?.value || '').trim(),
      bank_iban: compactIban(DAT.$('#dat-bank-iban')?.value || ''),
      bank_bic:  (DAT.$('#dat-bank-bic')?.value || '').replace(/\s/g, '').toUpperCase(),
      bank_name: (DAT.$('#dat-bank-name')?.value || '').trim(),
      bank_country: (DAT.$('#dat-bank-country')?.value || '').trim().toUpperCase(),
      logo_url:           (DAT.$('#dat-branding-logo-url')?.value || '').trim(),
      accent_color:       (DAT.$('#dat-branding-accent')?.value || '').trim().toUpperCase(),
      email_from_display: (DAT.$('#dat-branding-from-display')?.value || '').trim(),
    };
  }
  function isDirty() {
    const cur = currentValues();
    return cur.display_name        !== _lastSaved.display_name
        || cur.public_slug         !== _lastSaved.public_slug
        || cur.bank_account_holder !== _lastSaved.bank_account_holder
        || cur.bank_iban           !== _lastSaved.bank_iban
        || cur.bank_bic            !== _lastSaved.bank_bic
        || cur.bank_name           !== _lastSaved.bank_name
        || cur.bank_country        !== _lastSaved.bank_country
        || cur.logo_url            !== _lastSaved.logo_url
        || cur.accent_color        !== _lastSaved.accent_color
        || cur.email_from_display  !== _lastSaved.email_from_display;
  }
  function refreshFooter() {
    const dirty = isDirty();
    const footer = DAT.$('#dat-settings-footer');
    const save = DAT.$('#dat-settings-save');
    const cancel = DAT.$('#dat-settings-cancel');
    if (footer) footer.dataset.state = dirty ? 'dirty' : 'pristine';
    if (save) save.disabled = !dirty;
    if (cancel) cancel.disabled = !dirty;
  }

  // ─── Slug live preview + public-link card ────────────────────────
  function renderPublicLink() {
    const slug = (DAT.$('#dat-public-slug')?.value || '').trim().toLowerCase();
    const link = DAT.$('#dat-public-link');
    const empty = DAT.$('#dat-public-empty');
    const url = DAT.$('#dat-public-link-url');
    const openA = DAT.$('#dat-public-open');
    const status = DAT.$('#dat-public-status');
    const hint = DAT.$('#dat-public-slug-hint');
    const slugInp = DAT.$('#dat-public-slug');

    if (!slug) {
      if (link) link.hidden = true;
      if (empty) empty.hidden = false;
      if (status) {
        status.textContent = 'Not published';
        status.dataset.status = 'off';
        status.className = 'dat-status dat-status--muted';
      }
      slugInp?.removeAttribute('aria-invalid');
      hint?.classList.remove('dat-form-row__hint--error');
      return;
    }

    const valid = SLUG_RE.test(slug);
    slugInp?.setAttribute('aria-invalid', valid ? 'false' : 'true');
    hint?.classList.toggle('dat-form-row__hint--error', !valid);

    if (!valid) {
      if (link) link.hidden = true;
      if (empty) {
        empty.hidden = false;
        empty.classList.add('dat-public-empty--error');
      }
      if (status) {
        status.textContent = 'Invalid slug';
        status.dataset.status = 'invalid';
        status.className = 'dat-status dat-status--removed';
      }
      return;
    }

    if (empty) {
      empty.hidden = true;
      empty.classList.remove('dat-public-empty--error');
    }
    if (link) link.hidden = false;
    const fullUrl = location.origin + '/affiliate/join/' + slug;
    if (url) url.textContent = fullUrl;
    if (openA) openA.href = fullUrl;
    if (status) {
      status.textContent = 'Live';
      status.dataset.status = 'live';
      status.className = 'dat-status dat-status--active';
    }
  }

  // ─── Team status pill (top of Identity card) ─────────────────────
  function renderTeamStatus(data) {
    const pill = DAT.$('#dat-team-status');
    if (!pill || !data) return;
    const s = data.status || 'active';
    pill.textContent = DAT.humanize(s);
    pill.dataset.status = s;
    pill.className = 'dat-status dat-status--' + s;
    pill.title = `Team status: ${DAT.humanize(s)}`;
  }

  // ─── Overview tiles (read-only) ──────────────────────────────────
  function renderOverview(data) {
    if (!data) return;
    const set = (id, value) => { const el = DAT.$('#' + id); if (el) el.textContent = value; };
    const c = data.counters || {};
    set('dat-ov-members', String(data.active_members ?? 0));
    set('dat-ov-revenue', DAT.fmtCents(c.lifetime_revenue_cents || 0));
    set('dat-ov-commission', DAT.fmtCents(c.lifetime_commission_cents || 0));
    set('dat-ov-created', data.created_at ? DAT.fmtDate(data.created_at) : '—');
  }

  // ─── Save / Discard flow ─────────────────────────────────────────
  function setStatus(text, kind) {
    const el = DAT.$('#dat-settings-status');
    if (!el) return;
    el.textContent = text || '';
    el.dataset.kind = kind || 'info';
    if (kind === 'success') {
      setTimeout(() => { if (el.dataset.kind === 'success') el.textContent = ''; }, 2500);
    }
  }
  function setBusy(busy) {
    const btn = DAT.$('#dat-settings-save');
    if (!btn) return;
    btn.disabled = busy || !isDirty();
    btn.classList.toggle('ds-btn--loading', !!busy);
    const spinner = btn.querySelector('.dat-btn__spinner');
    if (spinner) spinner.hidden = !busy;
    btn.setAttribute('aria-busy', busy ? 'true' : 'false');
  }

  async function onSubmit(e) {
    e.preventDefault();
    const nameInp = DAT.$('#dat-team-display-name');
    const slugInp = DAT.$('#dat-public-slug');
    const cur = currentValues();

    if (!cur.display_name) {
      setStatus('Display name is required.', 'error');
      nameInp?.focus();
      return;
    }
    if (cur.display_name.length > 120) {
      setStatus('Display name must be 1–120 characters.', 'error');
      nameInp?.focus();
      return;
    }
    if (cur.public_slug && !SLUG_RE.test(cur.public_slug)) {
      setStatus("Slug must be 3–40 chars: a–z, 0–9, '-' (not at start/end).", 'error');
      slugInp?.focus();
      return;
    }

    setBusy(true);
    setStatus('Saving…', 'info');
    try {
      // Only include bank fields that actually changed. This lets the user
      // edit team identity without re-touching banking — and keeps the
      // server-side IBAN value intact when the field still shows its masked
      // form (which is dirty-equal to _lastSaved.bank_iban → not included).
      const patch = {
        display_name: cur.display_name,
        public_slug:  cur.public_slug,
      };
      if (cur.bank_account_holder !== _lastSaved.bank_account_holder) patch.bank_account_holder = cur.bank_account_holder;
      if (cur.bank_iban           !== _lastSaved.bank_iban)           patch.bank_iban           = cur.bank_iban;
      if (cur.bank_bic            !== _lastSaved.bank_bic)            patch.bank_bic            = cur.bank_bic;
      if (cur.bank_name           !== _lastSaved.bank_name)           patch.bank_name           = cur.bank_name;
      if (cur.bank_country        !== _lastSaved.bank_country)        patch.bank_country        = cur.bank_country;
      // Phase-4 branding fields
      if (cur.logo_url           !== _lastSaved.logo_url)           patch.logo_url           = cur.logo_url;
      if (cur.accent_color       !== _lastSaved.accent_color)       patch.accent_color       = cur.accent_color;
      if (cur.email_from_display !== _lastSaved.email_from_display) patch.email_from_display = cur.email_from_display;

      await DAT.apiPatch('/api/developer/affiliate/team', patch);
      // Refresh shared team-info (header etc.) + local snapshot
      const data = await DAT.apiGet('/api/developer/affiliate/team');
      hydrate(data);
      refreshFooter();
      setStatus('Saved ✓', 'success');
      DAT.toast('Settings saved', 'Your team settings are updated.', 'success');
    } catch (err) {
      setStatus('Save failed.', 'error');
      DAT.toast('Save failed', err.message || 'Could not save team settings.', 'error');
    } finally {
      setBusy(false);
    }
  }

  function onDiscard() {
    const nameInp = DAT.$('#dat-team-display-name');
    const slugInp = DAT.$('#dat-public-slug');
    if (nameInp) nameInp.value = _lastSaved.display_name;
    if (slugInp) slugInp.value = _lastSaved.public_slug;
    const setVal = (id, v) => { const el = DAT.$('#' + id); if (el) el.value = v || ''; };
    setVal('dat-bank-account-holder', _lastSaved.bank_account_holder);
    setVal('dat-bank-iban',           _lastSaved.bank_iban);
    setVal('dat-bank-bic',            _lastSaved.bank_bic);
    setVal('dat-bank-name',           _lastSaved.bank_name);
    setVal('dat-bank-country',        _lastSaved.bank_country);
    setVal('dat-branding-logo-url',    _lastSaved.logo_url);
    setVal('dat-branding-accent',      _lastSaved.accent_color);
    setVal('dat-branding-from-display', _lastSaved.email_from_display);
    const picker = DAT.$('#dat-branding-accent-picker');
    if (picker) picker.value = _lastSaved.accent_color || '#0000FF';
    renderPublicLink();
    renderBrandingPreview();
    refreshFooter();
    setStatus('Changes discarded.', 'info');
    setTimeout(() => setStatus('', 'info'), 1500);
  }

  // ─── Copy public-join URL to clipboard ───────────────────────────
  async function copyPublicUrl() {
    const url = DAT.$('#dat-public-link-url')?.textContent?.trim();
    if (!url) return;
    try {
      await navigator.clipboard.writeText(url);
      DAT.toast('Copied', 'Public join URL copied to clipboard.', 'success');
      const btn = DAT.$('#dat-public-copy');
      if (btn) {
        const label = btn.querySelector('span');
        const original = label?.textContent || 'Copy';
        if (label) label.textContent = 'Copied ✓';
        setTimeout(() => { if (label) label.textContent = original; }, 1500);
      }
    } catch (e) {
      DAT.toast('Copy failed', 'Your browser blocked clipboard access.', 'error');
    }
  }

  // ─── Initial hydrate from /api/developer/affiliate/team ──────────
  function hydrate(data) {
    if (!data) return;
    const nameInp = DAT.$('#dat-team-display-name');
    const slugInp = DAT.$('#dat-public-slug');
    if (nameInp) nameInp.value = data.display_name || '';
    if (slugInp) slugInp.value = data.public_slug || '';

    // Bank section — the server returns IBAN masked. We populate the visible
    // input with the masked form and remember that exact string as "saved",
    // so the dirty-detector treats "no edit" as clean even though the field
    // doesn't contain the real value.
    const bank = data.bank || {};
    const setVal = (id, v) => { const el = DAT.$('#' + id); if (el) el.value = v || ''; };
    setVal('dat-bank-account-holder', bank.account_holder);
    setVal('dat-bank-iban',           bank.iban_masked);
    setVal('dat-bank-bic',            bank.bic);
    setVal('dat-bank-name',           bank.bank_name);
    setVal('dat-bank-country',        bank.country);
    const bankStatus = DAT.$('#dat-bank-status');
    if (bankStatus) {
      const ok = !!bank.iban_set && !!bank.account_holder;
      bankStatus.textContent = ok ? 'Configured' : 'Not configured';
      bankStatus.dataset.status = ok ? 'active' : 'off';
      bankStatus.className = ok ? 'dat-status dat-status--active' : 'dat-status dat-status--muted';
    }

    // Phase-4 branding hydrate.
    const brand = data.branding || {};
    const setBrandVal = (id, v) => { const el = DAT.$('#' + id); if (el) el.value = v || ''; };
    setBrandVal('dat-branding-logo-url',     brand.logo_url);
    setBrandVal('dat-branding-accent',       brand.accent_color);
    setBrandVal('dat-branding-from-display', brand.email_from_display);
    const accentPicker = DAT.$('#dat-branding-accent-picker');
    if (accentPicker) accentPicker.value = brand.accent_color || '#0000FF';

    _lastSaved = {
      display_name: data.display_name || '',
      public_slug:  data.public_slug || '',
      bank_account_holder: bank.account_holder || '',
      bank_iban:           compactIban(bank.iban_masked || ''),
      bank_bic:            (bank.bic || '').toUpperCase(),
      bank_name:           bank.bank_name || '',
      bank_country:        (bank.country || '').toUpperCase(),
      logo_url:            brand.logo_url || '',
      accent_color:        (brand.accent_color || '').toUpperCase(),
      email_from_display:  brand.email_from_display || '',
    };
    renderTeamStatus(data);
    renderOverview(data);
    renderPublicLink();
    renderBrandingPreview();
    refreshFooter();
  }

  // Phase-4: live preview of the email CTA button.
  function renderBrandingPreview() {
    const logoUrl = (DAT.$('#dat-branding-logo-url')?.value || '').trim();
    const accent = (DAT.$('#dat-branding-accent')?.value || '#0000FF').trim() || '#0000FF';
    const valid = /^#[0-9A-Fa-f]{6}$/.test(accent);
    const btn = DAT.$('#dat-branding-preview-btn');
    if (btn) btn.style.background = valid ? accent : '#0000FF';
    const logo = DAT.$('#dat-branding-preview-logo');
    if (logo) {
      if (logoUrl && logoUrl.startsWith('https://')) {
        logo.src = logoUrl;
        logo.style.display = '';
      } else {
        logo.removeAttribute('src');
        logo.style.display = 'none';
      }
    }
  }

  document.addEventListener('DOMContentLoaded', async () => {
    const form = DAT.$('#dat-settings-form');
    if (form) form.addEventListener('submit', onSubmit);
    DAT.$('#dat-settings-cancel')?.addEventListener('click', onDiscard);
    DAT.$('#dat-public-copy')?.addEventListener('click', copyPublicUrl);

    // Bind input → live re-render of preview + footer
    DAT.$('#dat-team-display-name')?.addEventListener('input', refreshFooter);
    const slugInp = DAT.$('#dat-public-slug');
    if (slugInp) {
      slugInp.addEventListener('input', () => { renderPublicLink(); refreshFooter(); });
      slugInp.addEventListener('blur', renderPublicLink);
    }
    // Bank-detail inputs — dirty-state on every keystroke.
    ['dat-bank-account-holder', 'dat-bank-iban', 'dat-bank-bic', 'dat-bank-name', 'dat-bank-country']
      .forEach((id) => DAT.$('#' + id)?.addEventListener('input', refreshFooter));

    // Phase-4 branding inputs.
    ['dat-branding-logo-url', 'dat-branding-accent', 'dat-branding-from-display']
      .forEach((id) => DAT.$('#' + id)?.addEventListener('input', () => {
        renderBrandingPreview();
        refreshFooter();
      }));
    // Color picker ↔ hex text-input sync.
    const picker = DAT.$('#dat-branding-accent-picker');
    const accent = DAT.$('#dat-branding-accent');
    if (picker && accent) {
      picker.addEventListener('input', () => {
        accent.value = picker.value.toUpperCase();
        renderBrandingPreview();
        refreshFooter();
      });
      accent.addEventListener('input', () => {
        const v = accent.value.trim();
        if (/^#[0-9A-Fa-f]{6}$/.test(v)) picker.value = v;
      });
    }

    // Warn before navigating away with unsaved changes.
    window.addEventListener('beforeunload', (e) => {
      if (!isDirty()) return;
      e.preventDefault();
      e.returnValue = '';
    });

    // Initial fetch.
    try {
      const data = await DAT.apiGet('/api/developer/affiliate/team');
      hydrate(data);
    } catch (err) {
      DAT.toast('Settings', 'Could not load team data. Try refreshing.', 'error');
      console.error(err);
    }

    // Populate the next-payout tiles from the analytics overview endpoint
    // (read-only, no impact on dirty-state). Failure is silent — the tiles
    // already show "—" placeholders so a flaky analytics fetch just leaves
    // them as-is rather than crashing the settings page.
    try {
      const res = await DAT.apiGet('/api/developer/affiliate/team/analytics/overview');
      const ov = (res && res.overview) || {};
      const amountEl = DAT.$('#dat-k-next-amount');
      const dateEl   = DAT.$('#dat-k-next-date');
      if (amountEl) {
        const cents = ov.next_payout_amount_cents || 0;
        amountEl.textContent = '€' + (cents / 100).toLocaleString('en-US', {
          minimumFractionDigits: 2, maximumFractionDigits: 2,
        });
      }
      if (dateEl) {
        dateEl.textContent = ov.next_payout_date
          ? 'Earliest: ' + new Date(ov.next_payout_date).toLocaleDateString()
          : 'No holdback active';
      }
    } catch (_) { /* analytics endpoint optional on settings page */ }
  });
})();
