/**
 * POOOL Dropdown Component
 * ========================
 * A custom dropdown that matches the POOOL website design.
 *
 * Usage:
 *   <div class="poool-dropdown" data-dropdown>
 *     <label class="poool-dropdown__label">Country</label>
 *     <button class="poool-dropdown__trigger" data-dropdown-trigger>
 *       <span class="poool-dropdown__icon">🌏</span>
 *       <span class="poool-dropdown__value poool-dropdown__value--placeholder">Select country...</span>
 *       <svg class="poool-dropdown__chevron" width="20" height="20" viewBox="0 0 20 20" fill="none">
 *         <path d="M5 7.5L10 12.5L15 7.5" stroke="currentColor" stroke-width="1.67" stroke-linecap="round" stroke-linejoin="round"/>
 *       </svg>
 *     </button>
 *     <div class="poool-dropdown__panel" data-dropdown-panel>
 *       <div class="poool-dropdown__option" data-value="au" data-icon="🇦🇺">
 *         <span class="poool-dropdown__option-icon">🇦🇺</span>
 *         <span class="poool-dropdown__option-text">Australia</span>
 *         <svg class="poool-dropdown__check" width="20" height="20" viewBox="0 0 20 20" fill="none">
 *           <path d="M16.6667 5L7.50001 14.1667L3.33334 10" stroke="currentColor" stroke-width="1.67" stroke-linecap="round" stroke-linejoin="round"/>
 *         </svg>
 *       </div>
 *     </div>
 *   </div>
 *
 * JS:
 *   PooolDropdown.init()  — auto-init all [data-dropdown] elements
 *   new PooolDropdown(el) — init a specific element
 */

