/**
 * Developer Asset Detail — Edit Mode & Pending Changes
 *
 * Layers on top of developer-asset-detail.js:
 * - Checks for pending change requests
 * - Enables inline edit mode
 * - Submits changes via PUT /api/developer/assets/:id
 * - Shows appropriate feedback based on edit mode (direct vs review)
 */

(function () {
  let isEditMode = false;
  let originalValues = {}; // snapshot of current values when entering edit mode
  let projectStatus = "draft"; // updated from API response

  // Editable field definitions: { key, element selector, type }
  const EDITABLE_FIELDS = [
    { key: "description", selector: "#asset-description", type: "textarea" },
    { key: "title", selector: "#asset-title-main", type: "text" },
  ];

  // Wait for the original script to finish loading the asset
  const origLoad = window.loadAsset;

  // Extend the loadAsset function to also load pending changes
  if (typeof origLoad === "function") {
    // We rely on the DOMContentLoaded in the original script
    // and hook into the renderAll cycle
  }

  document.addEventListener("DOMContentLoaded", () => {
    // Button handlers
    document.getElementById("btn-edit-mode")?.addEventListener("click", toggleEditMode);
    document.getElementById("btn-save-changes")?.addEventListener("click", saveChanges);
    document.getElementById("btn-cancel-edit")?.addEventListener("click", cancelEdit);
  });

  // Called by the original renderAll() via a MutationObserver
  // or we can use a polling approach — let's use MutationObserver on #asset-content
  const observer = new MutationObserver((mutations) => {
    for (const m of mutations) {
      if (m.type === "attributes" && m.attributeName === "style") {
        const el = document.getElementById("asset-content");
        if (el && el.style.display !== "none") {
          onContentRendered();
          observer.disconnect();
        }
      }
    }
  });

  const contentEl = document.getElementById("asset-content");
  if (contentEl) {
    observer.observe(contentEl, { attributes: true });
  }

  function onContentRendered() {
    // The original script has populated assetData (global)
    if (typeof assetData !== "undefined" && assetData) {
      projectStatus = assetData.project_status || "draft";

      // Show edit toolbar
      const toolbar = document.getElementById("edit-toolbar");
      if (toolbar) toolbar.style.display = "flex";

      // Check for pending changes
      loadPendingChanges();

      // Auto-enter edit mode if navigated with ?edit=1
      const params = new URLSearchParams(window.location.search);
      if (params.get("edit") === "1") {
        enterEditMode();
        const url = new URL(window.location.href);
        url.searchParams.delete("edit");
        history.replaceState(null, "", url.toString());
      }
    }
  }

  async function loadPendingChanges() {
    if (!assetId) return;
    try {
      const resp = await fetch(`/api/developer/assets/${assetId}/pending-changes`);
      if (!resp.ok) return;
      const data = await resp.json();

      if (data.pending) {
        showPendingBanner(data.pending);
      }
    } catch (e) {
      // Silently fail
    }
  }

  function showPendingBanner(pending) {
    const banner = document.getElementById("pending-changes-banner");
    if (!banner) return;

    const date = new Date(pending.created_at);
    const dateStr = date.toLocaleDateString("en-US", {
      month: "long", day: "numeric", year: "numeric",
      hour: "2-digit", minute: "2-digit",
    });

    const proposed = pending.proposed_values || {};
    const fieldCount = Object.keys(proposed).length;
    const fieldNames = Object.keys(proposed).map(k =>
      k.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase())
    ).join(", ");

    document.getElementById("pending-banner-detail").innerHTML =
      `${fieldCount} field${fieldCount !== 1 ? 's' : ''} submitted for review on ${dateStr}<br>` +
      `<span style="font-size:11px; opacity:0.7;">Changed: ${fieldNames}</span>`;

    banner.classList.add("visible");
  }

  function toggleEditMode() {
    if (isEditMode) {
      cancelEdit();
    } else {
      enterEditMode();
    }
  }

  function enterEditMode() {
    isEditMode = true;
    document.body.classList.add("edit-active");

    const editBtn = document.getElementById("btn-edit-mode");
    if (editBtn) {
      editBtn.classList.add("active");
      editBtn.innerHTML = `
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
        Editing...`;
    }
    document.getElementById("btn-save-changes")?.classList.add("visible");
    document.getElementById("btn-cancel-edit")?.classList.add("visible");

    // Snapshot current values
    originalValues = {};
    if (typeof assetData !== "undefined" && assetData) {
      originalValues = { ...assetData };
    }

    // Make fields editable
    makeFieldsEditable();

    // Show mode info
    if (projectStatus !== "draft" && projectStatus !== "submitted") {
      showToast("info", "You're editing a live asset. Changes will be submitted for admin review.");
    }
  }

  function cancelEdit() {
    isEditMode = false;
    document.body.classList.remove("edit-active");

    const editBtn = document.getElementById("btn-edit-mode");
    if (editBtn) {
      editBtn.classList.remove("active");
      editBtn.innerHTML = `
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
        Edit`;
    }
    document.getElementById("btn-save-changes")?.classList.remove("visible");
    document.getElementById("btn-cancel-edit")?.classList.remove("visible");

    // Restore original values and remove edit inputs
    removeEditableFields();
  }

  function makeFieldsEditable() {
    // Title (inline text)
    makeInlineEditable("asset-title-main", "title", "text");

    // Description (textarea)
    makeInlineEditable("asset-description", "description", "textarea");

    // Property type from the grid — find by label text
    // These are dynamically rendered, so we need to use the grid
    const overviewCard = document.querySelector("#panel-overview .ad-card");
    if (overviewCard) {
      // We'll use a simpler approach: show an edit panel
      showEditPanel();
    }
  }

  function makeInlineEditable(elementId, fieldKey, inputType) {
    const el = document.getElementById(elementId);
    if (!el) return;

    el.classList.add("editable-field", "edit-mode");
    el.style.cursor = "text";

    // Create click handler to switch to input
    el._editHandler = () => {
      if (!isEditMode || el.classList.contains("editing")) return;

      const currentValue = el.textContent.trim();
      el.dataset.originalValue = currentValue;

      if (inputType === "textarea") {
        const input = document.createElement("textarea");
        input.className = "edit-input";
        input.value = currentValue === "—" || currentValue === "No description provided." ? "" : currentValue;
        input.rows = 5;
        input.style.display = "block";
        el.innerHTML = "";
        el.appendChild(input);
        el.classList.add("editing");
        input.focus();

        input.addEventListener("blur", () => {
          const newVal = input.value.trim();
          el.classList.remove("editing");
          el.textContent = newVal || "No description provided.";
          el.dataset.newValue = newVal;
          el.classList.toggle("dirty", newVal !== el.dataset.originalValue);
        });
      } else {
        const input = document.createElement("input");
        input.type = "text";
        input.className = "edit-input";
        input.value = currentValue === "—" ? "" : currentValue;
        input.style.display = "block";
        el.innerHTML = "";
        el.appendChild(input);
        el.classList.add("editing");
        input.focus();

        input.addEventListener("blur", () => {
          const newVal = input.value.trim();
          el.classList.remove("editing");
          el.textContent = newVal || "—";
          el.dataset.newValue = newVal;
          el.classList.toggle("dirty", newVal !== el.dataset.originalValue);
        });

        input.addEventListener("keydown", (e) => {
          if (e.key === "Enter") input.blur();
          if (e.key === "Escape") {
            el.classList.remove("editing");
            el.textContent = el.dataset.originalValue;
          }
        });
      }
    };

    el.addEventListener("click", el._editHandler);
  }

  function showEditPanel() {
    // Add an edit panel below the overview tab for structured fields
    const existing = document.getElementById("edit-fields-panel");
    if (existing) existing.remove();

    const a = typeof assetData !== "undefined" ? assetData : {};
    const panel = document.createElement("div");
    panel.id = "edit-fields-panel";
    panel.className = "ad-card";
    panel.style.cssText = "padding:24px; margin-top:24px;";

    panel.innerHTML = `
      <h3 style="font-size:15px; font-weight:700; margin:0 0 20px; display:flex; align-items:center; gap:8px;">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
        Edit Asset Details
      </h3>
      <div style="display:grid; grid-template-columns: 1fr 1fr; gap:16px;">
        ${editField("short_description", "Short Description", a.short_description, "text")}
        ${editField("video_url", "Video URL", a.video_url, "url")}
        ${editField("location_address", "Address", a.location_address, "text")}
        ${editField("property_type", "Property Type", a.property_type, "text")}
        ${editField("area", "Area", a.area, "text")}
        ${editField("lease_type", "Lease Type", a.lease_type, "text")}
        ${editField("lease_term_years", "Lease Term (years)", a.lease_term_years, "number")}
        ${editField("construction_status", "Construction Status", a.construction_status, "text")}
        ${editField("bedrooms", "Bedrooms", a.bedrooms, "number")}
        ${editField("bathrooms", "Bathrooms", a.bathrooms, "number")}
        ${editField("year_built", "Year Built", a.year_built, "number")}
        ${editField("land_size_sqm", "Land Size (sqm)", a.land_size_sqm, "number")}
        ${editField("building_size_sqm", "Building Size (sqm)", a.building_size_sqm, "number")}
        ${editField("annual_yield_bps", "Annual Yield (bps)", a.annual_yield_bps, "number")}
        ${editField("capital_appreciation_bps", "Capital Appreciation (bps)", a.capital_appreciation_bps, "number")}
        ${editField("occupancy_rate_bps", "Occupancy Rate (bps)", a.occupancy_rate_bps, "number")}
      </div>
    `;

    // Insert after overview tab panel
    const overviewPanel = document.getElementById("panel-overview");
    if (overviewPanel) {
      overviewPanel.appendChild(panel);
    }
  }

  function editField(key, label, value, type) {
    const val = value != null ? value : "";
    return `
      <div style="display:flex; flex-direction:column; gap:4px;">
        <label for="edit-${key}" style="font-size:12px; font-weight:600; color:var(--label-color, #475467); text-transform:uppercase; letter-spacing:0.04em;">${label}</label>
        <input
          type="${type === 'number' ? 'number' : 'text'}"
          id="edit-${key}"
          data-field="${key}"
          class="edit-field-input"
          value="${String(val).replace(/"/g, '&quot;')}"
          placeholder="${label}"
          style="padding:8px 12px; border:1px solid var(--card-border-color, #e5e7eb); border-radius:6px; font-size:14px; font-family:inherit; background:var(--content-bg, #fafafa); color:var(--value-color, #101828);"
        />
      </div>
    `;
  }

  function removeEditableFields() {
    // Remove click handlers
    ["asset-title-main", "asset-description"].forEach((id) => {
      const el = document.getElementById(id);
      if (el && el._editHandler) {
        el.removeEventListener("click", el._editHandler);
        el.classList.remove("editable-field", "edit-mode", "editing", "dirty");
        // Restore original text if dirty
        if (el.dataset.originalValue && el.classList.contains("dirty")) {
          el.textContent = el.dataset.originalValue;
        }
      }
    });

    // Remove edit panel
    const panel = document.getElementById("edit-fields-panel");
    if (panel) panel.remove();
  }

  async function saveChanges() {
    if (!isEditMode) return;

    // Collect changed fields
    const changes = {};

    // Title
    const titleEl = document.getElementById("asset-title-main");
    if (titleEl && titleEl.dataset.newValue !== undefined && titleEl.dataset.newValue !== originalValues.title) {
      changes.title = titleEl.dataset.newValue;
    }

    // Description
    const descEl = document.getElementById("asset-description");
    if (descEl && descEl.dataset.newValue !== undefined) {
      const newDesc = descEl.dataset.newValue;
      const origDesc = originalValues.description || "";
      if (newDesc !== origDesc) {
        changes.description = newDesc;
      }
    }

    // Structured edit fields
    document.querySelectorAll(".edit-field-input").forEach((input) => {
      const key = input.dataset.field;
      let newVal = input.value.trim();
      const origVal = originalValues[key];

      if (!newVal && !origVal) return; // both empty, skip

      // Type conversion for numeric fields
      if (input.type === "number" && newVal) {
        newVal = Number(newVal);
        if (isNaN(newVal)) return;
      }

      // Compare (coerce to string for comparison)
      if (String(newVal) !== String(origVal || "")) {
        changes[key] = newVal || null;
      }
    });

    if (Object.keys(changes).length === 0) {
      showToast("info", "No changes detected.");
      return;
    }

    // Submit
    const saveBtn = document.getElementById("btn-save-changes");
    if (saveBtn) {
      saveBtn.disabled = true;
      saveBtn.textContent = "Saving...";
    }

    try {
      const resp = await fetch(`/api/developer/assets/${assetId}`, {
        method: "PUT",
        headers: { 
          "Content-Type": "application/json",
          "X-CSRF-Token": getCsrfToken(),
        },
        body: JSON.stringify(changes),
      });

      const data = await resp.json();
      if (!resp.ok) throw new Error(data.error || "Failed to save");

      cancelEdit();

      if (data.mode === "direct") {
        showToast("success", "✓ Changes saved successfully!");
        // Reload to see updates
        setTimeout(() => {
          if (typeof loadAsset === "function") loadAsset();
        }, 500);
      } else if (data.mode === "review") {
        showToast("warning", "Changes submitted for admin review. They'll be applied once approved.");
        // Show the pending banner
        loadPendingChanges();
      } else if (data.mode === "none") {
        showToast("info", "No changes were detected.");
      }
    } catch (err) {
      showToast("error", "Error: " + err.message);
    } finally {
      if (saveBtn) {
        saveBtn.disabled = false;
        saveBtn.textContent = "Save Changes";
      }
    }
  }

  function showToast(type, msg) {
  if(window.showPooolToast) {
    window.showPooolToast(null, msg, type);
  }
}
})();
