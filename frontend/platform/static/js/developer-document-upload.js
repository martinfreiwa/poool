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
  }
});

function renderExistingFile(doc, sectionId, assetId) {
  const filesList = document.getElementById(`uploaded-files-list-${sectionId}`);
  if (!filesList) return;

  const fileId = `file-${sectionId}-${doc.id}`;
  const fileSize = formatFileSize(doc.file_size || 0);

  const fileItemHTML = `
    <div id="${fileId}" class="file-upload-item">
      <div class="file-content">
        <div class="file-type-icon">
          <img src="/static/images/icons/File%20type%20icon%20(1).svg" alt="File" width="40" height="40" />
        </div>
        <div class="file-info">
          <div class="file-details">
            <span class="file-name">${doc.title || doc.document_type}</span>
            <span class="file-size">${fileSize}</span>
          </div>
          <div class="file-progress">
            <div class="ds-progress ds-progress--sm">
              <div class="ds-progress__fill" style="width: 100%"></div>
            </div>
            <span class="progress-percentage" style="color: #12b76a">Uploaded ✓</span>
          </div>
        </div>
      </div>
      <button type="button" class="file-delete-btn" title="Delete" onclick="removeFile('${fileId}', '${assetId}', '${doc.id}')">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M2 4H14M5.33333 4V2.66667C5.33333 2.31305 5.47381 1.97391 5.72386 1.72386C5.97391 1.47381 6.31305 1.33333 6.66667 1.33333H9.33333C9.68696 1.33333 10.0261 1.47381 10.2761 1.72386C10.5262 1.97391 10.6667 2.31305 10.6667 2.66667V4M6.66667 7.33333V11.3333M9.33333 7.33333V11.3333M12.6667 4V12.6667C12.6667 13.0203 12.5262 13.3594 12.2761 13.6095C12.0261 13.8595 11.687 14 11.3333 14H4.66667C4.31305 14 3.97391 13.8595 3.72386 13.6095C3.47381 13.3594 3.33333 13.0203 3.33333 12.6667V4" stroke="#A4A7AE" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    </div>`;

  const tempDiv = document.createElement("div");
  tempDiv.innerHTML = fileItemHTML;
  filesList.appendChild(tempDiv.firstElementChild);
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

    if (file.size > 20 * 1024 * 1024) {
      showToast(`File ${file.name} is too large. Maximum size is 20 MB.`);
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

  const fileItemHTML = `
    <div id="${fileId}" class="file-upload-item">
      <div class="file-content">
        <div class="file-type-icon">
          <img src="/static/images/icons/File%20type%20icon%20(1).svg" alt="File" width="40" height="40" />
        </div>
        <div class="file-info">
          <div class="file-details">
            <span class="file-name">${file.name}</span>
            <span class="file-size">${fileSize}</span>
          </div>
          <div class="file-progress">
            <div class="ds-progress ds-progress--sm">
              <div class="ds-progress__fill" style="width: 0%"></div>
            </div>
            <span class="progress-percentage">Uploading...</span>
          </div>
        </div>
      </div>
      <button type="button" class="file-delete-btn" disabled title="Uploading...">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M2 4H14M5.33333 4V2.66667C5.33333 2.31305 5.47381 1.97391 5.72386 1.72386C5.97391 1.47381 6.31305 1.33333 6.66667 1.33333H9.33333C9.68696 1.33333 10.0261 1.47381 10.2761 1.72386C10.5262 1.97391 10.6667 2.31305 10.6667 2.66667V4M6.66667 7.33333V11.3333M9.33333 7.33333V11.3333M12.6667 4V12.6667C12.6667 13.0203 12.5262 13.3594 12.2761 13.6095C12.0261 13.8595 11.687 14 11.3333 14H4.66667C4.31305 14 3.97391 13.8595 3.72386 13.6095C3.47381 13.3594 3.33333 13.0203 3.33333 12.6667V4" stroke="#A4A7AE" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    </div>`;

  const tempDiv = document.createElement("div");
  tempDiv.innerHTML = fileItemHTML;
  const fileItem = tempDiv.firstElementChild;
  filesList.appendChild(fileItem);

  // Upload to backend
  uploadFile(file, sectionId, fileId);
}

/**
 * Upload a single file to POST /api/developer/draft/:id/documents
 */
async function uploadFile(file, sectionId, fileId) {
  const assetId = localStorage.getItem("draft_asset_id");
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

  // Extract CSRF token from cookie
  const getCsrfToken = () => {
    const value = `; ${document.cookie}`;
    const parts = value.split(`; csrf_token=`);
    if (parts.length === 2) return parts.pop().split(';').shift();
    return "";
  };

  try {
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
        deleteBtn.onclick = () =>
          removeFile(fileId, assetId, data.document_id);
      }
    }
  } catch (err) {
    console.error("Upload error:", err);
    markUploadFailed(fileId, err.message);
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
  showToast(message || "Upload failed");
}

/**
 * Remove a file from the UI and delete from backend.
 */
async function removeFile(fileId, assetId, documentId) {
  // Optimistic UI removal
  const fileItem = document.getElementById(fileId);
  if (fileItem) fileItem.remove();

  if (assetId && documentId) {
    const getCsrfToken = () => {
      const value = `; ${document.cookie}`;
      const parts = value.split(`; csrf_token=`);
      if (parts.length === 2) return parts.pop().split(';').shift();
      return "";
    };

    try {
      await fetch(
        `/api/developer/draft/${assetId}/documents/${documentId}`,
        { 
          method: "DELETE",
          headers: { "X-CSRF-Token": getCsrfToken() }
        }
      );
    } catch (err) {
      console.error("Delete error:", err);
    }
  }
}

function formatFileSize(bytes) {
  if (bytes === 0) return "0 Bytes";
  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
}

function showToast(message) {
  if(window.showPooolToast) {
    window.showPooolToast(null, message, "info");
  }
}
