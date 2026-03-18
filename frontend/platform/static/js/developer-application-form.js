// Application Form JavaScript – Wired to POST /api/developer/draft

/** Read the CSRF token from the cookie (shared helper). */
/**
 * Find the PooolDropdown instance that wraps a given native <select> element.
 * The poool-dropdown-init.js moves the <select> inside the wrapper div,
 * so we look for it there.
 */
function getPooolDropdownForSelect(selectId) {
  var el = document.getElementById(selectId);
  if (!el) return null;
  // After fromSelect(), the select is moved inside [data-dropdown]
  var wrapper = el.closest('[data-dropdown]');
  if (!wrapper || !wrapper._pooolDropdown) return null;
  return wrapper._pooolDropdown;
}

/**
 * Set value on a PooolDropdown that wraps the given native select.
 * Must be called AFTER the dropdown is initialized.
 */
function setDropdownVal(selectId, value) {
  if (!value) return;
  // Small retry loop — poool-dropdown-init runs 100ms after DOMContentLoaded
  // so we may need to wait for the custom dropdown to be ready.
  function trySet(attemptsLeft) {
    var el = document.getElementById(selectId);
    if (!el) return;
    var wrapper = el.closest('[data-dropdown]');
    if (wrapper && wrapper._pooolDropdown) {
      wrapper._pooolDropdown.setValue(value);
    } else if (attemptsLeft > 0) {
      setTimeout(function() { trySet(attemptsLeft - 1); }, 80);
    }
  }
  trySet(5);
}

/**
 * Read value from a PooolDropdown (or fall back to native select).
 */
function getDropdownVal(selectId) {
  var el = document.getElementById(selectId);
  if (!el) return '';
  var wrapper = el.closest('[data-dropdown]');
  if (wrapper && wrapper._pooolDropdown) {
    return wrapper._pooolDropdown.getValue() || '';
  }
  return el.value || '';
}

/**
 * Save & Exit — saves whatever the user has filled in so far (no full validation)
 * then navigates to the submissions list.
 */
