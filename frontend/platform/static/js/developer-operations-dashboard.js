/**
 * Developer Operations Dashboard — matrix grid view.
 * Calls GET /api/developer/operations/dashboard?year=YYYY and renders
 * one row per assigned villa with per-month status cells.
 */

const MONTH_NAMES = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];

// SVG icons (inline, matching the design system stroke style)
const ICON_CLOCK_ALERT = `<span style="display:inline-flex;align-items:center;justify-content:center;width:12px;height:12px;background:#D92D20;color:#fff;font-size:9px;font-weight:900;border-radius:50%;flex-shrink:0;line-height:1;vertical-align:middle">!</span>`;
const ICON_CLOCK       = `<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="flex-shrink:0;vertical-align:middle"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>`;
const ICON_WARN        = `<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>`;
const ICON_CROSS       = `<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>`;

// App state
let state = {
  data: null,         // MatrixDashboardResponse from API
  year: null,         // currently displayed year
  filter: 'all',      // 'all' | 'action' | 'docs' | 'rejected'
  bannerDismissed: false,
};

document.addEventListener('DOMContentLoaded', () => {
  state.year = currentBillingYear();
  load(state.year);

  document.getElementById('ops-banner-dismiss')?.addEventListener('click', () => {
    state.bannerDismissed = true;
    document.getElementById('ops-urgent-banner').style.display = 'none';
  });

  document.querySelectorAll('.ops-filter-tab').forEach(btn => {
    btn.addEventListener('click', () => {
      state.filter = btn.dataset.filter;
      document.querySelectorAll('.ops-filter-tab').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      renderMatrix();
    });
  });
});

async function load(year) {
  try {
    const resp = await fetch(`/api/developer/operations/dashboard?year=${year}`);
    if (!resp.ok) throw new Error(await responseError(resp));
    state.data = await resp.json();
    state.year = state.data.year;
    hideSkeleton();
    render();
  } catch (err) {
    hideSkeleton();
    showError(err.message);
  }
}

function hideSkeleton() {
  const el = document.getElementById('ops-skeleton');
  if (el) el.style.display = 'none';
}

// ── Billing period helpers ────────────────────────────────────────────────────

function currentBillingMonth() {
  // Current month (1–12) — developers can submit data for the current in-progress month.
  return new Date().getMonth() + 1;
}

function currentBillingYear() {
  return new Date().getFullYear();
}

// Deadline for a period (year, month) is the 28th of the following month.
function isOverdue(periodYear, periodMonth) {
  const now = new Date();
  const deadlineYear  = periodMonth === 12 ? periodYear + 1 : periodYear;
  const deadlineMonth = periodMonth === 12 ? 1 : periodMonth + 1;
  const deadline = new Date(deadlineYear, deadlineMonth - 1, 28);
  return now > deadline;
}

function isLateSubmit(periodYear, periodMonth, recordedAt) {
  const submitted = new Date(recordedAt);
  const deadlineYear  = periodMonth === 12 ? periodYear + 1 : periodYear;
  const deadlineMonth = periodMonth === 12 ? 1 : periodMonth + 1;
  const deadline = new Date(deadlineYear, deadlineMonth - 1, 28);
  return submitted > deadline;
}

// ── Render ────────────────────────────────────────────────────────────────────

function render() {
  const { assets } = state.data;

  // Show/hide containers
  document.getElementById('ops-stats').style.display = assets.length ? '' : 'none';
  document.getElementById('ops-matrix-wrap').style.display = assets.length ? '' : 'none';
  document.getElementById('ops-empty').style.display = assets.length ? 'none' : '';

  if (!assets.length) return;

  renderYearTabs();
  renderStats();
  renderNothingDue();
  renderBanner();
  renderMatrix();
  renderMobileList();
}

