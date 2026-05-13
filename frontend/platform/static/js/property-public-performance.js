/**
 * Property Public — Live Performance cards (Villa-Returns P3 / PDF §A5).
 * Minimal sales-acquisition surface for unauthenticated visitors.
 * No time-travel, no currency toggle, no chart — just 3 KPIs.
 *
 * Reuses public endpoint GET /api/villas/:asset_id/performance
 * (no auth required). Asset id is in data-asset-id on .financial-section
 * (injected by MiniJinja).
 *
 * Hooks into the existing financial-tabs switcher in property-detail.js,
 * which already knows about data-tab="live-performance" thanks to the
 * P3 edits — same switcher works on both auth'd and public pages.
 */

(function () {
  document.addEventListener("DOMContentLoaded", () => {
    const section = document.querySelector(".financial-section[data-asset-id]");
    const assetId = section?.dataset?.assetId;
    if (!assetId) return;
    hydrate(assetId);
  });

  async function hydrate(assetId) {
    const meta = document.getElementById("lp-pub-meta");
    try {
      const r = await fetch(`/api/villas/${encodeURIComponent(assetId)}/performance`);
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const p = await r.json();
      render(p);
    } catch (err) {
      if (meta) meta.textContent = `Live data unavailable: ${err.message}`;
    }
  }

  function render(p) {
    setText("lp-pub-yield", `${(p.annual_yield_bps / 100).toFixed(2)} %`);
    setText(
      "lp-pub-proj",
      p.projected_annual_net_return_bps != null
        ? `${(p.projected_annual_net_return_bps / 100).toFixed(2)} %`
        : "No forecast"
    );
    setText(
      "lp-pub-five",
      p.five_year_total_return_bps != null
        ? `${(p.five_year_total_return_bps / 100).toFixed(2)} %`
        : "No forecast"
    );
    const meta = document.getElementById("lp-pub-meta");
    if (meta) {
      const bits = [];
      if (p.months_with_data > 0) bits.push(`${p.months_with_data} month${p.months_with_data === 1 ? "" : "s"} of data`);
      if (p.valuation_date) bits.push(`Valuation: ${p.valuation_date}`);
      if (p.forecast_source_year) bits.push(`Forecast ${p.forecast_source_year}`);
      meta.textContent = bits.length ? bits.join(" · ") : "Live numbers from POOOL's calculation engine";
    }
  }

  function setText(id, value) {
    const el = document.getElementById(id);
    if (el) el.textContent = value;
  }
})();
