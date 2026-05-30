/**
 * POOOL Dropdown Auto-Init
 * ========================
 * Automatically converts opted-in native <select> elements on the page
 * to the unified POOOL custom dropdown design.
 *
 * Include this AFTER poool-dropdown.js:
 *   <script src="/static/js/poool-dropdown.js"></script>
 *   <script src="/static/js/poool-dropdown-init.js"></script>
 */
(function () {
  "use strict";

  // Resolve the size variant class for a <select>.
  // Priority: data-dropdown-size attr -> known small class map -> "" (default md).
  // Valid sizes: xs, sm, md, lg, xl.
  var VALID_SIZES = { xs: 1, sm: 1, md: 1, lg: 1, xl: 1 };
  function sizeClassFor(selectEl) {
    var size = selectEl.getAttribute("data-dropdown-size");
    if (!size) {
      // Map legacy mini/compact selects to a small variant.
      if (
        selectEl.classList.contains("lb-select-mini") ||
        selectEl.classList.contains("admin-select")
      ) {
        size = "sm";
      }
    }
    if (size === "md" || !VALID_SIZES[size]) return "";
    return "poool-dropdown--" + size;
  }

  function initAllDropdowns() {
    if (!window.PooolDropdown) {
      return;
    }

    // Convert native <select> elements that need the full custom menu.
    const selects = document.querySelectorAll(
      "select.settings-select, select.dropdown-select, select.form-select, select.input-dropdown, select.ds-select, select.ds-input, select.ad-select, select[data-poool-dropdown]",
    );

    selects.forEach(function (selectEl) {
      // Skip if already converted
      if (
        selectEl.style.display === "none" &&
        selectEl.closest("[data-dropdown]")
      ) {
        return;
      }

      try {
        PooolDropdown.fromSelect(selectEl, {
          placeholder: selectEl.options[0]
            ? selectEl.options[0].textContent
            : "Select...",
          noLabel: true,
          searchable: selectEl.hasAttribute('data-searchable'),
          className: sizeClassFor(selectEl),
        });
      } catch (e) {
        console.warn('Failed to init dropdown:', e);
      }
    });

    // Also init any manually-created [data-dropdown] elements
    PooolDropdown.init();
  }

  // Run on DOM ready
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initAllDropdowns);
  } else {
    // Small delay to ensure other scripts have loaded
    setTimeout(initAllDropdowns, 100);
  }

  // Re-init after HTMX swaps (for dynamically loaded content)
  document.addEventListener("htmx:afterSwap", function (e) {
    setTimeout(function () {
      var newSelects = e.detail.target.querySelectorAll(
        "select.settings-select, select.dropdown-select, select.form-select, select.input-dropdown, select.ds-select, select.ds-input, select.ad-select, select[data-poool-dropdown], select.admin-select",
      );
      newSelects.forEach(function (selectEl) {
        if (
          selectEl.style.display === "none" &&
          selectEl.closest("[data-dropdown]")
        ) {
          return;
        }
        try {
          PooolDropdown.fromSelect(selectEl, {
            noLabel: true,
            searchable: selectEl.hasAttribute('data-searchable'),
            className: selectEl.classList.contains("admin-select")
              ? "poool-dropdown--sm poool-dropdown--inline"
              : sizeClassFor(selectEl),
          });
        } catch (err) {
          console.warn('Failed to init dropdown after HTMX swap:', err);
        }
      });
      PooolDropdown.init(e.detail.target);
    }, 50);
  });
})();