// Build a tap-friendly per-villa card list (shown only at narrow viewports
// via CSS — the matrix above stays for ≥768).
function renderMobileList() {
  const root = document.getElementById('ops-mobile-list');
  if (!root) return;

  const { assets, year } = state.data;
  const billingYear  = currentBillingYear();
  const billingMonth = currentBillingMonth();

  root.innerHTML = '';

  assets.forEach(asset => {
    const card = document.createElement('article');
    card.className = 'ops-mcard';

    // Header
    const headerHtml = `
      <header class="ops-mcard__header">
        <div class="ops-mcard__title">${esc(asset.asset_title)}</div>
        <div class="ops-mcard__meta">Since ${MONTH_NAMES[asset.listed_month - 1]} ${asset.listed_year}</div>
      </header>`;

    // Annual doc badge
    const annualYear = asset.annual_doc_year;
    let annualBadge = '';
    if (annualYear >= asset.listed_year) {
      annualBadge = asset.annual_doc_uploaded
        ? `<span class="ops-mcard__chip ops-mcard__chip--ok">Annual ${annualYear} ✓</span>`
        : `<span class="ops-mcard__chip ops-mcard__chip--warn">Annual ${annualYear} missing</span>`;
    }

    // Build list of action items for current billing year
    const startMonth = (year === asset.listed_year) ? asset.listed_month : 1;
    const maxMonth   = (year === billingYear) ? billingMonth : 12;
    const actions = [];
    for (let m = startMonth; m <= maxMonth; m++) {
      const period = asset.periods.find(p => p.month === m);
      if (!period) {
        const overdue = isOverdue(year, m);
        actions.push({
          type: 'missing',
          month: m,
          label: 'Missing',
          overdue,
          href: `/developer/villas/${asset.asset_id}/operations/new?year=${year}&month=${m}`,
          cta: 'Submit',
          variant: 'primary',
        });
      } else if (period.status === 'draft') {
        actions.push({
          type: 'draft',
          month: m,
          label: 'Draft',
          overdue: isOverdue(year, m),
          href: `/developer/villas/${asset.asset_id}/operations/${period.log_id}`,
          cta: 'Continue',
          variant: 'primary',
        });
      } else if (period.status === 'rejected') {
        actions.push({
          type: 'rejected',
          month: m,
          label: 'Rejected',
          overdue: isOverdue(year, m),
          rejected_reason: period.rejected_reason,
          href: `/developer/villas/${asset.asset_id}/operations/${period.log_id}`,
          cta: 'Fix',
          variant: 'danger',
        });
      }
    }

    // Status pills: 12 small dots, latest one labeled
    const pills = [];
    for (let m = 1; m <= 12; m++) {
      const beforeStart = (year === asset.listed_year && m < asset.listed_month);
      const isFuture   = (year === billingYear && m > billingMonth) || (year > billingYear);
      const period = asset.periods.find(p => p.month === m);
      let cls = 'ops-mcard__pill';
      if (beforeStart || isFuture) cls += ' ops-mcard__pill--future';
      else if (!period) cls += ' ops-mcard__pill--missing';
      else cls += ' ops-mcard__pill--' + period.status;
      pills.push(`<span class="${cls}" title="${MONTH_NAMES[m-1]} ${year}" aria-label="${MONTH_NAMES[m-1]} ${year} — ${period ? period.status : (beforeStart || isFuture ? 'not due' : 'missing')}">${MONTH_NAMES[m-1][0]}</span>`);
    }

    // Action items HTML
    let actionsHtml = '';
    if (actions.length) {
      actionsHtml = `<ul class="ops-mcard__actions">` + actions.map(a => {
        const note = a.type === 'rejected' && a.rejected_reason
          ? `<div class="ops-mcard__action-note">"${esc(a.rejected_reason)}"</div>`
          : '';
        const tag = a.overdue ? `<span class="ops-mcard__action-tag">Overdue</span>` : '';
        return `<li class="ops-mcard__action">
          <div class="ops-mcard__action-info">
            <div class="ops-mcard__action-label">${MONTH_NAMES[a.month-1]} ${year} · ${a.label}${tag ? ' · ' : ''}${tag}</div>
            ${note}
          </div>
          <a href="${a.href}" class="ds-btn ds-btn--${a.variant} ds-btn--sm ops-mcard__action-cta">${a.cta} →</a>
        </li>`;
      }).join('') + `</ul>`;
    } else {
      actionsHtml = `<div class="ops-mcard__caught-up">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"/></svg>
        All caught up for ${year}
      </div>`;
    }

    card.innerHTML = `
      ${headerHtml}
      ${annualBadge ? `<div class="ops-mcard__chiprow">${annualBadge}</div>` : ''}
      <div class="ops-mcard__pills" aria-label="${year} monthly status overview">${pills.join('')}</div>
      ${actionsHtml}`;

    root.appendChild(card);
  });
}

