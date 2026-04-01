/**
 * POOOL Dropdown Auto-Init
 * ========================
 * Automatically converts ALL native <select> elements on the page
 * to the unified POOOL custom dropdown design.
 *
 * Include this AFTER poool-dropdown.js:
 *   <script src="/static/js/poool-dropdown.js"></script>
 *   <script src="/static/js/poool-dropdown-init.js"></script>
 */
(function () {
  "use strict";

  function initAllDropdowns() {
    if (!window.PooolDropdown) {
      return;
    }

    // Convert all native <select> elements
    const selects = document.querySelectorAll(
      "select.settings-select, select.dropdown-select, select.form-select, select.input-dropdown, select.admin-select",
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
          className: selectEl.classList.contains("admin-select")
            ? "poool-dropdown--sm poool-dropdown--inline"
            : "",
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
        "select.settings-select, select.dropdown-select, select.form-select, select.input-dropdown, select.admin-select",
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
              : "",
          });
        } catch (err) {
          console.warn('Failed to init dropdown after HTMX swap:', err);
        }
      });
      PooolDropdown.init(e.detail.target);
    }, 50);
  });
})();
