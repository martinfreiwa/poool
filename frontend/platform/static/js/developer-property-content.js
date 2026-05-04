/**
 * Developer Property Content (Step 4) — Form submission logic.
 *
 * Collects marketing content and financial data, then sends it to
 * PUT /api/developer/draft/:id and POST /api/developer/draft/:id/submit
 */

const MAX_ASSET_IMAGE_BYTES = 10 * 1024 * 1024;
const ALLOWED_IMAGE_MIME_TYPES = new Set(["image/jpeg", "image/png", "image/webp", "image/gif"]);
let activeImageUploadCount = 0;

function getCsrfToken() {
  const value = `; ${document.cookie}`;
  const parts = value.split(`; csrf_token=`);
  if (parts.length === 2) return parts.pop().split(";").shift();
  return "";
}

function getDraftId() {
  const urlParams = new URLSearchParams(window.location.search);
  return urlParams.get("draft_id") || localStorage.getItem("draft_asset_id") || "";
}

function parsePercentField(id) {
  const raw = document.getElementById(id)?.value || "";
  if (!raw.trim()) return null;
  const value = Number(raw.replace(/[^0-9.]/g, ""));
  return Number.isFinite(value) ? value : NaN;
}

function isValidPercent(value) {
  return value === null || (Number.isFinite(value) && value >= 0 && value <= 100);
}

function showPageToast(message, type) {
  if (window.showPooolToast) {
    window.showPooolToast(null, message, type || "info");
    return;
  }

  let toast = document.getElementById("form-error-toast");
  if (!toast) {
    toast = document.createElement("div");
    toast.id = "form-error-toast";
    toast.style.cssText =
      "position:fixed;top:24px;right:24px;background:#f04438;color:#fff;padding:16px 24px;border-radius:8px;z-index:9999;font-size:0.95rem;box-shadow:0 4px 12px rgba(0,0,0,0.15);max-width:400px;";
    document.body.appendChild(toast);
  }
  toast.textContent = message;
  toast.style.display = "block";
  setTimeout(() => {
    toast.style.display = "none";
  }, 5000);
}

async function readApiErrorMessage(resp, fallback) {
  let message = fallback || "Something went wrong. Please try again.";
  try {
    const raw = await resp.text();
    try {
      const parsed = JSON.parse(raw);
      message = parsed.error || parsed.message || message;
    } catch {
      const stripped = raw.replace(/<[^>]+>/g, "").trim();
      if (stripped && stripped.length < 300) message = stripped;
    }
  } catch {
    // Keep fallback.
  }
  if (resp.status === 401) return "You are not logged in. Please log in and try again.";
  return message;
}

/**
 * Save & Exit — persists whatever the user has typed so far (no validation)
 * then navigates to the submissions list.
 */