// Show an info banner when the current billing year has zero expected
// submissions (e.g. every villa was listed after this year's billing month).
function renderNothingDue() {
  const banner = document.getElementById('ops-nothing-due');
  if (!banner) return;

  const { assets, year } = state.data;
  const billingYear  = currentBillingYear();
  const billingMonth = currentBillingMonth();

  // Only relevant for current billing year
  if (year !== billingYear) {
    banner.style.display = 'none';
    return;
  }

  let totalExpected = 0;
  let earliestDue = null; // { year, month }
  assets.forEach(a => {
    const startYear  = a.listed_year;
    const startMonth = a.listed_month;
    const startInYear = (startYear === year) ? startMonth : 1;
    for (let m = startInYear; m <= billingMonth; m++) totalExpected++;

    // First period this asset will ever owe = listing month (the month it was listed)
    if (!earliestDue ||
        startYear < earliestDue.year ||
        (startYear === earliestDue.year && startMonth < earliestDue.month)) {
      earliestDue = { year: startYear, month: startMonth };
    }
  });

  if (totalExpected > 0) {
    banner.style.display = 'none';
    return;
  }

  // Deadline = 28th of month after the earliest due period
  let firstDueLabel = '—';
  let deadlineLabel = '';
  if (earliestDue) {
    firstDueLabel = `${MONTH_NAMES[earliestDue.month - 1]} ${earliestDue.year}`;
    const deadlineMonth = earliestDue.month === 12 ? 1 : earliestDue.month + 1;
    const deadlineYear  = earliestDue.month === 12 ? earliestDue.year + 1 : earliestDue.year;
    const deadline = new Date(deadlineYear, deadlineMonth - 1, 28);
    deadlineLabel = `Deadline ${MONTH_NAMES[deadlineMonth - 1]} 28, ${deadlineYear}`;
  }

  document.getElementById('ops-nothing-due-title').textContent =
    `No reports due yet — first report covers ${firstDueLabel}.`;
  document.getElementById('ops-nothing-due-sub').textContent = deadlineLabel
    ? `${deadlineLabel}. We'll surface the action here once the period opens.`
    : "We'll surface the action here once the first period opens.";
  banner.style.display = '';
}

function renderYearTabs() {
  const { assets, year } = state.data;

  // Collect years that have data or are the current year
  const nowYear = new Date().getFullYear();
  const yearsSet = new Set([nowYear]);
  assets.forEach(a => {
    if (a.listed_year) yearsSet.add(a.listed_year);
    if (a.listed_year && a.listed_year < nowYear) {
      for (let y = a.listed_year; y <= nowYear; y++) yearsSet.add(y);
    }
  });
  const years = Array.from(yearsSet).sort();

  const container = document.getElementById('ops-year-tabs');
  container.innerHTML = '';
  // Only render year switcher once user has 2+ years of submission history.
  // Single-year users get a cleaner topbar.
  if (years.length < 2) {
    container.style.display = 'none';
    return;
  }
  container.style.display = '';
  years.forEach(y => {
    const btn = document.createElement('button');
    btn.className = 'ops-year-tab' + (y === state.year ? ' active' : '');
    btn.textContent = y;
    btn.addEventListener('click', () => {
      if (y === state.year) return;
      state.year = y;
      state.data = null;
      load(y);
    });
    container.appendChild(btn);
  });
}

