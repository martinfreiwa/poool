/**
 * metric-card-animations.js
 *
 * GSAP-driven counter + gradient bar reveal for `.metric-card` tiles
 * (developer dashboard "Priority Metrics"). Mirrors showcase #25.
 *
 * What it does:
 *  - Reads `.metric-number[data-final-value]` text (e.g. "$1.5M", "$138.4k", "12")
 *  - Splits prefix / number / suffix, animates 0 → number, re-renders text on tick
 *  - Animates each card's top gradient strip from scaleX(0) → 1, staggered
 *
 * Re-runs after HTMX `htmx:afterSwap` events that replace the metrics section,
 * and after the in-page `metric-cards:refresh` custom event.
 *
 * Idempotent: a metric is only animated once per page render. Reset by
 * removing `data-anim-done` from the element.
 */
(function () {
  "use strict";

  if (typeof window === "undefined") return;

  // ─── GSAP presence guard ────────────────────────────────────
  // GSAP is loaded via external_scripts in dashboard.html. If absent
  // (e.g. offline / CDN block), fall back to instant render — values
  // are already in the DOM, so nothing visually breaks.
  function hasGsap() {
    return typeof window.gsap !== "undefined";
  }

  // ─── Value parser ───────────────────────────────────────────
  // Splits "<prefix><number><suffix>" while preserving the original format.
  // Examples:
  //   "$1.5M"     → { prefix: "$",  num: 1.5,    suffix: "M",  decimals: 1, commas: false }
  //   "$138.4k"   → { prefix: "$",  num: 138.4,  suffix: "k",  decimals: 1, commas: false }
  //   "$1,234"    → { prefix: "$",  num: 1234,   suffix: "",   decimals: 0, commas: true  }
  //   "12"        → { prefix: "",   num: 12,     suffix: "",   decimals: 0, commas: false }
  //   "—" / ""    → null (no animation)
  function parseMetric(raw) {
    if (!raw) return null;
    const m = raw.match(/^([^\d\-]*)(-?[\d,]+(?:\.\d+)?)(.*)$/);
    if (!m) return null;
    const numStr = m[2];
    const num = parseFloat(numStr.replace(/,/g, ""));
    if (!Number.isFinite(num)) return null;
    return {
      prefix: m[1],
      num,
      suffix: m[3],
      decimals: (numStr.split(".")[1] || "").length,
      commas: numStr.includes(","),
    };
  }

  function formatValue(parsed, v) {
    let body;
    if (parsed.commas) {
      body = Math.round(v).toLocaleString("en-US");
    } else if (parsed.decimals > 0) {
      body = v.toFixed(parsed.decimals);
    } else {
      body = Math.round(v).toString();
    }
    return parsed.prefix + body + parsed.suffix;
  }

  // ─── Animate a single metric card ───────────────────────────
  function animateCard(card, indexInRow) {
    if (!card || card.dataset.animDone === "1") return;

    const numberEl = card.querySelector(".metric-number");
    if (!numberEl) return;

    const raw = numberEl.getAttribute("data-final-value") || numberEl.textContent.trim();
    const parsed = parseMetric(raw);

    card.dataset.animDone = "1";

    // Always animate the top gradient strip — works even if value is unparseable
    card.style.setProperty("--bar-fill", "0");

    if (!hasGsap()) {
      // No GSAP → snap to end state
      card.style.setProperty("--bar-fill", "1");
      if (parsed) numberEl.textContent = formatValue(parsed, parsed.num);
      return;
    }

    const delay = indexInRow * 0.08;

    // 1. Top gradient strip reveal (scaleX 0 → 1)
    // GSAP's CSS var support is unreliable across browsers/versions, so we
    // animate a plain JS object and write the value via setProperty in onUpdate.
    const barState = { v: 0 };
    card.style.setProperty("--bar-fill", "0");
    gsap.to(barState, {
      v: 1,
      duration: 1.1,
      delay,
      ease: "power3.out",
      onUpdate: function () {
        card.style.setProperty("--bar-fill", barState.v.toFixed(3));
      },
      onComplete: function () {
        card.style.setProperty("--bar-fill", "1");
      },
    });

    // 2. Counter
    if (parsed) {
      // Start at zero immediately to avoid a frame of the final value
      numberEl.textContent = formatValue(parsed, 0);
      const tweenTarget = { v: 0 };
      gsap.to(tweenTarget, {
        v: parsed.num,
        duration: 1.6,
        delay,
        ease: "power2.out",
        onUpdate: function () {
          numberEl.textContent = formatValue(parsed, tweenTarget.v);
        },
        onComplete: function () {
          numberEl.textContent = formatValue(parsed, parsed.num);
        },
      });
    }
  }

  // ─── Run on all cards in `.metrics-section` ─────────────────
  function animateAll(root) {
    const scope = root && root.querySelector ? root : document;
    const cards = scope.querySelectorAll(".metric-card");
    cards.forEach((card, i) => animateCard(card, i));
  }

  function reset(root) {
    const scope = root && root.querySelector ? root : document;
    scope.querySelectorAll(".metric-card").forEach((c) => {
      delete c.dataset.animDone;
      c.style.removeProperty("--bar-fill");
    });
  }

  // ─── Boot ───────────────────────────────────────────────────
  function boot() {
    animateAll(document);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }

  // Re-run after HTMX swaps that touch the metrics section
  document.addEventListener("htmx:afterSwap", function (evt) {
    const target = evt && evt.detail && evt.detail.target;
    if (!target) return;
    if (target.id === "metrics-section" || target.querySelector?.(".metric-card")) {
      reset(target);
      animateAll(target);
    }
  });

  // Public hook for ad-hoc refresh
  document.addEventListener("metric-cards:refresh", function () {
    reset(document);
    animateAll(document);
  });

  window.MetricCardAnimations = { animateAll, reset };
})();
