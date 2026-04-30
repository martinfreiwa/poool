/**
 * Document Upload Step 3 — Real file upload to GCS via API.
 *
 * Each of the 6 document sections maps to a `document_type` in the DB:
 *   1 → proof_of_title
 *   2 → legal_basis
 *   3 → building_permit
 *   4 → tax_npwp (Tax Documentation)
 *   5 → id_card (KYC / Corporate Documents)
 *   6 → other (Declarations & Warranties)
 */

const SECTION_DOC_TYPES = {
  1: "proof_of_title",
  2: "legal_basis",
  3: "building_permit",
  4: "tax_npwp",
  5: "id_card",
  6: "other",
};

const MAX_DOCUMENT_BYTES = 20 * 1024 * 1024;
const ALLOWED_DOCUMENT_MIME_TYPES = new Set([
  "application/pdf",
  "application/msword",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "application/zip",
  "image/jpeg",
  "image/png",
  "image/webp",
]);
const ALLOWED_DOCUMENT_EXTENSIONS = new Set(["pdf", "doc", "docx", "zip", "jpg", "jpeg", "png", "webp"]);
let activeUploadCount = 0;

document.addEventListener("DOMContentLoaded", function () {
  // Initialize file upload for all sections (1-6)
  for (let sectionId = 1; sectionId <= 6; sectionId++) {
    initializeFileUpload(sectionId);
  }

  // Remove hardcoded demo files on page load
  document.querySelectorAll(".file-upload-item").forEach((el) => el.remove());

  const urlParams = new URLSearchParams(window.location.search);
  const urlDraftId = urlParams.get('draft_id');
  if (urlDraftId) {
    localStorage.setItem("draft_asset_id", urlDraftId);
  }

  const assetId = localStorage.getItem("draft_asset_id");
  if (assetId) {
    fetch(`/api/developer/draft/${assetId}`)
      .then(r => r.ok ? r.json() : null)
      .then(data => {
        if (!data || !data.documents) return;
        
        // Reverse mapping from doc_type to sectionId
        const TYPE_TO_SECTION = {};
        for (const [key, val] of Object.entries(SECTION_DOC_TYPES)) {
          TYPE_TO_SECTION[val] = key;
        }

        data.documents.forEach(doc => {
           const sectionId = TYPE_TO_SECTION[doc.document_type];
           if (sectionId) {
             renderExistingFile(doc, sectionId, assetId);
           }
        });
      })
      .catch(console.error);
  } else {
    showToast("No draft found. Please complete Property Info before uploading documents.", "error");
  }

  bindStepNavigation();
});

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

function getFileExtension(fileName) {
  const parts = String(fileName || "").split(".");
  return parts.length > 1 ? parts.pop().toLowerCase() : "";
}

function isAllowedDocumentFile(file) {
  return ALLOWED_DOCUMENT_MIME_TYPES.has(file.type) || ALLOWED_DOCUMENT_EXTENSIONS.has(getFileExtension(file.name));
}

function bindStepNavigation() {
  const backBtn = document.getElementById("form-back-btn");
  const saveExitBtn = document.getElementById("form-save-exit-btn");
  const nextBtn = document.getElementById("form-next-btn");

  if (backBtn) {
    backBtn.addEventListener("click", function () {
      const id = getDraftId();
      window.location.href = id ? `/developer/application-form?draft_id=${encodeURIComponent(id)}` : "/developer/application-form";
    });
  }

  if (saveExitBtn) {
    saveExitBtn.addEventListener("click", function () {
      const id = getDraftId();
      window.location.href = id ? `/developer/submissions?draft_id=${encodeURIComponent(id)}` : "/developer/submissions";
    });
  }

  if (nextBtn) {
    nextBtn.addEventListener("click", function () {
      const id = getDraftId();
      if (!id) {
        showToast("No draft found. Please complete Property Info before continuing.", "error");
        return;
      }
      if (activeUploadCount > 0) {
        showToast("Please wait for document uploads to finish before continuing.", "warning");
        return;
      }
      window.location.href = `/developer/property-content?draft_id=${encodeURIComponent(id)}`;
    });
  }
}

