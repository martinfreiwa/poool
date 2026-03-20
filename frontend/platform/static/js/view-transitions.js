/**
 * POOOL — Advanced View Transitions
 *
 * Features:
 *  1. Directional animations via view-transition-types
 *     (forwards = drill-down, backwards = go-up, lateral = same-level)
 *  2. Just-in-time view-transition-name assignment via pageswap/pagereveal
 *     (only the clicked card gets the hero name — cleaner + BFCache-safe)
 */
(function () {
  'use strict';

  /* ── Feature gate ── */
  // The pageswap/pagereveal events require Chrome ≥ 124.
  // If the browser doesn't support them, the default CSS-only
  // fade/slide still works — this is purely progressive enhancement.

  /* ── Helpers ── */

  /** Depth map — higher depth = further from the "root" list view. */
  function getDepth(pathname) {
    // Detail pages
    if (pathname.startsWith('/property/')) return 2;
    if (pathname.startsWith('/commodity/')) return 2;
    // Admin sub-pages (e.g. /admin/user-details)
    if (pathname.startsWith('/admin/') && pathname !== '/admin/' && pathname !== '/admin') return 2;
    // Everything else is top-level
    return 1;
  }

  /** Extract slug from /property/<slug> or /commodity/<slug> */
  function getSlug(pathname) {
    var m = pathname.match(/^\/(property|commodity)\/(.+)$/);
    return m ? m[2] : null;
  }

  /** Check if pathname is a listing page that contains cards */
  function isListingPage(pathname) {
    return pathname === '/' ||
           pathname === '/marketplace' ||
           pathname === '/commodities' ||
           pathname === '/commodities-marketplace';
  }

  /**
   * Find the first visible image element inside a property card.
   * Cards use <div class="property-image active" style="background-image:...">
   */
  function findCardImage(card) {
    return card.querySelector('.property-image.active') ||
           card.querySelector('.property-image:first-child');
  }

  /**
   * Find the card whose onclick (or data attribute) references the given slug.
   */
  function findCardBySlug(slug) {
    var cards = document.querySelectorAll('.property-card, .commodity-card');
    for (var i = 0; i < cards.length; i++) {
      var onclick = cards[i].getAttribute('onclick') || '';
      var dataId  = cards[i].getAttribute('data-property-id') || '';
      if (onclick.indexOf(slug) !== -1 || dataId === slug) {
        return cards[i];
      }
    }
    return null;
  }

  /* ── pageswap: fires on the OLD page right before it disappears ── */
  window.addEventListener('pageswap', function (e) {
    if (!e.viewTransition || !e.activation) return;

    var fromPath = new URL(e.activation.from.url).pathname;
    var toPath   = new URL(e.activation.entry.url).pathname;

    // Leaving a listing page → going to a detail page
    var slug = getSlug(toPath);
    if (slug && isListingPage(fromPath)) {
      var card = findCardBySlug(slug);
      if (card) {
        var img = findCardImage(card);
        if (img) {
          img.style.viewTransitionName = 'property-hero';
        }
      }
    }

    // Set direction type on the OLD page too (for old-snapshot animations)
    var fromDepth = getDepth(fromPath);
    var toDepth   = getDepth(toPath);
    if (toDepth > fromDepth) {
      e.viewTransition.types.add('forwards');
    } else if (toDepth < fromDepth) {
      e.viewTransition.types.add('backwards');
    }

    // Clean up names after transition finishes (BFCache safety)
    e.viewTransition.finished.then(function () {
      var els = document.querySelectorAll('[style*="view-transition-name"]');
      for (var j = 0; j < els.length; j++) {
        els[j].style.viewTransitionName = '';
      }
    }).catch(function () { /* transition skipped or aborted */ });
  });

  /* ── pagereveal: fires on the NEW page before the first paint ── */
  window.addEventListener('pagereveal', function (e) {
    if (!e.viewTransition) return;

    var activation = (typeof navigation !== 'undefined') ? navigation.activation : null;
    if (!activation || !activation.from) return;

    var fromPath = new URL(activation.from.url).pathname;
    var toPath   = new URL(activation.entry.url).pathname;

    // ── Set directional type ──
    var fromDepth = getDepth(fromPath);
    var toDepth   = getDepth(toPath);
    if (toDepth > fromDepth) {
      e.viewTransition.types.add('forwards');
    } else if (toDepth < fromDepth) {
      e.viewTransition.types.add('backwards');
    }

    // ── Arriving on a detail page FROM a listing ──
    var slug = getSlug(toPath);
    if (slug && isListingPage(fromPath)) {
      var heroEl = document.getElementById('gallery-main-image') ||
                   document.querySelector('.gallery-image:first-child');
      if (heroEl) {
        heroEl.style.viewTransitionName = 'property-hero';
        // Clean up after snapshots are taken
        e.viewTransition.ready.then(function () {
          heroEl.style.viewTransitionName = '';
        }).catch(function () { });
      }
    }

    // ── Going BACK to a listing page from a detail ──
    var fromSlug = getSlug(fromPath);
    if (fromSlug && isListingPage(toPath)) {
      var card = findCardBySlug(fromSlug);
      if (card) {
        var img = findCardImage(card);
        if (img) {
          img.style.viewTransitionName = 'property-hero';
          e.viewTransition.ready.then(function () {
            img.style.viewTransitionName = '';
          }).catch(function () { });
        }
      }
    }
  });

})();
