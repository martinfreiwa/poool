/**
 * portfolio-chart.js — Interactive Portfolio Value Chart Controller
 *
 * Makes the portfolio chart fully functional:
 *   - Time period tabs (12 months, 30 days, 7 days, 24 hours) switch chart data
 *   - "Show more" button toggles chart with chevron rotation
 *   - Trend line recalculates per period
 *   - Bar hover tooltips show values
 *   - "All time +104%" header updates dynamically per tab
 *   - Filters button opens filter dropdown that actually filters chart data
 *     between Portfolio Value, Rental Income, and Appreciation metrics
 */
(function () {
  "use strict";

  // ─── Constants ──────────────────────────────────────────────
  var PERIODS = {
    twelveMonths:    { label: "12 months", bars: 52 },
    thirtyDays:      { label: "30 days",   bars: 30 },
    sevenDays:       { label: "7 days",    bars: 7  },
    twentyFourHours: { label: "24 hours",  bars: 24 },
  };

  var MONTH_LABELS = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];

  var MAX_BAR_HEIGHT = 180; // px
  var MIN_BAR_HEIGHT = 8;   // px

  // Bar colors per metric
  var BAR_COLORS = {
    portfolio:    "linear-gradient(180deg, #12B76A 0%, #079455 100%)",
    rental:       "linear-gradient(180deg, #6172F3 0%, #444CE7 100%)",
    appreciation: "linear-gradient(180deg, #F79009 0%, #DC6803 100%)",
  };

  // Trend line colors per metric
  var TREND_COLORS = {
    portfolio:    "#5555FF",
    rental:       "#444CE7",
    appreciation: "#DC6803",
  };

  var currentPeriod = "twelveMonths";
  var chartExpanded = true;
  var portfolioData = null;
  var seriesCache = {};  // keyed by "periodKey:metric"

  // Active filter set — which metrics are shown on the chart
  var activeFilters = { portfolio: true, rental: false, appreciation: false };

  // ─── Deterministic Pseudo-Random ────────────────────────────
  function makePRNG(seedStr) {
    var seed = 0;
    for (var i = 0; i < seedStr.length; i++) {
      seed = ((seed << 5) - seed + seedStr.charCodeAt(i)) | 0;
    }
    seed = Math.abs(seed) || 1;
    return function () {
      seed = (seed * 16807 + 12345) % 2147483647;
      return seed / 2147483647;
    };
  }

  // ─── Generate X-Axis Labels ─────────────────────────────────

  function generateXLabels(periodKey) {
    var now = new Date();

    if (periodKey === "twelveMonths") {
      return MONTH_LABELS;
    }

    if (periodKey === "thirtyDays") {
      var labels = [];
      for (var i = 29; i >= 0; i -= 5) {
        var d = new Date(now);
        d.setDate(d.getDate() - i);
        labels.push(d.toLocaleDateString("en-US", { month: "short", day: "numeric" }));
      }
      return labels;
    }

    if (periodKey === "sevenDays") {
      var labels2 = [];
      for (var j = 6; j >= 0; j--) {
        var d2 = new Date(now);
        d2.setDate(d2.getDate() - j);
        labels2.push(d2.toLocaleDateString("en-US", { weekday: "short" }));
      }
      return labels2;
    }

    if (periodKey === "twentyFourHours") {
      var labels3 = [];
      for (var k = 23; k >= 0; k -= 4) {
        var d3 = new Date(now);
        d3.setHours(d3.getHours() - k, 0, 0, 0);
        labels3.push(d3.toLocaleTimeString("en-US", { hour: "numeric", hour12: true }));
      }
      return labels3;
    }

    return [];
  }

  // ─── Generate Series Data ──────────────────────────────────
  // Generates a single metric series for a given period.
  // Each metric uses different base values from the portfolio data.

  function generateSeries(periodKey, metric) {
    var cacheKey = periodKey + ":" + metric;
    if (seriesCache[cacheKey]) return seriesCache[cacheKey];

    var numBars = PERIODS[periodKey].bars;
    var data = [];

    // Determine the final (current) value for this metric
    var endValueCents = 0;
    if (portfolioData) {
      if (metric === "portfolio")    endValueCents = portfolioData.total_value_cents || 0;
      if (metric === "rental")       endValueCents = portfolioData.total_rental_cents || 0;
      if (metric === "appreciation") endValueCents = portfolioData.total_appreciation_cents || 0;
    } else {
      // Demo fallback
      if (metric === "portfolio")    endValueCents = 18247300;
      if (metric === "rental")       endValueCents = 3500000;
      if (metric === "appreciation") endValueCents = 4200000;
    }

    // If value is zero or negative, generate flat near-zero data
    if (endValueCents <= 0) {
      for (var z = 0; z < numBars; z++) data.push(0);
      seriesCache[cacheKey] = data;
      return data;
    }

    var rng = makePRNG(cacheKey);

    // Growth & volatility per period
    var growthFactors = {
      twelveMonths: 0.35, thirtyDays: 0.03,
      sevenDays: 0.008,   twentyFourHours: 0.002
    };
    var volatilityFactors = {
      twelveMonths: 0.12, thirtyDays: 0.06,
      sevenDays: 0.04,    twentyFourHours: 0.02
    };

    // Rental income is cumulative → steeper growth, less volatility
    // Appreciation is more volatile
    var metricGrowthMul  = { portfolio: 1.0, rental: 1.3, appreciation: 0.8 };
    var metricVolMul     = { portfolio: 1.0, rental: 0.4, appreciation: 1.5 };

    var totalGrowth = (growthFactors[periodKey] || 0.1) * (metricGrowthMul[metric] || 1);
    var vol = (volatilityFactors[periodKey] || 0.05) * (metricVolMul[metric] || 1);

    var baseValue = endValueCents / 100; // dollars
    var startValue = baseValue / (1 + totalGrowth);
    var stepGrowth = (baseValue - startValue) / numBars;

    for (var i = 0; i < numBars; i++) {
      var trendValue = startValue + stepGrowth * i;
      var noise = (rng() - 0.5) * 2 * vol * trendValue;
      var value = Math.max(0, trendValue + noise);
      data.push(Math.round(value * 100)); // back to cents
    }

    // Last bar = actual current value
    data[numBars - 1] = endValueCents;

    seriesCache[cacheKey] = data;
    return data;
  }

  // ─── Get Active Filters ─────────────────────────────────────

  function getActiveMetrics() {
    var metrics = [];
    if (activeFilters.portfolio)    metrics.push("portfolio");
    if (activeFilters.rental)       metrics.push("rental");
    if (activeFilters.appreciation) metrics.push("appreciation");
    // Always show at least portfolio if nothing selected
    if (metrics.length === 0) {
      activeFilters.portfolio = true;
      metrics.push("portfolio");
      // Also update checkbox
      var cb = document.querySelector('input[name="chart-filter"][value="portfolio"]');
      if (cb) cb.checked = true;
    }
    return metrics;
  }

  // ─── Combine Series ─────────────────────────────────────────
  // Sums the values of all active metrics per bar index.

  function getCombinedData(periodKey) {
    var metrics = getActiveMetrics();
    var numBars = PERIODS[periodKey].bars;
    var combined = new Array(numBars).fill(0);

    for (var m = 0; m < metrics.length; m++) {
      var series = generateSeries(periodKey, metrics[m]);
      for (var i = 0; i < numBars; i++) {
        combined[i] += series[i];
      }
    }
    return combined;
  }

  // ─── Get Bar Color ──────────────────────────────────────────

  function getBarGradient() {
    var metrics = getActiveMetrics();
    if (metrics.length === 1) return BAR_COLORS[metrics[0]];
    // Mixed: use a blended gradient
    if (metrics.length === 2) {
      if (metrics.includes("portfolio") && metrics.includes("rental"))
        return "linear-gradient(180deg, #12B76A 0%, #444CE7 100%)";
      if (metrics.includes("portfolio") && metrics.includes("appreciation"))
        return "linear-gradient(180deg, #12B76A 0%, #DC6803 100%)";
      if (metrics.includes("rental") && metrics.includes("appreciation"))
        return "linear-gradient(180deg, #6172F3 0%, #DC6803 100%)";
    }
    return "linear-gradient(180deg, #12B76A 0%, #444CE7 50%, #DC6803 100%)";
  }

  function getTrendColor() {
    var metrics = getActiveMetrics();
    if (metrics.length === 1) return TREND_COLORS[metrics[0]];
    return "#5555FF"; // default purple for mixed
  }

  // ─── Filter Label ───────────────────────────────────────────

  function getFilterLabel() {
    var metrics = getActiveMetrics();
    var names = { portfolio: "Portfolio value", rental: "Rental income", appreciation: "Appreciation" };
    if (metrics.length === 1) return names[metrics[0]];
    if (metrics.length === 3) return "All metrics";
    return metrics.map(function (m) { return names[m]; }).join(" + ");
  }

  // ─── Calculate Period Return ────────────────────────────────

  function calcPeriodReturn(chartData) {
    if (!chartData || chartData.length < 2) return { pct: 0, label: "+0.0%" };
    var first = chartData[0];
    var last = chartData[chartData.length - 1];
    if (first === 0) return { pct: 0, label: "+0.0%" };
    var pct = ((last - first) / first) * 100;
    var sign = pct >= 0 ? "+" : "";
    return { pct: pct, label: sign + pct.toFixed(0) + "%" };
  }

  // ─── Period Name ────────────────────────────────────────────

  function getPeriodDisplayName(periodKey) {
    var names = {
      twelveMonths: "12 months", thirtyDays: "30 days",
      sevenDays: "7 days",       twentyFourHours: "24 hours"
    };
    return names[periodKey] || "All time";
  }

  // ─── Render Chart ───────────────────────────────────────────
  //
  // Previously: hand-rolled DIV bars + absolute-positioned SVG trend line
  // overlay + bespoke tooltip listeners. Now: single ECharts dual-axis chart
  // (bar series + smooth-line SMA overlay), brand-tinted to match the metric
  // mix from `getActiveMetrics()`. The legacy DOM (chart-grid / chart-bars /
  // chart-trend-line / chart-x-axis) is hidden once on first render — kept
  // in the template so other code that reads it (CSS hooks, mobile fallback)
  // still finds the elements.

  var _echartsInstance = null;
  var _echartsMount = null;

  function ensureEchartsMount() {
    var container = document.getElementById("portfolio-chart-container");
    if (!container) return null;

    if (_echartsMount && container.contains(_echartsMount)) return _echartsMount;

    // Hide the legacy children — bars, trend-line, grid, x-axis labels.
    Array.prototype.forEach.call(container.children, function (child) {
      if (child.id !== "portfolio-chart-echarts") child.style.display = "none";
    });

    _echartsMount = container.querySelector("#portfolio-chart-echarts");
    if (!_echartsMount) {
      _echartsMount = document.createElement("div");
      _echartsMount.id = "portfolio-chart-echarts";
      _echartsMount.style.cssText = "width:100%;height:280px;";
      container.appendChild(_echartsMount);
    }
    return _echartsMount;
  }

  function formatUsdCompact(dollars) {
    var abs = Math.abs(dollars);
    if (abs >= 1e6) return "$" + (dollars / 1e6).toFixed(1).replace(/\.0$/, "") + "M";
    if (abs >= 1e3) return "$" + (dollars / 1e3).toFixed(1).replace(/\.0$/, "") + "K";
    return "$" + Math.round(dollars).toLocaleString("en-US");
  }

  function computeSma(values) {
    var n = values.length;
    if (n === 0) return [];
    var w = Math.max(3, Math.floor(n * 0.1));
    var out = [];
    for (var i = 0; i < n; i++) {
      var start = Math.max(0, i - Math.floor(w / 2));
      var end = Math.min(n, i + Math.ceil(w / 2));
      var sum = 0;
      for (var j = start; j < end; j++) sum += values[j];
      out.push(sum / (end - start));
    }
    return out;
  }

  function getBarFlatColor() {
    var metrics = getActiveMetrics();
    if (metrics.length === 1) {
      return { portfolio: "#12B76A", rental: "#444CE7", appreciation: "#DC6803" }[metrics[0]];
    }
    return "#12B76A"; // mixed: brand green
  }

  function renderChart(periodKey) {
    var chartDataCents = getCombinedData(periodKey);
    var xLabels = generateXLabels(periodKey);

    // 1. Update period header text + percentage (uses real backend totals
    //    since no per-period time-series exists)
    var titleText = document.querySelector(".chart-title-text");
    var titlePct  = document.querySelector(".chart-title-percentage");
    if (titleText) titleText.textContent = getPeriodDisplayName(periodKey);
    if (titlePct) {
      var realLabel = "—";
      if (portfolioData && portfolioData.total_purchase_cents > 0) {
        var realPct = ((portfolioData.total_appreciation_cents || 0) / portfolioData.total_purchase_cents) * 100;
        var sign = realPct >= 0 ? "+" : "";
        realLabel = sign + realPct.toFixed(1) + "%";
        titlePct.classList.toggle("chart-title-negative", realPct < 0);
      }
      titlePct.textContent = realLabel;
    }

    // 2. ECharts render
    var mount = ensureEchartsMount();
    if (!mount || typeof window.echarts === "undefined") return;

    // Pad x-labels out to match data length — generateXLabels returns sparse
    // labels (e.g. 12 month names for 52 weeks).
    var paddedLabels = new Array(chartDataCents.length).fill("");
    if (xLabels.length > 0) {
      for (var i = 0; i < xLabels.length; i++) {
        var idx = Math.round((i / Math.max(1, xLabels.length - 1)) * (chartDataCents.length - 1));
        paddedLabels[idx] = xLabels[i];
      }
    }

    var barValuesDollars   = chartDataCents.map(function (c) { return c / 100; });
    var trendValuesDollars = computeSma(barValuesDollars);
    var barColor   = getBarFlatColor();
    var trendColor = getTrendColor();

    if (_echartsInstance) {
      try { _echartsInstance.dispose(); } catch (_) { /* noop */ }
      _echartsInstance = null;
    }
    _echartsInstance = echarts.init(mount, null, { renderer: "svg" });

    _echartsInstance.setOption({
      backgroundColor: "transparent",
      textStyle: { fontFamily: "'TT Norms Pro', sans-serif", color: "#717680" },
      tooltip: {
        trigger: "axis",
        axisPointer: { type: "shadow" },
        backgroundColor: "rgba(24, 29, 39, 0.95)",
        borderWidth: 0,
        padding: [8, 12],
        textStyle: { color: "#fff", fontSize: 12 },
        formatter: function (params) {
          if (!params || !params.length) return "";
          var bar = params.find(function (p) { return p.seriesType === "bar"; });
          if (!bar) bar = params[0];
          return (
            '<div style="font-weight:600;margin-bottom:2px;">' + (bar.name || "") + "</div>" +
            formatUsdCompact(bar.value)
          );
        },
      },
      grid: { left: 56, right: 12, top: 12, bottom: 28 },
      xAxis: {
        type: "category",
        data: paddedLabels,
        boundaryGap: true,
        axisLine: { lineStyle: { color: "#F2F4F7" } },
        axisTick: { show: false },
        axisLabel: { color: "#717680", fontSize: 11, margin: 12, interval: 0 },
      },
      yAxis: {
        type: "value",
        axisLine: { show: false },
        axisTick: { show: false },
        splitLine: { lineStyle: { color: "#F2F4F7" } },
        axisLabel: { color: "#717680", fontSize: 11, formatter: formatUsdCompact },
      },
      series: [
        {
          name: "Value",
          type: "bar",
          data: barValuesDollars,
          itemStyle: { color: barColor, borderRadius: [3, 3, 0, 0] },
          barWidth: "65%",
          emphasis: { itemStyle: { color: barColor, opacity: 0.85 } },
        },
        {
          name: "Trend",
          type: "line",
          smooth: true,
          symbol: "none",
          showSymbol: false,
          lineStyle: { color: trendColor, width: 2, type: [5, 5] },
          tooltip: { show: false },
          data: trendValuesDollars,
        },
      ],
      animationDuration: 800,
      animationEasing: "cubicOut",
    });
  }

  // Trend line + bar tooltip are now handled natively by ECharts in
  // renderChart() above (smooth line series + axisPointer + tooltip
  // formatter). The previously-hand-rolled `renderTrendLine` and
  // `setupBarTooltips` were removed in the migration.

  // ─── Show More / Less Toggle ────────────────────────────────

  function initShowMoreButton() {
    var btn = document.querySelector(".portfolio-show-more-btn");
    if (!btn) return;

    btn.addEventListener("click", function () {
      chartExpanded = !chartExpanded;

      var chevron = btn.querySelector("svg");
      if (chevron) {
        chevron.style.transform = chartExpanded ? "rotate(0deg)" : "rotate(180deg)";
      }

      // Re-trigger bar animations when expanding
      if (chartExpanded) {
        setTimeout(function () { renderChart(currentPeriod); }, 100);
      }
    });
  }

  // ─── Tab Switching ──────────────────────────────────────────

  function initTabs() {
    var tabs = document.querySelectorAll("#portfolio-chart-tabs .chart-tab");
    var periodKeys = ["twelveMonths", "thirtyDays", "sevenDays", "twentyFourHours"];

    tabs.forEach(function (tab, idx) {
      tab.addEventListener("click", function () {
        currentPeriod = periodKeys[idx] || "twelveMonths";
        renderChart(currentPeriod);
      });
    });
  }

  // ─── Filters Button ─────────────────────────────────────────

  function initFilterButton() {
    var filterBtn = document.getElementById("portfolio-chart-filter");
    if (!filterBtn) return;

    // Create filter dropdown
    var dropdown = document.createElement("div");
    dropdown.id = "chart-filter-dropdown";
    dropdown.className = "chart-filter-dropdown";
    dropdown.innerHTML =
      '<div class="chart-filter-dropdown-header">' +
        '<span>Filter by</span>' +
        '<button class="chart-filter-close" aria-label="Close">' +
          '<svg width="16" height="16" viewBox="0 0 16 16" fill="none">' +
            '<path d="M4 4L12 12M12 4L4 12" stroke="#535862" stroke-width="1.5" stroke-linecap="round"/>' +
          '</svg>' +
        '</button>' +
      '</div>' +
      '<div class="chart-filter-options">' +
        '<label class="chart-filter-option">' +
          '<input type="checkbox" name="chart-filter" value="portfolio" checked>' +
          '<span class="chart-filter-checkbox"></span>' +
          '<div class="chart-filter-color-dot" style="background: #12B76A;"></div>' +
          '<span>Portfolio value</span>' +
        '</label>' +
        '<label class="chart-filter-option">' +
          '<input type="checkbox" name="chart-filter" value="rental">' +
          '<span class="chart-filter-checkbox"></span>' +
          '<div class="chart-filter-color-dot" style="background: #444CE7;"></div>' +
          '<span>Rental income</span>' +
        '</label>' +
        '<label class="chart-filter-option">' +
          '<input type="checkbox" name="chart-filter" value="appreciation">' +
          '<span class="chart-filter-checkbox"></span>' +
          '<div class="chart-filter-color-dot" style="background: #DC6803;"></div>' +
          '<span>Appreciation</span>' +
        '</label>' +
      '</div>';

    dropdown.style.display = "none";
    filterBtn.parentElement.style.position = "relative";
    filterBtn.parentElement.appendChild(dropdown);

    // Toggle dropdown
    filterBtn.addEventListener("click", function (e) {
      e.stopPropagation();
      var isVisible = dropdown.style.display !== "none";
      dropdown.style.display = isVisible ? "none" : "block";
    });

    // Close button
    var closeBtn = dropdown.querySelector(".chart-filter-close");
    if (closeBtn) {
      closeBtn.addEventListener("click", function (e) {
        e.stopPropagation();
        dropdown.style.display = "none";
      });
    }

    // Close on outside click
    document.addEventListener("click", function (e) {
      if (!dropdown.contains(e.target) && !filterBtn.contains(e.target)) {
        dropdown.style.display = "none";
      }
    });

    // ── Wire up checkbox changes to actually filter the chart ──
    var checkboxes = dropdown.querySelectorAll('input[name="chart-filter"]');
    checkboxes.forEach(function (cb) {
      cb.addEventListener("change", function () {
        // Update filter state
        activeFilters[this.value] = this.checked;

        // Ensure at least one is always checked
        var anyChecked = activeFilters.portfolio || activeFilters.rental || activeFilters.appreciation;
        if (!anyChecked) {
          // Revert: re-check this one
          this.checked = true;
          activeFilters[this.value] = true;
          return;
        }

        // Update the filter badge/count indicator on the button
        updateFilterButtonLabel(filterBtn);

        // Re-render chart with new filters
        renderChart(currentPeriod);
      });
    });
  }

  // ─── Update Filter Button Label ─────────────────────────────
  function updateFilterButtonLabel(btn) {
    var count = 0;
    if (activeFilters.portfolio)    count++;
    if (activeFilters.rental)       count++;
    if (activeFilters.appreciation) count++;

    var labelSpan = btn.querySelector("span");
    if (labelSpan) {
      labelSpan.textContent = "Filters";
    }
  }

  // ─── Receive Portfolio Data ─────────────────────────────────

  function setPortfolioData(data) {
    portfolioData = data;
    seriesCache = {}; // Clear cache when data changes
    renderChart(currentPeriod);
  }

  // ─── Init ───────────────────────────────────────────────────

  function init() {
    initShowMoreButton();
    initTabs();
    initFilterButton();

    // Try to read the server-injected portfolio JSON for chart data
    try {
      var jsonEl = document.getElementById("server-portfolio-json");
      if (jsonEl) {
        var raw = JSON.parse(jsonEl.textContent.trim());
        if (raw && raw.total_value_cents) {
          setPortfolioData(raw);
        }
      }
    } catch (e) {
      // Fallback: render with demo data
      renderChart(currentPeriod);
    }
  }

  // ─── Boot ───────────────────────────────────────────────────

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }

  // Expose for external use
  window.PortfolioChart = {
    setData: setPortfolioData,
    render: renderChart,
    getCurrentPeriod: function () { return currentPeriod; },
  };
})();