(function () {
  "use strict";

  const CHEVRON_SVG = `<svg width="20" height="20" viewBox="0 0 20 20" fill="none"><path d="M5 7.5L10 12.5L15 7.5" stroke="currentColor" stroke-width="1.67" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

  const CHECK_SVG = `<svg width="20" height="20" viewBox="0 0 20 20" fill="none"><path d="M16.6667 5L7.50001 14.1667L3.33334 10" stroke="currentColor" stroke-width="1.67" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

  const SEARCH_SVG = `<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.33"/><path d="M10.5 10.5L13.5 13.5" stroke="currentColor" stroke-width="1.33" stroke-linecap="round"/></svg>`;

  class PooolDropdown {
    constructor(el, options = {}) {
      this.el = el;
      this.options = {
        searchable: el.hasAttribute("data-searchable"),
        placeholder: el.getAttribute("data-placeholder") || "Select...",
        onChange: options.onChange || null,
        ...options,
      };

      this.isOpen = false;
      this.selectedValue = null;
      this.selectedText = "";
      this.focusedIndex = -1;

      this._init();
    }

    _init() {
      this.trigger = this.el.querySelector("[data-dropdown-trigger]");
      this.panel = this.el.querySelector("[data-dropdown-panel]");
      this.valueEl = this.trigger?.querySelector(".poool-dropdown__value");
      this.optionEls = Array.from(
        this.panel?.querySelectorAll(".poool-dropdown__option") || [],
      );

      // Set initial placeholder
      if (this.valueEl && !this.selectedValue) {
        this.valueEl.textContent = this.options.placeholder;
        this.valueEl.classList.add("poool-dropdown__value--placeholder");
      }

      // Check for pre-selected option
      const preSelected = this.optionEls.find((opt) =>
        opt.classList.contains("poool-dropdown__option--selected"),
      );
      if (preSelected) {
        this._selectOption(preSelected, false);
      }

      // Add search if needed
      if (this.options.searchable && this.panel) {
        this._addSearch();
      }

      // Bind events
      this._bindEvents();

      // Mark as initialized
      this.el.setAttribute("data-dropdown-initialized", "");
      // Expose instance on the DOM element so external code can call setValue()
      this.el._pooolDropdown = this;
    }

    _addSearch() {
      const searchWrapper = document.createElement("div");
      searchWrapper.className = "poool-dropdown__search";
      searchWrapper.innerHTML = `
        <span class="poool-dropdown__search-icon">${SEARCH_SVG}</span>
        <input type="text" class="poool-dropdown__search-input" placeholder="Search..." autocomplete="off">
      `;
      this.panel.insertBefore(searchWrapper, this.panel.firstChild);

      this.searchInput = searchWrapper.querySelector(
        ".poool-dropdown__search-input",
      );
      this.searchInput.addEventListener("input", (e) =>
        this._filterOptions(e.target.value),
      );
      this.searchInput.addEventListener("keydown", (e) =>
        this._handleSearchKeydown(e),
      );
    }

    _bindEvents() {
      // Toggle on trigger click
      this.trigger?.addEventListener("click", (e) => {
        e.stopPropagation();
        this.toggle();
      });

      // Option clicks
      this.optionEls.forEach((opt) => {
        opt.addEventListener("click", (e) => {
          e.stopPropagation();
          if (!opt.classList.contains("poool-dropdown__option--disabled")) {
            this._selectOption(opt, true);
            this.close();
          }
        });

        opt.addEventListener("mouseenter", () => {
          this._clearFocus();
          opt.classList.add("poool-dropdown__option--focused");
        });

        opt.addEventListener("mouseleave", () => {
          opt.classList.remove("poool-dropdown__option--focused");
        });
      });

      // Close on outside click
      document.addEventListener("click", (e) => {
        if (this.isOpen && !this.el.contains(e.target)) {
          this.close();
        }
      });

      // Keyboard navigation
      this.trigger?.addEventListener("keydown", (e) => this._handleKeydown(e));
    }

    _handleKeydown(e) {
      switch (e.key) {
        case "Enter":
        case " ":
          e.preventDefault();
          this.toggle();
          break;
        case "ArrowDown":
          e.preventDefault();
          if (!this.isOpen) this.open();
          else this._focusNext();
          break;
        case "ArrowUp":
          e.preventDefault();
          if (!this.isOpen) this.open();
          else this._focusPrev();
          break;
        case "Escape":
          e.preventDefault();
          this.close();
          break;
        case "Tab":
          this.close();
          break;
      }
    }

    _handleSearchKeydown(e) {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          this._focusNext();
          break;
        case "ArrowUp":
          e.preventDefault();
          this._focusPrev();
          break;
        case "Enter":
          e.preventDefault();
          const focused = this.panel.querySelector(
            ".poool-dropdown__option--focused",
          );
          if (focused) {
            this._selectOption(focused, true);
            this.close();
          }
          break;
        case "Escape":
          e.preventDefault();
          this.close();
          break;
      }
    }

    _focusNext() {
      const visible = this._getVisibleOptions();
      if (visible.length === 0) return;
      this.focusedIndex = Math.min(this.focusedIndex + 1, visible.length - 1);
      this._updateFocus(visible);
    }

    _focusPrev() {
      const visible = this._getVisibleOptions();
      if (visible.length === 0) return;
      this.focusedIndex = Math.max(this.focusedIndex - 1, 0);
      this._updateFocus(visible);
    }

    _getVisibleOptions() {
      return this.optionEls.filter(
        (opt) =>
          !opt.classList.contains("poool-dropdown__option--disabled") &&
          opt.style.display !== "none",
      );
    }

    _clearFocus() {
      this.optionEls.forEach((opt) =>
        opt.classList.remove("poool-dropdown__option--focused"),
      );
    }

    _updateFocus(visible) {
      this._clearFocus();
      if (visible[this.focusedIndex]) {
        visible[this.focusedIndex].classList.add(
          "poool-dropdown__option--focused",
        );
        visible[this.focusedIndex].scrollIntoView({ block: "nearest" });
      }
    }

    _filterOptions(query) {
      const q = query.toLowerCase().trim();
      let hasVisible = false;

      this.optionEls.forEach((opt) => {
        const text =
          opt
            .querySelector(".poool-dropdown__option-text")
            ?.textContent?.toLowerCase() || "";
        const match = !q || text.includes(q);
        opt.style.display = match ? "" : "none";
        if (match) hasVisible = true;
      });

      // Show/hide "no results" message
      let emptyMsg = this.panel.querySelector(".poool-dropdown__empty");
      if (!hasVisible) {
        if (!emptyMsg) {
          emptyMsg = document.createElement("div");
          emptyMsg.className = "poool-dropdown__empty";
          emptyMsg.textContent = "No results found";
          this.panel.appendChild(emptyMsg);
        }
        emptyMsg.style.display = "";
      } else if (emptyMsg) {
        emptyMsg.style.display = "none";
      }

      this.focusedIndex = -1;
    }

    _selectOption(opt, fireEvent) {
      // Remove selected from all
      this.optionEls.forEach((o) =>
        o.classList.remove("poool-dropdown__option--selected"),
      );

      // Mark new selection
      opt.classList.add("poool-dropdown__option--selected");

      // Update value display
      this.selectedValue = opt.getAttribute("data-value");
      this.selectedText =
        opt.querySelector(".poool-dropdown__option-text")?.textContent || "";

      if (this.valueEl) {
        this.valueEl.textContent = this.selectedText;
        this.valueEl.classList.remove("poool-dropdown__value--placeholder");
        this.valueEl.classList.add("poool-dropdown__value--selected");
      }

      // Copy icon if present
      const optIcon = opt.querySelector(".poool-dropdown__option-icon");
      const triggerIcon = this.trigger.querySelector(".poool-dropdown__icon");
      if (optIcon && triggerIcon) {
        triggerIcon.innerHTML = optIcon.innerHTML;
      }

      // Fire change event
      if (fireEvent) {
        const event = new CustomEvent("dropdown:change", {
          detail: { value: this.selectedValue, text: this.selectedText },
          bubbles: true,
        });
        this.el.dispatchEvent(event);

        if (this.options.onChange) {
          this.options.onChange(this.selectedValue, this.selectedText);
        }
      }
    }

    toggle() {
      this.isOpen ? this.close() : this.open();
    }

    open() {
      if (this.isOpen) return;
      this.isOpen = true;
      this.trigger?.classList.add("poool-dropdown__trigger--active");
      this.panel?.classList.add("poool-dropdown__panel--open");
      this.el.classList.add("poool-dropdown--open");

      // Position check (open upward if near bottom)
      this._checkPosition();

      // Focus search if present
      if (this.searchInput) {
        setTimeout(() => this.searchInput.focus(), 50);
      }

      this.focusedIndex = -1;
    }

    close() {
      if (!this.isOpen) return;
      this.isOpen = false;
      this.trigger?.classList.remove("poool-dropdown__trigger--active");
      this.panel?.classList.remove("poool-dropdown__panel--open");
      this.el.classList.remove("poool-dropdown--open");

      // Reset search
      if (this.searchInput) {
        this.searchInput.value = "";
        this._filterOptions("");
      }

      this.focusedIndex = -1;
    }

    _checkPosition() {
      if (!this.panel) return;
      const rect = this.el.getBoundingClientRect();
      const panelHeight = 280; // max-height
      const spaceBelow = window.innerHeight - rect.bottom;

      if (spaceBelow < panelHeight && rect.top > panelHeight) {
        this.panel.classList.add("poool-dropdown__panel--up");
      } else {
        this.panel.classList.remove("poool-dropdown__panel--up");
      }
    }

    // Public API
    getValue() {
      return this.selectedValue;
    }

    setValue(value) {
      const opt = this.optionEls.find(
        (o) => o.getAttribute("data-value") === value,
      );
      if (opt) this._selectOption(opt, false);
    }

    reset() {
      this.optionEls.forEach((o) =>
        o.classList.remove("poool-dropdown__option--selected"),
      );
      this.selectedValue = null;
      this.selectedText = "";
      if (this.valueEl) {
        this.valueEl.textContent = this.options.placeholder;
        this.valueEl.classList.add("poool-dropdown__value--placeholder");
        this.valueEl.classList.remove("poool-dropdown__value--selected");
      }
    }

    destroy() {
      this.el.removeAttribute("data-dropdown-initialized");
    }

    // Static factory
    static init(container = document) {
      const dropdowns = container.querySelectorAll(
        "[data-dropdown]:not([data-dropdown-initialized])",
      );
      return Array.from(dropdowns).map((el) => new PooolDropdown(el));
    }

    /**
     * Create a dropdown from a native <select> element.
     * Replaces it with the custom POOOL dropdown while keeping the hidden <select> synced.
     */
    static fromSelect(selectEl, options = {}) {
      const wrapper = document.createElement("div");
      wrapper.className =
        "poool-dropdown" + (options.className ? " " + options.className : "");
      wrapper.setAttribute("data-dropdown", "");

      // Label
      const label = selectEl
        .closest(".settings-form-row, .filter-dropdown")
        ?.querySelector("label, .settings-form-row__label, .filter-label-text");
      if (label && !options.noLabel) {
        const labelEl = document.createElement("label");
        labelEl.className = "poool-dropdown__label";
        labelEl.textContent = label.textContent.replace("*", "").trim();
        if (label.textContent.includes("*")) {
          labelEl.classList.add("poool-dropdown__label--required");
        }
        wrapper.appendChild(labelEl);
      }

      // Icon from wrapper
      const existingIcon = selectEl
        .closest(".settings-select-wrapper, .dropdown-wrapper")
        ?.querySelector(".settings-select-wrapper__icon-left, .dropdown-icon");
      let iconContent = "";
      if (existingIcon) {
        if (existingIcon.tagName.toLowerCase() === "img") {
          // Prevent class conflicts by removing absolute positioning class from the clone
          const clonedIcon = existingIcon.cloneNode(true);
          clonedIcon.className = "";
          iconContent = clonedIcon.outerHTML;
        } else {
          iconContent = existingIcon.innerHTML || existingIcon.textContent;
        }
      }

      // Trigger
      const trigger = document.createElement("button");
      trigger.className = "poool-dropdown__trigger";
      trigger.type = "button";
      trigger.setAttribute("data-dropdown-trigger", "");

      if (iconContent) {
        trigger.innerHTML += `<span class="poool-dropdown__icon">${iconContent}</span>`;
      }

      const selectedOption = selectEl.options[selectEl.selectedIndex];
      const valueSpan = document.createElement("span");
      valueSpan.className =
        "poool-dropdown__value poool-dropdown__value--selected";
      valueSpan.textContent = selectedOption
        ? selectedOption.textContent
        : options.placeholder || "Select...";
      trigger.appendChild(valueSpan);
      trigger.innerHTML += `<span class="poool-dropdown__chevron">${CHEVRON_SVG}</span>`;

      wrapper.appendChild(trigger);

      // Panel
      const panel = document.createElement("div");
      panel.className = "poool-dropdown__panel";
      panel.setAttribute("data-dropdown-panel", "");

      Array.from(selectEl.options).forEach((opt) => {
        const optionEl = document.createElement("div");
        optionEl.className = "poool-dropdown__option";
        optionEl.setAttribute("data-value", opt.value);

        if (opt.selected) {
          optionEl.classList.add("poool-dropdown__option--selected");
        }

        const dataIcon =
          opt.getAttribute("data-icon") || opt.getAttribute("data-flag");
        const iconHtml = dataIcon
          ? `<span class="poool-dropdown__option-icon">${dataIcon}</span>`
          : "";

        optionEl.innerHTML = `
          ${iconHtml}
          <span class="poool-dropdown__option-text">${opt.textContent}</span>
          <span class="poool-dropdown__check">${CHECK_SVG}</span>
        `;

        panel.appendChild(optionEl);
      });

      wrapper.appendChild(panel);

      // Hide original select but keep it for form submission
      selectEl.style.display = "none";
      selectEl.parentNode.insertBefore(wrapper, selectEl);
      wrapper.appendChild(selectEl); // Move select inside wrapper

      // Init dropdown
      const dropdown = new PooolDropdown(wrapper, {
        ...options,
        onChange: (value, text) => {
          // Sync with hidden select
          selectEl.value = value;
          selectEl.dispatchEvent(new Event("change", { bubbles: true }));
          if (options.onChange) options.onChange(value, text);
        },
      });

      return dropdown;
    }
  }

  // Expose globally
  window.PooolDropdown = PooolDropdown;

  // Auto-init on DOM ready
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => PooolDropdown.init());
  } else {
    PooolDropdown.init();
  }
})();