async function saveAndExitStep2(btn) {
  const originalText = btn.textContent.trim();
  btn.disabled = true;
  btn.textContent = 'Saving…';

  const getVal = (id) => (document.getElementById(id) || {}).value || '';
  const getInt = (id) => { const v = parseInt(document.getElementById(id)?.value); return isNaN(v) ? null : v; };
  const getFloat = (id) => { const v = parseFloat(document.getElementById(id)?.value); return isNaN(v) ? null : v; };

  const assetType = window.selectedAssetType || localStorage.getItem('selectedAssetType') || 'real_estate';
  const assetTypeMap = {
    'real-estate': 'real_estate', 'real_estate': 'real_estate',
    'commercial-property': 'commercial_property', 'commercial_property': 'commercial_property',
    'commodity': 'commodity', 'commodities': 'commodity',
    'business': 'business',
    'startup': 'startup', 'startups': 'startup',
    'land-plot': 'land_plot', 'land-plots': 'land_plot', 'land_plot': 'land_plot',
  };

  const purchasePrice = parseFloat(getVal('purchase-price').replace(/[^0-9.]/g, '')) || 0;
  let tokenPrice = parseFloat(getVal('minimum-share-price').replace(/[^0-9.]/g, '')) || 500;
  if (tokenPrice < 500) tokenPrice = 500; // enforce minimum
  const totalValueCents = Math.round(purchasePrice * 100);
  const tokenPriceCents = Math.round(tokenPrice * 100);

  // Build payload — always include required fields for POST (CreateDraftAsset)
  // Required by backend: title, asset_type, total_value_cents, token_price_cents, tokens_total
  const existingId = localStorage.getItem('draft_asset_id');
  const title = getVal('property-name');

  const payload = {
    // Always include required fields with defaults for new drafts (POST)
    title: title || 'Untitled Draft',
    asset_type: assetTypeMap[assetType] || 'real_estate',
    total_value_cents: totalValueCents || 0,
    token_price_cents: tokenPriceCents || 50000,
    tokens_total: tokenPriceCents > 0 ? (Math.ceil(totalValueCents / tokenPriceCents) || 1) : 1,
  };

  // For PUT (update), we can strip out fields with no value — backend handles optionals
  // For POST (create), we already have defaults above
  const fields = [
    ['property_type', getDropdownVal('property-type')],
    ['area', getDropdownVal('area')],
    ['address', getVal('address')],
    ['city', getVal('city')],
    ['country', getVal('country')],
    ['lease_type', getDropdownVal('lease-type')],
    ['construction_status', getDropdownVal('status')],
  ];
  fields.forEach(([key, val]) => { if (val) payload[key] = val; });
  const numFields = [
    ['lease_term_years', getInt('lease-term')],
    ['land_size_sqm', getFloat('land-size')],
    ['building_size_sqm', getFloat('building-size')],
    ['bedrooms', getInt('bedrooms')],
    ['bathrooms', getInt('bathrooms')],
    ['year_built', getInt('year-built')],
  ];
  numFields.forEach(([key, val]) => { if (val !== null) payload[key] = val; });

  try {
    let url = existingId ? `/api/developer/draft/${existingId}` : '/api/developer/draft';
    let method = existingId ? 'PUT' : 'POST';

    let resp = await fetch(url, {
      method,
      headers: { 
        'Content-Type': 'application/json',
        'X-CSRF-Token': getCsrfToken()
      },
      body: JSON.stringify(payload),
    });

    // If PUT failed (stale draft ID), clear it and fall back to POST
    if (!resp.ok && existingId) {
      console.warn('Save & Exit: PUT failed (' + resp.status + '), falling back to POST (new draft)');
      localStorage.removeItem('draft_asset_id');
      resp = await fetch('/api/developer/draft', {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
          'X-CSRF-Token': getCsrfToken()
        },
        body: JSON.stringify(payload),
      });
    }

    if (resp.ok) {
      const text = await resp.text();
      try {
        const data = JSON.parse(text);
        const savedId = data.asset_id;
        if (savedId) localStorage.setItem('draft_asset_id', savedId);
      } catch { /* non-JSON success response is fine */ }
    } else {
      const rawText = await resp.text().catch(() => '');
      let msg = `Save failed (${resp.status})`;
      try { const j = JSON.parse(rawText); msg = j.error || j.message || msg; } catch {}
      console.warn('Save & Exit: server error:', msg);
    }
  } catch (err) {
    console.warn('Save & Exit: network error', err);
  } finally {
    window.location.href = '/developer/submissions';
  }
}