function renderStats() {
  const { assets, year } = state.data;
  const billingMonth = currentBillingMonth();
  const billingYear  = currentBillingYear();

  let missing = 0, drafts = 0, review = 0, published = 0, docsMissing = 0;

  assets.forEach(a => {
    // Determine which months this asset should have submitted (up to billing month for billing year)
    const maxMonth = (year === billingYear) ? billingMonth : 12;
    const startMonth = (year === a.listed_year) ? a.listed_month : 1;

    for (let m = startMonth; m <= maxMonth; m++) {
      const period = a.periods.find(p => p.month === m);
      const status = period ? period.status : 'missing';

      if (status === 'missing' || !period) missing++;
      else if (status === 'draft') drafts++;
      else if (status === 'submitted') review++;
      else if (status === 'published') published++;

      if (period && !period.has_period_docs && ['published','approved','submitted'].includes(status)) {
        docsMissing++;
      }
    }
  });

  setText('stat-missing',   missing);
  setText('stat-drafts',    drafts);
  setText('stat-review',    review);
  setText('stat-published', published);
  setText('stat-docs',      docsMissing);
}

function renderBanner() {
  if (state.bannerDismissed) return;

  const { assets, year } = state.data;
  const billingYear  = currentBillingYear();
  const billingMonth = currentBillingMonth();

  // Only show banner for the current billing year
  if (year !== billingYear) {
    document.getElementById('ops-urgent-banner').style.display = 'none';
    return;
  }

  // Deadline = 28th of next month after billing month
  const deadlineMonth = billingMonth === 12 ? 1 : billingMonth + 1;
  const deadlineYear  = billingMonth === 12 ? billingYear + 1 : billingYear;
  const deadline = new Date(deadlineYear, deadlineMonth - 1, 28);
  const daysLeft = Math.ceil((deadline - new Date()) / 86400000);

  const chips = [];
  assets.forEach(a => {
    a.periods.forEach(p => {
      if (['rejected'].includes(p.status)) {
        chips.push({ label: `${a.asset_title} · ${MONTH_NAMES[p.month-1]} rejected`, href: `/developer/villas/${a.asset_id}/operations/${p.log_id}`, warn: false });
      }
    });
    // Missing months up to and including billing month
    for (let m = (year === a.listed_year ? a.listed_month : 1); m <= billingMonth; m++) {
      const period = a.periods.find(p => p.month === m);
      if (!period) {
        chips.push({ label: `${a.asset_title} · ${MONTH_NAMES[m-1]} missing`, href: `/developer/villas/${a.asset_id}/operations/new?year=${year}&month=${m}`, warn: false });
      } else if (period.status === 'draft') {
        chips.push({ label: `${a.asset_title} · ${MONTH_NAMES[m-1]} draft`, href: `/developer/villas/${a.asset_id}/operations/${period.log_id}`, warn: true });
      }
    }
  });

  const banner = document.getElementById('ops-urgent-banner');
  if (!chips.length) {
    banner.style.display = 'none';
    return;
  }

  banner.style.display = '';
  document.getElementById('ops-banner-title').textContent =
    `${chips.length} submission${chips.length !== 1 ? 's' : ''} required · due ${MONTH_NAMES[deadlineMonth-1]} 28 · ${daysLeft} day${daysLeft !== 1 ? 's' : ''} left`;

  const rowsEl = document.getElementById('ops-banner-chips');
  rowsEl.innerHTML = '';
  chips.forEach((c, i) => {
    const [assetPart, ...rest] = c.label.split(' · ');
    const periodPart = rest.join(' · ');
    const isWarn    = c.warn;                          // draft
    const isDanger  = periodPart.includes('rejected'); // rejected
    const btnClass  = isDanger ? 'ds-btn--danger' : isWarn ? 'ds-btn--secondary' : 'ds-btn--primary';
    const btnLabel  = isDanger ? 'Fix &amp; Resubmit' : isWarn ? 'Continue draft →' : 'Submit →';
    const dotColor  = isDanger ? '#D92D20' : isWarn ? '#B54708' : '#D92D20';

    const row = document.createElement('a');
    row.className = 'ops-action-queue__row';
    row.href = c.href;
    if (i < chips.length - 1) row.classList.add('ops-action-queue__row--border');
    row.innerHTML = `
      <div class="ops-action-queue__dot" style="background:${dotColor}1A;color:${dotColor}">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
      </div>
      <div class="ops-action-queue__row-main">
        <span class="ops-action-queue__row-asset">${esc(assetPart)}</span>
        <span class="ops-action-queue__row-period">${esc(periodPart)}</span>
      </div>
      <span class="ds-btn ds-btn--sm ${btnClass} ops-action-queue__row-btn">${btnLabel}</span>`;
    rowsEl.appendChild(row);
  });
}