function createFileItem(fileId, options) {
  const fileItem = document.createElement("div");
  fileItem.id = fileId;
  fileItem.className = "file-upload-item";

  const content = document.createElement("div");
  content.className = "file-content";

  const icon = document.createElement("div");
  icon.className = "file-type-icon";
  icon.innerHTML = '<img src="/static/images/icons/File%20type%20icon%20(1).svg" alt="File" width="40" height="40" />';

  const info = document.createElement("div");
  info.className = "file-info";

  const details = document.createElement("div");
  details.className = "file-details";

  const fileName = document.createElement("span");
  fileName.className = "file-name";
  fileName.textContent = options.name || "Document";

  const fileSize = document.createElement("span");
  fileSize.className = "file-size";
  fileSize.textContent = options.size || "";

  details.append(fileName, fileSize);
  info.appendChild(details);

  const progress = document.createElement("div");
  progress.className = "file-progress";
  progress.innerHTML = '<div class="ds-progress ds-progress--sm"><div class="ds-progress__fill" style="width: 0%"></div></div>';

  const progressText = document.createElement("span");
  progressText.className = "progress-percentage";
  progressText.textContent = options.progressText || "Uploading...";
  if (options.progressColor) progressText.style.color = options.progressColor;
  progress.appendChild(progressText);
  info.appendChild(progress);

  content.append(icon, info);

  const deleteBtn = document.createElement("button");
  deleteBtn.type = "button";
  deleteBtn.className = "file-delete-btn";
  deleteBtn.disabled = Boolean(options.disabled);
  deleteBtn.title = options.disabled ? "Uploading..." : "Delete";
  deleteBtn.setAttribute("aria-label", options.disabled ? "Uploading, please wait" : "Delete file");
  deleteBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M2 4H14M5.33333 4V2.66667C5.33333 2.31305 5.47381 1.97391 5.72386 1.72386C5.97391 1.47381 6.31305 1.33333 6.66667 1.33333H9.33333C9.68696 1.33333 10.0261 1.47381 10.2761 1.72386C10.5262 1.97391 10.6667 2.31305 10.6667 2.66667V4M6.66667 7.33333V11.3333M9.33333 7.33333V11.3333M12.6667 4V12.6667C12.6667 13.0203 12.5262 13.3594 12.2761 13.6095C12.0261 13.8595 11.687 14 11.3333 14H4.66667C4.31305 14 3.97391 13.8595 3.72386 13.6095C3.47381 13.3594 3.33333 13.0203 3.33333 12.6667V4" stroke="#A4A7AE" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  if (typeof options.onDelete === "function") {
    deleteBtn.addEventListener("click", options.onDelete);
  }

  fileItem.append(content, deleteBtn);

  if (options.complete) {
    const fill = fileItem.querySelector(".ds-progress__fill");
    if (fill) fill.style.width = "100%";
  }

  return fileItem;
}

function renderExistingFile(doc, sectionId, assetId) {
  const filesList = document.getElementById(`uploaded-files-list-${sectionId}`);
  if (!filesList) return;

  const fileId = `file-${sectionId}-${doc.id}`;
  const fileSize = formatFileSize(doc.file_size || 0);
  filesList.appendChild(createFileItem(fileId, {
    name: doc.title || doc.document_type,
    size: fileSize,
    progressText: "Uploaded ✓",
    progressColor: "#12b76a",
    complete: true,
    onDelete: () => removeFile(fileId, assetId, doc.id),
  }));
}

function initializeFileUpload(sectionId) {
  const fileInput = document.getElementById(`file-input-${sectionId}`);
  const uploadArea = document.querySelector(`#file-upload-area-${sectionId}`);
  const dragOverlay = uploadArea
    ? uploadArea.querySelector(".drag-overlay")
    : null;

  if (!fileInput || !uploadArea) return;

  // Fix accept attribute (B12 fix)
  fileInput.setAttribute(
    "accept",
    ".pdf,.doc,.docx,.zip,.jpg,.jpeg,.png,.webp"
  );

  fileInput.addEventListener("change", function (e) {
    handleFiles(e.target.files, sectionId);
    // Reset input so re-selecting the same file triggers change
    fileInput.value = "";
  });

  uploadArea.addEventListener("dragover", function (e) {
    e.preventDefault();
    e.stopPropagation();
    if (dragOverlay) dragOverlay.style.display = "flex";
  });

  uploadArea.addEventListener("dragleave", function (e) {
    e.preventDefault();
    e.stopPropagation();
    if (dragOverlay) dragOverlay.style.display = "none";
  });

  uploadArea.addEventListener("drop", function (e) {
    e.preventDefault();
    e.stopPropagation();
    if (dragOverlay) dragOverlay.style.display = "none";
    handleFiles(e.dataTransfer.files, sectionId);
  });

  uploadArea.addEventListener("click", function (e) {
    if (e.target.closest('.upload-link') || e.target.closest('.hidden-file-input')) {
      return;
    }
    fileInput.click();
  });
}

