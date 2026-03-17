/**
 * sidebar-community.js — Global community card dismiss handler
 * =============================================================
 * Loaded on every page via head.html so that the sidebar
 * "Community soon" card can be dismissed from any page.
 *
 * Buttons handled:
 *   #featured-card-close   — X button
 *   #featured-card-dismiss — "Dismiss" text button
 *   #featured-card-action  — "What's new?" button
 */
(function () {
  "use strict";

  var STORAGE_KEY = "community-card-dismissed";

  function hideCard(card) {
    if (!card) return;
    card.style.transition = "max-height 0.3s ease, opacity 0.3s ease";
    card.style.maxHeight = card.offsetHeight + "px";
    requestAnimationFrame(function () {
      card.style.maxHeight = "0";
      card.style.opacity = "0";
      card.style.overflow = "hidden";
      setTimeout(function () {
        card.style.display = "none";
      }, 310);
    });
    try {
      localStorage.setItem(STORAGE_KEY, "1");
    } catch (e) { /* private browsing */ }
  }

  function init() {
    // Respect previously dismissed state
    try {
      if (localStorage.getItem(STORAGE_KEY) === "1") {
        document
          .querySelectorAll("#nav-featured-card, .nav-featured-card")
          .forEach(function (card) {
            card.style.display = "none";
          });
        return; // No need to wire buttons if already hidden
      }
    } catch (e) { /* private browsing */ }

    // Close (X) button
    document
      .querySelectorAll("#featured-card-close")
      .forEach(function (btn) {
        if (btn.getAttribute("data-community-wired")) return;
        btn.setAttribute("data-community-wired", "1");
        btn.addEventListener("click", function (e) {
          e.preventDefault();
          e.stopPropagation();
          var card = btn.closest(
            "#nav-featured-card, .sidebar__featured-card, .nav-featured-card"
          );
          hideCard(card);
        });
      });

    // Dismiss button
    document
      .querySelectorAll("#featured-card-dismiss")
      .forEach(function (btn) {
        if (btn.getAttribute("data-community-wired")) return;
        btn.setAttribute("data-community-wired", "1");
        btn.addEventListener("click", function (e) {
          e.preventDefault();
          e.stopPropagation();
          var card = btn.closest(
            "#nav-featured-card, .sidebar__featured-card, .nav-featured-card"
          );
          hideCard(card);
        });
      });

    // "What's new?" button — opens changelog
    document
      .querySelectorAll("#featured-card-action")
      .forEach(function (btn) {
        if (btn.getAttribute("data-community-wired")) return;
        btn.setAttribute("data-community-wired", "1");
        btn.addEventListener("click", function () {
          window.open("/changelog", "_blank", "noopener");
        });
      });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