async function saveAndExitStep4(btn) {
  const originalText = btn ? btn.textContent : "";
  if (btn) { btn.disabled = true; btn.textContent = 'Saving...'; }

  const assetId = getDraftId();
  if (!assetId) {
    // No draft to save, just navigate
    window.location.href = '/developer/submissions';
    return;
  }

  const getVal = (id) => document.getElementById(id)?.value || '';

  const payload = {};
  const assetTitle = getVal('asset-title');
  if (assetTitle) payload.title = assetTitle.trim();
  const shortDesc = getVal('short-description');
  if (shortDesc) payload.short_description = shortDesc.trim();
  const fullDesc = getVal('full-description');
  if (fullDesc) payload.description = fullDesc.trim();
  const locationDesc = getVal('location-description');
  if (locationDesc) payload.location_description = locationDesc.trim();
  const mapsLink = getVal('maps-link');
  if (mapsLink) payload.google_maps_url = mapsLink.trim();
  const youtubeLink = getVal('youtube-link');
  if (youtubeLink) payload.video_url = youtubeLink.trim();

  const rentalYield = parsePercentField('rental-yield');
  if (rentalYield !== null) payload.annual_yield_bps = Math.round(rentalYield * 100);
  const capitalApp = parsePercentField('capital-appreciation');
  if (capitalApp !== null) payload.capital_appreciation_bps = Math.round(capitalApp * 100);
  const investorShare = parsePercentField('investor-share');
  if (investorShare !== null) payload.investor_share_bps = Math.round(investorShare * 100);
  const occupancyRate = parsePercentField('occupancy-rate');
  if (occupancyRate !== null) payload.occupancy_rate_bps = Math.round(occupancyRate * 100);

  if (![rentalYield, capitalApp, investorShare, occupancyRate].every(isValidPercent)) {
    showPageToast("Percent fields must be between 0 and 100.", "error");
    if (btn) { btn.disabled = false; btn.textContent = originalText; }
    return;
  }

  try {
    const resp = await fetch(`/api/developer/draft/${assetId}`, {
      method: 'PUT',
      headers: { 
        'Content-Type': 'application/json',
        'X-CSRF-Token': getCsrfToken()
      },
      body: JSON.stringify(payload),
    });
    if (!resp.ok) {
      const message = await readApiErrorMessage(resp, `Save failed (${resp.status})`);
      showPageToast(message, "error");
      if (btn) { btn.disabled = false; btn.textContent = originalText; }
      return;
    }
    window.location.href = '/developer/submissions';
  } catch (err) {
    console.warn('Save & Exit: could not save draft', err);
    showPageToast("Connection lost - your draft was not saved. Please try again.", "error");
    if (btn) { btn.disabled = false; btn.textContent = originalText; }
  }
}

