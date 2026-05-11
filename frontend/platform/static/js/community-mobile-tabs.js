/**
 * community-mobile-tabs.js — WS1.5
 *
 * Listens for horizontal swipes on the community content area and switches
 * to the next/prev visible community tab. Active only on viewports
 * ≤ 640px to avoid hijacking desktop scrolls. Skips when the gesture
 * starts on an input/textarea/select/contenteditable element.
 */
(function () {
  "use strict";

  const SWIPE_MIN_X = 60; // px
  const SWIPE_MAX_Y = 40; // px

  function visibleTabs() {
    return Array.from(document.querySelectorAll(".community-tab-btn"))
      .filter((b) => !b.classList.contains("ds-sr-only") && b.offsetParent !== null);
  }

  function activeTabIndex(tabs) {
    return tabs.findIndex((b) => b.classList.contains("active"));
  }

  function activateTabAt(tabs, idx) {
    if (idx < 0 || idx >= tabs.length) return;
    const target = tabs[idx];
    if (typeof window.switchCommunityTab === "function") {
      window.switchCommunityTab(target);
    } else {
      target.click();
    }
    target.scrollIntoView({ behavior: "smooth", inline: "center", block: "nearest" });
  }

  function setup() {
    const main = document.querySelector(".lb-main");
    if (!main || main.dataset.swipeBound === "1") return;
    main.dataset.swipeBound = "1";

    let startX = 0;
    let startY = 0;
    let active = false;

    main.addEventListener("touchstart", (event) => {
      if (window.innerWidth > 640) return;
      const target = event.target;
      if (target.closest("input, textarea, select, [contenteditable=\"true\"]")) return;
      const t = event.touches[0];
      startX = t.clientX;
      startY = t.clientY;
      active = true;
    }, { passive: true });

    main.addEventListener("touchend", (event) => {
      if (!active) return;
      active = false;
      const t = event.changedTouches[0];
      const dx = t.clientX - startX;
      const dy = Math.abs(t.clientY - startY);
      if (Math.abs(dx) < SWIPE_MIN_X || dy > SWIPE_MAX_Y) return;
      const tabs = visibleTabs();
      const idx = activeTabIndex(tabs);
      if (idx < 0) return;
      const next = dx < 0 ? idx + 1 : idx - 1;
      activateTabAt(tabs, next);
    }, { passive: true });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", setup);
  } else {
    setup();
  }
})();
