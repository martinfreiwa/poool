/**
 * Commodities Marketplace – Button & Filter Wiring
 * ===================================================
 * Handles all interactive elements on /commodities-marketplace
 *
 * Buttons covered:
 *  1. Status tabs:      Available / Funded / Exited  (HTMX + active-class sync)
 *  2. More filters btn: toggle commodity-specific extra-filter panel
 *  3. Search btn:       trigger search (wired in marketplace-search.js, re-confirmed here)
 *  4. Clear btn:        reset all filters and search
 *  5. KYC "Learn more" secondary btn
 *  6. Sidebar community-card: Close (X), Dismiss, What's new?
 *  7. Profile dropdown: Sign-out, Documentation, Switch account (handled by profile-dropdown.js)
 */

(function () {
    "use strict";

    // ─── 1. Status Tabs – active class management ───────────────────────────
    function initStatusTabs() {
        const tabs = document.querySelectorAll(
            "#filter-bar-status-tabs .status-tab"
        );
        if (!tabs.length) return;

        tabs.forEach(function (tab) {
            tab.addEventListener("click", function () {
                tabs.forEach(function (t) {
                    t.classList.remove("active");
                });
                tab.classList.add("active");
            });
        });

        // After every HTMX swap back into #commodities-content-wrapper,
        // reinitialise cards so the search/filter JS can see the new DOM.
        document.addEventListener("htmx:afterSwap", function (evt) {
            if (
                evt.detail.target &&
                evt.detail.target.id === "commodities-content-wrapper"
            ) {
                // Re-run search init so filters work on freshly-swapped content
                setTimeout(function () {
                    if (typeof window._initMarketplaceSearch === "function") {
                        window._initMarketplaceSearch();
                    }
                    if (typeof window.initializePropertyCards === "function") {
                        window.initializePropertyCards(evt.detail.target);
                    }
                    // Re-attach card animation
                    document
                        .querySelectorAll(".property-card")
                        .forEach(function (card) {
                            card.classList.remove("loaded");
                            setTimeout(function () {
                                card.classList.add("loaded");
                            }, 50);
                        });
                }, 100);
            }
        });
    }

    // ─── 2. More Filters – commodity-type extra panel ────────────────────────
    var moreFiltersOpen = false;
    var extraPanel = null;

    function buildExtraFilterPanel() {
        const panel = document.createElement("div");
        panel.id = "commodities-extra-filters";
        panel.style.cssText = [
            "display:none",
            "position:absolute",
            "top:calc(100% + 8px)",
            "left:0",
            "background:#fff",
            "border:1px solid #d5d7da",
            "border-radius:12px",
            "padding:20px 24px",
            "box-shadow:0 8px 24px rgba(10,13,18,0.12)",
            "z-index:200",
            "min-width:360px",
            "gap:16px",
            "flex-direction:column",
        ].join(";");

        panel.innerHTML = `
      <div style="font-weight:700;font-size:14px;color:#181d27;margin-bottom:4px;">More Filters</div>
      <div style="display:flex;flex-direction:column;gap:12px;">
        <!-- Commodity type -->
        <div>
          <label style="font-size:13px;font-weight:500;color:#414651;display:block;margin-bottom:6px;">Commodity Type</label>
          <select id="filter-commodity-type"
            style="width:100%;height:40px;border:1px solid #d5d7da;border-radius:8px;padding:0 12px;font-size:14px;color:#181d27;background:#fff;cursor:pointer;appearance:none;-webkit-appearance:none;">
            <option value="any" selected>All types</option>
            <option value="agriculture">Agriculture</option>
          </select>
        </div>
        <!-- Min / max price -->
        <div style="display:flex;gap:12px;">
          <div style="flex:1;">
            <label style="font-size:13px;font-weight:500;color:#414651;display:block;margin-bottom:6px;">Min Value ($)</label>
            <input id="filter-min-price" type="number" min="0" placeholder="0"
              style="width:100%;height:40px;border:1px solid #d5d7da;border-radius:8px;padding:0 12px;font-size:14px;color:#181d27;background:#fff;box-sizing:border-box;" />
          </div>
          <div style="flex:1;">
            <label style="font-size:13px;font-weight:500;color:#414651;display:block;margin-bottom:6px;">Max Value ($)</label>
            <input id="filter-max-price" type="number" min="0" placeholder="Any"
              style="width:100%;height:40px;border:1px solid #d5d7da;border-radius:8px;padding:0 12px;font-size:14px;color:#181d27;background:#fff;box-sizing:border-box;" />
          </div>
        </div>
        <!-- Min yield -->
        <div>
          <label style="font-size:13px;font-weight:500;color:#414651;display:block;margin-bottom:6px;">Minimum Annual Yield (%)</label>
          <input id="filter-min-yield" type="number" min="0" max="100" step="0.1" placeholder="0"
            style="width:100%;height:40px;border:1px solid #d5d7da;border-radius:8px;padding:0 12px;font-size:14px;color:#181d27;background:#fff;box-sizing:border-box;" />
        </div>
        <!-- Actions -->
        <div style="display:flex;gap:10px;margin-top:4px;">
          <button id="extra-filter-apply" style="flex:1;height:40px;background:var(--primary-color,#0ea5e9);color:#fff;border:none;border-radius:8px;font-weight:600;font-size:14px;cursor:pointer;">
            Apply
          </button>
          <button id="extra-filter-reset" style="flex:1;height:40px;background:#fff;color:#414651;border:1px solid #d5d7da;border-radius:8px;font-weight:600;font-size:14px;cursor:pointer;">
            Reset
          </button>
        </div>
      </div>
    `;

        // Apply button
        panel.querySelector("#extra-filter-apply").addEventListener("click", function () {
            applyExtraFilters();
            closeExtraPanel();
        });

        // Reset button
        panel.querySelector("#extra-filter-reset").addEventListener("click", function () {
            panel.querySelector("#filter-commodity-type").value = "any";
            panel.querySelector("#filter-min-price").value = "";
            panel.querySelector("#filter-max-price").value = "";
            panel.querySelector("#filter-min-yield").value = "";
            applyExtraFilters();
        });

        return panel;
    }

    function applyExtraFilters() {
        const commodityType = document.getElementById("filter-commodity-type");
        const minPrice = document.getElementById("filter-min-price");
        const maxPrice = document.getElementById("filter-max-price");
        const minYield = document.getElementById("filter-min-yield");

        const typeVal = commodityType ? commodityType.value : "any";
        const minP = minPrice ? parseFloat(minPrice.value) || 0 : 0;
        const maxP = maxPrice ? parseFloat(maxPrice.value) || Infinity : Infinity;
        const minY = minYield ? parseFloat(minYield.value) || 0 : 0;

        const cards = document.querySelectorAll("#property-grid .property-card");
        cards.forEach(function (card) {
            let visible = true;

            // Price filter (data-price is in dollars)
            const price = parseFloat(card.dataset.price) || 0;
            if (price < minP || price > maxP) visible = false;

            if (visible && typeVal !== "any") {
                const cardType = (card.dataset.commodityType || "").toLowerCase();
                const badge = card.querySelector(".badge-text");
                const badgeText = badge ? badge.textContent.toLowerCase() : "";
                if (cardType !== typeVal && !badgeText.includes(typeVal)) visible = false;
            }

            // Yield filter — read from card's investment-value cells
            if (visible && minY > 0) {
                const dataYield = parseFloat(card.dataset.yield);
                const rows = card.querySelectorAll(".investment-row");
                let yieldPct = Number.isFinite(dataYield) ? dataYield : 0;
                rows.forEach(function (row) {
                    const label = row.querySelector(".investment-label");
                    const value = row.querySelector(".investment-value");
                    const labelText = label ? label.textContent.trim().toLowerCase() : "";
                    if (
                        value &&
                        (labelText.includes("yield") ||
                            labelText.includes("annualised net return"))
                    ) {
                        yieldPct = parseFloat(value.textContent) || 0;
                    }
                });
                if (yieldPct < minY) visible = false;
            }

            card.style.display = visible ? "" : "none";
            if (visible) {
                card.style.opacity = "1";
                card.style.pointerEvents = "";
            }
        });
    }

    function openExtraPanel() {
        if (!extraPanel) return;
        extraPanel.style.display = "flex";
        moreFiltersOpen = true;
        const moreBtn = document.getElementById("filter-bar-more-filters");
        if (moreBtn) {
            moreBtn.style.background = "#f0fdf8";
            moreBtn.setAttribute("aria-expanded", "true");
        }
    }

    function closeExtraPanel() {
        if (!extraPanel) return;
        extraPanel.style.display = "none";
        moreFiltersOpen = false;
        const moreBtn = document.getElementById("filter-bar-more-filters");
        if (moreBtn) {
            moreBtn.style.background = "";
            moreBtn.setAttribute("aria-expanded", "false");
        }
    }

    function initMoreFilters() {
        const moreBtn = document.getElementById("filter-bar-more-filters");
        if (!moreBtn) return;
        if (moreBtn.dataset.commoditiesMoreFiltersReady === "true") return;
        moreBtn.dataset.commoditiesMoreFiltersReady = "true";

        // Build and inject panel as sibling of the filter container
        extraPanel = buildExtraFilterPanel();
        const filterContainer = document.getElementById("filter-bar-container");
        if (filterContainer) {
            filterContainer.style.position = "relative";
            filterContainer.appendChild(extraPanel);
        }

        moreBtn.addEventListener("click", function (e) {
            e.stopPropagation();
            if (moreFiltersOpen) {
                closeExtraPanel();
            } else {
                openExtraPanel();
            }
        });

        // Close on outside click
        document.addEventListener("click", function (e) {
            if (
                moreFiltersOpen &&
                extraPanel &&
                !extraPanel.contains(e.target) &&
                e.target !== moreBtn &&
                !moreBtn.contains(e.target)
            ) {
                closeExtraPanel();
            }
        });
    }

    // ─── 3 & 4. Search / Clear – already wired in marketplace-search.js ─────
    // No extra wiring needed; just confirm the clear button also resets extra filters.
    function extendClearButton() {
        const clearBtn = document.getElementById("filter-bar-clear-btn");
        if (!clearBtn) return;
        clearBtn.addEventListener("click", function () {
            // Reset extra filters panel inputs too
            ["filter-commodity-type", "filter-min-price", "filter-max-price", "filter-min-yield"]
                .forEach(function (id) {
                    const el = document.getElementById(id);
                    if (el) {
                        el.tagName === "SELECT" ? (el.value = "any") : (el.value = "");
                    }
                });
        });
    }

    // ─── 5. KYC "Learn more" button ─────────────────────────────────────────
    function initKycLearnMore() {
        document.querySelectorAll(".kyc-banner-btn-secondary").forEach(function (btn) {
            if (!btn.getAttribute("data-wired")) {
                btn.setAttribute("data-wired", "1");
                btn.addEventListener("click", function () {
                    window.open("https://docs.poool.com/kyc", "_blank", "noopener");
                });
            }
        });
    }

    // ─── 6. Sidebar community card buttons ──────────────────────────────────
    function initSidebarCommunityCard() {
        // Close (X) button
        document.querySelectorAll("#featured-card-close").forEach(function (btn) {
            if (!btn.getAttribute("data-wired")) {
                btn.setAttribute("data-wired", "1");
                btn.addEventListener("click", function () {
                    const card = btn.closest("#nav-featured-card, .sidebar__featured-card, .nav-featured-card");
                    if (card) {
                        card.style.maxHeight = card.offsetHeight + "px";
                        requestAnimationFrame(function () {
                            card.style.transition = "max-height 0.3s ease, opacity 0.3s ease";
                            card.style.maxHeight = "0";
                            card.style.opacity = "0";
                            card.style.overflow = "hidden";
                            setTimeout(function () { card.style.display = "none"; }, 310);
                        });
                    }
                    try { localStorage.setItem("community-card-dismissed", "1"); } catch (e) { }
                });
            }
        });

        // Dismiss button  — same as close
        document.querySelectorAll("#featured-card-dismiss").forEach(function (btn) {
            if (!btn.getAttribute("data-wired")) {
                btn.setAttribute("data-wired", "1");
                btn.addEventListener("click", function () {
                    const card = btn.closest("#nav-featured-card, .sidebar__featured-card, .nav-featured-card");
                    if (card) {
                        card.style.transition = "opacity 0.25s ease";
                        card.style.opacity = "0";
                        setTimeout(function () { card.style.display = "none"; }, 260);
                    }
                    try { localStorage.setItem("community-card-dismissed", "1"); } catch (e) { }
                });
            }
        });

        // "What's new?" button — navigate to changelog placeholder
        document.querySelectorAll("#featured-card-action").forEach(function (btn) {
            if (!btn.getAttribute("data-wired")) {
                btn.setAttribute("data-wired", "1");
                btn.addEventListener("click", function () {
                    window.open("/changelog", "_blank", "noopener");
                });
            }
        });

        // Respect previously dismissed state
        try {
            if (localStorage.getItem("community-card-dismissed") === "1") {
                document.querySelectorAll("#nav-featured-card, .nav-featured-card").forEach(function (card) {
                    card.style.display = "none";
                });
            }
        } catch (e) { }
    }

    // ─── Boot ────────────────────────────────────────────────────────────────
    function init() {
        initStatusTabs();
        initMoreFilters();
        extendClearButton();
        initKycLearnMore();
        initSidebarCommunityCard();
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", init);
    } else {
        init();
    }
})();
