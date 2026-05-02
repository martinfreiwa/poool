/**
 * Compliance & OJK Reports — mp-compliance.js
 *
 * Handles:
 *   - Dynamic dropdowns (year/quarter populated from current date)
 *   - Compliance health banner with auto-refresh + last-updated counter
 *   - Pre-export summary (rows, excluded, est. size, last-export hash)
 *   - Pre-export confirmation modal with re-generation warning + focus-trap
 *   - Date-range presets (7d, 30d, this quarter, YTD)
 *   - Format selection per export (CSV / JSON)
 *   - Export history table with mark-as-submitted action
 */
(function () {
  'use strict';

  // ── Formatting helpers ────────────────────────────────────────
  const fmtNum = (n) => Number(n || 0).toLocaleString('en-US');
  const fmtBytes = (b) => {
    const n = Number(b || 0);
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / 1048576).toFixed(1)} MB`;
    return `${(n / 1073741824).toFixed(2)} GB`;
  };
  const fmtRelative = (iso) => {
    if (!iso) return 'Never';
    const t = new Date(iso).getTime();
    const diff = (Date.now() - t) / 1000;
    if (diff < 60) return 'just now';
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    if (diff < 604800) return `${Math.floor(diff / 86400)}d ago`;
    return new Date(iso).toLocaleDateString();
  };
  const fmtIsoDate = (d) => {
    if (!d) return '';
    if (typeof d === 'string') return d.slice(0, 10);
    const yr = d.getFullYear();
    const mo = String(d.getMonth() + 1).padStart(2, '0');
    const dy = String(d.getDate()).padStart(2, '0');
    return `${yr}-${mo}-${dy}`;
  };
  const fmtPrettyDate = (d) => {
    if (!d) return '';
    const dt = typeof d === 'string' ? new Date(d) : d;
    return dt.toLocaleDateString(undefined, { day: '2-digit', month: 'short', year: 'numeric' });
  };
  const truncHash = (h) => (h ? `${h.slice(0, 8)}…${h.slice(-4)}` : '—');
  const escapeHtml = (s) =>
    String(s == null ? '' : s)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;').replace(/'/g, '&#39;');

  const TYPE_LABELS = {
    ojk_quarterly: 'OJK Quarterly',
    travel_rule: 'Travel Rule',
    tax_fiscal: 'Tax / Fiscal',
  };

  const ENDPOINT = {
    ojk_quarterly: 'ojk-report',
    travel_rule: 'travel-rule',
    tax_fiscal: 'tax-export',
  };

  // ── State ────────────────────────────────────────────────────
  const state = {
    summary: null,
    pendingExport: null,
    autoRefreshTimer: null,
    triggerEl: null,
    focusTrapHandler: null,
  };

  function toast(message, kind) {
    if (typeof mpToast !== 'undefined') mpToast(message, kind || 'info');
  }

  // ── Dynamic dropdown population ──────────────────────────────
  function currentQuarter() {
    const now = new Date();
    const q = Math.floor(now.getMonth() / 3) + 1;
    return { year: now.getFullYear(), q };
  }

  function populateDropdowns() {
    const cur = currentQuarter();
    const sel = document.getElementById('ojk-quarter');
    if (sel) {
      const items = [];
      for (let yOff = 0; yOff <= 2; yOff += 1) {
        const yr = cur.year - yOff;
        for (let q = 4; q >= 1; q -= 1) {
          const isFuture = (yr > cur.year) || (yr === cur.year && q > cur.q);
          if (isFuture) continue;
          const isCurrent = yr === cur.year && q === cur.q;
          const months = ['Jan – Mar', 'Apr – Jun', 'Jul – Sep', 'Oct – Dec'][q - 1];
          items.push(
            `<option value="${yr}-Q${q}"${(yOff === 0 && q === cur.q - 1) || (cur.q === 1 && yOff === 1 && q === 4) ? ' selected' : ''}>` +
            `Q${q} ${yr} (${months})${isCurrent ? ' — in progress' : ''}</option>`
          );
        }
      }
      sel.innerHTML = items.join('');
    }

    const taxSel = document.getElementById('tax-year');
    if (taxSel) {
      const items = [];
      const lastFy = cur.year - 1;
      for (let yr = lastFy; yr >= lastFy - 4; yr -= 1) {
        const sel = yr === lastFy ? ' selected' : '';
        items.push(`<option value="${yr}"${sel}>FY ${yr}</option>`);
      }
      taxSel.innerHTML = items.join('');
    }

    const histFilter = document.getElementById('mp-history-filter');
    if (histFilter && !histFilter.dataset.populated) histFilter.dataset.populated = '1';
  }

  // ── Health banner render ─────────────────────────────────────
  function renderHealthBanner(summary) {
    const banner = document.getElementById('mp-compliance-health');
    const cutoff = document.getElementById('mp-health-cutoff');
    const wrap = document.getElementById('mp-health-deadlines');
    if (!banner || !wrap) return;

    const order = { overdue: 3, due_soon: 2, pending: 1, submitted: 0 };
    const worst = (summary.deadlines || []).reduce(
      (acc, d) => (order[d.status] || 0) > (order[acc] || 0) ? d.status : acc,
      'submitted'
    );
    banner.dataset.state = worst === 'overdue' ? 'overdue'
      : worst === 'due_soon' ? 'due_soon' : 'ok';

    cutoff.textContent = `Data as of ${new Date(summary.generated_at).toLocaleString()}`;

    wrap.innerHTML = (summary.deadlines || []).map((d) => {
      const lbl = TYPE_LABELS[d.export_type] || d.export_type;
      let dueText;
      if (d.status === 'submitted') {
        dueText = `Submitted ${fmtRelative(d.last_submitted_at)} ✓`;
      } else if (d.status === 'overdue') {
        dueText = `Overdue by ${Math.abs(d.days_until_due)} day${Math.abs(d.days_until_due) === 1 ? '' : 's'}`;
      } else {
        dueText = `Due ${fmtPrettyDate(d.due_date)} (${d.days_until_due}d)`;
      }
      return `
        <div class="mp-deadline" data-status="${escapeHtml(d.status)}">
          <span class="mp-deadline__label">${escapeHtml(lbl)}</span>
          <span class="mp-deadline__period">${escapeHtml(d.period_label)}</span>
          <span class="mp-deadline__due">${escapeHtml(dueText)}</span>
        </div>`;
    }).join('') || '<span class="mp-deadline__label">No upcoming deadlines.</span>';
  }

  // ── Card preview render ──────────────────────────────────────
  function renderPreviews(summary) {
    (summary.previews || []).forEach((p) => {
      const card = document.querySelector(`[data-export-type="${p.export_type}"]`);
      if (!card) return;
      const set = (sel, val) => {
        const el = card.querySelector(sel);
        if (el) el.textContent = val;
      };
      set('[data-preview-rows]', fmtNum(p.row_count));
      set('[data-preview-excluded]', fmtNum(p.excluded_count || 0));
      set('[data-preview-size]', fmtBytes(p.estimated_bytes));
      const last = card.querySelector('[data-preview-last]');
      if (last) {
        last.textContent = p.last_export_at
          ? `${fmtRelative(p.last_export_at)} · ${truncHash(p.last_export_hash)}`
          : 'Never';
      }
      const badge = card.querySelector('[data-status-badge]');
      if (badge) {
        const deadline = (summary.deadlines || []).find((d) => d.export_type === p.export_type);
        if (deadline) {
          badge.dataset.state = deadline.status;
          badge.textContent = deadline.status === 'submitted' ? 'Submitted'
            : deadline.status === 'overdue' ? 'Overdue'
              : deadline.status === 'due_soon' ? 'Due soon' : 'Pending';
        } else {
          badge.dataset.state = '';
          badge.textContent = '—';
        }
      }
    });
  }

  // ── Last-refreshed ticker ────────────────────────────────────
  let lastRefreshTs = null;
  function tickRefreshLabel() {
    const el = document.getElementById('mp-health-refreshed');
    if (!el) return;
    if (!lastRefreshTs) { el.textContent = 'never'; return; }
    el.textContent = `updated ${fmtRelative(lastRefreshTs)}`;
  }

  // ── Fetch summary ────────────────────────────────────────────
  async function refreshSummary() {
    const banner = document.getElementById('mp-compliance-health');
    if (banner) banner.dataset.state = 'loading';

    const params = new URLSearchParams();
    const q = document.getElementById('ojk-quarter')?.value;
    const fromD = document.getElementById('aml-start')?.value;
    const toD = document.getElementById('aml-end')?.value;
    const yr = document.getElementById('tax-year')?.value;
    if (q) params.set('quarter', q);
    if (fromD) params.set('from_date', fromD);
    if (toD) params.set('to_date', toD);
    if (yr) params.set('year', yr);

    try {
      const res = await fetch(`/api/admin/marketplace/compliance/summary?${params}`, {
        credentials: 'same-origin',
        headers: { Accept: 'application/json' },
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      state.summary = data;
      lastRefreshTs = new Date().toISOString();
      renderHealthBanner(data);
      renderPreviews(data);
      tickRefreshLabel();
    } catch (e) {
      const cutoff = document.getElementById('mp-health-cutoff');
      if (cutoff) cutoff.textContent = `Failed to load summary: ${e.message}`;
      if (banner) banner.dataset.state = 'error';
    }
  }

  function startAutoRefresh() {
    if (state.autoRefreshTimer) clearInterval(state.autoRefreshTimer);
    state.autoRefreshTimer = setInterval(() => {
      if (document.visibilityState === 'visible') refreshSummary();
    }, 60_000);
    setInterval(tickRefreshLabel, 15_000);
  }

  // ── Modal focus-trap ─────────────────────────────────────────
  function trapFocus(panel) {
    const selector = 'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';
    const handler = (e) => {
      if (e.key !== 'Tab') return;
      const els = Array.from(panel.querySelectorAll(selector)).filter((el) => !el.disabled);
      if (!els.length) return;
      const first = els[0];
      const last = els[els.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        last.focus(); e.preventDefault();
      } else if (!e.shiftKey && document.activeElement === last) {
        first.focus(); e.preventDefault();
      }
    };
    panel.addEventListener('keydown', handler);
    return () => panel.removeEventListener('keydown', handler);
  }

  function showConfirmModal({ type, params, button, filename, summarySection, format }) {
    const modal = document.getElementById('mp-confirm-modal');
    const body = document.getElementById('mp-confirm-body');
    const goBtn = document.getElementById('mp-confirm-go');
    if (!modal || !body || !goBtn) return;

    state.pendingExport = { type, params, button, filename };
    state.triggerEl = document.activeElement;

    const lbl = TYPE_LABELS[type] || type;
    const period = summarySection?.period_label || '—';
    const rows = fmtNum(summarySection?.row_count ?? 0);
    const excluded = fmtNum(summarySection?.excluded_count ?? 0);
    const size = fmtBytes(summarySection?.estimated_bytes ?? 0);
    const cutoff = summarySection?.data_cutoff
      ? new Date(summarySection.data_cutoff).toLocaleString() : '—';
    const lastAt = summarySection?.last_export_at;
    const lastCount = summarySection?.last_export_count ?? 0;
    const lastHash = summarySection?.last_export_hash;

    let warnHtml = '';
    if (lastAt) {
      const sameSize = Number(lastCount) === Number(summarySection?.row_count);
      warnHtml = `
        <div class="mp-modal__warn">
          ⚠️ This period was already exported ${fmtRelative(lastAt)}
          (${fmtNum(lastCount)} rows · <span class="mp-hash">${truncHash(lastHash)}</span>).
          ${sameSize
            ? 'Re-running should produce an identical file.'
            : `Current preview shows <strong>${rows}</strong> rows — content may have changed since the last run.`}
        </div>`;
    }

    body.innerHTML = `
      <dl class="mp-modal__summary">
        <dt>Report</dt><dd>${escapeHtml(lbl)}</dd>
        <dt>Period</dt><dd>${escapeHtml(period)}</dd>
        <dt>Format</dt><dd>${escapeHtml((format || 'csv').toUpperCase())}</dd>
        <dt>Rows</dt><dd>${rows}</dd>
        <dt>Excluded</dt><dd>${excluded}</dd>
        <dt>Estimated size</dt><dd>${size}</dd>
        <dt>Data cutoff</dt><dd>${escapeHtml(cutoff)}</dd>
        <dt>Filename</dt><dd>${escapeHtml(filename)}</dd>
      </dl>
      ${warnHtml}
      <p class="mp-field__hint" style="margin-top:12px;">
        This download will be recorded in the audit log with your user ID, timestamp, and a SHA-256 content hash.
      </p>`;
    modal.hidden = false;

    const panel = modal.querySelector('.mp-modal__panel');
    state.focusTrapHandler = trapFocus(panel);
    setTimeout(() => goBtn.focus(), 0);
  }

  function closeModal() {
    const modal = document.getElementById('mp-confirm-modal');
    if (modal) modal.hidden = true;
    state.pendingExport = null;
    if (state.focusTrapHandler) { state.focusTrapHandler(); state.focusTrapHandler = null; }
    if (state.triggerEl && typeof state.triggerEl.focus === 'function') {
      state.triggerEl.focus();
      state.triggerEl = null;
    }
  }

  // ── Download helpers ─────────────────────────────────────────
  function setButtonLoading(btn, isLoading) {
    btn.disabled = isLoading;
    btn.setAttribute('aria-busy', String(isLoading));
    if (isLoading) {
      btn.dataset.originalHtml = btn.innerHTML;
      btn.textContent = 'Generating…';
    } else if (btn.dataset.originalHtml) {
      btn.innerHTML = btn.dataset.originalHtml;
      delete btn.dataset.originalHtml;
    }
  }

  function showStatus(btn, message, kind) {
    let s = btn.parentElement?.querySelector('.mp-export-status');
    if (!s) {
      s = document.createElement('div');
      s.className = 'mp-export-status';
      s.setAttribute('role', 'status');
      s.setAttribute('aria-live', 'polite');
      btn.parentElement?.appendChild(s);
    }
    s.textContent = message || '';
    s.style.color = kind === 'error' ? 'var(--admin-danger, #DC2626)' : 'var(--admin-text-muted)';
  }

  async function performDownload(url, filename, button, isJson) {
    setButtonLoading(button, true);
    showStatus(button, `Preparing ${filename}…`);
    try {
      const accept = isJson ? 'application/json' : 'text/csv';
      const res = await fetch(url, {
        credentials: 'same-origin',
        headers: { Accept: accept },
      });
      if (!res.ok) {
        let detail = '';
        try { const b = await res.json(); detail = b.error ? ` ${b.error}` : ''; } catch (_) {}
        throw new Error(`Export failed (HTTP ${res.status})${detail}`);
      }
      const ct = res.headers.get('content-type') || '';
      const expected = isJson ? 'application/json' : 'text/csv';
      if (!ct.includes(expected)) throw new Error('Unexpected response type.');
      const blob = await res.blob();
      const obj = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = obj;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(obj);
      showStatus(button, `${filename} downloaded.`);
      toast(`${filename} downloaded.`, 'success');
      refreshSummary();
      refreshHistory();
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Export failed.';
      showStatus(button, msg, 'error');
      toast(msg, 'error');
    } finally {
      setButtonLoading(button, false);
    }
  }

  function findPreview(type) {
    return (state.summary?.previews || []).find((p) => p.export_type === type) || null;
  }

  function getFormat(type) {
    const sel = document.querySelector(`[data-format-for="${type}"]`);
    return (sel?.value || 'csv').toLowerCase();
  }

  // ── Export triggers ──────────────────────────────────────────
  function validateDateRange(start, end) {
    if (start && end && start > end) return 'Start date cannot be after end date.';
    return '';
  }

  function setupExportButtons() {
    const ojkBtn = document.getElementById('btn-export-ojk');
    if (ojkBtn) {
      ojkBtn.addEventListener('click', () => {
        const quarter = document.getElementById('ojk-quarter')?.value;
        const fmt = getFormat('ojk_quarterly');
        showConfirmModal({
          type: 'ojk_quarterly',
          params: `quarter=${encodeURIComponent(quarter)}&format=${fmt}`,
          button: ojkBtn,
          filename: `ojk_report_${quarter}.${fmt}`,
          summarySection: findPreview('ojk_quarterly'),
          format: fmt,
        });
      });
    }

    const amlBtn = document.getElementById('btn-export-aml');
    if (amlBtn) {
      amlBtn.addEventListener('click', async () => {
        const start = document.getElementById('aml-start')?.value || '';
        const end = document.getElementById('aml-end')?.value || '';
        const err = validateDateRange(start, end);
        const errEl = document.getElementById('aml-error');
        if (errEl) errEl.textContent = err || '';
        if (err) { toast(err, 'error'); return; }
        if (!start || !end) {
          if (errEl) errEl.textContent = 'Both start and end dates required for travel-rule export.';
          return;
        }
        // 4-eye: create approval request, do NOT trigger download.
        const reason = window.prompt('Reason for this PII export (visible to second admin):', '');
        if (reason === null) return;
        try {
          const res = await fetch('/api/admin/marketplace/compliance/requests', {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              export_type: 'travel_rule',
              period_label: `${start}..${end}`,
              period_start: start,
              period_end: end,
              reason: reason || null,
            }),
          });
          if (!res.ok) {
            const body = await res.json().catch(() => ({}));
            throw new Error(body.error || `HTTP ${res.status}`);
          }
          toast('Approval request created. A second admin must approve before download.', 'success');
          refreshApprovals();
        } catch (e) {
          toast(`Could not create request: ${e.message}`, 'error');
        }
      });
    }

    const taxBtn = document.getElementById('btn-export-tax');
    if (taxBtn) {
      taxBtn.addEventListener('click', () => {
        const year = document.getElementById('tax-year')?.value || '2025';
        const fmt = getFormat('tax_fiscal');
        showConfirmModal({
          type: 'tax_fiscal',
          params: `year=${encodeURIComponent(year)}&format=${fmt}`,
          button: taxBtn,
          filename: `tax_export_FY${year}.${fmt}`,
          summarySection: findPreview('tax_fiscal'),
          format: fmt,
        });
      });
    }

    document.getElementById('mp-confirm-go')?.addEventListener('click', () => {
      const p = state.pendingExport;
      if (!p) return;
      const url = `/api/admin/marketplace/compliance/${ENDPOINT[p.type]}${p.params ? `?${p.params}` : ''}`;
      const button = p.button;
      const filename = p.filename;
      const isJson = filename.endsWith('.json');
      closeModal();
      performDownload(url, filename, button, isJson);
    });

    document.querySelectorAll('[data-modal-close]').forEach((el) => {
      el.addEventListener('click', closeModal);
    });
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && !document.getElementById('mp-confirm-modal')?.hidden) {
        closeModal();
      }
    });
  }

  // ── Date presets ─────────────────────────────────────────────
  function applyPreset(preset) {
    const today = new Date();
    let from, to;
    if (preset === '7') {
      to = new Date(today);
      from = new Date(today); from.setDate(from.getDate() - 7);
    } else if (preset === '30') {
      to = new Date(today);
      from = new Date(today); from.setDate(from.getDate() - 30);
    } else if (preset === 'quarter') {
      const qStartMonth = Math.floor(today.getMonth() / 3) * 3;
      from = new Date(today.getFullYear(), qStartMonth, 1);
      to = new Date(today);
    } else if (preset === 'ytd') {
      from = new Date(today.getFullYear(), 0, 1);
      to = new Date(today);
    } else {
      return;
    }
    const fromEl = document.getElementById('aml-start');
    const toEl = document.getElementById('aml-end');
    if (fromEl) fromEl.value = fmtIsoDate(from);
    if (toEl) toEl.value = fmtIsoDate(to);
    refreshSummary();
  }

  function setupPresets() {
    document.querySelectorAll('.mp-preset-chip').forEach((chip) => {
      chip.addEventListener('click', () => {
        document.querySelectorAll('.mp-preset-chip').forEach((c) => c.setAttribute('aria-pressed', 'false'));
        chip.setAttribute('aria-pressed', 'true');
        applyPreset(chip.dataset.preset);
      });
    });
  }

  function debounce(fn, ms) {
    let t; return (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); };
  }

  function setupReactiveInputs() {
    const debounced = debounce(refreshSummary, 250);
    ['ojk-quarter', 'aml-start', 'aml-end', 'tax-year'].forEach((id) => {
      document.getElementById(id)?.addEventListener('change', debounced);
    });
    document.getElementById('mp-health-refresh')?.addEventListener('click', refreshSummary);
  }

  // ── Export history ───────────────────────────────────────────
  async function refreshHistory() {
    const tbody = document.getElementById('mp-history-tbody');
    if (!tbody) return;
    const filter = document.getElementById('mp-history-filter')?.value || '';
    const params = new URLSearchParams();
    if (filter) params.set('export_type', filter);
    params.set('limit', '50');

    tbody.innerHTML = `<tr><td colspan="9" class="mp-table-empty">Loading…</td></tr>`;
    try {
      const res = await fetch(`/api/admin/marketplace/compliance/exports?${params}`, {
        credentials: 'same-origin', headers: { Accept: 'application/json' },
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const rows = await res.json();
      if (!rows.length) {
        tbody.innerHTML = `<tr><td colspan="9" class="mp-table-empty">No exports recorded yet.</td></tr>`;
        return;
      }
      tbody.innerHTML = rows.map((r) => `
        <tr data-row-id="${r.id}">
          <td title="${escapeHtml(new Date(r.requested_at).toLocaleString())}">${escapeHtml(fmtRelative(r.requested_at))}</td>
          <td>${escapeHtml(TYPE_LABELS[r.export_type] || r.export_type)}</td>
          <td>${escapeHtml(r.period_label)}</td>
          <td>${escapeHtml(r.requested_by_email || '—')}</td>
          <td class="num">${fmtNum(r.row_count)}</td>
          <td class="num">${fmtBytes(r.byte_size)}</td>
          <td><span class="mp-hash" title="${escapeHtml(r.content_sha256)}">${escapeHtml(truncHash(r.content_sha256))}</span></td>
          <td><span class="mp-status-pill" data-status="${escapeHtml(r.submission_status)}">${escapeHtml(r.submission_status)}</span></td>
          <td class="actions">${
            r.submission_status === 'submitted'
              ? `<span class="mp-field__hint">${escapeHtml(fmtRelative(r.submitted_at))}</span>`
              : `<button type="button" class="admin-btn admin-btn--sm admin-btn--ghost" data-mark-submitted="${r.id}">Mark submitted</button>`
          }</td>
        </tr>`).join('');
    } catch (e) {
      tbody.innerHTML = `<tr><td colspan="9" class="mp-table-empty">Failed to load: ${escapeHtml(e.message)}</td></tr>`;
    }
  }

  function setupHistoryHandlers() {
    document.getElementById('mp-history-refresh')?.addEventListener('click', refreshHistory);
    document.getElementById('mp-history-filter')?.addEventListener('change', refreshHistory);
    document.getElementById('mp-history-tbody')?.addEventListener('click', async (e) => {
      const target = e.target;
      if (!(target instanceof HTMLElement)) return;
      const id = target.getAttribute('data-mark-submitted');
      if (!id) return;
      const notes = window.prompt('Optional notes (regulator submission reference):', '') || null;
      target.disabled = true;
      target.textContent = 'Saving…';
      try {
        const res = await fetch(
          `/api/admin/marketplace/compliance/exports/${id}/mark-submitted`,
          {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ notes }),
          });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        toast('Marked as submitted.', 'success');
        refreshHistory();
        refreshSummary();
      } catch (err) {
        toast(`Could not update: ${err.message}`, 'error');
        target.disabled = false;
        target.textContent = 'Mark submitted';
      }
    });
  }

  // ── Locale switcher ──────────────────────────────────────────
  function setupLocaleSwitcher() {
    const sel = document.getElementById('mp-locale-switch');
    if (!sel || !window.MPI18n) return;
    sel.value = window.MPI18n.locale;
    sel.addEventListener('change', () => window.MPI18n.setLocale(sel.value));
  }

  // ── Inline date validation + dd/mm/yyyy preview ─────────────
  function isoToDDMMYYYY(iso) {
    if (!iso) return '—';
    const [y, m, d] = iso.split('-');
    if (!y || !m || !d) return iso;
    return `${d}/${m}/${y}`;
  }

  function setupDateValidation() {
    const start = document.getElementById('aml-start');
    const end = document.getElementById('aml-end');
    const startDisp = document.getElementById('aml-start-display');
    const endDisp = document.getElementById('aml-end-display');
    const errEl = document.getElementById('aml-error');

    function update() {
      if (startDisp) startDisp.textContent = isoToDDMMYYYY(start?.value);
      if (endDisp) endDisp.textContent = isoToDDMMYYYY(end?.value);
      const sv = start?.value || '';
      const ev = end?.value || '';
      let msg = '';
      if (sv && ev && sv > ev) {
        msg = (window.MPI18n?.t('err.dateOrder')) || 'Start date cannot be after end date.';
      }
      if (errEl) errEl.textContent = msg;
    }
    start?.addEventListener('input', update);
    end?.addEventListener('input', update);
    update();
  }

  // ── Approvals panel ──────────────────────────────────────────
  async function refreshApprovals() {
    const tbody = document.getElementById('mp-approvals-tbody');
    if (!tbody) return;
    try {
      const res = await fetch(
        '/api/admin/marketplace/compliance/requests?status=pending',
        { credentials: 'same-origin', headers: { Accept: 'application/json' } }
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const rows = await res.json();
      if (!rows.length) {
        tbody.innerHTML = `<tr><td colspan="6" class="mp-table-empty">${
          escapeHtml(window.MPI18n?.t('approval.empty') || 'No pending requests.')
        }</td></tr>`;
        return;
      }
      tbody.innerHTML = rows.map((r) => `
        <tr data-req-id="${r.id}">
          <td title="${escapeHtml(new Date(r.requested_at).toLocaleString())}">${escapeHtml(fmtRelative(r.requested_at))}</td>
          <td>${escapeHtml(TYPE_LABELS[r.export_type] || r.export_type)}</td>
          <td>${escapeHtml(r.period_label)}</td>
          <td>${escapeHtml(r.requested_by_email || '—')}</td>
          <td>${escapeHtml(r.requested_reason || '—')}</td>
          <td class="actions">
            <button type="button" class="admin-btn admin-btn--sm admin-btn--primary"
              data-req-action="approve" data-req-id="${r.id}"
              data-req-type="${escapeHtml(r.export_type)}" data-req-period="${escapeHtml(r.period_label)}"
              data-req-from="${escapeHtml(r.period_start || '')}" data-req-to="${escapeHtml(r.period_end || '')}">
              ${escapeHtml(window.MPI18n?.t('approval.approve') || 'Approve')}
            </button>
            <button type="button" class="admin-btn admin-btn--sm admin-btn--ghost"
              data-req-action="deny" data-req-id="${r.id}">
              ${escapeHtml(window.MPI18n?.t('approval.deny') || 'Deny')}
            </button>
          </td>
        </tr>`).join('');
    } catch (e) {
      tbody.innerHTML = `<tr><td colspan="6" class="mp-table-empty">Failed to load: ${escapeHtml(e.message)}</td></tr>`;
    }
  }

  function setupApprovalHandlers() {
    document.getElementById('mp-approvals-refresh')?.addEventListener('click', refreshApprovals);
    document.getElementById('mp-approvals-tbody')?.addEventListener('click', async (e) => {
      const btn = e.target.closest('[data-req-action]');
      if (!btn) return;
      const id = btn.dataset.reqId;
      const action = btn.dataset.reqAction;
      const notes = window.prompt(`${action === 'approve' ? 'Approve' : 'Deny'} reason (optional):`, '') || null;
      btn.disabled = true;
      try {
        const res = await fetch(
          `/api/admin/marketplace/compliance/requests/${id}/${action}`,
          {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ notes }),
          }
        );
        if (!res.ok) {
          const b = await res.json().catch(() => ({}));
          throw new Error(b.error || `HTTP ${res.status}`);
        }
        if (action === 'approve') {
          const data = await res.json();
          // Auto-trigger token-gated download for the approver.
          const params = new URLSearchParams();
          if (btn.dataset.reqFrom) params.set('from_date', btn.dataset.reqFrom);
          if (btn.dataset.reqTo) params.set('to_date', btn.dataset.reqTo);
          params.set('format', 'csv');
          params.set('token', data.download_token);
          const filename = `travel_rule_${btn.dataset.reqFrom || 'start'}_to_${btn.dataset.reqTo || 'end'}.csv`;
          const url = `/api/admin/marketplace/compliance/travel-rule?${params}`;
          toast(`Approved. Token expires ${fmtRelative(data.token_expires_at)}.`, 'success');
          performDownload(url, filename, btn, false);
        } else {
          toast('Request denied.', 'success');
        }
        refreshApprovals();
        refreshHistory();
      } catch (err) {
        toast(`Action failed: ${err.message}`, 'error');
        btn.disabled = false;
      }
    });
  }

  // ── Schedules panel ──────────────────────────────────────────
  async function refreshSchedules() {
    const tbody = document.getElementById('mp-schedules-tbody');
    if (!tbody) return;
    try {
      const res = await fetch('/api/admin/marketplace/compliance/schedules', {
        credentials: 'same-origin', headers: { Accept: 'application/json' },
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const rows = await res.json();
      if (!rows.length) {
        tbody.innerHTML = `<tr><td colspan="6" class="mp-table-empty">${
          escapeHtml(window.MPI18n?.t('schedule.empty') || 'No schedules configured.')
        }</td></tr>`;
        return;
      }
      tbody.innerHTML = rows.map((s) => `
        <tr data-schedule-id="${s.id}">
          <td>${escapeHtml(TYPE_LABELS[s.export_type] || s.export_type)}</td>
          <td>${escapeHtml(s.cadence)}</td>
          <td>${escapeHtml(s.format)}</td>
          <td>${escapeHtml(s.delivery_email)}</td>
          <td title="${escapeHtml(s.next_run_at ? new Date(s.next_run_at).toLocaleString() : '')}">${
            s.next_run_at ? escapeHtml(fmtRelative(s.next_run_at)) : '—'
          }</td>
          <td class="actions">
            <button type="button" class="admin-btn admin-btn--sm admin-btn--ghost" data-schedule-delete="${s.id}">Delete</button>
          </td>
        </tr>`).join('');
    } catch (e) {
      tbody.innerHTML = `<tr><td colspan="6" class="mp-table-empty">Failed to load: ${escapeHtml(e.message)}</td></tr>`;
    }
  }

  function setupSchedules() {
    document.getElementById('mp-schedule-create-btn')?.addEventListener('click', () => {
      const m = document.getElementById('mp-schedule-modal');
      if (m) {
        m.hidden = false;
        m.querySelector('input,select,textarea')?.focus();
      }
    });
    document.getElementById('mp-schedule-form')?.addEventListener('submit', async (e) => {
      e.preventDefault();
      const fd = new FormData(e.target);
      const body = Object.fromEntries(fd.entries());
      try {
        const res = await fetch('/api/admin/marketplace/compliance/schedules', {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        });
        if (!res.ok) {
          const b = await res.json().catch(() => ({}));
          throw new Error(b.error || `HTTP ${res.status}`);
        }
        toast('Schedule created.', 'success');
        document.getElementById('mp-schedule-modal').hidden = true;
        e.target.reset();
        refreshSchedules();
      } catch (err) {
        toast(`Could not create: ${err.message}`, 'error');
      }
    });
    document.getElementById('mp-schedules-tbody')?.addEventListener('click', async (e) => {
      const btn = e.target.closest('[data-schedule-delete]');
      if (!btn) return;
      if (!confirm('Delete this schedule?')) return;
      const id = btn.dataset.scheduleDelete;
      try {
        const res = await fetch(`/api/admin/marketplace/compliance/schedules/${id}`, {
          method: 'DELETE', credentials: 'same-origin',
        });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        toast('Schedule deleted.', 'success');
        refreshSchedules();
      } catch (err) {
        toast(`Delete failed: ${err.message}`, 'error');
      }
    });
  }

  // ── Compare-mode (vs prior quarter) ─────────────────────────
  async function refreshCompare() {
    const body = document.getElementById('mp-compare-body');
    if (!body) return;
    const quarter = document.getElementById('ojk-quarter')?.value;
    const params = new URLSearchParams();
    if (quarter) params.set('quarter', quarter);
    try {
      const res = await fetch(`/api/admin/marketplace/compliance/compare?${params}`, {
        credentials: 'same-origin', headers: { Accept: 'application/json' },
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const d = await res.json();
      const fmtIDR = (cents) => {
        const v = Number(cents || 0) / 100;
        return `Rp ${v.toLocaleString('id-ID', { maximumFractionDigits: 0 })}`;
      };
      const trend = (n) => n == null ? 'flat' : n > 0.5 ? 'up' : n < -0.5 ? 'down' : 'flat';
      const arrow = (n) => n == null ? '·' : n > 0.5 ? '▲' : n < -0.5 ? '▼' : '◆';
      const pct = (n) => n == null ? 'n/a' : `${n >= 0 ? '+' : ''}${n.toFixed(1)}%`;
      body.innerHTML = `
        <div class="mp-compare">
          <div class="mp-compare__metric">
            <div class="mp-compare__label">${escapeHtml(window.MPI18n?.t('compare.volume') || 'Volume')} (${escapeHtml(d.current_label)})</div>
            <div class="mp-compare__value">${escapeHtml(fmtIDR(d.current_volume_cents))}</div>
            <div class="mp-compare__delta" data-trend="${trend(d.volume_delta_pct)}">
              ${arrow(d.volume_delta_pct)} ${escapeHtml(pct(d.volume_delta_pct))}
              <span class="mp-text-muted">vs ${escapeHtml(d.previous_label)}</span>
            </div>
          </div>
          <div class="mp-compare__metric">
            <div class="mp-compare__label">${escapeHtml(window.MPI18n?.t('compare.trades') || 'Trades')} (${escapeHtml(d.current_label)})</div>
            <div class="mp-compare__value">${fmtNum(d.current_trades)}</div>
            <div class="mp-compare__delta" data-trend="${trend(d.trades_delta_pct)}">
              ${arrow(d.trades_delta_pct)} ${escapeHtml(pct(d.trades_delta_pct))}
              <span class="mp-text-muted">vs ${escapeHtml(d.previous_label)}</span>
            </div>
          </div>
        </div>`;
    } catch (e) {
      body.innerHTML = `<div class="mp-table-empty">Failed: ${escapeHtml(e.message)}</div>`;
    }
  }

  // ── Footer meta from /meta ──────────────────────────────────
  async function refreshMeta() {
    try {
      const res = await fetch('/api/admin/marketplace/compliance/meta', {
        credentials: 'same-origin', headers: { Accept: 'application/json' },
      });
      if (!res.ok) return;
      const d = await res.json();
      const sources = document.getElementById('mp-footer-sources');
      const schema = document.getElementById('mp-footer-schema');
      const support = document.getElementById('mp-footer-support');
      if (sources) sources.textContent = (d.data_sources || []).join(', ');
      if (schema) schema.textContent = `${d.schema_version} · ${d.ojk_regulation} · ${d.fatf_recommendation}`;
      if (support) {
        support.textContent = d.support_email;
        support.href = `mailto:${d.support_email}`;
      }
    } catch (_) { /* silent */ }
  }

  // ── Keyboard shortcuts ──────────────────────────────────────
  function setupShortcuts() {
    let chord = null;
    document.getElementById('mp-shortcuts-help')?.addEventListener('click', () => {
      document.getElementById('mp-shortcuts-modal').hidden = false;
    });
    document.addEventListener('keydown', (e) => {
      const inField = ['INPUT', 'TEXTAREA', 'SELECT'].includes(e.target.tagName)
        || e.target.isContentEditable;
      if (inField) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;

      // Esc handled per modal already
      if (e.key === '?') {
        document.getElementById('mp-shortcuts-modal').hidden = false; return;
      }
      if (chord === 'g') {
        chord = null;
        if (e.key === 'h') document.querySelector('.mp-compliance-history')?.scrollIntoView({behavior:'smooth'});
        else if (e.key === 'p') document.getElementById('mp-approvals-section')?.scrollIntoView({behavior:'smooth'});
        return;
      }
      if (e.key === 'g') { chord = 'g'; setTimeout(() => chord = null, 1500); return; }
      if (e.key === 'r') { refreshSummary(); refreshHistory(); refreshApprovals(); refreshSchedules(); refreshCompare(); return; }
      if (e.key === 'o') { document.getElementById('ojk-quarter')?.focus(); return; }
      if (e.key === 'a') { document.getElementById('aml-start')?.focus(); return; }
      if (e.key === 't') { document.getElementById('tax-year')?.focus(); return; }
      if (e.key === 'e') { document.getElementById('btn-export-ojk')?.click(); return; }
      if (e.key === 'n') { document.getElementById('mp-schedule-create-btn')?.click(); return; }
    });
  }

  // ── Boot ─────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', () => {
    populateDropdowns();
    setupExportButtons();
    setupPresets();
    setupReactiveInputs();
    setupHistoryHandlers();
    setupLocaleSwitcher();
    setupDateValidation();
    setupApprovalHandlers();
    setupSchedules();
    setupShortcuts();
    refreshSummary();
    refreshHistory();
    refreshApprovals();
    refreshSchedules();
    refreshCompare();
    refreshMeta();
    startAutoRefresh();

    // Re-run compare when quarter changes
    document.getElementById('ojk-quarter')?.addEventListener('change', refreshCompare);
    // Re-apply i18n on locale change for keys defined statically
    document.addEventListener('mp-locale-change', () => {
      refreshHistory(); refreshApprovals(); refreshSchedules(); refreshCompare();
    });
  });
})();
