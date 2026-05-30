/* global window, document */

/** Analytics dashboard — comprehensive metrics + 4 SVG charts + CSV/PDF
 *  export. All renders are pure DOM (no third-party chart libs).
 *
 *  Endpoints used:
 *    GET /api/developer/affiliate/team/analytics/overview?from&to
 *    GET /api/developer/affiliate/team/analytics/timeseries?from&to
 *    GET /api/developer/affiliate/team/by-member?from&to
 *    GET /api/developer/affiliate/team/products?from&to
 */
(function () {
  'use strict';

  const DAT = window.DAT;
  if (!DAT) return;

  // ─── Range helpers ─────────────────────────────────────────────────────
  function pad(n) { return n < 10 ? '0' + n : '' + n; }
  function isoOf(d) { return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
  function today() { return new Date(); }
  function startOfMonth(d) { return new Date(d.getFullYear(), d.getMonth(), 1); }
  function endOfMonth(d) { return new Date(d.getFullYear(), d.getMonth() + 1, 0); }
  function startOfYear(d) { return new Date(d.getFullYear(), 0, 1); }
  function daysAgo(n) { const d = today(); d.setDate(d.getDate() - n); return d; }

  function presetRange(preset) {
    const t = today();
    switch (preset) {
      case '7d':   return { from: isoOf(daysAgo(6)), to: isoOf(t) };
      case '14d':  return { from: isoOf(daysAgo(13)), to: isoOf(t) };
      case '30d':  return { from: isoOf(daysAgo(29)), to: isoOf(t) };
      case '90d':  return { from: isoOf(daysAgo(89)), to: isoOf(t) };
      case 'ytd':  return { from: isoOf(startOfYear(t)), to: isoOf(t) };
      case 'all':  return { from: '2024-01-01', to: isoOf(t) }; // far enough back
      default:     return { from: isoOf(daysAgo(29)), to: isoOf(t) };
    }
  }

  // Human-readable label for a preset — shown in the topbar trigger button.
  function presetLabel(preset) {
    switch (preset) {
      case '7d':  return 'Last 7 days';
      case '14d': return 'Last 14 days';
      case '30d': return 'Last 30 days';
      case '90d': return 'Last 90 days';
      case 'ytd': return 'This year';
      case 'all': return 'All time';
      case 'custom': return 'Custom range';
      default: return 'Last 30 days';
    }
  }

  function currentRange() {
    const url = new URL(window.location.href);
    const preset = url.searchParams.get('preset');
    const from = url.searchParams.get('from');
    const to = url.searchParams.get('to');
    if (from && to) return { from, to, preset: preset || 'custom' };
    return Object.assign({ preset: preset || '30d' }, presetRange(preset || '30d'));
  }

  function persistRange(from, to, preset) {
    const url = new URL(window.location.href);
    url.searchParams.set('from', from);
    url.searchParams.set('to', to);
    if (preset) url.searchParams.set('preset', preset);
    window.history.replaceState({}, '', url);
  }

  // ─── Format helpers ────────────────────────────────────────────────────
  function fmtInt(n) { return (n || 0).toLocaleString(); }
  function fmtPct(num, den) {
    if (!den) return '—';
    return ((num / den) * 100).toFixed(1) + '%';
  }
  function fmtDate(iso) {
    if (!iso) return '—';
    try { return new Date(iso).toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' }); }
    catch { return iso; }
  }

  // ─── Smart number formatting ───────────────────────────────────────────
  /// Abbreviate cents to `€1,234` < €10k, `€45.2K` < €1M, `€3.4M` else.
  /// EUR matches the leaderboard + rest of the platform; previously USD.
  function fmtCentsSmart(cents) {
    const v = (cents || 0) / 100;
    if (v === 0) return '€0';
    const abs = Math.abs(v);
    if (abs >= 1_000_000) return '€' + (v / 1_000_000).toFixed(1).replace(/\.0$/, '') + 'M';
    if (abs >= 10_000)    return '€' + (v / 1_000).toFixed(1).replace(/\.0$/, '') + 'K';
    return DAT.fmtCents(cents);
  }
  /// Long-form exact EUR for tooltips: `€1.234.567`.
  function fmtCentsExact(cents) {
    const v = (cents || 0) / 100;
    return new Intl.NumberFormat('de-DE', {
      style: 'currency', currency: 'EUR', maximumFractionDigits: 0,
    }).format(v);
  }
  function fmtIntSmart(n) {
    const v = n || 0;
    if (v >= 1_000_000) return (v / 1_000_000).toFixed(1).replace(/\.0$/, '') + 'M';
    if (v >= 10_000)    return (v / 1_000).toFixed(1).replace(/\.0$/, '') + 'K';
    return v.toLocaleString();
  }

  /// Render a tile value with optional skeleton-clear + exact-tooltip.
  function setValue(id, smartText, exactTitle) {
    const el = DAT.$('#' + id);
    if (!el) return;
    el.textContent = smartText;
    if (exactTitle) el.title = exactTitle;
  }

  /// Compute and render a delta pill (↑ +12% vs prev period / ↓ -8% / · flat).
  /// `prevWindow` is an optional `{from, to}` pair that, when present, gets
  /// surfaced in the element's `title` so users can see *which* period the
  /// percentage compares against (e.g. "vs prev 04-15 → 04-30").
  function renderDelta(id, current, previous, prevWindow) {
    const el = DAT.$('#' + id);
    if (!el) return;
    el.className = 'dat-delta';
    const cur = current || 0;
    const prev = previous || 0;
    const winSuffix = prevWindow && prevWindow.from && prevWindow.to
      ? ' (' + prevWindow.from + ' → ' + prevWindow.to + ')'
      : '';
    if (prev === 0 && cur === 0) {
      el.textContent = '';
      el.removeAttribute('title');
      return;
    }
    if (prev === 0) {
      el.textContent = 'New';
      el.classList.add('dat-delta--up');
      el.setAttribute('aria-label', 'New activity, no prior period to compare');
      el.title = 'No activity in the previous comparable window' + winSuffix;
      return;
    }
    const pct = ((cur - prev) / prev) * 100;
    if (Math.abs(pct) < 0.5) {
      el.textContent = '· flat';
      el.classList.add('dat-delta--flat');
      el.setAttribute('aria-label', 'No change vs previous period');
      el.title = 'No meaningful change vs previous window' + winSuffix;
      return;
    }
    const sign = pct > 0 ? '↑' : '↓';
    el.textContent = sign + ' ' + Math.abs(pct).toFixed(0) + '% vs prev';
    el.classList.add(pct > 0 ? 'dat-delta--up' : 'dat-delta--down');
    el.setAttribute('aria-label',
      (pct > 0 ? 'Up ' : 'Down ') + Math.abs(pct).toFixed(0) + ' percent versus previous period');
    el.title = (pct > 0 ? '+' : '−') + Math.abs(pct).toFixed(1) + '% vs previous window' + winSuffix;
  }

  // ─── KPI tiles render ──────────────────────────────────────────────────
  function renderKpis(overview) {
    const p = overview.period || {};
    const prev = overview.previous_period || {};
    // Backend now ships prev_from / prev_to so deltas can name the window.
    const prevWin = (overview.prev_from && overview.prev_to)
      ? { from: overview.prev_from, to: overview.prev_to }
      : null;

    // Hero row (rich formatting + deltas)
    setValue('dat-k-revenue', fmtCentsSmart(p.gross_revenue_cents), fmtCentsExact(p.gross_revenue_cents));
    renderDelta('dat-d-revenue', p.gross_revenue_cents, prev.gross_revenue_cents, prevWin);

    setValue('dat-k-commission', fmtCentsSmart(p.commission_cents), fmtCentsExact(p.commission_cents));
    renderDelta('dat-d-commission', p.commission_cents, prev.commission_cents, prevWin);

    // Next-payout tiles moved to Settings → Payouts & Banking. The setValue
    // helper + DOM lookups are null-safe, so these calls are no-ops on the
    // dashboard but still hydrate the settings page when its IDs are present.
    setValue('dat-k-next-amount', fmtCentsSmart(overview.next_payout_amount_cents),
      fmtCentsExact(overview.next_payout_amount_cents));
    const dateEl = DAT.$('#dat-k-next-date');
    if (dateEl) {
      dateEl.textContent = overview.next_payout_date
        ? 'Earliest: ' + fmtDate(overview.next_payout_date)
        : 'No holdback active';
    }
    const ctaEl = DAT.$('#dat-next-payout-cta');
    if (ctaEl) {
      const claimable = overview.payable_commission_cents || 0;
      if (claimable >= 5000) {
        ctaEl.hidden = false;
        ctaEl.href = '/affiliate/dashboard';
      } else {
        ctaEl.hidden = true;
      }
    }

    // Secondary row — 4 cards: Conversion / Qualified / Payouts (composite) / Members.
    setValue('dat-k-conv', fmtPct(p.signups_count, p.clicks_count));
    DAT.$('#dat-k-conv-sub').textContent =
      'Click-through ' + fmtIntSmart(p.signups_count) + ' / ' + fmtIntSmart(p.clicks_count);

    setValue('dat-k-qualified', fmtIntSmart(p.qualified_count), String(p.qualified_count || 0));
    // Qualified can exceed signups in a period because attribution carries
    // forward — a signup from an earlier window can qualify in this one.
    // The previous label "% of signups" rendered confusing values like
    // "136% of signups" and made the data look broken. We now show the raw
    // total + an explanatory title (visible on hover/long-press).
    const qSub = DAT.$('#dat-k-qualified-sub');
    if (qSub) {
      qSub.textContent = 'Qualified in window';
      qSub.title = 'Includes signups that qualified during this window, even if they originally signed up earlier (attribution carry-over).';
    }

    // Payouts composite — single tile with 3 inline rows (Pending / Payable
    // / Paid out). Frees one card slot in the secondary row.
    setValue('dat-k-pay-pending', fmtCentsSmart(overview.pending_commission_cents),
      fmtCentsExact(overview.pending_commission_cents));
    setValue('dat-k-pay-payable', fmtCentsSmart(overview.payable_commission_cents),
      fmtCentsExact(overview.payable_commission_cents));
    setValue('dat-k-pay-paid', fmtCentsSmart(overview.paid_commission_cents),
      fmtCentsExact(overview.paid_commission_cents));

    setValue('dat-k-members', fmtIntSmart(overview.active_members));
    const openReq = overview.open_payout_requests || 0;
    DAT.$('#dat-k-members-sub').textContent =
      openReq + ' open payout' + (openReq === 1 ? '' : 's');

    // Last-updated indicator + screen-reader-friendly live region.
    const updatedAt = overview.computed_at ? new Date(overview.computed_at) : new Date();
    const lu = DAT.$('#dat-last-updated');
    if (lu) lu.textContent = 'Updated ' + updatedAt.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  // ─── SVG chart helpers ────────────────────────────────────────────────
  const SVG_NS = 'http://www.w3.org/2000/svg';
  function svg(tag, attrs) {
    const el = document.createElementNS(SVG_NS, tag);
    for (const k in attrs) el.setAttribute(k, attrs[k]);
    return el;
  }

  /// Aggregate daily series into week or month buckets.
  function aggregateSeries(daily, resolution) {
    if (resolution === 'day' || !daily || daily.length === 0) return daily || [];
    const buckets = new Map();
    daily.forEach((d) => {
      const dt = new Date(d.bucket_date);
      let key;
      if (resolution === 'week') {
        // ISO week start (Monday)
        const day = dt.getUTCDay() || 7;
        const monday = new Date(dt);
        monday.setUTCDate(dt.getUTCDate() - (day - 1));
        key = monday.toISOString().slice(0, 10);
      } else { // month
        key = dt.toISOString().slice(0, 7) + '-01';
      }
      const existing = buckets.get(key) || {
        bucket_date: key,
        clicks_count: 0, signups_count: 0, qualified_count: 0,
        gross_revenue_cents: 0, commission_cents: 0,
      };
      existing.clicks_count += d.clicks_count || 0;
      existing.signups_count += d.signups_count || 0;
      existing.qualified_count += d.qualified_count || 0;
      existing.gross_revenue_cents += d.gross_revenue_cents || 0;
      existing.commission_cents += d.commission_cents || 0;
      buckets.set(key, existing);
    });
    return Array.from(buckets.values()).sort((a, b) =>
      a.bucket_date < b.bucket_date ? -1 : 1);
  }

  /** Line chart: revenue + commission over time. Hero-sized, with
   *  hover-crosshair tooltip, dash pattern on revenue (color-blind),
   *  smart axis label suppression, explicit empty-state. */
  let _trendDaily = [];
  let _trendResolution = 'day';

  function renderTrendChart(daily) {
    _trendDaily = daily || [];
    const host = DAT.$('#dat-chart-trend');
    DAT.clear(host);

    const series = aggregateSeries(_trendDaily, _trendResolution);

    // Single-metric mode: pick the active metric's value-key and label so the
    // rest of the renderer is metric-agnostic.
    const metric = _trendMetric === 'revenue' ? 'revenue' : 'commission';
    const valueKey = metric === 'revenue' ? 'gross_revenue_cents' : 'commission_cents';
    const metricLabel = metric === 'revenue' ? 'Revenue' : 'Commission';

    // Sum the active metric across the series to detect "all zero" state.
    const total = series.reduce((s, p) => s + (p[valueKey] || 0), 0);
    if (!series.length || total === 0) {
      const wrap = DAT.el('div', { class: 'dat-chart-empty dat-chart-empty--hero' });
      wrap.appendChild(DAT.el('strong', { class: 'dat-chart-empty__title' },
        `No ${metricLabel.toLowerCase()} yet in this period`));
      wrap.appendChild(DAT.el('p', { class: 'dat-chart-empty__msg' },
        `When customers your team refers complete a purchase, daily ${metricLabel.toLowerCase()} appears here.`));
      const cta = DAT.el('button', {
        type: 'button',
        class: 'dat-chart-empty__cta',
      }, 'Try a wider date range →');
      cta.addEventListener('click', () => {
        const allBtn = document.querySelector('.dat-preset[data-preset="all"]');
        if (allBtn) allBtn.click();
      });
      wrap.appendChild(cta);
      host.appendChild(wrap);
      return;
    }

    // ── ECharts migration (was: hand-rolled SVG line + dashed 7d MA overlay) ──
    // Per-axis formatter — euros, smart suffix.
    const eurFormatter = (cents) => {
      const eur = cents / 100;
      if (eur === 0) return '€0';
      if (Math.abs(eur) >= 1_000_000) return '€' + (eur / 1_000_000).toFixed(1).replace(/\.0$/, '') + 'M';
      if (Math.abs(eur) >= 1_000)     return '€' + (eur / 1_000).toFixed(1).replace(/\.0$/, '') + 'K';
      return '€' + Math.round(eur).toLocaleString();
    };

    const labels = series.map((s) => (s.bucket_date || '').slice(5));
    const values = series.map((s) => (s[valueKey] || 0) / 100); // to euros for axis math

    // 7-day moving average overlay — only at daily resolution, ≥7 points.
    let maValues = null;
    if (_trendResolution === 'day' && series.length >= 7) {
      maValues = [];
      const w = 7;
      for (let i = 0; i < series.length; i++) {
        const lo = Math.max(0, i - w + 1);
        let sum = 0, n = 0;
        for (let j = lo; j <= i; j++) { sum += (series[j][valueKey] || 0); n++; }
        maValues.push(n > 0 ? (sum / n) / 100 : 0);
      }
    }

    if (typeof window.PooolLineChart === "undefined") {
      host.innerHTML = `<div style="height:280px;display:flex;align-items:center;justify-content:center;color:#dc2626;font-size:12px;">Chart library unavailable</div>`;
      return;
    }

    // Create a fresh DIV mount inside host (host is the `#dat-chart-trend`
    // container; DAT.clear() emptied it just above).
    const mount = document.createElement('div');
    mount.style.width = '100%';
    mount.style.height = '280px';
    mount.setAttribute('role', 'img');
    mount.setAttribute('aria-label',
      `Daily ${metricLabel.toLowerCase()} trend. Peak ${fmtCentsSmart(Math.max(1, ...series.map((s) => s[valueKey] || 0)))} ` +
      `over ${series.length} data points.`);
    host.appendChild(mount);

    const chartSeries = [
      { name: metricLabel, values, area: true },
    ];
    if (maValues) {
      chartSeries.push({
        name: '7-day avg',
        values: maValues,
        dashed: true,
        color: '#0000FF',
        opacity: 0.55,
        markEnd: false,
      });
    }

    window.PooolLineChart.render(mount, {
      labels,
      series: chartSeries,
      formatter: (v) => eurFormatter(v * 100),
      height: 280,
    });
  }

  /// Re-render trend chart at the requested time-resolution.
  /// Scoped to `[data-res]` buttons only so the sibling metric toggle
  /// (commission/revenue, `[data-metric]`) is not affected.
  function setTrendResolution(res) {
    _trendResolution = res;
    document.querySelectorAll('.dat-res-btn[data-res]').forEach((b) => {
      b.classList.toggle('dat-res-btn--active', b.dataset.res === res);
    });
    renderTrendChart(_trendDaily);
  }

  /// Active metric for the trend chart — either 'commission' or 'revenue'.
  /// Single-metric view replaces the previous dual-axis "both" mode whose
  /// right-side axis labels clipped under narrow viewports.
  let _trendMetric = 'commission';

  function setTrendMetric(metric) {
    if (metric !== 'commission' && metric !== 'revenue') return;
    _trendMetric = metric;
    document.querySelectorAll('.dat-res-btn[data-metric]').forEach((b) => {
      b.classList.toggle('dat-res-btn--active', b.dataset.metric === metric);
    });
    const titleEl = document.getElementById('dat-chart-trend-title');
    if (titleEl) {
      titleEl.textContent = metric === 'commission' ? 'Commission trend' : 'Revenue trend';
    }
    renderTrendChart(_trendDaily);
  }

  /** Funnel: stacked horizontal stage cards. Each stage row gets a label,
   *  a count, and a brand-tinted bar whose width encodes its value as a
   *  fraction of the funnel's peak count. Between rows a compact pill shows
   *  the conversion direction + rate + absolute delta, so anomalies like
   *  "qualified > signups" from late attribution read as a clear gain
   *  rather than a broken funnel shape. */
  function renderFunnelChart(period) {
    const host = DAT.$('#dat-chart-funnel');
    DAT.clear(host);
    const c = period.clicks_count || 0;
    const s = period.signups_count || 0;
    const q = period.qualified_count || 0;

    const stages = [
      { label: 'Clicks',    value: c },
      { label: 'Signups',   value: s },
      { label: 'Qualified', value: q },
    ];
    const max = Math.max(c, s, q, 1);

    const wrap = DAT.el('div', {
      class: 'dat-funnel-wrap',
      role: 'img',
      'aria-label':
        `Conversion funnel: ${stages.map((st) => `${st.label} ${fmtInt(st.value)}`).join(', ')}`,
    });

    stages.forEach((stg, i) => {
      const pct  = (stg.value / max) * 100;
      const stage = DAT.el('div', { class: 'dat-funnel-stage' });
      const head  = DAT.el('div', { class: 'dat-funnel-stage__head' });
      head.appendChild(DAT.el('span', { class: 'dat-funnel-stage__label' }, stg.label));
      head.appendChild(DAT.el('span', { class: 'dat-funnel-stage__value' }, fmtInt(stg.value)));
      stage.appendChild(head);
      const track = DAT.el('div', { class: 'dat-funnel-stage__track' });
      const fill  = DAT.el('div', { class: 'dat-funnel-stage__fill', 'data-stage-index': String(i) });
      // CSS uses --w (% width). The custom property lets us animate via
      // transition without writing inline width churn on re-renders.
      fill.style.width = pct.toFixed(2) + '%';
      track.appendChild(fill);
      stage.appendChild(track);
      wrap.appendChild(stage);

      // Drop-off / gain chip between this stage and the next
      if (i < stages.length - 1) {
        const next     = stages[i + 1].value;
        const prev     = stg.value;
        const ratio    = prev > 0 ? next / prev : 0;
        const delta    = next - prev;
        const isGain   = delta > 0;
        const isFlat   = delta === 0;
        const sign     = isGain ? '↑' : (isFlat ? '·' : '↓');
        const ratioPct = (ratio * 100).toFixed(1) + '%';
        const tone     = isGain ? 'gain' : (isFlat ? 'flat' : 'drop');
        const chip = DAT.el('div', { class: `dat-funnel-step dat-funnel-step--${tone}` });
        chip.appendChild(DAT.el('span', { class: 'dat-funnel-step__arrow' }, sign));
        chip.appendChild(DAT.el('span', { class: 'dat-funnel-step__pct' }, ratioPct));
        chip.appendChild(DAT.el('span', { class: 'dat-funnel-step__delta' },
          (delta > 0 ? '+' : '') + fmtInt(delta)));
        wrap.appendChild(chip);
      }
    });

    host.appendChild(wrap);
  }

  /** Horizontal bar list: top members or top assets.
   *  `emptyCta` (optional) renders an action when no data. */
  function renderHBarChart(host, rows, labelKey, valueKey, formatValue, emptyCta) {
    DAT.clear(host);
    if (!rows || rows.length === 0) {
      const wrap = DAT.el('div', { class: 'dat-chart-empty' });
      wrap.appendChild(DAT.el('p', { class: 'dat-chart-empty__msg' }, 'No data in this period.'));
      if (emptyCta) {
        const cta = DAT.el('a', {
          class: 'dat-chart-empty__cta',
          href: emptyCta.href,
        }, emptyCta.label);
        wrap.appendChild(cta);
      }
      host.appendChild(wrap);
      return;
    }
    const top = rows.slice(0, 10);
    const max = Math.max(...top.map((r) => r[valueKey] || 0), 1);
    const wrap = DAT.el('div', { class: 'dat-hbars' });
    top.forEach((r) => {
      const row = DAT.el('div', { class: 'dat-hbar__row' });
      row.appendChild(DAT.el('span', { class: 'dat-hbar__label' }, r[labelKey] || '—'));
      const barWrap = DAT.el('div', { class: 'dat-hbar__wrap' });
      const fill = DAT.el('span', { class: 'dat-hbar__fill' });
      fill.style.width = ((r[valueKey] || 0) / max) * 100 + '%';
      barWrap.appendChild(fill);
      row.appendChild(barWrap);
      row.appendChild(DAT.el('span', { class: 'dat-hbar__value' }, formatValue(r[valueKey] || 0)));
      wrap.appendChild(row);
    });
    host.appendChild(wrap);
  }

  // ─── Tables + insight lists ───────────────────────────────────────────
  let lastMembers = [];
  let lastAssets = [];

  function tableEmptyState(message) {
    return DAT.el('div', { class: 'dat-table-empty-state' },
      DAT.el('span', { class: 'dat-table-empty-state__logo', 'aria-hidden': 'true' }),
      DAT.el('span', { class: 'dat-table-empty-state__msg' }, message),
    );
  }

  function renderMembersTable(rows) {
    const tbody = DAT.$('#dat-bymember-tbody');
    DAT.clear(tbody);
    if (!rows || rows.length === 0) {
      tbody.appendChild(DAT.el(
        'tr', {},
        DAT.el('td', { colspan: 7, class: 'dat-empty' },
          tableEmptyState("No activity yet. Members appear here once they drive their first referral click.")),
      ));
      return;
    }
    rows.forEach((r) => {
      tbody.appendChild(DAT.el(
        'tr', {},
        DAT.el('td', {}, r.full_name || '—'),
        DAT.el('td', {}, r.email || '—'),
        DAT.el('td', { class: 'dat-table__num' }, fmtInt(r.clicks_count)),
        DAT.el('td', { class: 'dat-table__num' }, fmtInt(r.signups_count)),
        DAT.el('td', { class: 'dat-table__num' }, fmtInt(r.qualified_count)),
        DAT.el('td', { class: 'dat-table__num' }, DAT.fmtCents(r.gross_revenue_cents)),
        DAT.el('td', { class: 'dat-table__num' }, DAT.fmtCents(r.commission_cents)),
      ));
    });
  }

  function renderAssetsTable(rows) {
    const tbody = DAT.$('#dat-byasset-tbody');
    DAT.clear(tbody);
    if (!rows || rows.length === 0) {
      tbody.appendChild(DAT.el(
        'tr', {},
        DAT.el('td', { colspan: 4, class: 'dat-empty' },
          tableEmptyState('No sales yet. Properties appear here once a customer your team referred completes a purchase.')),
      ));
      return;
    }
    rows.forEach((r) => {
      tbody.appendChild(DAT.el(
        'tr', {},
        DAT.el('td', {}, r.asset_name || '—'),
        DAT.el('td', { class: 'dat-table__num' }, fmtInt(r.units_sold)),
        DAT.el('td', { class: 'dat-table__num' }, DAT.fmtCents(r.gross_revenue_cents)),
        DAT.el('td', { class: 'dat-table__num' }, DAT.fmtCents(r.commission_cents)),
      ));
    });
  }

  function renderInsights(_top, deficits) {
    // "Top performers" was removed — it duplicated the "Top members by
    // commission" horizontal-bar chart immediately above. Only the
    // "Members at risk" deficits list remains as an actionable insight.
    const dEl = DAT.$('#dat-rank-deficit');
    if (!dEl) return;
    DAT.clear(dEl);
    if (!deficits || deficits.length === 0) {
      dEl.appendChild(DAT.el('li', { class: 'dat-empty' }, 'Every active member converted.'));
      return;
    }
    deficits.forEach((m) => {
      dEl.appendChild(DAT.el(
        'li', { class: 'dat-rank__item dat-rank__item--warning' },
        DAT.el('span', { class: 'dat-rank__name' }, m.full_name || m.email || '—'),
        DAT.el('span', { class: 'dat-rank__value' }, fmtInt(m.clicks_count) + ' clicks · 0 commission'),
      ));
    });
  }

  // ─── CSV export ────────────────────────────────────────────────────────
  // FG2 fix: drop the local csvEscape+downloadCsv pair (CRLF mismatch with
  // shell.js, no UTF-8 BOM → Excel mis-detected charset on German diacritics).
  // Use DAT.downloadCsv from shell.js which is RFC-4180 compliant + writes
  // a UTF-8 BOM. Convert header+rows-of-objects to the 2-D array shape
  // shell.js expects.
  function downloadCsv(filename, headers, rows) {
    const body = rows.map((r) => headers.map((h) => r[h]));
    DAT.downloadCsv(filename, [headers, ...body]);
  }

  function exportFullCsv(range) {
    const memHeaders = ['Member', 'Email', 'Clicks', 'Signups', 'Qualified', 'Revenue (EUR)', 'Commission (EUR)'];
    const memRows = lastMembers.map((m) => ({
      Member: m.full_name || '',
      Email: m.email || '',
      Clicks: m.clicks_count,
      Signups: m.signups_count,
      Qualified: m.qualified_count,
      'Revenue (EUR)': ((m.gross_revenue_cents || 0) / 100).toFixed(2),
      'Commission (EUR)': ((m.commission_cents || 0) / 100).toFixed(2),
    }));
    downloadCsv(
      `affiliate-team-members_${range.from}_${range.to}.csv`,
      memHeaders, memRows,
    );
  }

  function exportMembersCsv(range) { exportFullCsv(range); }

  function exportAssetsCsv(range) {
    const headers = ['Asset', 'Units', 'Revenue (EUR)', 'Commission (EUR)'];
    const rows = lastAssets.map((a) => ({
      Asset: a.asset_name || '',
      Units: a.units_sold,
      'Revenue (EUR)': ((a.gross_revenue_cents || 0) / 100).toFixed(2),
      'Commission (EUR)': ((a.commission_cents || 0) / 100).toFixed(2),
    }));
    downloadCsv(`affiliate-team-assets_${range.from}_${range.to}.csv`, headers, rows);
  }

  function exportPdf() {
    // Native print → user picks "Save as PDF" in the browser dialog. The
    // print-stylesheet (in developer-affiliate-team.css) hides the sidebar,
    // toolbar, and modal so the printed page is just metrics + charts.
    window.print();
  }

  // ─── Data load ─────────────────────────────────────────────────────────
  async function loadAll(range) {
    const qs = `?from=${encodeURIComponent(range.from)}&to=${encodeURIComponent(range.to)}`;

    DAT.$('#dat-chart-trend-sub').textContent = `${range.from} → ${range.to}`;

    const [ov, ts, mem, ass] = await Promise.allSettled([
      DAT.apiGet('/api/developer/affiliate/team/analytics/overview' + qs),
      DAT.apiGet('/api/developer/affiliate/team/analytics/timeseries' + qs),
      DAT.apiGet('/api/developer/affiliate/team/by-member' + qs),
      DAT.apiGet('/api/developer/affiliate/team/products' + qs),
    ]);

    if (ov.status === 'fulfilled') {
      const o = ov.value.overview;
      renderKpis(o);
      renderFunnelChart(o.period || {});
      renderInsights(o.top_performers, o.deficit_members);
    } else {
      ['revenue', 'commission', 'next-amount', 'conv', 'qualified',
       'pending', 'payable', 'paid', 'members'].forEach((k) => {
        const el = DAT.$('#dat-k-' + k);
        if (el) el.textContent = '—';
      });
      const lu = DAT.$('#dat-last-updated');
      if (lu) lu.textContent = 'Failed to load — retry shortly.';
    }
    if (ts.status === 'fulfilled') {
      renderTrendChart(ts.value.series || []);
    } else {
      DAT.clear(DAT.$('#dat-chart-trend'));
      DAT.$('#dat-chart-trend').appendChild(DAT.el('div', { class: 'dat-chart-empty' }, 'Failed to load trend.'));
    }
    if (mem.status === 'fulfilled') {
      lastMembers = mem.value.rows || [];
      renderMembersTable(lastMembers);
      renderHBarChart(
        DAT.$('#dat-chart-members'),
        lastMembers, 'full_name', 'commission_cents', DAT.fmtCents,
        lastMembers.length === 0 ? { label: 'Invite a member →', href: '/developer/affiliate-team/members' } : null,
      );
    } else {
      // FC3 fix: surface the failure to the user instead of leaving the
      // table stuck on "Loading…" forever. Previously the missing else
      // meant a rejected promise silently disappeared.
      const tbody = DAT.$('#dat-bymember-tbody');
      if (tbody) {
        DAT.clear(tbody);
        tbody.appendChild(DAT.el(
          'tr', null,
          DAT.el('td', { colspan: 7, class: 'dat-empty dat-empty--error' },
            'Failed to load member breakdown. Try refreshing.'),
        ));
      }
      DAT.clear(DAT.$('#dat-chart-members'));
      DAT.$('#dat-chart-members').appendChild(
        DAT.el('div', { class: 'dat-chart-empty' }, 'Failed to load top members.'),
      );
    }
    if (ass.status === 'fulfilled') {
      lastAssets = ass.value.rows || [];
      renderAssetsTable(lastAssets);
      renderHBarChart(
        DAT.$('#dat-chart-assets'),
        lastAssets, 'asset_name', 'gross_revenue_cents', DAT.fmtCents,
        lastAssets.length === 0 ? { label: 'Browse marketplace →', href: '/marketplace' } : null,
      );
    } else {
      // FC3 fix: same as mem above. Make the failure visible.
      const tbody = DAT.$('#dat-byasset-tbody');
      if (tbody) {
        DAT.clear(tbody);
        tbody.appendChild(DAT.el(
          'tr', null,
          DAT.el('td', { colspan: 4, class: 'dat-empty dat-empty--error' },
            'Failed to load assets breakdown. Try refreshing.'),
        ));
      }
      DAT.clear(DAT.$('#dat-chart-assets'));
      DAT.$('#dat-chart-assets').appendChild(
        DAT.el('div', { class: 'dat-chart-empty' }, 'Failed to load top assets.'),
      );
    }
  }

  // ─── Preset / filter wiring ────────────────────────────────────────────
  function applyRangeUi(range) {
    const fromEl = DAT.$('#dat-an-from');
    const toEl   = DAT.$('#dat-an-to');
    if (fromEl) fromEl.value = range.from;
    if (toEl)   toEl.value   = range.to;
    const customBtn = DAT.$('.dat-preset--custom');
    if (customBtn) customBtn.hidden = range.preset !== 'custom';
    document.querySelectorAll('.dat-preset').forEach((b) => {
      b.classList.toggle('dat-preset--active', b.dataset.preset === range.preset);
    });
    // Paint the topbar trigger button label so the active period is visible
    // even with the popover closed.
    const label = DAT.$('#dat-topbar-range-label');
    if (label) {
      if (range.preset === 'custom' && range.from && range.to) {
        label.textContent = `${range.from} → ${range.to}`;
      } else {
        label.textContent = presetLabel(range.preset);
      }
    }
  }

  /** Debounce timer for date-input auto-apply (avoid hammering during typing). */
  let dateChangeTimer = null;
  function scheduleCustomApply() {
    if (dateChangeTimer) clearTimeout(dateChangeTimer);
    dateChangeTimer = setTimeout(() => {
      const from = DAT.$('#dat-an-from').value;
      const to = DAT.$('#dat-an-to').value;
      if (!from || !to) return;
      if (from > to) return; // silently ignore invalid until both correct
      const r = { from, to, preset: 'custom' };
      applyRangeUi(r);
      persistRange(from, to, 'custom');
      loadAll(r);
    }, 350);
  }

  function openTopbarRange() {
    const popover = DAT.$('#dat-topbar-range-popover');
    const trigger = DAT.$('#dat-topbar-range-trigger');
    if (!popover) return;
    popover.removeAttribute('hidden');
    if (trigger) trigger.setAttribute('aria-expanded', 'true');
  }
  function closeTopbarRange() {
    const popover = DAT.$('#dat-topbar-range-popover');
    const trigger = DAT.$('#dat-topbar-range-trigger');
    if (!popover) return;
    popover.setAttribute('hidden', '');
    if (trigger) trigger.setAttribute('aria-expanded', 'false');
  }

  function wireToolbar() {
    document.querySelectorAll('.dat-preset').forEach((btn) => {
      btn.addEventListener('click', () => {
        const preset = btn.dataset.preset;
        if (preset === 'custom') return; // 'Custom' pill is informational
        const r = Object.assign({ preset }, presetRange(preset));
        applyRangeUi(r);
        persistRange(r.from, r.to, preset);
        loadAll(r);
        closeTopbarRange();
      });
    });

    // Topbar date-range popover — click trigger toggles the panel; clicking
    // outside or pressing Escape closes it.
    const trigger = DAT.$('#dat-topbar-range-trigger');
    const popover = DAT.$('#dat-topbar-range-popover');
    if (trigger && popover) {
      trigger.addEventListener('click', (e) => {
        e.stopPropagation();
        const open = popover.hasAttribute('hidden');
        if (open) openTopbarRange(); else closeTopbarRange();
      });
      document.addEventListener('click', (e) => {
        if (popover.hasAttribute('hidden')) return;
        if (popover.contains(e.target) || trigger.contains(e.target)) return;
        closeTopbarRange();
      });
      document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') closeTopbarRange();
      });
    }

    // Auto-apply when user picks a custom date (debounced 350ms).
    ['#dat-an-from', '#dat-an-to'].forEach((sel) => {
      const el = DAT.$(sel);
      if (el) el.addEventListener('change', scheduleCustomApply);
    });

    // Export buttons — page-level "Export CSV" CTA in the sticky toolbar
    // downloads the full member breakdown. Older granular handlers are
    // still bound defensively so re-introducing those buttons just
    // requires un-hiding the markup.
    const bind = (sel, fn) => { const el = DAT.$(sel); if (el) el.addEventListener('click', fn); };
    bind('#dat-page-export-csv',    () => exportFullCsv(currentRange()));
    bind('#dat-export-csv',         () => exportFullCsv(currentRange()));
    bind('#dat-export-csv-members', () => exportMembersCsv(currentRange()));
    bind('#dat-export-csv-assets',  () => exportAssetsCsv(currentRange()));
    bind('#dat-export-pdf',         exportPdf);
  }

  /// Roving-tabindex keyboard navigation for resolution toggle:
  /// ← / → cycle, Home/End jump, Enter/Space activate. Scoped to
  /// `[data-res]` so the metric toggle (`[data-metric]`) is independent.
  function wireResolutionToggle() {
    const btns = Array.from(document.querySelectorAll('.dat-res-btn[data-res]'));
    btns.forEach((btn, idx) => {
      const isActive = btn.classList.contains('dat-res-btn--active');
      btn.setAttribute('tabindex', isActive ? '0' : '-1');
      btn.setAttribute('aria-pressed', isActive ? 'true' : 'false');
      btn.addEventListener('click', () => {
        setTrendResolution(btn.dataset.res);
        focusActiveResBtn();
      });
      btn.addEventListener('keydown', (e) => {
        let nextIdx = null;
        if (e.key === 'ArrowRight') nextIdx = (idx + 1) % btns.length;
        else if (e.key === 'ArrowLeft') nextIdx = (idx - 1 + btns.length) % btns.length;
        else if (e.key === 'Home') nextIdx = 0;
        else if (e.key === 'End') nextIdx = btns.length - 1;
        else if (e.key === ' ' || e.key === 'Enter') {
          e.preventDefault();
          setTrendResolution(btn.dataset.res);
          focusActiveResBtn();
          return;
        }
        if (nextIdx !== null) {
          e.preventDefault();
          setTrendResolution(btns[nextIdx].dataset.res);
          focusActiveResBtn();
          btns[nextIdx].focus();
        }
      });
    });
  }
  function focusActiveResBtn() {
    document.querySelectorAll('.dat-res-btn[data-res]').forEach((b) => {
      const isActive = b.classList.contains('dat-res-btn--active');
      b.setAttribute('tabindex', isActive ? '0' : '-1');
      b.setAttribute('aria-pressed', isActive ? 'true' : 'false');
    });
  }

  /// Mirror of wireResolutionToggle for the Commission/Revenue metric toggle.
  /// Same keyboard model so the two toggle groups feel identical to use.
  function wireMetricToggle() {
    const btns = Array.from(document.querySelectorAll('.dat-res-btn[data-metric]'));
    btns.forEach((btn, idx) => {
      const isActive = btn.classList.contains('dat-res-btn--active');
      btn.setAttribute('tabindex', isActive ? '0' : '-1');
      btn.setAttribute('aria-pressed', isActive ? 'true' : 'false');
      btn.addEventListener('click', () => {
        setTrendMetric(btn.dataset.metric);
        focusActiveMetricBtn();
      });
      btn.addEventListener('keydown', (e) => {
        let nextIdx = null;
        if (e.key === 'ArrowRight') nextIdx = (idx + 1) % btns.length;
        else if (e.key === 'ArrowLeft') nextIdx = (idx - 1 + btns.length) % btns.length;
        else if (e.key === 'Home') nextIdx = 0;
        else if (e.key === 'End') nextIdx = btns.length - 1;
        else if (e.key === ' ' || e.key === 'Enter') {
          e.preventDefault();
          setTrendMetric(btn.dataset.metric);
          focusActiveMetricBtn();
          return;
        }
        if (nextIdx !== null) {
          e.preventDefault();
          setTrendMetric(btns[nextIdx].dataset.metric);
          focusActiveMetricBtn();
          btns[nextIdx].focus();
        }
      });
    });
  }
  function focusActiveMetricBtn() {
    document.querySelectorAll('.dat-res-btn[data-metric]').forEach((b) => {
      const isActive = b.classList.contains('dat-res-btn--active');
      b.setAttribute('tabindex', isActive ? '0' : '-1');
      b.setAttribute('aria-pressed', isActive ? 'true' : 'false');
    });
  }

  /// Hide the trend chart's discoverability hint after the user's first
  /// pointer-interaction with the card — bound at init so it fires regardless
  /// of whether the chart has data (empty-state has no SVG overlay).
  function wireTrendHoverHint() {
    const card = DAT.$('#dat-trend-card');
    if (!card) return;
    const dismiss = () => card.classList.add('dat-chart-card--touched');
    card.addEventListener('mouseenter', dismiss, { once: true });
    card.addEventListener('touchstart', dismiss, { once: true, passive: true });
    card.addEventListener('focusin', dismiss, { once: true });
  }

  document.addEventListener('DOMContentLoaded', () => {
    const r = currentRange();
    applyRangeUi(r);
    wireToolbar();
    wireResolutionToggle();
    wireMetricToggle();
    wireTrendHoverHint();
    loadAll(r);
    loadCohort();
  });

  // ── Phase-5: cohort retention heatmap ──────────────────────────────
  async function loadCohort() {
    const wrap = document.getElementById('dat-cohort-wrap');
    if (!wrap) return;
    try {
      const res = await fetch('/api/developer/affiliate/team/analytics/cohort?months=12',
        { credentials: 'same-origin' });
      if (!res.ok) {
        wrap.innerHTML = '<p class="dat-empty">No cohort data yet.</p>';
        return;
      }
      const data = await res.json();
      const cells = (data && data.cells) || [];
      if (!cells.length) {
        wrap.innerHTML = '<p class="dat-empty">No cohort data yet. Once team members acquire customers, their retention will appear here.</p>';
        return;
      }
      // Pivot to (cohort_month → period_index → active_users)
      const months = [];
      const grid = new Map();
      for (const c of cells) {
        if (!grid.has(c.cohort_month)) {
          months.push(c.cohort_month);
          grid.set(c.cohort_month, { size: c.cohort_size, cols: new Map() });
        }
        grid.get(c.cohort_month).cols.set(c.period_index, c.active_users);
      }
      const maxPeriod = Math.min(11, Math.max(0, ...cells.map(function (c) { return c.period_index; })));

      let html = '<table class="dat-cohort"><thead><tr>';
      html += '<th>Cohort</th><th>Size</th>';
      for (let p = 0; p <= maxPeriod; p++) html += '<th>M+' + p + '</th>';
      html += '</tr></thead><tbody>';
      for (const m of months) {
        const row = grid.get(m);
        html += '<tr><th>' + m + '</th><td class="dat-cohort__size">' + row.size + '</td>';
        for (let p = 0; p <= maxPeriod; p++) {
          const active = row.cols.get(p) || 0;
          const pct = row.size > 0 ? Math.round((active / row.size) * 100) : 0;
          // 0% → very light, 100% → strong Electric Blue.
          const alpha = Math.max(0.06, Math.min(1, pct / 100));
          const bg = 'rgba(0, 0, 255, ' + alpha.toFixed(2) + ')';
          const fg = pct > 50 ? '#fff' : '#101828';
          html += '<td class="dat-cohort__cell" style="background:' + bg + ';color:' + fg + ';" title="'
            + active + ' of ' + row.size + ' active in M+' + p + '">'
            + (active === 0 ? '·' : pct + '%')
            + '</td>';
        }
        html += '</tr>';
      }
      html += '</tbody></table>';
      wrap.innerHTML = html;
    } catch (_) {
      wrap.innerHTML = '<p class="dat-empty">Could not load cohort data.</p>';
    }
  }
})();
