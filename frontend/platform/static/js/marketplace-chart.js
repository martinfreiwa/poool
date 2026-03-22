/**
 * marketplace-chart.js — Candlestick Chart Integration (Task 5.4)
 *
 * Renders OHLCV candlestick charts using ApexCharts, powered by the
 * GET /api/marketplace/:asset_id/candles backend API.
 *
 * Features:
 * - 7 interval buttons (1m, 5m, 15m, 1h, 4h, 1d, 1w)
 * - 24h chart summary header (last price, change%, high/low)
 * - Real-time trade updates via MarketBus events
 * - Responsive, dark-themed design matching POOOL's aesthetic
 * - Graceful fallback to mock area chart if API unavailable
 *
 * Requires: ApexCharts CDN loaded, marketplace-event-bus.js
 */
const MarketChart = (function () {
  "use strict";

  let _chart = null;
  let _assetId = null;
  let _interval = "1h";
  let _container = null;

  const INTERVALS = ["1m", "5m", "15m", "1h", "4h", "1d", "1w"];

  // ═══════════════════════════════════════════════════════════════
  // ── API ─────────────────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  async function fetchCandles(assetId, interval) {
    try {
      const res = await fetch(
        `/api/marketplace/${assetId}/candles?interval=${interval}`
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return await res.json();
    } catch (err) {
      console.warn("[Chart] Candle API unavailable, using mock:", err.message);
      return null;
    }
  }

  async function fetchChartSummary(assetId) {
    try {
      const res = await fetch(
        `/api/marketplace/${assetId}/chart-summary`
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return await res.json();
    } catch {
      return null;
    }
  }

  // ═══════════════════════════════════════════════════════════════
  // ── MOCK DATA (fallback) ────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function generateMockCandles(count = 100, basePrice = 10500) {
    const candles = [];
    const now = Date.now();
    let price = basePrice;

    for (let i = count; i > 0; i--) {
      const timestamp = now - i * 3600 * 1000;
      const volatility = (Math.random() - 0.48) * 300;
      const open = price;
      const close = Math.max(5000, price + volatility);
      const high = Math.max(open, close) + Math.random() * 150;
      const low = Math.min(open, close) - Math.random() * 150;
      const volume = Math.floor(Math.random() * 50) + 5;

      candles.push({
        timestamp: new Date(timestamp).toISOString(),
        open_cents: Math.round(open),
        high_cents: Math.round(high),
        low_cents: Math.round(low),
        close_cents: Math.round(close),
        volume,
        trade_count: Math.floor(Math.random() * 10) + 1,
      });

      price = close;
    }

    return candles;
  }

  // ═══════════════════════════════════════════════════════════════
  // ── CHART RENDERING ─────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function renderChart(candles) {
    if (!_container || typeof ApexCharts === "undefined") return;

    // Destroy existing chart
    if (_chart) {
      _chart.destroy();
      _chart = null;
    }

    if (!candles || candles.length === 0) {
      _container.innerHTML =
        '<div style="display:flex;align-items:center;justify-content:center;height:300px;color:#7a7f87;font-size:14px;">No trading data available</div>';
      return;
    }

    // Transform to ApexCharts candlestick format
    const ohlcData = candles.map((c) => ({
      x: new Date(c.timestamp),
      y: [
        c.open_cents / 100,
        c.high_cents / 100,
        c.low_cents / 100,
        c.close_cents / 100,
      ],
    }));

    const volumeData = candles.map((c) => ({
      x: new Date(c.timestamp),
      y: c.volume,
    }));

    const options = {
      chart: {
        type: "candlestick",
        height: 360,
        fontFamily: "'TT Norms Pro', -apple-system, sans-serif",
        background: "transparent",
        foreColor: "#7a7f87",
        toolbar: {
          show: true,
          tools: {
            download: false,
            selection: true,
            zoom: true,
            zoomin: true,
            zoomout: true,
            pan: true,
            reset: true,
          },
        },
        animations: {
          enabled: true,
          easing: "easeinout",
          speed: 400,
        },
      },
      series: [
        {
          name: "Price",
          type: "candlestick",
          data: ohlcData,
        },
      ],
      plotOptions: {
        candlestick: {
          colors: {
            upward: "#00c896", // POOOL green
            downward: "#ef4444", // Red
          },
          wick: {
            useFillColor: true,
          },
        },
      },
      xaxis: {
        type: "datetime",
        labels: {
          style: {
            colors: "#7a7f87",
            fontSize: "11px",
          },
        },
        axisBorder: { show: false },
        axisTicks: { show: false },
      },
      yaxis: {
        tooltip: { enabled: true },
        labels: {
          style: {
            colors: "#7a7f87",
            fontSize: "11px",
          },
          formatter: (val) => "$" + val.toFixed(2),
        },
      },
      grid: {
        borderColor: "rgba(255,255,255,0.06)",
        strokeDashArray: 3,
      },
      tooltip: {
        theme: "dark",
        custom: function ({ seriesIndex, dataPointIndex, w }) {
          const o = w.globals.seriesCandleO[seriesIndex][dataPointIndex];
          const h = w.globals.seriesCandleH[seriesIndex][dataPointIndex];
          const l = w.globals.seriesCandleL[seriesIndex][dataPointIndex];
          const c = w.globals.seriesCandleC[seriesIndex][dataPointIndex];
          const vol = candles[dataPointIndex]?.volume || 0;

          return `
            <div style="padding:8px 12px;font-size:12px;line-height:1.6;">
              <div style="font-weight:600;margin-bottom:4px;color:#e1e3e6;">
                ${new Date(candles[dataPointIndex]?.timestamp).toLocaleString()}
              </div>
              <div><span style="color:#7a7f87;">O</span> $${o.toFixed(2)}</div>
              <div><span style="color:#7a7f87;">H</span> $${h.toFixed(2)}</div>
              <div><span style="color:#7a7f87;">L</span> $${l.toFixed(2)}</div>
              <div><span style="color:#7a7f87;">C</span> $${c.toFixed(2)}</div>
              <div><span style="color:#7a7f87;">Vol</span> ${vol}</div>
            </div>
          `;
        },
      },
    };

    _chart = new ApexCharts(_container, options);
    _chart.render();
  }

  // ═══════════════════════════════════════════════════════════════
  // ── INTERVAL TOOLBAR ────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function createToolbar(parentEl) {
    const toolbar = document.createElement("div");
    toolbar.className = "chart-toolbar";
    toolbar.style.cssText =
      "display:flex;gap:4px;margin-bottom:12px;flex-wrap:wrap;align-items:center;";

    INTERVALS.forEach((iv) => {
      const btn = document.createElement("button");
      btn.className = `chart-interval-btn${iv === _interval ? " active" : ""}`;
      btn.textContent = iv;
      btn.dataset.interval = iv;
      btn.style.cssText = `
        padding:4px 10px;border-radius:6px;border:1px solid rgba(255,255,255,0.1);
        background:${iv === _interval ? "rgba(0,200,150,0.15)" : "transparent"};
        color:${iv === _interval ? "#00c896" : "#7a7f87"};font-size:12px;
        font-weight:500;cursor:pointer;transition:all 0.2s;font-family:inherit;
      `;

      btn.addEventListener("mouseenter", () => {
        if (!btn.classList.contains("active")) {
          btn.style.background = "rgba(255,255,255,0.05)";
          btn.style.color = "#e1e3e6";
        }
      });
      btn.addEventListener("mouseleave", () => {
        if (!btn.classList.contains("active")) {
          btn.style.background = "transparent";
          btn.style.color = "#7a7f87";
        }
      });

      btn.addEventListener("click", () => {
        setInterval(iv);
        toolbar.querySelectorAll(".chart-interval-btn").forEach((b) => {
          b.classList.remove("active");
          b.style.background = "transparent";
          b.style.color = "#7a7f87";
        });
        btn.classList.add("active");
        btn.style.background = "rgba(0,200,150,0.15)";
        btn.style.color = "#00c896";
      });

      toolbar.appendChild(btn);
    });

    parentEl.insertBefore(toolbar, parentEl.firstChild);
  }

  // ═══════════════════════════════════════════════════════════════
  // ── CHART SUMMARY HEADER ────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function createSummaryHeader(parentEl) {
    const header = document.createElement("div");
    header.id = "chart-summary";
    header.className = "chart-summary";
    header.style.cssText =
      "display:flex;gap:20px;margin-bottom:12px;flex-wrap:wrap;font-size:13px;";

    header.innerHTML = `
      <div>
        <span style="color:#7a7f87;">Last</span>
        <span id="chart-last-price" style="font-weight:600;color:#e1e3e6;margin-left:6px;">—</span>
      </div>
      <div>
        <span style="color:#7a7f87;">24h Change</span>
        <span id="chart-24h-change" style="font-weight:600;margin-left:6px;">—</span>
      </div>
      <div>
        <span style="color:#7a7f87;">24h High</span>
        <span id="chart-24h-high" style="font-weight:500;color:#e1e3e6;margin-left:6px;">—</span>
      </div>
      <div>
        <span style="color:#7a7f87;">24h Low</span>
        <span id="chart-24h-low" style="font-weight:500;color:#e1e3e6;margin-left:6px;">—</span>
      </div>
      <div>
        <span style="color:#7a7f87;">24h Vol</span>
        <span id="chart-24h-vol" style="font-weight:500;color:#e1e3e6;margin-left:6px;">—</span>
      </div>
    `;

    parentEl.insertBefore(header, parentEl.firstChild);
  }

  async function updateSummary() {
    const summary = await fetchChartSummary(_assetId);
    if (!summary) return;

    const lastEl = document.getElementById("chart-last-price");
    const changeEl = document.getElementById("chart-24h-change");
    const highEl = document.getElementById("chart-24h-high");
    const lowEl = document.getElementById("chart-24h-low");
    const volEl = document.getElementById("chart-24h-vol");

    if (lastEl && summary.last_price_cents != null) {
      lastEl.textContent = "$" + (summary.last_price_cents / 100).toFixed(2);
    }

    if (changeEl && summary.change_24h_pct != null) {
      const pct = summary.change_24h_pct;
      const isUp = pct >= 0;
      changeEl.textContent =
        (isUp ? "+" : "") + pct.toFixed(2) + "%";
      changeEl.style.color = isUp ? "#00c896" : "#ef4444";
    }

    if (highEl && summary.high_24h_cents != null) {
      highEl.textContent = "$" + (summary.high_24h_cents / 100).toFixed(2);
    }

    if (lowEl && summary.low_24h_cents != null) {
      lowEl.textContent = "$" + (summary.low_24h_cents / 100).toFixed(2);
    }

    if (volEl && summary.volume_24h != null) {
      volEl.textContent = summary.volume_24h.toLocaleString() + " shares";
    }
  }

  // ═══════════════════════════════════════════════════════════════
  // ── LOAD DATA ───────────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  async function loadCandles() {
    const data = await fetchCandles(_assetId, _interval);

    if (data && data.candles && data.candles.length > 0) {
      renderChart(data.candles);
    } else {
      // Fallback to mock data
      const mockCandles = generateMockCandles(100);
      renderChart(mockCandles);
    }
  }

  function setInterval(interval) {
    _interval = interval;
    loadCandles();
  }

  // ═══════════════════════════════════════════════════════════════
  // ── INIT / DESTROY ──────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function init(containerId, assetId) {
    _assetId = assetId;
    _container = document.getElementById(containerId);
    if (!_container) {
      console.warn("[Chart] Container not found:", containerId);
      return;
    }

    // Clear existing content
    _container.innerHTML = "";

    // Get the parent panel (the section containing the chart)
    const panel = _container.closest(".tv3-chart-panel") || _container.parentElement;

    // Add summary header and interval toolbar (before the chart container)
    if (panel && !document.getElementById("chart-summary")) {
      createSummaryHeader(panel);
      createToolbar(panel);
    }

    // Load initial data
    loadCandles();
    updateSummary();

    // Subscribe to live trade events for real-time candle updates
    if (window.MarketBus) {
      window.MarketBus.on("trade", () => {
        // Debounce: reload candles 1s after last trade
        clearTimeout(_reloadTimer);
        _reloadTimer = setTimeout(() => {
          loadCandles();
          updateSummary();
        }, 1000);
      });
    }
  }

  let _reloadTimer = null;

  function destroy() {
    if (_chart) {
      _chart.destroy();
      _chart = null;
    }
    clearTimeout(_reloadTimer);
  }

  return { init, destroy, setInterval: setInterval };
})();

window.MarketChart = MarketChart;
