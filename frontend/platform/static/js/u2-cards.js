/**
 * u2-cards.js
 *
 * Upgrades legacy `.metric-card` stat tiles on /developer/dashboard
 * to the unified `.u2` split layout: brand icon chip + KPI on LEFT,
 * full-height sparkline on RIGHT.
 *
 * Pure DOM injection — no template changes, no backend restart.
 * Idempotent (each card only upgraded once via `u2-applied` marker).
 * Falls back gracefully if dashboard structure unexpected.
 */
(function () {
  "use strict";
  if (typeof window === "undefined") return;

  // ─── Icon library (matches showcase palette) ────────────────
  const ICONS = {
    wallet:   '<path d="M20 12V8H6a2 2 0 0 1-2-2c0-1.1.9-2 2-2h12v4"/><path d="M4 6v12c0 1.1.9 2 2 2h14v-4"/><path d="M18 12a2 2 0 0 0 0 4h4v-4z"/>',
    building: '<path d="M3 21h18"/><path d="M5 21V7l8-4v18"/><path d="M19 21V11l-6-4"/>',
    trending: '<path d="M3 17l6-6 4 4 8-8"/><path d="M14 7h7v7"/>',
    bookmark: '<path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"/>',
    target:   '<circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2"/>',
    users:    '<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/>',
    chart:    '<line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/>',
  };

  // ─── Map metric label → semantic icon ───────────────────────
  function iconFor(label) {
    const l = (label || "").toLowerCase();
    if (l.includes("raised") || l.includes("revenue") || l.includes("sales")) return ICONS.trending;
    if (l.includes("remaining") || l.includes("wallet") || l.includes("balance")) return ICONS.wallet;
    if (l.includes("target") || l.includes("goal")) return ICONS.target;
    if (l.includes("asset") || l.includes("propert")) return ICONS.building;
    if (l.includes("investor") || l.includes("user") || l.includes("member")) return ICONS.users;
    if (l.includes("saved") || l.includes("favori")) return ICONS.bookmark;
    return ICONS.chart;
  }

  // ─── Generate sparkline path based on trend direction ───────
  // viewBox 120×100. Returns { area, line, endX, endY }.
  function sparkPath(trend) {
    let pts;
    if (trend === "up") {
      pts = [[0,80],[20,72],[40,60],[60,55],[80,40],[100,28],[120,15]];
    } else if (trend === "down") {
      pts = [[0,30],[20,38],[40,32],[60,48],[80,55],[100,70],[120,78]];
    } else {
      // flat / neutral / no change
      pts = [[0,50],[20,54],[40,48],[60,52],[80,48],[100,54],[120,50]];
    }
    const line = "M" + pts.map(p => p.join(" ")).join(" L");
    const area = line + ` L${pts[pts.length-1][0]} 100 L0 100 Z`;
    return { area, line, endX: pts[pts.length-1][0], endY: pts[pts.length-1][1] };
  }

  // ─── Ensure shared SVG gradient defs exist (id-referenced) ─
  function ensureDefs() {
    if (document.getElementById("u2-shared-defs")) return;
    const wrap = document.createElement("div");
    wrap.id = "u2-shared-defs";
    wrap.style.cssText = "position:absolute;width:0;height:0;overflow:hidden;";
    wrap.setAttribute("aria-hidden", "true");
    wrap.innerHTML = `
      <svg width="0" height="0">
        <defs>
          <linearGradient id="brandLineGradient" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%"   stop-color="#0000FF"/>
            <stop offset="100%" stop-color="#03FF88"/>
          </linearGradient>
          <linearGradient id="brandAreaGradient" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%"   stop-color="#0000FF" stop-opacity="0.14"/>
            <stop offset="60%"  stop-color="#03FF88" stop-opacity="0.06"/>
            <stop offset="100%" stop-color="#03FF88" stop-opacity="0"/>
          </linearGradient>
        </defs>
      </svg>`;
    document.body.appendChild(wrap);
  }

  // ─── Detect trend from existing .metric-change class ────────
  function detectTrend(card) {
    const change = card.querySelector(".metric-change");
    if (change && change.classList.contains("up")) return "up";
    if (change && change.classList.contains("down")) return "down";
    return "flat";
  }

  // ─── Upgrade a single .metric-card to split layout ──────────
  function upgrade(card) {
    if (card.classList.contains("u2-applied")) return;

    const heading = card.querySelector(".metric-heading");
    const content = card.querySelector(".metric-content");
    if (!heading || !content) return;

    const label = heading.textContent.trim();
    const trend = detectTrend(card);
    const path = sparkPath(trend);

    // Build LEFT column wrapper
    const main = document.createElement("div");
    main.className = "u2-stat__main";

    const top = document.createElement("div");
    top.className = "u2-stat__top";
    top.innerHTML = `
      <div class="u2-icon u2-icon--brand" title="${label}">
        <svg viewBox="0 0 24 24" stroke="currentColor">${iconFor(label)}</svg>
      </div>
    `;

    // Horizontal layout: icon LEFT, text stack RIGHT.
    // Wrap heading + content in a text column so they stack right of the icon.
    const text = document.createElement("div");
    text.className = "u2-stat__text";
    text.appendChild(heading);
    text.appendChild(content);

    main.appendChild(top);
    main.appendChild(text);

    // Replace card contents with [main]. Sparkline intentionally omitted —
    // looked noisy with placeholder data; revisit when real historical data lands.
    card.innerHTML = "";
    card.appendChild(main);
    card.classList.add("u2-applied");
  }

  function applyAll() {
    const cards = document.querySelectorAll(".metric-card");
    if (!cards.length) return;
    ensureDefs();
    cards.forEach(upgrade);
  }

  // Run on DOMContentLoaded, and re-run after HTMX swaps
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", applyAll);
  } else {
    applyAll();
  }
  document.addEventListener("htmx:afterSwap", applyAll);
  document.addEventListener("metric-cards:refresh", applyAll);
})();
