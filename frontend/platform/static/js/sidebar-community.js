/**
 * sidebar-community.js — Global community card dismiss handler
 * =============================================================
 * Loaded on every page via head.html so that the sidebar
 * "Community soon" / "Chat to support" card can be dismissed from any page.
 *
 * Uses event delegation so it works even when the sidebar is stamped into the
 * DOM dynamically from a <template> after DOMContentLoaded.
 *
 * Buttons handled (desktop sidebar):
 *   #featured-card-close                        — X button (dismiss card)
 *   #featured-card-dismiss                      — "Dismiss" text button
 *   #featured-card-action                       — "What's new?" button
 *   .sidebar__featured-button--chat             — "Chat to support" button
 *
 * Buttons handled (mobile burger menu):
 *   .mobile-burger-menu__featured-close         — X button (dismiss card)
 *   .mobile-burger-menu__featured-button--chat  — "Chat to support" button
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

  function hideDismissedCards() {
    try {
      if (localStorage.getItem(STORAGE_KEY) === "1") {
        document
          .querySelectorAll("#nav-featured-card, .nav-featured-card")
          .forEach(function (card) {
            card.style.display = "none";
          });
      }
    } catch (e) { /* private browsing */ }
  }

  // Use event delegation on document so handlers work for dynamically
  // injected elements (e.g. sidebar cloned from a <template>).
  document.addEventListener("click", function (e) {
    var target = e.target;

    // ── Desktop sidebar: X / close button ─────────────────────────────────
    var closeBtn = target.closest("#featured-card-close");
    if (closeBtn) {
      e.preventDefault();
      e.stopPropagation();
      var card = closeBtn.closest(
        "#nav-featured-card, .sidebar__featured-card, .nav-featured-card"
      );
      hideCard(card);
      return;
    }

    // ── Desktop sidebar: Dismiss button ───────────────────────────────────
    var dismissBtn = target.closest("#featured-card-dismiss");
    if (dismissBtn) {
      e.preventDefault();
      e.stopPropagation();
      var dCard = dismissBtn.closest(
        "#nav-featured-card, .sidebar__featured-card, .nav-featured-card"
      );
      hideCard(dCard);
      return;
    }

    // ── Desktop sidebar: "What's new?" button ─────────────────────────────
    var actionBtn = target.closest("#featured-card-action");
    if (actionBtn) {
      window.open("/changelog", "_blank", "noopener");
      return;
    }

    // ── Desktop sidebar: "Chat to support" button ─────────────────────────
    var chatBtn = target.closest(".sidebar__featured-button--chat");
    if (chatBtn) {
      window.location.href = "/support";
      return;
    }

    // ── Mobile menu: X / close button ─────────────────────────────────────
    var mobileCloseBtn = target.closest(".mobile-burger-menu__featured-close");
    if (mobileCloseBtn) {
      e.stopPropagation();
      var mCard = mobileCloseBtn.closest(".mobile-burger-menu__featured-card");
      if (mCard) {
        mCard.style.display = "none";
      }
      return;
    }

    // ── Mobile menu: "Chat to support" button ─────────────────────────────
    var mobileChatBtn = target.closest(".mobile-burger-menu__featured-button--chat");
    if (mobileChatBtn) {
      window.location.href = "/support";
      return;
    }
  }, true); // capture phase so we beat any stopPropagation in child handlers

  // Hide already-dismissed cards whenever DOM is ready (handles normal pages)
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", hideDismissedCards);
  } else {
    hideDismissedCards();
  }
})();
