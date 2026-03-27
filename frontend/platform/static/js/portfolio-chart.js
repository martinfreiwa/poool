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

  function renderChart(periodKey) {
    var chartData = getCombinedData(periodKey);
    var periodReturn = calcPeriodReturn(chartData);
    var xLabels = generateXLabels(periodKey);
    var barGradient = getBarGradient();
    var trendColor = getTrendColor();

    // 1. Update period header text
    var titleText = document.querySelector(".chart-title-text");
    var titlePct  = document.querySelector(".chart-title-percentage");
    if (titleText) titleText.textContent = getPeriodDisplayName(periodKey);
    if (titlePct) {
      titlePct.textContent = periodReturn.label;
      if (periodReturn.pct < 0) {
        titlePct.classList.add("chart-title-negative");
      } else {
        titlePct.classList.remove("chart-title-negative");
      }
    }

    // 2. Render bars
    var barsContainer = document.querySelector(".chart-bars");
    if (!barsContainer) return;

    var maxVal = Math.max.apply(null, chartData);
    var minVal = Math.min.apply(null, chartData);
    var range = maxVal - minVal || 1;

    var barsHtml = "";
    for (var idx = 0; idx < chartData.length; idx++) {
      var value = chartData[idx];
      var normalizedHeight = ((value - minVal) / range) * (MAX_BAR_HEIGHT - MIN_BAR_HEIGHT) + MIN_BAR_HEIGHT;
      var height = Math.round(normalizedHeight);
      var delay = Math.round((idx / chartData.length) * 800);
      var displayValue = "$" + new Intl.NumberFormat("en-US").format(Math.round(value / 100));

      barsHtml += '<div class="chart-bar-week" '
        + 'data-week="' + (idx + 1) + '" '
        + 'data-height="' + height + '" '
        + 'data-value="' + displayValue + '" '
        + 'style="animation-delay: ' + delay + 'ms; height: ' + height + 'px; background: ' + barGradient + ';" '
        + 'title="' + displayValue + '"></div>';
    }

    barsContainer.innerHTML = barsHtml;

    // Re-trigger animation
    barsContainer.style.display = "none";
    void barsContainer.offsetHeight; // force reflow
    barsContainer.style.display = "";

    // 3. Render trend line
    renderTrendLine(chartData, trendColor);

    // 4. Update X-Axis labels
    var xAxisContainer = document.querySelector(".chart-x-axis");
    if (xAxisContainer) {
      xAxisContainer.innerHTML = xLabels.map(function (label) { return "<span>" + label + "</span>"; }).join("");
    }

    // 4.5. Update Y-Axis labels
    var gridLines = document.querySelectorAll(".grid-line");
    if (gridLines.length > 0) {
      var numLines = gridLines.length;
      for (var yIdx = 0; yIdx < numLines; yIdx++) {
        var pct = (numLines - 1 - yIdx) / (numLines - 1);
        var valCents = minVal + (range * pct);
        var valDollars = valCents / 100;
        
        var formatted = "";
        if (Math.abs(valDollars) >= 1000000) {
          formatted = "$" + (valDollars / 1000000).toFixed(1).replace(/\.0$/, "") + "M";
        } else if (Math.abs(valDollars) >= 1000) {
          formatted = "$" + (valDollars / 1000).toFixed(1).replace(/\.0$/, "") + "K";
        } else {
          formatted = "$" + Math.round(valDollars);
        }
        
        var labelEl = gridLines[yIdx].querySelector(".chart-y-axis-label");
        if (!labelEl) {
          labelEl = document.createElement("span");
          labelEl.className = "chart-y-axis-label";
          gridLines[yIdx].insertBefore(labelEl, gridLines[yIdx].firstChild);
        }
        labelEl.textContent = formatted;
      }
    }

    // 5. Add bar hover tooltip behavior
    setupBarTooltips();
  }

  // ─── Render Trend Line ──────────────────────────────────────

  function renderTrendLine(chartData, color) {
    var trendSvg = document.querySelector(".chart-trend-line");
    if (!trendSvg) return;

    var viewBoxWidth = 1048;
    var viewBoxHeight = 213;
    var padding = 20;
    var numPoints = chartData.length;

    var maxVal = Math.max.apply(null, chartData);
    var minVal = Math.min.apply(null, chartData);
    var range = maxVal - minVal || 1;

    // Simple moving average (window = ~10% of data)
    var w = Math.max(3, Math.floor(numPoints * 0.1));
    var smaData = [];
    for (var i = 0; i < numPoints; i++) {
      var start = Math.max(0, i - Math.floor(w / 2));
      var end = Math.min(numPoints, i + Math.ceil(w / 2));
      var sum = 0;
      for (var j = start; j < end; j++) sum += chartData[j];
      smaData.push(sum / (end - start));
    }

    // Build SVG path
    var pathParts = [];
    for (var k = 0; k < smaData.length; k++) {
      var x = padding + (k / (numPoints - 1)) * (viewBoxWidth - 2 * padding);
      var y = viewBoxHeight - padding - ((smaData[k] - minVal) / range) * (viewBoxHeight - 2 * padding);
      pathParts.push((k === 0 ? "M " : "L ") + x.toFixed(0) + " " + y.toFixed(0));
    }

    var pathEl = trendSvg.querySelector("path");
    if (pathEl) {
      pathEl.setAttribute("d", pathParts.join(" "));
      pathEl.setAttribute("stroke", color || "#5555FF");
    }
  }

  // ─── Bar Tooltips ───────────────────────────────────────────

  function setupBarTooltips() {
    var existingTooltip = document.getElementById("chart-bar-tooltip");
    if (existingTooltip) existingTooltip.remove();

    var tooltip = document.createElement("div");
    tooltip.id = "chart-bar-tooltip";
    tooltip.className = "chart-bar-tooltip";
    tooltip.style.display = "none";
    var container = document.querySelector(".portfolio-chart-container");
    if (container) container.appendChild(tooltip);

    var bars = document.querySelectorAll(".chart-bar-week");
    bars.forEach(function (bar) {
      bar.addEventListener("mouseenter", function () {
        var val = this.getAttribute("data-value");
        tooltip.textContent = val;
        tooltip.style.display = "block";

        var rect = this.getBoundingClientRect();
        var containerRect = this.closest(".portfolio-chart-container").getBoundingClientRect();
        tooltip.style.left = (rect.left - containerRect.left + rect.width / 2) + "px";
        tooltip.style.top = (rect.top - containerRect.top - 30) + "px";
      });

      bar.addEventListener("mouseleave", function () {
        tooltip.style.display = "none";
      });
    });
  }

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
