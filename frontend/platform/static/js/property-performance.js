/**
 * Property Live Performance — Villa-Returns P3 (+ as-of time-travel, USD/IDR toggle).
 * Hydrates the "Live performance" tab on property.html with KPIs from
 *   GET /api/villas/:asset_id/performance?as_of=...
 * Asset id is read from data-asset-id on .financial-section.
 *
 * State held on the IIFE closure:
 *   - displayCurrency: "USD" | "IDR"
 *   - asOf: ISO yyyy-mm-dd | null (latest)
 */

(function () {
  let assetId = null;
  let displayCurrency = "USD";
  let asOf = null; // YYYY-MM-DD or null
  let lastBundle = null;
  let lastChartPoints = null;

  document.addEventListener("DOMContentLoaded", () => {
    const section = document.querySelector(".financial-section[data-asset-id]");
    assetId = section?.dataset?.assetId;
    if (!assetId) return;
    wireControls();
    hydrate();
  });

  function wireControls() {
    document.querySelectorAll(".lp-cur-btn").forEach((b) => {
      b.addEventListener("click", () => {
        displayCurrency = b.dataset.cur;
        document.querySelectorAll(".lp-cur-btn").forEach((other) => {
          const active = other === b;
          other.classList.toggle("lp-cur-active", active);
          other.style.background = active ? "var(--admin-accent, #2563eb)" : "transparent";
          other.style.color = active ? "white" : "inherit";
        });
        // No refetch needed — re-render from cached data (both currencies present).
        if (lastBundle) render(lastBundle);
        const svg = document.getElementById("lp-nav-chart");
        const metaEl = document.getElementById("lp-chart-meta");
        if (svg && lastChartPoints) renderNavChart(svg, metaEl, lastChartPoints);
      });
    });

    const asofEl = document.getElementById("lp-asof");
    const resetEl = document.getElementById("lp-asof-reset");
    if (asofEl) {
      asofEl.addEventListener("change", () => {
        asOf = asofEl.value || null;
        hydrate();
      });
    }
    if (resetEl) {
      resetEl.addEventListener("click", () => {
        asOf = null;
        if (asofEl) asofEl.value = "";
        hydrate();
      });
    }
  }

  async function hydrate() {
    const meta = document.getElementById("lp-meta");
    const banner = document.getElementById("lp-asof-banner");
    const bannerText = document.getElementById("lp-asof-banner-text");
    if (banner) banner.style.display = asOf ? "" : "none";
    if (bannerText && asOf) {
      bannerText.textContent = `Viewing as published at ${asOf}. Values may have been corrected since this date.`;
    }
    try {
      let url = `/api/villas/${encodeURIComponent(assetId)}/performance`;
      if (asOf) {
        url += `?as_of=${encodeURIComponent(asOf + "T23:59:59Z")}`;
      }
      const r = await fetch(url);
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      lastBundle = await r.json();
      render(lastBundle);
    } catch (err) {
      if (meta) meta.textContent = `Live data unavailable: ${err.message}`;
    }
    // Independent: NAV history series for the chart.
    await loadNavChart();
  }

  async function loadNavChart() {
    const svg = document.getElementById("lp-nav-chart");
    const meta = document.getElementById("lp-chart-meta");
    if (!svg) return;
    try {
      const r = await fetch(`/api/villas/${encodeURIComponent(assetId)}/history?metric=nav`);
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const series = await r.json();
      lastChartPoints = series.points || [];
      renderNavChart(svg, meta, lastChartPoints);
    } catch (err) {
      svg.innerHTML = `<text x="300" y="90" text-anchor="middle" font-size="12" fill="#dc2626">Chart failed: ${escapeText(err.message)}</text>`;
    }
  }

  function renderNavChart(svg, metaEl, points) {
    const idr = displayCurrency === "IDR";
    if (!points.length) {
      svg.innerHTML = `<text x="300" y="90" text-anchor="middle" font-size="12" fill="#6b7280">No valuation history yet</text>`;
      if (metaEl) metaEl.textContent = "";
      return;
    }
    const values = points.map((p) => Number(idr ? p.value_idr_cents : p.value_usd_cents));
    const maxV = Math.max(...values, 1);
    const minV = Math.min(...values);
    const padTop = 12, padBot = 24, padL = 56, padR = 12;
    const W = 600, H = 180;
    const innerW = W - padL - padR;
    const innerH = H - padTop - padBot;

    // X positions: spread points evenly across the inner width.
    const xs = points.map((_, i) => padL + (points.length === 1 ? innerW / 2 : (i * innerW) / (points.length - 1)));
    const yFor = (v) => {
      if (maxV === minV) return padTop + innerH / 2;
      return padTop + innerH - ((v - minV) / (maxV - minV)) * innerH;
    };

    // Step-function path: NAV is constant between publishes.
    let d = "";
    points.forEach((p, i) => {
      const x = xs[i];
      const y = yFor(Number(idr ? p.value_idr_cents : p.value_usd_cents));
      if (i === 0) d += `M ${x} ${y}`;
      else d += ` H ${x} V ${y}`;
    });

    const axis = `
      <line x1="${padL}" y1="${padTop}" x2="${padL}" y2="${padTop + innerH}" stroke="#e5e7eb" />
      <line x1="${padL}" y1="${padTop + innerH}" x2="${W - padR}" y2="${padTop + innerH}" stroke="#e5e7eb" />
    `;
    const yLabelMax = `<text x="${padL - 6}" y="${padTop + 4}" text-anchor="end" font-size="10" fill="#6b7280">${formatValue(maxV, idr)}</text>`;
    const yLabelMin = `<text x="${padL - 6}" y="${padTop + innerH}" text-anchor="end" font-size="10" fill="#6b7280">${formatValue(minV, idr)}</text>`;
    const dots = points
      .map((p, i) => `<circle cx="${xs[i]}" cy="${yFor(Number(idr ? p.value_idr_cents : p.value_usd_cents))}" r="3" fill="#2563eb"><title>${escapeText(p.date)} — ${formatValue(Number(idr ? p.value_idr_cents : p.value_usd_cents), idr)}</title></circle>`)
      .join("");
    const xLabels = points
      .map((p, i) => {
        if (points.length > 6 && i % Math.ceil(points.length / 6) !== 0 && i !== points.length - 1) return "";
        return `<text x="${xs[i]}" y="${H - 6}" text-anchor="middle" font-size="10" fill="#6b7280">${escapeText(p.date)}</text>`;
      })
      .join("");

    svg.innerHTML = `
      ${axis}
      ${yLabelMax}
      ${yLabelMin}
      <path d="${d}" fill="none" stroke="#2563eb" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
      ${dots}
      ${xLabels}
    `;
    if (metaEl) {
      metaEl.textContent = `${points.length} valuation${points.length === 1 ? "" : "s"} · ${idr ? "IDR" : "USD"} · NAV per token`;
    }
  }

  function formatValue(cents, idrFirst) {
    return idrFirst ? formatIdr(cents) : formatUsd(cents);
  }

  function fmtSpDelta(bps) {
    if (bps == null) return "—";
    const pct = bps / 100;
    const sign = pct > 0 ? "+" : "";
    return `${sign}${pct.toFixed(2)} %`;
  }

  function escapeText(s) {
    return String(s == null ? "" : s).replace(/[<>&"']/g, (c) =>
      ({ "<": "&lt;", ">": "&gt;", "&": "&amp;", '"': "&quot;", "'": "&#39;" }[c])
    );
  }

  function render(p) {
    const idr = displayCurrency === "IDR";
    setText("lp-nav",
      (idr ? p.nav_token_idr_cents : p.nav_token_usd_cents) > 0
        ? format(p.nav_token_idr_cents, p.nav_token_usd_cents, idr)
        : "Not yet valued");
    setText("lp-period",
      p.latest_period_year && p.latest_period_month
        ? `${p.latest_period_year}-${String(p.latest_period_month).padStart(2, "0")}`
        : "No data yet");
    setText("lp-latest-dist",
      (idr ? p.latest_distributable_idr_cents : p.latest_distributable_usd_cents) > 0
        ? format(p.latest_distributable_idr_cents, p.latest_distributable_usd_cents, idr)
        : "—");
    setText("lp-12m-dist", format(p.last_12m_distributable_idr_cents, p.last_12m_distributable_usd_cents, idr));
    setText("lp-yield", `${(p.annual_yield_bps / 100).toFixed(2)} %`);
    setText("lp-monthly-yield", `${(p.annual_yield_bps / 1200).toFixed(2)} %`);
    setText(
      "lp-proj-annual",
      p.projected_annual_net_return_bps != null
        ? `${(p.projected_annual_net_return_bps / 100).toFixed(2)} %`
        : "No forecast"
    );
    setText(
      "lp-five-year",
      p.five_year_total_return_bps != null
        ? `${(p.five_year_total_return_bps / 100).toFixed(2)} %`
        : "No forecast"
    );
    setText("lp-sp-3m",  fmtSpDelta(p.share_price_3m_bps));
    setText("lp-sp-6m",  fmtSpDelta(p.share_price_6m_bps));
    setText("lp-sp-12m", fmtSpDelta(p.share_price_12m_bps));

    const meta = document.getElementById("lp-meta");
    if (meta) {
      const parts = [];
      if (p.valuation_date) parts.push(`Valuation: ${p.valuation_date}`);
      if (p.tokenized_pct_bps) parts.push(`Tokenized ${(p.tokenized_pct_bps / 100).toFixed(2)}%`);
      if (p.tokens_in_pool) parts.push(`${Number(p.tokens_in_pool).toLocaleString()} tokens in pool`);
      parts.push(`Based on ${p.months_with_data} month${p.months_with_data === 1 ? "" : "s"} of data`);
      meta.textContent = parts.join(" · ");
    }
  }

  function format(idrCents, usdCents, idrFirst) {
    const primary = idrFirst ? formatIdr(idrCents) : formatUsd(usdCents);
    const secondary = idrFirst ? formatUsd(usdCents) : formatIdr(idrCents);
    return `${primary} (${secondary})`;
  }

  function formatUsd(cents) {
    const usd = Number(cents) / 100;
    return `USD ${usd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  }

  function formatIdr(cents) {
    const idr = Number(cents) / 100;
    return `IDR ${idr.toLocaleString(undefined, { maximumFractionDigits: 0 })}`;
  }

  function setText(id, value) {
    const el = document.getElementById(id);
    if (el) el.textContent = value;
  }
})();