function handleFiles(files, sectionId) {
  const filesList = document.getElementById(
    `uploaded-files-list-${sectionId}`
  );
  if (!filesList) return;

  for (let i = 0; i < files.length; i++) {
    const file = files[i];

    if (file.size > MAX_DOCUMENT_BYTES) {
      showToast(`File ${file.name} is too large. Maximum size is 20 MB.`, "error");
      continue;
    }

    if (!isAllowedDocumentFile(file)) {
      showToast(`File ${file.name} has an unsupported format. Use PDF, DOC, DOCX, ZIP, PNG, JPG, or WebP.`, "warning");
      continue;
    }

    addFileAndUpload(file, sectionId);
  }
}

/**
 * Add a file to the UI list and upload it to the backend.
 */
function addFileAndUpload(file, sectionId) {
  const filesList = document.getElementById(
    `uploaded-files-list-${sectionId}`
  );
  const fileId = `file-${sectionId}-${Date.now()}-${Math.random()
    .toString(36)
    .substr(2, 9)}`;

  const fileSize = formatFileSize(file.size);

  filesList.appendChild(createFileItem(fileId, {
    name: file.name,
    size: fileSize,
    disabled: true,
  }));

  // Upload to backend
  uploadFile(file, sectionId, fileId);
}

/**
 * Upload a single file to POST /api/developer/draft/:id/documents
 */
async function uploadFile(file, sectionId, fileId) {
  const assetId = getDraftId();
  if (!assetId) {
    markUploadFailed(
      fileId,
      "No draft found. Please go back to Step 2 first."
    );
    return;
  }

  const documentType = SECTION_DOC_TYPES[sectionId] || "other";

  const formData = new FormData();
  formData.append("file", file);
  formData.append("document_type", documentType);
  formData.append("title", file.name);

  try {
    activeUploadCount += 1;
    const res = await fetch(`/api/developer/draft/${assetId}/documents`, {
      method: "POST",
      headers: { "X-CSRF-Token": getCsrfToken() },
      body: formData,
    });

    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      throw new Error(err.error || `Upload failed (${res.status})`);
    }

    const data = await res.json();

    // Update progress to 100%
    const fileItem = document.getElementById(fileId);
    if (fileItem) {
      const progressFill = fileItem.querySelector(".ds-progress__fill");
      const progressText = fileItem.querySelector(".progress-percentage");
      if (progressFill) progressFill.style.width = "100%";
      if (progressText) {
        progressText.textContent = "Uploaded ✓";
        progressText.style.color = "#12b76a";
      }
      // Enable delete button and store the document_id
      const deleteBtn = fileItem.querySelector(".file-delete-btn");
      if (deleteBtn) {
        deleteBtn.disabled = false;
        deleteBtn.title = "Delete";
        deleteBtn.addEventListener("click", () => removeFile(fileId, assetId, data.document_id));
      }
    }
  } catch (err) {
    console.error("Upload error:", err);
    markUploadFailed(fileId, err.message);
  } finally {
    activeUploadCount = Math.max(0, activeUploadCount - 1);
  }
}

function markUploadFailed(fileId, message) {
  const fileItem = document.getElementById(fileId);
  if (!fileItem) return;

  const progressText = fileItem.querySelector(".progress-percentage");
  const progressFill = fileItem.querySelector(".ds-progress__fill");
  if (progressText) {
    progressText.textContent = "Failed";
    progressText.style.color = "#f04438";
  }
  if (progressFill) {
    progressFill.style.width = "100%";
    progressFill.style.backgroundColor = "#f04438";
  }
  const deleteBtn = fileItem.querySelector(".file-delete-btn");
  if (deleteBtn) {
    deleteBtn.disabled = false;
    deleteBtn.title = "Remove failed upload";
    deleteBtn.addEventListener("click", () => fileItem.remove(), { once: true });
  }
  showToast(message || "Upload failed", "error");
}

/**
 * Remove a file from the UI and delete from backend.
 */
async function removeFile(fileId, assetId, documentId) {
  const fileItem = document.getElementById(fileId);
  const deleteBtn = fileItem ? fileItem.querySelector(".file-delete-btn") : null;
  if (deleteBtn) deleteBtn.disabled = true;

  if (assetId && documentId) {
    try {
      const res = await fetch(
        `/api/developer/draft/${assetId}/documents/${documentId}`,
        { 
          method: "DELETE",
          headers: { "X-CSRF-Token": getCsrfToken() }
        }
      );
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.error || `Delete failed (${res.status})`);
      }
      if (fileItem) fileItem.remove();
    } catch (err) {
      console.error("Delete error:", err);
      showToast(err.message || "Document could not be deleted.", "error");
      if (deleteBtn) deleteBtn.disabled = false;
    }
  } else if (fileItem) {
    fileItem.remove();
  }
}

function formatFileSize(bytes) {
  if (bytes === 0) return "0 Bytes";
  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
}

function showToast(message, type) {
  if(window.showPooolToast) {
    window.showPooolToast(null, message, type || "info");
  }
}
