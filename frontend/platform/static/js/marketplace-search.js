/**
 * Marketplace Search & Filter
 * ============================
 * Enables real-time search and dropdown filtering on the marketplace
 * and commodities-marketplace pages.
 *
 * Searches across: property title, location, badge text, price.
 * Dropdown filters: location, investment/duration type, property/asset type.
 *
 * Also wires the sidebar search to filter marketplace assets:
 *   - If already on /marketplace or /commodities-marketplace → filters in place
 *   - Otherwise → navigates to /marketplace?q=<query>
 */
(function () {
  "use strict";

  function initMarketplaceSearch() {
    const searchInput = document.getElementById("filter-bar-search-input");
    const searchBtn = document.getElementById("filter-bar-search-btn");
    const clearBtn = document.getElementById("filter-bar-clear-btn");

    function getPropertyGrid() {
      return (
        document.getElementById("property-grid") ||
        document.getElementById("commodity-grid")
      );
    }

    if (!searchInput || !getPropertyGrid()) return;
    if (searchInput.dataset.marketplaceSearchReady === "true") return;
    searchInput.dataset.marketplaceSearchReady = "true";

    // ─── Search Logic ───
    function performSearch() {
      const query = searchInput.value.toLowerCase().trim();
      const propertyGrid = getPropertyGrid();
      if (!propertyGrid) return;
      const cards = propertyGrid.querySelectorAll(".property-card");

      cards.forEach(function (card) {
        // Gather all searchable text from the card
        const title = card.querySelector(".property-title");
        const location = card.querySelector('[id*="-location-text"]');
        const badge = card.querySelector(".badge-text");
        const price = card.querySelector(".property-price");
        const duration = card.querySelector('[id*="-duration-value"]');

        const searchableText = [
          title ? title.textContent : "",
          location ? location.textContent : "",
          badge ? badge.textContent : "",
          price ? price.textContent : "",
          duration ? duration.textContent : "",
        ]
          .join(" ")
          .toLowerCase();

        if (!query || searchableText.includes(query)) {
          card.style.display = "";
          card.style.opacity = "1";
          card.style.pointerEvents = "";
        } else {
          card.style.display = "none";
        }
      });

      // Show "no results" message
      updateNoResultsMessage(propertyGrid, cards);
    }

    // ─── Dropdown Filter Logic ───
    function performDropdownFilter() {
      const locationSelect = document.getElementById(
        "filter-bar-location-select",
      );
      const investmentSelect = document.getElementById(
        "filter-bar-investment-select",
      );
      const propertyTypeSelect = document.getElementById(
        "filter-bar-property-select",
      );

      const locationVal = locationSelect ? locationSelect.value : "";
      const investmentVal = investmentSelect ? investmentSelect.value : "";
      const propertyVal = propertyTypeSelect ? propertyTypeSelect.value : "";
      const isCommoditiesPage = window.location.pathname === "/commodities-marketplace";
      const propertyGrid = getPropertyGrid();
      if (!propertyGrid) return;

      const cards = propertyGrid.querySelectorAll(".property-card");

      cards.forEach(function (card) {
        let visible = true;

        // 1. Location Filter
        if (locationVal && locationVal !== "any") {
          const locationText = (card.dataset.location || "").toLowerCase();
          const areaText = (card.dataset.area || "").toLowerCase();
          const filterText = locationVal.replace(/-/g, " ").toLowerCase();

          // For "bali" filter, match any Bali location
          if (locationVal === "bali") {
            if (!locationText.includes("bali") && !areaText.includes("bali")) {
              visible = false;
            }
          } else {
            // For specific sub-locations like "bali canggu", check both location and area
            const parts = filterText.split(" ");
            const areaFilter = parts.length > 1 ? parts.slice(1).join(" ") : "";

            if (areaFilter) {
              // Check if area matches the specific sub-location
              if (!locationText.includes(areaFilter) && !areaText.includes(areaFilter)) {
                visible = false;
              }
            } else if (locationText && !locationText.includes(filterText)) {
              visible = false;
            }
          }
        }

        // 2. Asset Type Filter (Priority: data-asset-type)
        if (visible && propertyVal && propertyVal !== "any") {
          const assetType = (card.dataset.assetType || "").toLowerCase();
          const commodityType = (card.dataset.commodityType || "").toLowerCase();
          const badgeEl = card.querySelector(".badge-text");
          const badgeText = badgeEl ? badgeEl.textContent.toLowerCase() : "";

          if (isCommoditiesPage) {
            if (commodityType !== propertyVal && !badgeText.includes(propertyVal)) visible = false;
          } else if (["real_estate", "commercial_property", "land_plot"].includes(propertyVal)) {
            if (assetType !== propertyVal) visible = false;
          } else if (propertyVal === "commercial") {
            if (assetType !== "commercial" && assetType !== "commercial_property" && !badgeText.includes("commercial")) visible = false;
          } else if (propertyVal === "villa" || propertyVal === "residential") {
            if (assetType !== "villa" && assetType !== "residential" && assetType !== "real_estate" && badgeText.includes("commercial")) visible = false;
          }
        }

        // 3. Investment Type / Duration Filter
        if (visible && investmentVal && investmentVal !== "any") {
          const duration = (card.dataset.duration || "").toLowerCase();
          const durationMonths = parseInt(card.dataset.durationMonths || "", 10);
          const durationEl = card.querySelector('[id*="-duration-value"]');
          const durationText = duration || (durationEl ? durationEl.textContent.toLowerCase() : "");

          if (durationText) {
            const monthsMatch = durationText.match(/(\d+)\s*month/);
            const months = Number.isFinite(durationMonths) ? durationMonths : (monthsMatch ? parseInt(monthsMatch[1], 10) : null);
            if (investmentVal === "short-term" || investmentVal === "0-6") {
              if (months !== null) {
                if (months > 12) visible = false;
              } else if (!durationText.includes("month") || durationText.includes("year")) visible = false;
            } else if (investmentVal === "7-12") {
              if (months === null || months < 7 || months > 12) visible = false;
            } else if (investmentVal === "13plus") {
              if (months === null || months < 13) visible = false;
            } else if (investmentVal === "long-term" || investmentVal === "2-5" || investmentVal === "5plus") {
              if (months !== null) {
                if (months <= 12) visible = false;
              } else if (!durationText.includes("year")) visible = false;
            }
          }
        }

        // 4. Text Search
        const query = searchInput.value.toLowerCase().trim();
        if (visible && query) {
          const title = card.querySelector(".property-title");
          const locationText = (card.dataset.location || card.querySelector(".category-text")?.textContent || "").toLowerCase();
          const badgeText = card.querySelector(".badge-text")?.textContent.toLowerCase() || "";
          const priceText = (card.dataset.price || card.querySelector(".property-price")?.textContent || "").toLowerCase();

          const searchableText = [
            title ? title.textContent : "",
            locationText,
            badgeText,
            priceText,
          ].join(" ").toLowerCase();

          if (!searchableText.includes(query)) {
            visible = false;
          }
        }

        card.style.display = visible ? "" : "none";
        if (visible) {
          card.style.opacity = "1";
          card.style.pointerEvents = "";
        }
      });

      updateNoResultsMessage(propertyGrid, cards);
    }

    // ─── No Results Message ───
    function updateNoResultsMessage(grid, cards) {
      let existingMsg = grid.querySelector(".marketplace-no-results");
      const visibleCount = Array.from(cards).filter(
        (c) => c.style.display !== "none",
      ).length;

      if (visibleCount === 0) {
        if (!existingMsg) {
          existingMsg = document.createElement("div");
          existingMsg.className = "marketplace-no-results";
          existingMsg.style.cssText =
            "grid-column: 1 / -1; text-align: center; padding: 60px 20px; color: #717680; font-size: 16px;";
          existingMsg.setAttribute("role", "status");
          const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
          icon.setAttribute("width", "48");
          icon.setAttribute("height", "48");
          icon.setAttribute("viewBox", "0 0 24 24");
          icon.setAttribute("fill", "none");
          icon.style.cssText = "margin: 0 auto 16px; display: block;";
          const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");
          circle.setAttribute("cx", "11");
          circle.setAttribute("cy", "11");
          circle.setAttribute("r", "7");
          circle.setAttribute("stroke", "#A4A7AE");
          circle.setAttribute("stroke-width", "2");
          const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
          path.setAttribute("d", "M16 16L21 21");
          path.setAttribute("stroke", "#A4A7AE");
          path.setAttribute("stroke-width", "2");
          path.setAttribute("stroke-linecap", "round");
          icon.append(circle, path);
          const title = document.createElement("p");
          title.style.cssText = "font-weight: 600; color: #414651; margin-bottom: 4px;";
          title.textContent = window.location.pathname === "/commodities-marketplace" ? "No commodities found" : "No properties found";
          const hint = document.createElement("p");
          hint.textContent = "Try adjusting your search or filter criteria";
          existingMsg.append(icon, title, hint);
          grid.appendChild(existingMsg);
        }
        existingMsg.style.display = "";
      } else if (existingMsg) {
        existingMsg.remove();
      }
    }

    // ─── Event Bindings ───

    // Live search on typing (with debounce)
    let searchTimeout;
    searchInput.addEventListener("input", function () {
      clearTimeout(searchTimeout);
      searchTimeout = setTimeout(performDropdownFilter, 200);
    });

    // Search on Enter key
    searchInput.addEventListener("keydown", function (e) {
      if (e.key === "Enter") {
        e.preventDefault();
        clearTimeout(searchTimeout);
        performDropdownFilter();
      }
    });

    // Search button click
    if (searchBtn) {
      searchBtn.addEventListener("click", function (e) {
        e.preventDefault();
        clearTimeout(searchTimeout);
        performDropdownFilter();
      });
    }

    // Clear button
    if (clearBtn) {
      clearBtn.addEventListener("click", function (e) {
        e.preventDefault();
        searchInput.value = "";
        const propertyGrid = getPropertyGrid();
        if (!propertyGrid) return;

        // Reset native selects
        [
          "filter-bar-location-select",
          "filter-bar-investment-select",
          "filter-bar-property-select",
        ].forEach(function (id) {
          const sel = document.getElementById(id);
          if (sel) {
            sel.selectedIndex = 0;
            sel.dispatchEvent(new Event("change", { bubbles: true }));
          }
        });

        // Reset POOOL dropdowns if they exist
        document
          .querySelectorAll(".filter-dropdown .poool-dropdown")
          .forEach(function (dd) {
            const firstOption = dd.querySelector(".poool-dropdown__option");
            if (firstOption) {
              firstOption.click();
            }
          });

        // Show all cards
        const cards = propertyGrid.querySelectorAll(".property-card");
        cards.forEach(function (card) {
          card.style.display = "";
          card.style.opacity = "1";
          card.style.pointerEvents = "";
        });

        // Hide no-results message
        const msg = propertyGrid.querySelector(".marketplace-no-results");
        if (msg) msg.remove();
      });
    }

    // Listen for dropdown changes (both native and POOOL dropdown)
    [
      "filter-bar-location-select",
      "filter-bar-investment-select",
      "filter-bar-property-select",
    ].forEach(function (id) {
      const sel = document.getElementById(id);
      if (sel) {
        sel.addEventListener("change", function () {
          performDropdownFilter();
        });
      }
    });

    // Also listen for POOOL custom dropdown change events
    if (document.body.dataset.marketplaceDropdownFilterReady !== "true") {
      document.body.dataset.marketplaceDropdownFilterReady = "true";
      document.addEventListener("dropdown:change", function (e) {
        const dropdown = e.target.closest(".filter-dropdown");
        if (dropdown) {
          // Small delay to let POOOL dropdown sync values
          setTimeout(performDropdownFilter, 50);
        }
      });
    }
  }

  window._initMarketplaceSearch = initMarketplaceSearch;

  // ─── Sidebar Search Wiring ───
  function initSidebarSearch() {
    const sidebarInput = document.getElementById("sidebar-search-input");
    if (!sidebarInput) return;

    const isMarketplace =
      window.location.pathname === "/marketplace" ||
      window.location.pathname === "/commodities-marketplace";

    // If we landed on marketplace with a ?q= param, pre-fill the filter inputs
    const urlParams = new URLSearchParams(window.location.search);
    const qParam = urlParams.get("q");
    if (qParam && isMarketplace) {
      // Fill the sidebar input for visual feedback
      sidebarInput.value = qParam;
      // Fill the filter bar search input and trigger filtering
      const filterInput = document.getElementById("filter-bar-search-input");
      if (filterInput) {
        filterInput.value = qParam;
        // Small delay so cards are rendered before filtering
        setTimeout(function () {
          filterInput.dispatchEvent(new Event("input", { bubbles: true }));
        }, 300);
      }
    } else {
      // Clear sidebar input if not on marketplace or no query (fixes persistence bug)
      sidebarInput.value = "";
    }

    function runSidebarSearch() {
      const query = sidebarInput.value.trim();

      if (isMarketplace) {
        // Filter asset cards directly — mirror into the filter-bar input
        const filterInput = document.getElementById("filter-bar-search-input");
        if (filterInput) {
          filterInput.value = query;
          filterInput.dispatchEvent(new Event("input", { bubbles: true }));
        }
      } else {
        // Navigate to marketplace with query
        if (query) {
          window.location.href =
            "/marketplace?q=" + encodeURIComponent(query);
        } else {
          window.location.href = "/marketplace";
        }
      }
    }

    // Debounced live filter while on marketplace
    let sidebarTimeout;
    sidebarInput.addEventListener("input", function () {
      if (isMarketplace) {
        clearTimeout(sidebarTimeout);
        sidebarTimeout = setTimeout(runSidebarSearch, 200);
      }
    });

    // Navigate / filter on Enter
    sidebarInput.addEventListener("keydown", function (e) {
      if (e.key === "Enter") {
        e.preventDefault();
        clearTimeout(sidebarTimeout);
        runSidebarSearch();
      }
    });
  }

  // Run on DOM ready
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", function () {
      initMarketplaceSearch();
      // initSidebarSearch();
    });
  } else {
    // Small delay to ensure HTML is fully parsed
    setTimeout(function () {
      initMarketplaceSearch();
      // initSidebarSearch(); // Disabled in favor of global-search.js
    }, 150);
  }

  // Re-init after HTMX swaps (for tab changes)
  document.addEventListener("htmx:afterSwap", function () {
    setTimeout(function () {
      initMarketplaceSearch();
      // initSidebarSearch();
    }, 200);
  });
})();
