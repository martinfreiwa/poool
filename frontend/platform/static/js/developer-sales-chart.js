/**
 * developer-sales-chart.js
 *
 * Replaces the server-rendered SVG sales-over-time chart with an ECharts
 * smooth-area-line (charts-showcase #14 style). Avoids any Rust changes by
 * recovering the data points from the existing rendered DOM:
 *   - x labels    ← `#chart-x-axis span` text
 *   - y axis max  ← topmost `#chart-y-axis span` label (parses "$200k" → 200000)
 *   - data values ← `path.chart-line` `d` attribute (extracts y coords, inverts)
 *
 * Why DOM-recovery instead of a backend data feed:
 *   - The Rust developer dashboard code is currently mid-WIP; adding a new
 *     response field would bundle with that change. The path string is
 *     a stable existing surface — parsing it client-side keeps the change
 *     fully scoped to the frontend.
 *
 * Re-runs on initial load and after every HTMX swap of `#sales-chart-card`
 * (period tabs).
 *
 * Requires: window.echarts, window.PooolLineChart.
 */
(function () {
  "use strict";

  if (typeof window === "undefined") return;

  // ─── Selectors mirror the existing developer-chart.html template ───
  const SEL = {
    card:     "#sales-chart-card",
    svg:      "#sales-chart-card .chart-area svg",
    linePath: "#sales-chart-card path.chart-line",
    xAxis:    "#sales-chart-card #chart-x-axis span",
    yAxis:    "#sales-chart-card #chart-y-axis span",
    sidebarYAxis: "#sales-chart-card #chart-y-axis",
  };

  // ─── Parse "$200k", "$1.5M", "200" → number of dollars ───
  function parseAxisLabel(raw) {
    if (!raw) return null;
    const m = raw.replace(/\s/g, "").match(/^\$?(-?[\d.,]+)\s*([kKmMbB])?$/);
    if (!m) return null;
    const num = parseFloat(m[1].replace(/,/g, ""));
    if (!Number.isFinite(num)) return null;
    const mult = { k: 1e3, K: 1e3, m: 1e6, M: 1e6, b: 1e9, B: 1e9 }[m[2]] || 1;
    return num * mult;
  }

  // ─── Extract Y coords from an SVG path "M x y L x y L x y …" ───
  function pathToYCoords(d) {
    if (!d) return [];
    // Matches commands "M 100 50" or "L 200.5 80.2" — captures the y value.
    const re = /[MLml]\s*(-?[\d.]+)[\s,]+(-?[\d.]+)/g;
    const ys = [];
    let m;
    while ((m = re.exec(d)) !== null) ys.push(parseFloat(m[2]));
    return ys;
  }

  function readChartData() {
    const card = document.querySelector(SEL.card);
    if (!card) return null;

    const xs = Array.from(document.querySelectorAll(SEL.xAxis)).map((s) =>
      s.textContent.trim()
    );
    const yAxisLabels = Array.from(document.querySelectorAll(SEL.yAxis)).map((s) =>
      s.textContent.trim()
    );
    const pathEl = document.querySelector(SEL.linePath);
    const svg = document.querySelector(SEL.svg);
    if (!pathEl || !svg) return null;

    const d = pathEl.getAttribute("d") || "";
    const ys = pathToYCoords(d);
    if (ys.length === 0) return null;

    // SVG viewBox dictates coordinate space. Default in the template: 0 0 1002 240.
    const viewBox = (svg.getAttribute("viewBox") || "0 0 1002 240").split(/\s+/);
    const vbHeight = parseFloat(viewBox[3]) || 240;

    // y-axis labels are rendered top-to-bottom (max → 0). Use first non-empty
    // as the axis max, in dollars.
    let axisMax = null;
    for (const label of yAxisLabels) {
      const v = parseAxisLabel(label);
      if (v !== null && v > 0) { axisMax = v; break; }
    }
    if (axisMax === null) {
      // Fallback: assume y coords already map directly (use 1 as max scale)
      axisMax = 1;
    }

    // Invert: y=0 → axisMax, y=vbHeight → 0
    const values = ys.map((y) => Math.max(0, (1 - y / vbHeight) * axisMax));

    // Distribute labels evenly across the data points (template renders sparse
    // labels — fewer X labels than data points). Use empty strings for gaps.
    const labels = new Array(values.length).fill("");
    if (xs.length > 0) {
      for (let i = 0; i < xs.length; i++) {
        const idx = Math.round((i / Math.max(1, xs.length - 1)) * (values.length - 1));
        labels[idx] = xs[i];
      }
    }

    return { labels, values, axisMax };
  }

  function formatUsd(v) {
    if (Math.abs(v) >= 1e9) return "$" + (v / 1e9).toFixed(1).replace(/\.0$/, "") + "B";
    if (Math.abs(v) >= 1e6) return "$" + (v / 1e6).toFixed(1).replace(/\.0$/, "") + "M";
    if (Math.abs(v) >= 1e3) return "$" + (v / 1e3).toFixed(1).replace(/\.0$/, "") + "k";
    return "$" + Math.round(v).toLocaleString("en-US");
  }

  // ─── Mount: hide old SVG, insert ECharts container, render ──────
  function mount() {
    if (typeof window.PooolLineChart === "undefined") return;

    const card = document.querySelector(SEL.card);
    if (!card) return;

    const data = readChartData();
    if (!data) return;

    const chartArea = card.querySelector(".chart-area");
    if (!chartArea) return;

    // Hide the original Y-axis labels column (ECharts renders its own)
    const yAxisCol = card.querySelector("#chart-y-axis");
    if (yAxisCol) yAxisCol.style.display = "none";

    // Hide the original SVG + x-axis spans
    const oldSvg = chartArea.querySelector("svg");
    if (oldSvg) oldSvg.style.display = "none";
    const oldXAxis = chartArea.querySelector("#chart-x-axis");
    if (oldXAxis) oldXAxis.style.display = "none";

    // Create / reuse ECharts container as a sibling of the old svg
    let ecContainer = chartArea.querySelector("#sales-chart-echarts");
    if (!ecContainer) {
      ecContainer = document.createElement("div");
      ecContainer.id = "sales-chart-echarts";
      ecContainer.style.width = "100%";
      ecContainer.style.height = "260px";
      chartArea.insertBefore(ecContainer, chartArea.firstChild);
    }

    window.PooolLineChart.render(ecContainer, {
      labels: data.labels,
      values: data.values,
      yMax: data.axisMax,
      formatter: formatUsd,
      height: 260,
    });
  }

  window.DeveloperSalesChart = { mount };

  function safeMount() {
    try { mount(); } catch (e) { console.warn("[dev-sales-chart] mount failed:", e); }
  }

  // Initial render — defer one tick so any sibling scripts that mutate the
  // chart card in DOMContentLoaded run first.
  function boot() { setTimeout(safeMount, 0); }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }

  // HTMX swap → re-mount. Two paths because htmx fires `htmx:afterSwap`
  // with different `detail.target` semantics depending on whether the
  // swap is outerHTML (target = parent) or innerHTML (target = the swap
  // target itself). MutationObserver below covers any case the event
  // misses (programmatic `htmx.ajax`, oob swaps, etc).
  document.body.addEventListener("htmx:afterSwap", function () {
    if (document.getElementById("sales-chart-card") &&
        !document.getElementById("sales-chart-echarts")) {
      safeMount();
    }
  });

  // MutationObserver: catch any DOM mutation that replaces the chart card
  // (HTMX outerHTML, manual innerHTML, etc.) and re-render ECharts.
  const sectionRoot = document.getElementById("sales-chart-section") || document.body;
  const mo = new MutationObserver(() => {
    if (document.getElementById("sales-chart-card") &&
        !document.getElementById("sales-chart-echarts")) {
      safeMount();
    }
  });
  mo.observe(sectionRoot, { childList: true, subtree: true });
})();