function renderMatrix() {
  const { assets, year } = state.data;
  const billingYear  = currentBillingYear();
  const billingMonth = currentBillingMonth();

  // Filter assets
  let filtered = assets;
  if (state.filter === 'action') {
    filtered = assets.filter(a => hasAction(a, year, billingYear, billingMonth));
  } else if (state.filter === 'docs') {
    filtered = assets.filter(a => a.periods.some(p => !p.has_period_docs && ['published','approved','submitted'].includes(p.status)));
  } else if (state.filter === 'rejected') {
    filtered = assets.filter(a => a.periods.some(p => p.status === 'rejected'));
  }

  // Update filter counts
  updateFilterCount('fc-all',    assets.filter(a => hasAction(a, year, billingYear, billingMonth)).length, 'red');
  updateFilterCount('fc-action', assets.filter(a => hasAction(a, year, billingYear, billingMonth)).length, 'red');
  updateFilterCount('fc-docs',   assets.filter(a => a.periods.some(p => !p.has_period_docs && ['published','approved','submitted'].includes(p.status))).length, 'amber');

  // Build header
  const thead = document.getElementById('ops-matrix-thead');
  const thRow = document.createElement('tr');

  const thAsset = th('ops-col-asset', 'Asset', true);
  thRow.appendChild(thAsset);

  const thHist = th('ops-col-history', 'Prior yrs');
  thRow.appendChild(thHist);

  for (let m = 1; m <= 12; m++) {
    const isCurrent = (year === billingYear && m === billingMonth);
    const cell = th('ops-col-month' + (isCurrent ? ' ops-th-current' : ''), '');
    cell.innerHTML = MONTH_NAMES[m-1] + (isCurrent ? `<span class="ops-th-cur-label">← current</span>` : '');
    thRow.appendChild(cell);
  }

  const thProg = th('ops-col-progress', year + '');
  thRow.appendChild(thProg);

  thead.innerHTML = '';
  thead.appendChild(thRow);

  // Build body
  const tbody = document.getElementById('ops-matrix-tbody');
  tbody.innerHTML = '';

  if (!filtered.length) {
    const tr = document.createElement('tr');
    const td = document.createElement('td');
    td.colSpan = 16;
    td.style.cssText = 'text-align:center;padding:40px 32px;color:#667085;font-size:13px';
    td.innerHTML = `<div style="display:flex;flex-direction:column;align-items:center;gap:12px">
      <img src="/static/images/logos/Logo%20Pool.svg" alt="POOOL" style="height:28px;opacity:1">
      <span>No assets match this filter.</span>
    </div>`;
    tr.appendChild(td);
    tbody.appendChild(tr);
    return;
  }

  filtered.forEach(asset => {
    const tr = document.createElement('tr');

    // Asset cell
    const tdAsset = document.createElement('td');
    tdAsset.className = 'ops-asset-cell';
    const startYear  = asset.listed_year;
    const startMonth = asset.listed_month;
    const sinceLabel = `${asset.asset_title.split(',')[1] ? '' : ''}${MONTH_NAMES[startMonth-1]} ${startYear}`;

    let assetHtml = `<div class="ops-asset-name">${esc(asset.asset_title)}</div>
      <div class="ops-asset-meta">Since ${sinceLabel}</div>`;

    // Annual docs badge
    const annualYear = asset.annual_doc_year;
    if (annualYear >= startYear) {
      if (asset.annual_doc_uploaded) {
        assetHtml += `<span class="ops-asset-annual ops-asset-annual--ok">Annual ${annualYear} ✓</span>`;
      } else {
        assetHtml += `<span class="ops-asset-annual ops-asset-annual--miss">Annual ${annualYear} missing</span>`;
      }
    }

    // Rejection note (latest rejected period)
    const rejected = asset.periods.filter(p => p.status === 'rejected');
    if (rejected.length) {
      const r = rejected[rejected.length - 1];
      const reason = r.rejected_reason ? ` · "${esc(r.rejected_reason)}"` : '';
      assetHtml += `<div class="ops-asset-reject">${ICON_CROSS} ${MONTH_NAMES[r.month-1]} rejected${reason}</div>`;
    }

    // Period docs warning (most recent missing-docs month)
    const missingDocsPeriod = asset.periods.find(p =>
      !p.has_period_docs && ['submitted','approved','published'].includes(p.status)
    );
    if (missingDocsPeriod) {
      assetHtml += `<div class="ops-asset-warn">${ICON_WARN} Period docs missing · ${MONTH_NAMES[missingDocsPeriod.month-1]} ${year}</div>`;
    }

    tdAsset.innerHTML = assetHtml;
    tr.appendChild(tdAsset);

    // History cell
    const tdHist = document.createElement('td');
    tdHist.className = 'ops-hist-cell';
    const pub  = asset.prior_published_count;
    const exp  = asset.prior_expected_count;
    if (exp === 0) {
      tdHist.innerHTML = `<span class="ops-hist-none">First year</span>`;
    } else if (pub >= exp) {
      tdHist.innerHTML = `<span class="ops-hist-pill ops-hist-ok">✓ ${pub}/${exp}</span>`;
    } else {
      tdHist.innerHTML = `<span class="ops-hist-pill ops-hist-part">${pub}/${exp}</span>`;
    }
    tr.appendChild(tdHist);

    // Month cells
    let publishedCount = 0;
    let expectedCount  = 0;
    const maxExpected  = (year === billingYear) ? billingMonth : 12;

    for (let m = 1; m <= 12; m++) {
      const isCurrent = (year === billingYear && m === billingMonth);
      const isFuture  = (year === billingYear && m > billingMonth) ||
                        (year > billingYear);
      const beforeStart = (year === startYear && m < startMonth);

      const tdCell = document.createElement('td');
      tdCell.className = 'ops-status-cell' + (isCurrent ? ' is-current' : '');

      if (beforeStart) {
        tdCell.classList.add('is-inert');
        tdCell.setAttribute('aria-label', `${MONTH_NAMES[m-1]} ${year} — before listing`);
        tdCell.innerHTML = `<div class="ops-dot ops-dot--future"><div class="ops-dot__icon" aria-hidden="true">—</div></div>`;
        tr.appendChild(tdCell);
        continue;
      }

      if (isFuture) {
        tdCell.classList.add('is-inert');
        tdCell.setAttribute('aria-label', `${MONTH_NAMES[m-1]} ${year} — not yet due`);
        tdCell.innerHTML = `<div class="ops-dot ops-dot--future"><div class="ops-dot__icon" aria-hidden="true">—</div></div>`;
        tr.appendChild(tdCell);
        continue;
      }

      if (!isFuture && !beforeStart && m <= maxExpected) expectedCount++;

      const period = asset.periods.find(p => p.month === m);

      if (!period) {
        // Missing — entire dot is a link
        const overdue = isOverdue(year, m);
        if (overdue) tdCell.classList.add('is-overdue');
        tdCell.classList.add('has-action');

        const overdueTag = overdue
          ? `<span class="ops-dot__overdue-tag">${ICON_CLOCK_ALERT} Overdue</span>`
          : '';
        const newHref = `/developer/villas/${asset.asset_id}/operations/new?year=${year}&month=${m}`;
        const a11y = `Submit ${MONTH_NAMES[m-1]} ${year} for ${asset.asset_title}${overdue ? ' (overdue)' : ''}`;

        tdCell.innerHTML = `
          <a href="${newHref}" class="ops-dot ops-dot--missing ops-dot--link${overdue ? ' is-overdue-dot' : ''}" aria-label="${esc(a11y)}">
            <div class="ops-dot__icon" aria-hidden="true">!</div>
            <div class="ops-dot__status-label">Missing</div>
            ${overdueTag}
          </a>
          <a href="${newHref}" class="ops-dot__hover-action" tabindex="-1" aria-hidden="true">
            <span class="ds-btn ds-btn--primary ds-btn--sm">Submit →</span>
            <span class="ops-dot__hover-note">Not started · ${MONTH_NAMES[m-1]} ${year}</span>
          </a>`;
        tr.appendChild(tdCell);
        continue;
      }

      const { status, log_id, recorded_at, rejected_reason, has_period_docs } = period;
      const dateLabel = fmtDate(recorded_at);
      const overdue = isOverdue(year, m) && ['missing','draft','rejected'].includes(status);
      const late    = ['published','approved','submitted'].includes(status) && isLateSubmit(year, m, recorded_at);

      if (overdue) tdCell.classList.add('is-overdue');
      if (['draft', 'rejected'].includes(status)) tdCell.classList.add('has-action');
      if (status === 'published') publishedCount++;

      let dotClass = `ops-dot--${status}`;
      let iconLabel = { published:'✓', approved:'A', submitted:'S', draft:'D', rejected:'R', missing:'!' }[status] || '?';

      const overdueTag = overdue
        ? `<span class="ops-dot__overdue-tag">${ICON_CLOCK_ALERT} Overdue</span>`
        : '';
      const lateTag = late
        ? `<span class="ops-dot__late-submit">${ICON_CLOCK} Late</span>`
        : '';
      const docBadge = !has_period_docs && ['submitted','approved','published'].includes(status)
        ? `<div class="ops-dot__doc-badge" title="Period documents not uploaded"></div>`
        : '';

      let dateOrStatus = '';
      if (['published','approved','submitted'].includes(status)) {
        dateOrStatus = `<div class="ops-dot__date">${dateLabel}${lateTag ? '<br>' + lateTag : ''}</div>`;
      } else {
        dateOrStatus = `<div class="ops-dot__status-label">${capitalize(status)}</div>${overdueTag}`;
      }

      // All states: entire dot is a link to the log detail / form
      const logHref = status === 'missing'
        ? `/developer/villas/${asset.asset_id}/operations/new?year=${year}&month=${m}`
        : `/developer/villas/${asset.asset_id}/operations/${log_id}`;

      let hoverLabel = '';
      if (status === 'rejected') {
        const note = rejected_reason ? esc(rejected_reason) : 'Rejected by admin';
        hoverLabel = `<span class="ds-btn ds-btn--danger ds-btn--sm">Fix &amp; Resubmit</span>
          <span class="ops-dot__hover-note">${note}</span>`;
      } else if (status === 'draft') {
        hoverLabel = `<span class="ds-btn ds-btn--primary ds-btn--sm">Submit →</span>
          <span class="ops-dot__hover-note">Draft saved · ${MONTH_NAMES[m-1]} ${year}</span>`;
      } else {
        hoverLabel = `<span class="ds-btn ds-btn--secondary ds-btn--sm">View</span>`;
      }

      const a11y = `${capitalize(status)} · ${MONTH_NAMES[m-1]} ${year} · ${asset.asset_title}${overdue ? ' (overdue)' : ''}${late ? ' (late)' : ''}`;
      tdCell.innerHTML = `
        <a href="${logHref}" class="ops-dot ${dotClass} ops-dot--link${overdue ? ' is-overdue-dot' : ''}" aria-label="${esc(a11y)}">
          <div class="ops-dot__icon" aria-hidden="true">${iconLabel}</div>
          ${dateOrStatus}
        </a>
        ${docBadge}
        <a href="${logHref}" class="ops-dot__hover-action" tabindex="-1" aria-hidden="true">${hoverLabel}</a>`;

      tr.appendChild(tdCell);
    }

    // Progress cell
    const tdProg = document.createElement('td');
    tdProg.className = 'ops-progress-cell';
    const pct = expectedCount > 0 ? publishedCount / expectedCount : 0;
    const fillClass = pct >= 1 ? '' : pct >= 0.5 ? ' ds-progress__fill--warn' : ' ds-progress__fill--danger';
    const fracColor = publishedCount === 0 && expectedCount > 0 ? 'color:#D92D20' : '';
    tdProg.innerHTML = `
      <div class="ops-progress-cell__fraction" style="${fracColor}">${publishedCount} / ${expectedCount}</div>
      <div class="ds-progress"><div class="ds-progress__fill${fillClass}" style="width:${Math.round(pct*100)}%"></div></div>`;
    tr.appendChild(tdProg);

    tbody.appendChild(tr);
  });
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function hasAction(asset, year, billingYear, billingMonth) {
  // Asset needs action if it has rejected/draft or a missing month in the billing window
  if (asset.periods.some(p => ['rejected','draft'].includes(p.status))) return true;
  const maxMonth = (year === billingYear) ? billingMonth : 12;
  const startMonth = (year === asset.listed_year) ? asset.listed_month : 1;
  for (let m = startMonth; m <= maxMonth; m++) {
    if (!asset.periods.find(p => p.month === m)) return true;
  }
  return false;
}

function updateFilterCount(id, count, variant) {
  const el = document.getElementById(id);
  if (!el) return;
  if (count > 0) {
    el.textContent = count;
    el.style.display = '';
  } else {
    el.style.display = 'none';
  }
}

function th(cls, text, left = false) {
  const el = document.createElement('th');
  el.className = cls;
  if (left) el.style.textAlign = 'left';
  el.textContent = text;
  return el;
}

function fmtDate(iso) {
  if (!iso) return '';
  const d = new Date(iso);
  return `${MONTH_NAMES[d.getMonth()]} ${d.getDate()}`;
}

function capitalize(s) {
  return s ? s.charAt(0).toUpperCase() + s.slice(1) : '';
}

function esc(s) {
  return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

function setText(id, val) {
  const el = document.getElementById(id);
  if (el) el.textContent = val;
}

function showError(msg) {
  document.getElementById('ops-stats').style.display = 'none';
  document.getElementById('ops-matrix-wrap').style.display = 'none';
  document.getElementById('ops-empty').style.display = 'none';
  const errEl = document.getElementById('ops-error');
  errEl.style.display = '';
  const titleEl = document.getElementById('ops-error-title');
  if (titleEl) titleEl.textContent = `Failed to load: ${msg}`;
}

async function responseError(resp) {
  try {
    const b = await resp.json();
    return b.error || b.message || `HTTP ${resp.status}`;
  } catch {
    return `HTTP ${resp.status}`;
  }
}