document.addEventListener("DOMContentLoaded", function () {
  const urlParams = new URLSearchParams(window.location.search);
  const urlDraftId = urlParams.get('draft_id');
  if (urlDraftId) {
    localStorage.setItem("draft_asset_id", urlDraftId);
  }

  const existingDraftId = localStorage.getItem("draft_asset_id");
  if (existingDraftId) {
    fetch(`/api/developer/draft/${existingDraftId}`)
        .then(r => {
            if (!r.ok) {
              // Stale draft ID — clear it so next save creates a fresh draft
              console.warn('Draft ' + existingDraftId + ' not found (status ' + r.status + '), clearing stale ID');
              localStorage.removeItem('draft_asset_id');
              return null;
            }
            return r.json();
        })
        .then(data => {
            if (!data) return;
            const setVal = (id, val) => {
              const el = document.getElementById(id);
              if (el && val !== undefined && val !== null) {
                el.value = String(val);
                // Also sync PooolDropdown custom UI if it replaced this select
                setDropdownVal(id, String(val));
              }
            };
            setVal("property-name", data.title);
            setVal("property-type", data.property_type);
            setVal("area", data.area);
            setVal("address", data.location_address);
            setVal("city", data.city);
            setVal("country", data.country);
            setVal("lease-type", data.lease_type);
            setVal("lease-term", data.lease_term_years);
            setVal("land-size", data.land_size_sqm);
            setVal("building-size", data.building_size_sqm);
            setVal("bedrooms", data.bedrooms);
            setVal("bathrooms", data.bathrooms);
            setVal("status", data.construction_status);
            setVal("year-built", data.year_built);
            if (data.total_value_cents) {
              var ppVal = Math.round(data.total_value_cents / 100);
              document.getElementById('purchase-price') && (document.getElementById('purchase-price').value = ppVal.toLocaleString('en-US'));
            }
            if (data.token_price_cents) {
              var spVal = Math.round(data.token_price_cents / 100);
              document.getElementById('minimum-share-price') && (document.getElementById('minimum-share-price').value = spVal.toLocaleString('en-US'));
            }
        })
        .catch(console.error);
  }

  // ─── Next Step: Collect form fields and POST or PUT to backend ───
  const nextBtn = document.getElementById("form-next-btn");
  if (nextBtn) {
    nextBtn.addEventListener("click", async function (e) {
      e.preventDefault();

      // Clear all previous errors
      clearAllFieldErrors();
      dismissFormError();

      // Read the asset type from step 1 (stored in localStorage / window)
      const assetType = window.selectedAssetType
        || localStorage.getItem("selectedAssetType")
        || "real_estate";

      // Collect form fields
      const getVal = (id) => (document.getElementById(id) || {}).value || "";
      const getInt = (id) => parseInt(document.getElementById(id)?.value);
      const getFloat = (id) => parseFloat(document.getElementById(id)?.value);

      const title = getVal("property-name");
      const propertyType = getDropdownVal("property-type");
      const area = getDropdownVal("area");
      const address = getVal("address");
      const city = getVal("city");
      const country = getVal("country");
      const leaseType = getDropdownVal("lease-type");
      const leaseTerm = getInt("lease-term");
      const landSize = getFloat("land-size");
      const buildingSize = getFloat("building-size");
      const bedrooms = getInt("bedrooms");
      const bathrooms = getInt("bathrooms");
      const status = getDropdownVal("status");
      const yearBuilt = getInt("year-built");

      // Financials
      const purchasePrice = parseFloat(getVal("purchase-price").replace(/[^0-9.]/g, "")) || 0;
      let tokenPrice = parseFloat(getVal("minimum-share-price").replace(/[^0-9.]/g, "")) || 500;
      if (tokenPrice < 500) tokenPrice = 500; // enforce $500 minimum

      // Convert dollars to cents
      const totalValueCents = Math.round(purchasePrice * 100);
      const tokenPriceCents = Math.round(tokenPrice * 100);
      const tokensTotal = tokenPriceCents > 0
        ? Math.ceil(totalValueCents / tokenPriceCents)
        : 1000;

      // ─── Validate required fields ───
      let hasErrors = false;

      if (!title.trim()) {
        showFieldError("property-name", "Property name is required");
        hasErrors = true;
      }

      if (!propertyType) {
        showFieldError("property-type", "Please select a property type");
        hasErrors = true;
      }

      if (!area) {
        showFieldError("area", "Please select an area");
        hasErrors = true;
      }

      if (!address || !address.trim()) {
        showFieldError("address", "Address is required");
        hasErrors = true;
      }

      if (!leaseType) {
        showFieldError("lease-type", "Please select a lease type");
        hasErrors = true;
      }

      if (isNaN(leaseTerm) || leaseTerm <= 0) {
        showFieldError("lease-term", "Please enter lease term");
        hasErrors = true;
      }

      if (isNaN(landSize) || landSize <= 0) {
        showFieldError("land-size", "Please enter land size");
        hasErrors = true;
      }

      if (isNaN(buildingSize) || buildingSize <= 0) {
        showFieldError("building-size", "Please enter building size");
        hasErrors = true;
      }

      if (isNaN(bedrooms) || bedrooms < 0) {
        showFieldError("bedrooms", "Bedrooms required");
        hasErrors = true;
      }

      if (isNaN(bathrooms) || bathrooms < 0) {
        showFieldError("bathrooms", "Bathrooms required");
        hasErrors = true;
      }

      if (!status) {
        showFieldError("status", "Please select status");
        hasErrors = true;
      }

      if (isNaN(yearBuilt) || yearBuilt < 1800 || yearBuilt > 2100) {
        showFieldError("year-built", "Please enter a valid year");
        hasErrors = true;
      }

      if (totalValueCents <= 0) {
        showFieldError("purchase-price", "Please enter a valid purchase price");
        hasErrors = true;
      }

      if (tokenPriceCents < 50000) {
        showFieldError("minimum-share-price", "Minimum share price is $500");
        hasErrors = true;
      }

      if (hasErrors) {
        // Scroll to first error
        const firstErrorEl = document.querySelector('.field-error-active');
        if (firstErrorEl) {
          firstErrorEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
        // Removed showFormError as requested to prevent error below the card
        return;
      }

      // Disable button to prevent double-submit
      nextBtn.disabled = true;
      nextBtn.querySelector("span").textContent = "Saving…";

      const assetTypeMap = {
        "real-estate": "real_estate",
        "real_estate": "real_estate",
        "commercial-property": "commercial_property",
        "commercial_property": "commercial_property",
        "commodity": "commodity",
        "commodities": "commodity",
        "business": "business",
        "startup": "startup",
        "startups": "startup",
        "land-plot": "land_plot",
        "land-plots": "land_plot",
        "land_plot": "land_plot",
      };

      const payload = {
        title: title,
        asset_type: assetTypeMap[assetType] || "real_estate",
        property_type: propertyType || null,
        area: area || null,
        address: address || null,
        city: city || null,
        country: country || null,
        lease_type: leaseType || null,
        lease_term_years: leaseTerm,
        land_size_sqm: landSize,
        building_size_sqm: buildingSize,
        bedrooms: bedrooms,
        bathrooms: bathrooms,
        construction_status: status || null,
        year_built: yearBuilt,
        total_value_cents: totalValueCents,
        token_price_cents: tokenPriceCents,
        tokens_total: tokensTotal,
      };

      try {
        const existingId = localStorage.getItem("draft_asset_id");
        let url = existingId ? `/api/developer/draft/${existingId}` : "/api/developer/draft";
        let method = existingId ? "PUT" : "POST";

        let resp = await fetch(url, {
          method: method,
          headers: { 
            "Content-Type": "application/json",
            "X-CSRF-Token": getCsrfToken()
          },
          body: JSON.stringify(payload),
        });

        // If PUT failed (stale/deleted draft), clear ID and fall back to POST
        if (!resp.ok && existingId) {
          console.warn('Next Step: PUT failed (' + resp.status + '), falling back to POST (new draft)');
          localStorage.removeItem('draft_asset_id');
          resp = await fetch('/api/developer/draft', {
            method: 'POST',
            headers: { 
              'Content-Type': 'application/json',
              'X-CSRF-Token': getCsrfToken()
            },
            body: JSON.stringify(payload),
          });
        }

        if (resp.ok) {
          const rawText = await resp.text();
          let data = {};
          try { data = JSON.parse(rawText); } catch { /* ok */ }
          const savedId = data.asset_id || existingId;
          if (savedId) localStorage.setItem("draft_asset_id", savedId);
          window.location.href = "/developer/document-upload-step3?draft_id=" + savedId;
        } else {
          // Read body once as text, then try to extract a message from it
          let errMessage = "Something went wrong. Please try again.";
          try {
            const rawText = await resp.text();
            // Try JSON first (our API routes return JSON errors when possible)
            try {
              const errData = JSON.parse(rawText);
              if (errData.error) errMessage = errData.error;
              else if (errData.message) errMessage = errData.message;
            } catch {
              // Plain text or HTML error (AppError returns HTML) — extract text content
              const stripped = rawText.replace(/<[^>]+>/g, '').trim();
              if (stripped && stripped.length < 300) errMessage = stripped;
            }
          } catch { /* body read failed */ }
          if (resp.status === 403) errMessage = "Session expired — please refresh the page and try again.";
          if (resp.status === 401) errMessage = "You are not logged in. Please log in and try again.";
          showFormError(errMessage);
          nextBtn.disabled = false;
          nextBtn.querySelector("span").textContent = "Next Step";
        }
      } catch (err) {
        // True network error (server unreachable, connection dropped, etc.)
        showFormError("Connection lost — please check your internet and try again.");
        console.error('Network error on draft save:', err);
        nextBtn.disabled = false;
        nextBtn.querySelector("span").textContent = "Next Step";
      }
    });
  }

  // ─── File Upload Functionality ───
  const fileInput = document.getElementById("file-input");
  const uploadArea = document.querySelector(".file-upload-area");
  const dragOverlay = document.querySelector(".drag-overlay");

  if (fileInput) {
    fileInput.addEventListener("change", function (e) {
      handleFiles(e.target.files);
    });
  }

  if (uploadArea) {
    uploadArea.addEventListener("dragover", function (e) {
      e.preventDefault();
      e.stopPropagation();
      dragOverlay.style.display = "flex";
    });

    uploadArea.addEventListener("dragleave", function (e) {
      e.preventDefault();
      e.stopPropagation();
      dragOverlay.style.display = "none";
    });

    uploadArea.addEventListener("drop", function (e) {
      e.preventDefault();
      e.stopPropagation();
      dragOverlay.style.display = "none";
      handleFiles(e.dataTransfer.files);
    });
  }

  // ─── Area conversion (m² ↔ sqft) ───
  const totalAreaInput = document.getElementById("total-area");
  const totalAreaSqftInput = document.getElementById("total-area-sqft");
  const buildingAreaInput = document.getElementById("building-area");
  const buildingAreaSqftInput = document.getElementById("building-area-sqft");

  if (totalAreaInput && totalAreaSqftInput) {
    totalAreaInput.addEventListener("input", function () {
      const m2 = parseFloat(this.value) || 0;
      totalAreaSqftInput.value = Math.round(m2 * 10.764).toString().padStart(3, "0");
    });
    totalAreaSqftInput.addEventListener("input", function () {
      const sqft = parseFloat(this.value) || 0;
      totalAreaInput.value = Math.round(sqft / 10.764);
    });
  }

  if (buildingAreaInput && buildingAreaSqftInput) {
    buildingAreaInput.addEventListener("input", function () {
      const m2 = parseFloat(this.value) || 0;
      buildingAreaSqftInput.value = Math.round(m2 * 10.764).toString().padStart(3, "0");
    });
    buildingAreaSqftInput.addEventListener("input", function () {
      const sqft = parseFloat(this.value) || 0;
      buildingAreaInput.value = Math.round(sqft / 10.764);
    });
  }

  // ─── Clear field errors on input ───
  document.querySelectorAll('input, select, textarea').forEach(function (el) {
    el.addEventListener('input', function () {
      clearFieldError(this.id);
    });
    el.addEventListener('change', function () {
      clearFieldError(this.id);
    });
  });

  // ─── Currency / number formatting ────────────────────────────────────
  // Format a number string with thousand separators (e.g. 12000000 → 12,000,000)
  function formatWithCommas(rawStr) {
    var num = rawStr.replace(/[^0-9]/g, '');
    if (!num) return '';
    return parseInt(num, 10).toLocaleString('en-US');
  }

  // Strip formatting back to plain integer string
  function stripFormatting(str) {
    return str.replace(/[^0-9]/g, '');
  }

  // Attach live formatting to all currency inputs
  document.querySelectorAll('[data-format-currency]').forEach(function (el) {
    el.addEventListener('input', function () {
      var raw = stripFormatting(this.value);
      var cursor = this.selectionStart;
      var oldLen = this.value.length;
      this.value = raw ? parseInt(raw, 10).toLocaleString('en-US') : '';
      // Restore cursor roughly
      var newLen = this.value.length;
      this.setSelectionRange(cursor + (newLen - oldLen), cursor + (newLen - oldLen));
    });

    // On blur: validate minimum for share price
    if (el.id === 'minimum-share-price') {
      el.addEventListener('blur', function () {
        var raw = parseInt(stripFormatting(this.value), 10) || 0;
        if (raw < 500 && raw > 0) {
          this.value = '500';
          showFieldError('minimum-share-price', 'Minimum share price is $500');
        } else if (raw === 0) {
          this.value = '500';
        }
        if (this.value && !this.value.includes(',')) {
          var n = parseInt(stripFormatting(this.value), 10);
          if (!isNaN(n)) this.value = n.toLocaleString('en-US');
        }
      });
    }
  });

  // Format any pre-filled value on load
  document.querySelectorAll('[data-format-currency]').forEach(function (el) {
    if (el.value) {
      var raw = stripFormatting(el.value);
      if (raw) el.value = parseInt(raw, 10).toLocaleString('en-US');
    }
  });
});


// ═══════════════════════════════════════════════════════
//  Error Display System
// ═══════════════════════════════════════════════════════

// ─── XSS-safe HTML escaper for form messages ───────────────────────
function escFormHtml(str) {
  if (typeof str !== 'string') return String(str);
  var d = document.createElement('div');
  d.appendChild(document.createTextNode(str));
  return d.innerHTML;
}

/**
 * Show a per-field inline error (red border + helper text below the field)
 */
function showFieldError(fieldId, message) {
  var field = document.getElementById(fieldId);
  if (!field) return;

  field.classList.add('field-error-active');
  
  // If it's a hidden select inside a custom dropdown, highlight the dropdown trigger
  var customDropdown = field.closest('.poool-dropdown');
  if (customDropdown) {
    var trigger = customDropdown.querySelector('.poool-dropdown__trigger');
    if (trigger) {
      trigger.style.borderColor = '#F04438';
      trigger.style.boxShadow = '0 0 0 3px rgba(240, 68, 56, 0.1)';
      trigger.classList.add('field-error-active');
    }
  } else {
    field.style.borderColor = '#F04438';
    field.style.boxShadow = '0 0 0 3px rgba(240, 68, 56, 0.1)';
  }

  // Check if error message already exists
  // For custom dropdowns, we want the error below the dropdown wrapper
  var parent = customDropdown ? customDropdown.parentElement : (field.closest('.form-input-wrapper') || field.closest('.form-field') || field.parentElement);
  var existingMsg = parent.querySelector('.field-error-msg');
  if (existingMsg) existingMsg.remove();

  var errorMsg = document.createElement('span');
  errorMsg.className = 'field-error-msg';
  errorMsg.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#F04438" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="flex-shrink:0"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg> ' + escFormHtml(message);
  errorMsg.style.cssText = 'display:flex;align-items:center;gap:6px;color:#F04438;font-size:13px;font-weight:500;margin-top:6px;animation:fadeIn 0.2s ease';

  parent.appendChild(errorMsg);
}

/**
 * Clear error state from a specific field
 */
function clearFieldError(fieldId) {
  var field = document.getElementById(fieldId);
  if (!field) return;

  field.classList.remove('field-error-active');
  field.style.borderColor = '';
  field.style.boxShadow = '';

  var customDropdown = field.closest('.poool-dropdown');
  if (customDropdown) {
    var trigger = customDropdown.querySelector('.poool-dropdown__trigger');
    if (trigger) {
      trigger.style.borderColor = '';
      trigger.style.boxShadow = '';
      trigger.classList.remove('field-error-active');
    }
  }

  var parent = customDropdown ? customDropdown.parentElement : (field.closest('.form-input-wrapper') || field.closest('.form-field') || field.parentElement);
  if (parent) {
    var errorMsg = parent.querySelector('.field-error-msg');
    if (errorMsg) errorMsg.remove();
  }
}

/**
 * Clear all field-level errors
 */
function clearAllFieldErrors() {
  document.querySelectorAll('.field-error-active').forEach(function (el) {
    el.classList.remove('field-error-active');
    el.style.borderColor = '';
    el.style.boxShadow = '';
  });
  document.querySelectorAll('.field-error-msg').forEach(function (el) { el.remove(); });
}

/**
 * Show a top-level form error banner (toast-style)
 */
function showFormError(message) {
  dismissFormError();

  var errorDiv = document.createElement("div");
  errorDiv.id = "form-api-error";
  errorDiv.className = "form-error-toast";
  errorDiv.innerHTML =
    '<div class="form-error-toast__content">' +
      '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#F04438" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
        '<circle cx="12" cy="12" r="10"/>' +
        '<line x1="12" y1="8" x2="12" y2="12"/>' +
        '<line x1="12" y1="16" x2="12.01" y2="16"/>' +
      '</svg>' +
      '<span>' + escFormHtml(message) + '</span>' +
    '</div>' +
    '<button class="form-error-toast__close" onclick="dismissFormError()" aria-label="Dismiss">' +
      '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#F04438" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
        '<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>' +
      '</svg>' +
    '</button>';

  errorDiv.style.cssText =
    'display:flex;align-items:center;justify-content:space-between;gap:12px;' +
    'background:#FEF3F2;border:1px solid #FEE4E2;border-radius:12px;' +
    'padding:14px 20px;margin:0 0 20px;' +
    'animation:slideDown 0.3s ease;';

  var formActions = document.getElementById("form-actions");
  if (formActions) {
    formActions.parentNode.insertBefore(errorDiv, formActions);
  }

  setTimeout(function () { dismissFormError(); }, 8000);
}

/**
 * Dismiss the form error banner
 */
function dismissFormError() {
  var existing = document.getElementById("form-api-error");
  if (existing) {
    existing.style.animation = 'fadeOut 0.2s ease';
    setTimeout(function () { if (existing.parentNode) existing.remove(); }, 200);
  }
}

/**
 * Show an inline toast notification (replaces alert() calls)
 */
function showToast(message, type) {
  type = type || 'error';
  var colors = {
    error:   { bg: '#FEF3F2', border: '#FEE4E2', text: '#B42318', icon: '#F04438' },
    warning: { bg: '#FFFAEB', border: '#FEDF89', text: '#B54708', icon: '#F79009' },
    success: { bg: '#ECFDF3', border: '#ABEFC6', text: '#027A48', icon: '#12B76A' },
  };
  var c = colors[type] || colors.error;

  var existing = document.getElementById('file-toast');
  if (existing) existing.remove();

  var toast = document.createElement('div');
  toast.id = 'file-toast';
  toast.style.cssText =
    'position:fixed;top:24px;right:24px;z-index:10000;' +
    'display:flex;align-items:center;gap:10px;' +
    'background:' + c.bg + ';border:1px solid ' + c.border + ';' +
    'border-radius:12px;padding:12px 20px;' +
    'box-shadow:0 4px 12px rgba(0,0,0,0.1);' +
    'font-size:14px;font-weight:500;color:' + c.text + ';' +
    'animation:slideDown 0.3s ease;max-width:400px;';
  toast.innerHTML =
    '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="' + c.icon + '" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
      '<circle cx="12" cy="12" r="10"/>' +
      '<line x1="12" y1="8" x2="12" y2="12"/>' +
      '<line x1="12" y1="16" x2="12.01" y2="16"/>' +
    '</svg>' +
    '<span>' + escFormHtml(message) + '</span>';

  document.body.appendChild(toast);
  setTimeout(function () {
    toast.style.animation = 'fadeOut 0.3s ease';
    setTimeout(function () { if (toast.parentNode) toast.remove(); }, 300);
  }, 4000);
}

// ─── File Upload Handlers ───
function handleFiles(files) {
  var filesList = document.getElementById("uploaded-files-list");

  for (var i = 0; i < files.length; i++) {
    var file = files[i];

    if (file.size > 20 * 1024 * 1024) {
      showToast('"' + file.name + '" is too large (max 20MB)', 'error');
      continue;
    }

    var allowedTypes = [
      "image/webp", "image/jpg", "image/jpeg", "image/png",
      "application/pdf",
      "application/msword",
      "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
      "application/zip",
      "video/mp4",
    ];

    if (allowedTypes.indexOf(file.type) === -1) {
      showToast('"' + file.name + '" — unsupported format. Use images, PDF, DOC, or ZIP.', 'warning');
      continue;
    }

    addFileToList(file);
  }
}

function addFileToList(file) {
  var filesList = document.getElementById("uploaded-files-list");
  var fileId = "file-" + Date.now() + "-" + Math.random().toString(36).substr(2, 9);
  var fileExt = file.name.split(".").pop().toUpperCase();
  var fileSize = formatFileSize(file.size);

  var fileItemHTML =
    '<div id="' + fileId + '" class="file-upload-item">' +
      '<div class="file-content">' +
        '<div class="file-type-icon">' +
          '<div class="file-icon-page">' +
            '<div class="file-icon-earmark"></div>' +
            '<div class="file-type-badge">' + fileExt + '</div>' +
          '</div>' +
        '</div>' +
        '<div class="file-info">' +
          '<div class="file-details">' +
            '<span class="file-name">' + escFormHtml(file.name) + '</span>' +
            '<span class="file-size">' + fileSize + '</span>' +
          '</div>' +
          '<div class="file-progress">' +
            '<div class="ds-progress ds-progress--sm">' +
              '<div class="ds-progress__fill" style="width: 0%"></div>' +
            '</div>' +
            '<span class="progress-percentage">0%</span>' +
          '</div>' +
        '</div>' +
      '</div>' +
      '<button type="button" class="file-delete-btn" onclick="removeFile(\'' + fileId + '\')">' +
        '<svg width="16" height="16" viewBox="0 0 16 16" fill="none">' +
          '<path d="M2 4H14M5.33333 4V2.66667C5.33333 2.31305 5.47381 1.97391 5.72386 1.72386C5.97391 1.47381 6.31305 1.33333 6.66667 1.33333H9.33333C9.68696 1.33333 10.0261 1.47381 10.2761 1.72386C10.5262 1.97391 10.6667 2.31305 10.6667 2.66667V4M6.66667 7.33333V11.3333M9.33333 7.33333V11.3333M12.6667 4V12.6667C12.6667 13.0203 12.5262 13.3594 12.2761 13.6095C12.0261 13.8595 11.687 14 11.3333 14H4.66667C4.31305 14 3.97391 13.8595 3.72386 13.6095C3.47381 13.3594 3.33333 13.0203 3.33333 12.6667V4" stroke="#A4A7AE" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>' +
        '</svg>' +
      '</button>' +
    '</div>';

  var tempDiv = document.createElement("div");
  tempDiv.innerHTML = fileItemHTML;
  var fileItem = tempDiv.firstElementChild;
  filesList.appendChild(fileItem);
  simulateUpload(fileId);
}

function simulateUpload(fileId) {
  var fileItem = document.getElementById(fileId);
  var progressFill = fileItem.querySelector(".ds-progress__fill");
  var progressPercentage = fileItem.querySelector(".progress-percentage");

  var progress = 0;
  var interval = setInterval(function () {
    progress += Math.random() * 20;
    if (progress > 100) progress = 100;

    progressFill.style.width = progress + "%";
    progressPercentage.textContent = Math.round(progress) + "%";

    if (progress >= 100) {
      clearInterval(interval);
      setTimeout(function () {
        var progressContainer = fileItem.querySelector(".file-progress");
        if (progressContainer) progressContainer.style.display = "none";
      }, 500);
    }
  }, 200);
}

function removeFile(fileId) {
  var fileItem = document.getElementById(fileId);
  if (fileItem) fileItem.remove();
}

function formatFileSize(bytes) {
  if (bytes === 0) return "0 Bytes";
  var k = 1024;
  var sizes = ["Bytes", "KB", "MB", "GB"];
  var i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
}

// ─── CSS Animations (injected once) ───
if (!document.getElementById('form-error-animations')) {
  var style = document.createElement('style');
  style.id = 'form-error-animations';
  style.textContent =
    '@keyframes slideDown { from { opacity: 0; transform: translateY(-10px); } to { opacity: 1; transform: translateY(0); } }' +
    '@keyframes fadeOut { from { opacity: 1; } to { opacity: 0; } }' +
    '@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }' +
    '.form-error-toast__content { display:flex;align-items:center;gap:10px;font-size:14px;font-weight:500;color:#B42318; }' +
    '.form-error-toast__close { background:none;border:none;cursor:pointer;padding:4px;border-radius:6px;display:flex;align-items:center;justify-content:center;opacity:0.6;transition:opacity 0.2s; }' +
    '.form-error-toast__close:hover { opacity:1;background:rgba(240,68,56,0.08); }';
  document.head.appendChild(style);
}
