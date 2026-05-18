/**
 * poool-line-chart.js
 *
 * Shared ECharts smooth-area-line renderer — POOOL brand style.
 * Mirrors charts-showcase.html #14 (electric-blue → mint-green gradient line +
 * fading area, no axis tick noise, dark tooltip, smooth curve).
 *
 * Usage:
 *   PooolLineChart.render(container, {
 *     labels:  ["Jan", "Feb", ...],          // x-axis categories
 *     values:  [120, 180, 160, ...],         // numeric data
 *     formatter: (v) => "$" + v.toFixed(0),  // optional tooltip + y-label formatter
 *     yMax: 500,                             // optional explicit y-axis max
 *     height: 240,                           // optional override (default 240)
 *     showAxis: true,                        // optional — render axis labels (default true)
 *   });
 *
 * Returns the ECharts instance so callers can dispose / resize.
 *
 * Requires: window.echarts (loaded via <script src="…echarts.min.js">).
 */
(function () {
  "use strict";

  if (typeof window === "undefined") return;

  const BRAND = {
    lineFrom: "#0000FF",   // electric blue
    lineTo:   "#03FF88",   // mint
    areaFrom: "rgba(0, 0, 255, 0.25)",
    areaTo:   "rgba(3, 255, 136, 0)",
    text:     "#717680",
    grid:     "#F2F4F7",
    endpoint: "#03FF88",
  };

  function hasEcharts() {
    return typeof window.echarts !== "undefined";
  }

  function buildOption(opts) {
    const formatter = typeof opts.formatter === "function"
      ? opts.formatter
      : (v) => v.toLocaleString("en-US");

    return {
      backgroundColor: "transparent",
      textStyle: {
        color: BRAND.text,
        fontFamily: "'TT Norms Pro', sans-serif",
      },
      tooltip: {
        trigger: "axis",
        backgroundColor: "rgba(24, 29, 39, 0.95)",
        borderWidth: 0,
        padding: [8, 12],
        textStyle: { color: "#fff", fontSize: 12 },
        formatter: (params) => {
          const p = Array.isArray(params) ? params[0] : params;
          return `<div style="font-weight:600;margin-bottom:2px;">${p.name}</div>${formatter(p.value)}`;
        },
        axisPointer: {
          type: "line",
          lineStyle: { color: BRAND.lineFrom, opacity: 0.4, width: 1 },
        },
      },
      grid: {
        left: opts.showAxis === false ? 0 : 50,
        right: 12,
        top: 12,
        bottom: opts.showAxis === false ? 0 : 28,
        containLabel: false,
      },
      xAxis: {
        type: "category",
        data: opts.labels || [],
        boundaryGap: false,
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: {
          show: opts.showAxis !== false,
          color: BRAND.text,
          fontSize: 11,
          margin: 12,
        },
      },
      yAxis: {
        type: "value",
        max: opts.yMax,
        axisLine: { show: false },
        axisTick: { show: false },
        splitLine: { lineStyle: { color: BRAND.grid, type: "solid" } },
        axisLabel: {
          show: opts.showAxis !== false,
          color: BRAND.text,
          fontSize: 11,
          formatter: (v) => formatter(v),
        },
      },
      series: buildSeries(opts),
      animation: true,
      animationDuration: 1200,
      animationEasing: "cubicOut",
    };
  }

  // Build one or many series. `opts.series` (array) takes priority for
  // multi-line charts; fall back to single `opts.values`. Each series item
  // may set: { name, values, dashed, color, area, step, markEnd }.
  function buildSeries(opts) {
    if (Array.isArray(opts.series) && opts.series.length) {
      return opts.series.map((s, i) => buildOneSeries({
        name:     s.name,
        values:   s.values,
        dashed:   !!s.dashed,
        color:    s.color,
        area:     s.area !== false,
        step:     s.step,
        markEnd:  s.markEnd !== false && i === 0,
        opacity:  s.opacity,
      }));
    }
    return [buildOneSeries({
      values:  opts.values || [],
      area:    opts.area !== false,
      step:    opts.step,
      markEnd: opts.endpoint !== false,
    })];
  }

  function buildOneSeries(s) {
    const base = {
      name: s.name,
      type: "line",
      smooth: !s.step,
      step: s.step ? (typeof s.step === "string" ? s.step : "end") : false,
      symbol: "circle",
      symbolSize: 0,
      showSymbol: false,
      emphasis: { focus: "series" },
      data: s.values || [],
    };

    if (s.markEnd) {
      base.markPoint = {
        symbol: "circle",
        symbolSize: 10,
        data: [{ type: "max", name: "last" }],
        itemStyle: {
          color: "#fff",
          borderColor: BRAND.endpoint,
          borderWidth: 2.5,
          shadowColor: "rgba(3, 255, 136, 0.3)",
          shadowBlur: 8,
        },
        label: { show: false },
      };
    }

    const strokeColor = s.color || {
      type: "linear", x: 0, y: 0, x2: 1, y2: 0,
      colorStops: [
        { offset: 0, color: BRAND.lineFrom },
        { offset: 1, color: BRAND.lineTo },
      ],
    };

    base.lineStyle = {
      width: s.dashed ? 1.5 : 2.5,
      color: strokeColor,
      type: s.dashed ? [4, 3] : "solid",
      opacity: s.opacity != null ? s.opacity : 1,
    };

    if (s.area && !s.dashed) {
      base.areaStyle = {
        color: typeof strokeColor === "string"
          ? strokeColor
          : {
              type: "linear", x: 0, y: 0, x2: 0, y2: 1,
              colorStops: [
                { offset: 0, color: BRAND.areaFrom },
                { offset: 1, color: BRAND.areaTo },
              ],
            },
        opacity: typeof strokeColor === "string" ? 0.18 : 1,
      };
    }

    return base;
  }

  function render(container, opts) {
    if (!container) return null;
    if (typeof container === "string") {
      container = document.getElementById(container);
      if (!container) return null;
    }
    if (!hasEcharts()) {
      console.warn("[PooolLineChart] echarts not loaded — render skipped");
      return null;
    }

    // Resize container if explicit height given
    if (opts.height) container.style.height = opts.height + "px";
    if (!container.style.height) container.style.height = "240px";

    // Reuse existing instance if present (avoids leaks on re-render)
    let chart = echarts.getInstanceByDom(container);
    if (chart) chart.dispose();

    chart = echarts.init(container, null, { renderer: "svg" });
    chart.setOption(buildOption(opts));

    // Auto-resize on window resize (debounced)
    let t = null;
    const onResize = () => {
      clearTimeout(t);
      t = setTimeout(() => chart.resize(), 120);
    };
    window.addEventListener("resize", onResize);
    chart.__pooolResize = onResize;

    return chart;
  }

  function dispose(chart) {
    if (!chart) return;
    if (chart.__pooolResize) window.removeEventListener("resize", chart.__pooolResize);
    try { chart.dispose(); } catch (_) { /* noop */ }
  }

  window.PooolLineChart = { render, dispose, BRAND };
})();
