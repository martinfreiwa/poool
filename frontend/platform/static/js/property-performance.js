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

  function renderNavChart(svgOrContainer, metaEl, points) {
    const idr = displayCurrency === "IDR";

    // The legacy template still ships <svg id="lp-nav-chart">; ECharts needs
    // a block-level DIV to mount into. On first call we replace the SVG with
    // a sibling DIV that keeps the same id so subsequent lookups still find it.
    let host = svgOrContainer;
    if (host && host.tagName && host.tagName.toLowerCase() === "svg") {
      const div = document.createElement("div");
      div.id = host.id;
      div.style.width = "100%";
      div.style.height = "200px";
      host.parentNode.replaceChild(div, host);
      host = div;
    }
    if (!host) return;

    if (!points.length) {
      host.innerHTML = `<div style="height:200px;display:flex;align-items:center;justify-content:center;color:#6b7280;font-size:12px;">No valuation history yet</div>`;
      if (metaEl) metaEl.textContent = "";
      return;
    }

    const values = points.map((p) => Number(idr ? p.value_idr_cents : p.value_usd_cents) / 100);
    const labels = points.map((p) => p.date);

    if (typeof window.PooolLineChart === "undefined") {
      // Library not loaded — surface a readable fallback rather than blanking.
      host.innerHTML = `<div style="height:200px;display:flex;align-items:center;justify-content:center;color:#dc2626;font-size:12px;">Chart library unavailable</div>`;
      return;
    }

    window.PooolLineChart.render(host, {
      labels,
      values,
      step: "end",                   // NAV constant between publishes
      formatter: (v) => formatValue(v * 100, idr),
      height: 200,
    });

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