document.addEventListener("DOMContentLoaded", function () {
  const form = document.getElementById("property-content-form");
  if (!form) return;

  const urlParams = new URLSearchParams(window.location.search);
  const urlDraftId = urlParams.get('draft_id');
  if (urlDraftId) {
    localStorage.setItem("draft_asset_id", urlDraftId);
  }
  const backBtn = document.getElementById("form-back-btn");
  if (backBtn) {
    backBtn.addEventListener("click", function () {
      const id = getDraftId();
      window.location.href = id ? `/developer/document-upload-step3?draft_id=${encodeURIComponent(id)}` : "/developer/document-upload-step3";
    });
  }

  // ── Pre-fill form from existing draft data ─────────────────────────
  const assetId = getDraftId();
  if (assetId) {
    fetch(`/api/developer/draft/${assetId}`)
      .then((r) => (r.ok ? r.json() : null))
      .then((data) => {
        if (!data) return;
        setVal("asset-title", data.title);
        setVal("short-description", data.short_description);
        setVal("full-description", data.description);
        setVal("location-description", data.location_description);
        setVal("maps-link", data.google_maps_url);
        setVal("youtube-link", data.video_url);
        // Financial fields — convert basis points back to %
        if (data.annual_yield_bps)
          setVal("rental-yield", (data.annual_yield_bps / 100).toString());
        if (data.capital_appreciation_bps)
          setVal(
            "capital-appreciation",
            (data.capital_appreciation_bps / 100).toString()
          );
        if (data.investor_share_bps)
          setVal(
            "investor-share",
            (data.investor_share_bps / 100).toString()
          );
          if (data.occupancy_rate_bps)
            setVal(
              "occupancy-rate",
              (data.occupancy_rate_bps / 100).toString()
            );

          // Populate assetImages from existing draft
          if (data.images && Array.isArray(data.images)) {
            assetImages = data.images.map(img => ({
               id: img.id,
               url: img.url,
               is_cover: img.is_cover,
               sort_order: img.sort_order
            }));
            assetImages.sort((a, b) => a.sort_order - b.sort_order);
            renderImageGallery();
          }
        })
        .catch((err) => console.warn("Could not pre-fill form:", err));
    }

    // ── Asset Images Logic ───────────────────────────────────────────────
    let assetImages = [];
    const galleryEl = document.getElementById("image-gallery");
    const fileInput = document.getElementById("file-input-media");
    const dropArea = document.getElementById("file-upload-area-media");
    const subtitle = document.getElementById("media-subtitle");

    function renderImageGallery() {
      if (!galleryEl) return;
      galleryEl.innerHTML = "";
      assetImages.forEach((img, index) => {
        img.sort_order = index; // enforce sequential sort_order
        const item = document.createElement("div");
        item.className = "uploaded-image-item";
        item.draggable = true;
        item.dataset.id = img.id;
        item.dataset.index = index;
        
        // Drag events
        item.addEventListener("dragstart", handleDragStart);
        item.addEventListener("dragover", handleDragOver);
        item.addEventListener("drop", handleDropImage);
        item.addEventListener("dragend", handleDragEnd);

        const image = document.createElement("img");
        image.className = "uploaded-image";
        image.alt = "Property Image";
        image.src = img.url || "";
        if (img.is_cover) image.style.border = "3px solid #03FF88";
        item.appendChild(image);

        if (img.is_cover) {
          const badge = document.createElement("div");
          badge.textContent = "COVER";
          badge.style.cssText = "position:absolute;top:8px;left:8px;background:#03FF88;color:#000;font-size:10px;padding:2px 6px;border-radius:4px;font-weight:bold;";
          item.appendChild(badge);
        } else {
          const setCoverBtn = document.createElement("button");
          setCoverBtn.type = "button";
          setCoverBtn.className = "set-cover-btn";
          setCoverBtn.dataset.id = img.id;
          setCoverBtn.textContent = "Set Cover";
          setCoverBtn.style.cssText = "position:absolute;top:8px;left:8px;background:rgba(0,0,0,0.6);color:#fff;border:none;border-radius:4px;font-size:10px;padding:2px 6px;cursor:pointer;";
          item.appendChild(setCoverBtn);
        }

        const removeBtn = document.createElement("button");
        removeBtn.type = "button";
        removeBtn.className = "image-remove-btn";
        removeBtn.dataset.id = img.id;
        removeBtn.style.cssText = "background:rgba(255,0,0,0.8);color:#fff;";
        removeBtn.setAttribute("aria-label", "Remove image");
        removeBtn.innerHTML = '<svg width="10" height="10" viewBox="0 0 14 14" fill="none" aria-hidden="true"><path d="M10.5 3.5L3.5 10.5M3.5 3.5l7 7" stroke="white" stroke-width="1.5" stroke-linecap="round"/></svg>';
        item.appendChild(removeBtn);
        galleryEl.appendChild(item);
      });

      // Attach event listeners
      galleryEl.querySelectorAll(".image-remove-btn").forEach(btn => {
        btn.addEventListener("click", async (e) => {
          e.preventDefault();
          const imgId = btn.dataset.id;
          try {
            const r = await fetch(`/api/developer/draft/${assetId}/images/${imgId}`, { 
              method: 'DELETE',
              headers: { "X-CSRF-Token": getCsrfToken() }
            });
            if (!r.ok) throw new Error("Failed to delete");
            assetImages = assetImages.filter(i => i.id !== imgId);
            renderImageGallery();
            syncImageOrder();
          } catch(err) {
            showFormError("Could not delete image.");
          }
        });
      });

      galleryEl.querySelectorAll(".set-cover-btn").forEach(btn => {
        btn.addEventListener("click", (e) => {
          e.preventDefault();
          const imgId = btn.dataset.id;
          assetImages.forEach(i => i.is_cover = (i.id === imgId));
          renderImageGallery();
          syncImageOrder();
        });
      });

      updateImageSubtitle();
    }

    function updateImageSubtitle() {
      if (!subtitle) return;
      const count = assetImages.length;
      if (count === 0) {
        subtitle.style.color = "#F04438";
        subtitle.textContent = `Please upload at least 1 photo (8–16 recommended)`;
      } else if (count > 16) {
        subtitle.style.color = "#F04438";
        subtitle.textContent = `Too many photos — max 16 (Currently: ${count})`;
      } else if (count < 8) {
        subtitle.style.color = "#DC6803";
        subtitle.textContent = `Currently: ${count} — 8–16 photos recommended`;
      } else {
        subtitle.style.color = "#039855";
        subtitle.textContent = `Currently: ${count} — looks good!`;
      }
    }

    let draggedItemIdx = null;
    function handleDragStart(e) {
      draggedItemIdx = parseInt(this.dataset.index);
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", draggedItemIdx);
      this.style.opacity = "0.5";
    }
    function handleDragOver(e) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      return false;
    }
    function handleDropImage(e) {
      e.stopPropagation();
      const targetIdx = parseInt(this.dataset.index);
      if (draggedItemIdx !== null && draggedItemIdx !== targetIdx) {
        // Swap elements based on their indexes
        const items = [...assetImages];
        const [removed] = items.splice(draggedItemIdx, 1);
        items.splice(targetIdx, 0, removed);
        assetImages = items;
        renderImageGallery();
        syncImageOrder();
      }
      return false;
    }
    function handleDragEnd(e) {
      this.style.opacity = "1";
    }

    async function syncImageOrder() {
      if (!assetImages.length) return;
      const payload = assetImages.map((img, i) => ({
        id: img.id,
        sort_order: i,
        is_cover: !!img.is_cover
      }));
      await fetch(`/api/developer/draft/${assetId}/images/reorder`, {
        method: "PUT",
        headers: { 
          "Content-Type": "application/json",
          "X-CSRF-Token": getCsrfToken()
        },
        body: JSON.stringify(payload)
      }).catch(err => console.warn("Failed to sync image order", err));
    }

    // ── Upload Handlers ──────────────────────────────────────────────────
    if (fileInput) {
      fileInput.addEventListener("change", (e) => {
        handleFiles(e.target.files);
        fileInput.value = ""; // Reset so re-selecting same files works
      });
    }
    if (dropArea) {
      dropArea.addEventListener("dragover", e => {
        e.preventDefault();
        dropArea.style.borderColor = "#03FF88";
      });
      dropArea.addEventListener("dragleave", e => {
        e.preventDefault();
        dropArea.style.borderColor = "#d5d7da";
      });
      dropArea.addEventListener("drop", e => {
        e.preventDefault();
        dropArea.style.borderColor = "#d5d7da";
        handleFiles(e.dataTransfer.files);
      });
      dropArea.addEventListener("click", e => {
        if (e.target.closest('.upload-link') || e.target.closest('input[type="file"]')) {
          return;
        }
        if (fileInput) fileInput.click();
      });
    }

    async function handleFiles(files) {
      if (!files || !files.length) return;
      const arr = [];
      Array.from(files).forEach((file) => {
        if (!ALLOWED_IMAGE_MIME_TYPES.has(file.type)) {
          showFormError(`${file.name} must be a JPG, PNG, WebP, or GIF image.`);
          return;
        }
        if (file.size > MAX_ASSET_IMAGE_BYTES) {
          showFormError(`${file.name} must be 10MB or smaller.`);
          return;
        }
        arr.push(file);
      });
      if (!arr.length) return;

      if (assetImages.length + arr.length > 16) {
        showFormError("Maximum 16 images allowed.");
        return;
      }

      // Show local previews immediately so the user sees instant feedback
      const placeholders = arr.map((file, idx) => {
        const tempId = `uploading-${Date.now()}-${idx}`;
        const localUrl = URL.createObjectURL(file);

        const item = document.createElement("div");
        item.id = tempId;
        item.className = "uploaded-image-item";

        const img = document.createElement("img");
        img.className = "uploaded-image";
        img.src = localUrl;
        img.alt = file.name;
        item.appendChild(img);

        const overlay = document.createElement("div");
        overlay.className = "upload-spinner-overlay";
        overlay.innerHTML = '<div class="upload-spinner"></div>';
        item.appendChild(overlay);

        if (galleryEl) galleryEl.appendChild(item);
        updateImageSubtitle();
        return { tempId, localUrl };
      });

      // Upload all files in parallel for speed
      activeImageUploadCount += arr.length;
      const uploadPromises = arr.map((file, idx) => {
        const formData = new FormData();
        formData.append("file", file);
        formData.append("sort_order", (assetImages.length + idx).toString());
        const isCover = (assetImages.length === 0 && idx === 0);
        formData.append("is_cover", isCover.toString());

        return fetch(`/api/developer/draft/${assetId}/images`, {
          method: 'POST',
          headers: { "X-CSRF-Token": getCsrfToken() },
          body: formData
        })
        .then(res => {
          if (!res.ok) throw new Error("Upload failed");
          return res.json();
        })
        .then(data => ({
          id: data.image_id,
          url: data.image_url,
          is_cover: data.is_cover,
          sort_order: assetImages.length + idx
        }))
        .catch(err => {
          showFormError(`Failed to upload ${file.name}`);
          return null;
        })
        .finally(() => {
          activeImageUploadCount = Math.max(0, activeImageUploadCount - 1);
          // Remove this file's placeholder once upload finishes
          const p = placeholders[idx];
          const el = document.getElementById(p.tempId);
          if (el) el.remove();
          URL.revokeObjectURL(p.localUrl);
        });
      });

      const results = await Promise.all(uploadPromises);
      const successful = results.filter(r => r !== null);
      assetImages.push(...successful);
      renderImageGallery();
    }

  function setVal(id, val) {
    const el = document.getElementById(id);
    if (el && val) el.value = val;
  }

  form.addEventListener("submit", async function (e) {
    e.preventDefault();
    clearAllFieldErrors();
    dismissFormError();

    // ── Collect required fields ──────────────────────────────────────────
    const assetTitle = document.getElementById("asset-title")?.value || "";
    const shortDesc = document.getElementById("short-description")?.value || "";

    let hasErrors = false;

    if (!assetTitle.trim()) {
      showFieldError("asset-title", "Asset Title is required");
      hasErrors = true;
    }
    if (!shortDesc.trim()) {
      showFieldError("short-description", "Short Description is required");
      hasErrors = true;
    }

    if (hasErrors) {
      showFormError("Please fill in all required fields.");
      return;
    }

    // ── Collect all fields ───────────────────────────────────────────────
    const fullDesc =
      document.getElementById("full-description")?.value || "";
    const locationDesc =
      document.getElementById("location-description")?.value || "";
    const mapsLink =
      document.getElementById("maps-link")?.value || "";
    const youtubeLink =
      document.getElementById("youtube-link")?.value || "";

    // Financial fields — convert percentages to basis points (× 100)
    const rentalYield = parsePercentField("rental-yield");
    const capitalAppreciation = parsePercentField("capital-appreciation");
    const investorShare = parsePercentField("investor-share");
    const occupancyRate = parsePercentField("occupancy-rate");

    // ── Get draft asset ID from localStorage ─────────────────────────────
    const assetId = getDraftId();
    if (!assetId) {
      showFormError(
        "No draft asset found. Please go back to Step 2 and create one first."
      );
      return;
    }
    if (activeImageUploadCount > 0) {
      showFormError("Please wait for image uploads to finish before submitting.");
      return;
    }
    if (assetImages.length === 0) {
      showFormError("Please upload at least one property image before submitting.");
      return;
    }
    if (![rentalYield, capitalAppreciation, investorShare, occupancyRate].every(isValidPercent)) {
      showFormError("Percent fields must be between 0 and 100.");
      return;
    }

    // ── Build payload ────────────────────────────────────────────────────
    const payload = {
      title: assetTitle.trim(),
      short_description: shortDesc.trim(),
      description: fullDesc.trim() || null,
      location_description: locationDesc.trim() || null,
      google_maps_url: mapsLink.trim() || null,
      video_url: youtubeLink.trim() || null,
      annual_yield_bps: rentalYield === null ? null : Math.round(rentalYield * 100),
      capital_appreciation_bps: capitalAppreciation === null ? null : Math.round(capitalAppreciation * 100),
      investor_share_bps: investorShare === null ? null : Math.round(investorShare * 100),
      occupancy_rate_bps: occupancyRate === null ? null : Math.round(occupancyRate * 100),
      submission_step: 4,
    };

    // ── Submit button loading state ──────────────────────────────────────
    const submitBtn = form.querySelector('button[type="submit"]');
    const originalText = submitBtn ? submitBtn.textContent : "";
    if (submitBtn) {
      submitBtn.disabled = true;
      submitBtn.textContent = "Saving...";
    }

    try {
      // 1. Update the draft with all content + financial data
      const updateRes = await fetch(`/api/developer/draft/${assetId}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json", "X-CSRF-Token": getCsrfToken() },
        body: JSON.stringify(payload),
      });

      if (!updateRes.ok) {
        const err = await updateRes.json().catch(() => ({}));
        throw new Error(err.error || "Failed to save property content");
      }

      // 2. Submit the draft for review
      const submitRes = await fetch(`/api/developer/draft/${assetId}/submit`, {
        method: "POST",
        headers: { "Content-Type": "application/json", "X-CSRF-Token": getCsrfToken() },
      });

      if (!submitRes.ok) {
        const err = await submitRes.json().catch(() => ({}));
        throw new Error(err.error || "Failed to submit for review");
      }

      // Success — clear draft state and redirect to the success page
      localStorage.removeItem("draft_asset_id");
      localStorage.removeItem("selectedAssetType");
      window.location.href = "/developer/submission-success";
    } catch (err) {
      console.error("Submission error:", err);
      showFormError(err.message || "An unexpected error occurred.");
      if (submitBtn) {
        submitBtn.disabled = false;
        submitBtn.textContent = originalText;
      }
    }
  });

  // ── Error display helpers (consistent with application-form.js) ─────────

  function showFieldError(fieldId, message) {
    const field = document.getElementById(fieldId);
    if (!field) return;
    field.classList.add("input-error");
    const existing = field.parentElement.querySelector(".field-error-msg");
    if (existing) existing.remove();
    const span = document.createElement("span");
    span.className = "field-error-msg";
    span.textContent = message;
    span.style.color = "#f04438";
    span.style.fontSize = "0.85rem";
    span.style.marginTop = "4px";
    span.style.display = "block";
    field.parentElement.appendChild(span);
  }

  function clearAllFieldErrors() {
    document
      .querySelectorAll(".field-error-msg")
      .forEach((el) => el.remove());
    document
      .querySelectorAll(".input-error")
      .forEach((el) => el.classList.remove("input-error"));
  }

  function showFormError(message) {
    let toast = document.getElementById("form-error-toast");
    if (!toast) {
      toast = document.createElement("div");
      toast.id = "form-error-toast";
      toast.style.cssText =
        "position:fixed;top:24px;right:24px;background:#f04438;color:#fff;padding:16px 24px;border-radius:8px;z-index:9999;font-size:0.95rem;box-shadow:0 4px 12px rgba(0,0,0,0.15);max-width:400px;";
      document.body.appendChild(toast);
    }
    toast.textContent = message;
    toast.style.display = "block";
    setTimeout(() => {
      toast.style.display = "none";
    }, 5000);
  }

  function dismissFormError() {
    const toast = document.getElementById("form-error-toast");
    if (toast) toast.style.display = "none";
  }
});
