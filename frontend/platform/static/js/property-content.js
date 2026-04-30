// Property Content Page JavaScript
document.addEventListener("DOMContentLoaded", function () {
  // Initialize form validation
  initializeFormValidation();

  // Initialize character counters
  initializeCharacterCounters();

  // Initialize file upload handlers
  initializeFileUpload();

  // Handle form submission
  const form = document.getElementById("property-content-form");
  if (form) {
    form.addEventListener("submit", handleFormSubmit);
  }
});

// Initialize form validation
function initializeFormValidation() {
  // Short description validation (80-150 characters)
  const shortDesc = document.getElementById("short-description");
  if (shortDesc) {
    shortDesc.addEventListener("input", function () {
      validateCharacterCount(this, 80, 150);
    });
  }

  // Full description validation (300-850 characters)
  const fullDesc = document.getElementById("full-description");
  if (fullDesc) {
    fullDesc.addEventListener("input", function () {
      validateCharacterCount(this, 300, 850);
    });
  }

  // Location description validation (300-850 characters)
  const locationDesc = document.getElementById("location-description");
  if (locationDesc) {
    locationDesc.addEventListener("input", function () {
      validateCharacterCount(this, 300, 850);
    });
  }

  // Numeric field validation
  const numericFields = document.querySelectorAll(".numeric-input");
  numericFields.forEach((field) => {
    field.addEventListener("input", function () {
      validateNumericField(this);
    });
  });

  // URL field validation
  const urlFields = document.querySelectorAll('input[type="url"]');
  urlFields.forEach((field) => {
    field.addEventListener("input", function () {
      validateURLField(this);
    });
  });
}

// Validate character count
function validateCharacterCount(field, min, max) {
  const length = field.value.length;
  const hint = field.parentElement.querySelector(".form-hint");

  if (length < min) {
    field.classList.add("error");
    field.classList.remove("success");
    if (hint) {
      hint.style.color = "#F04438";
      hint.textContent = `Must be at least ${min} characters. Currently: ${length}`;
    }
  } else if (length > max) {
    field.classList.add("error");
    field.classList.remove("success");
    if (hint) {
      hint.style.color = "#F04438";
      hint.textContent = `Must be no more than ${max} characters. Currently: ${length}`;
    }
  } else {
    field.classList.remove("error");
    field.classList.add("success");
    if (hint) {
      hint.style.color = "#535862";
      hint.textContent = `Must be between ${min} and ${max} characters. Currently: ${length}`;
    }
  }
}

// Validate numeric field
function validateNumericField(field) {
  const value = parseFloat(field.value);
  const min = parseFloat(field.min);
  const max = parseFloat(field.max);

  if (isNaN(value)) {
    field.classList.add("error");
    field.classList.remove("success");
    return;
  }

  if (min !== undefined && value < min) {
    field.classList.add("error");
    field.classList.remove("success");
  } else if (max !== undefined && value > max) {
    field.classList.add("error");
    field.classList.remove("success");
  } else {
    field.classList.remove("error");
    field.classList.add("success");
  }
}

// Validate URL field
function validateURLField(field) {
  const urlPattern = /^https?:\/\/.+/;

  if (field.value && !urlPattern.test(field.value)) {
    field.classList.add("error");
    field.classList.remove("success");
  } else if (field.value) {
    field.classList.remove("error");
    field.classList.add("success");
  } else {
    field.classList.remove("error");
    field.classList.remove("success");
  }
}

// Initialize character counters
function initializeCharacterCounters() {
  const textFields = document.querySelectorAll('textarea, input[type="text"]');
  textFields.forEach((field) => {
    const hint = field.parentElement.querySelector(".form-hint");
    if (hint && hint.textContent.includes("characters")) {
      field.addEventListener("input", function () {
        updateCharacterCount(this, hint);
      });
    }
  });
}

// Update character count display
function updateCharacterCount(field, hint) {
  const length = field.value.length;
  const originalText = hint.textContent;
  const match = originalText.match(/(\d+) and (\d+)/);

  if (match) {
    const min = match[1];
    const max = match[2];
    hint.textContent = `Must be between ${min} and ${max} characters. Currently: ${length}`;
  }
}

// Handle form submission
function handleFormSubmit(e) {
  e.preventDefault();

  // Validate all required fields
  const requiredFields = document.querySelectorAll("[required]");
  let isValid = true;

  requiredFields.forEach((field) => {
    if (!field.value.trim()) {
      field.classList.add("error");
      isValid = false;
    }
  });

  // Check for any error fields
  const errorFields = document.querySelectorAll(".error");
  if (errorFields.length > 0) {
    isValid = false;
    errorFields[0].focus();
    showNotification("Please fix the errors before submitting", "error");
    return;
  }

  if (isValid) {
    // Mock success workflow by redirecting
    window.location.href = '/developer/submission-success';
  }
}

// Remove uploaded image
function removeImage(imageId) {
  const imageElement = document.getElementById(`uploaded-image-${imageId}`);
  if (imageElement) {
    imageElement.remove();
    updateImageCount();
    updateMediaSectionHeight();
  }
}

// Remove uploaded document
function removeDocument(docId) {
  const docElement = document.getElementById(`document-item-${docId}`);
  if (docElement) {
    docElement.remove();
  }
}

// Update image count
function updateImageCount() {
  const images = document.querySelectorAll(".uploaded-image-item");
  const subtitle = document.getElementById("media-subtitle");
  if (subtitle) {
    const count = images.length;
    if (count > 0) {
      subtitle.style.color = "#535862";
      subtitle.textContent = `${count} photo${count !== 1 ? "s" : ""} uploaded`;
    } else {
      subtitle.style.color = "#535862";
      subtitle.textContent = "Please upload photos";
    }
  }
}

// Show notification
function showNotification(message, type) {
  // Create notification element
  const notification = document.createElement("div");
  notification.className = `notification notification-${type}`;
  notification.textContent = message;
  notification.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        padding: 16px 24px;
        background: ${type === "success" ? "#98FB96" : "#FEF2F2"};
        color: ${type === "success" ? "#181D27" : "#D92D20"};
        border-radius: 8px;
        box-shadow: 0px 4px 12px rgba(0, 0, 0, 0.1);
        z-index: 10000;
        animation: slideIn 0.3s ease;
    `;

  document.body.appendChild(notification);

  // Remove after 3 seconds
  setTimeout(() => {
    notification.style.animation = "slideOut 0.3s ease";
    setTimeout(() => {
      notification.remove();
    }, 300);
  }, 3000);
}

// Update media section height based on number of images
function updateMediaSectionHeight() {
  const mediaSection = document.getElementById("media-content-section");
  const imageGallery = document.getElementById("image-gallery");

  if (!mediaSection || !imageGallery) return;

  const images = imageGallery.querySelectorAll(".uploaded-image-item");
  const numImages = images.length;

  // Base height when no images (minimum height)
  const baseHeight = 419;

  // Each row of images adds 110px (106px + 4px gap)
  const imagesPerRow = 5;
  const numRows = Math.ceil(numImages / imagesPerRow);
  const imageGalleryHeight = numRows > 0 ? numRows * 110 + 16 : 0; // 16px for spacing

  // Calculate new height
  const newHeight = baseHeight + imageGalleryHeight;

  // Calculate height difference
  const currentHeight = parseInt(mediaSection.style.height) || baseHeight;
  const heightDifference = newHeight - currentHeight;

  // Update media section height
  mediaSection.style.height = newHeight + "px";

  // Update positions of subsequent sections
  updateSubsequentSectionPositions(heightDifference);
}

// Update positions of sections after media section
function updateSubsequentSectionPositions(heightDifference) {
  if (heightDifference === 0) return;

  // Update financials section
  const financialsSection = document.querySelector(".financials-section");
  if (financialsSection) {
    const currentTop = parseInt(financialsSection.style.top) || 2003;
    financialsSection.style.top = currentTop + heightDifference + "px";
  }

  // Update documents section
  const documentsSection = document.querySelector(".documents-section");
  if (documentsSection) {
    const currentTop = parseInt(documentsSection.style.top) || 2393;
    documentsSection.style.top = currentTop + heightDifference + "px";
  }

  // Update form navigation buttons
  const formNavigation = document.querySelector(".form-navigation");
  if (formNavigation) {
    const currentTop = parseInt(formNavigation.style.top) || 3156;
    formNavigation.style.top = currentTop + heightDifference + "px";
  }

  // Update main container height
  const mainContainer = document.querySelector(".property-content-main");
  if (mainContainer) {
    const currentHeight = parseInt(mainContainer.style.height) || 3280;
    mainContainer.style.height = currentHeight + heightDifference + "px";
  }
}

// Initialize file upload functionality
function initializeFileUpload() {
  const fileInput = document.getElementById("file-input-media");
  if (!fileInput) return;

  // Handle file input changes
  fileInput.addEventListener("change", function (e) {
    const files = Array.from(e.target.files);
    handleFileUploads(files);
  });

  // Handle drag and drop
  const uploadArea = document.getElementById("file-upload-area-media");
  if (uploadArea) {
    uploadArea.addEventListener("dragover", function (e) {
      e.preventDefault();
      uploadArea.classList.add("drag-active");
    });

    uploadArea.addEventListener("dragleave", function (e) {
      e.preventDefault();
      if (!uploadArea.contains(e.relatedTarget)) {
        uploadArea.classList.remove("drag-active");
      }
    });

    uploadArea.addEventListener("drop", function (e) {
      e.preventDefault();
      uploadArea.classList.remove("drag-active");
      const files = Array.from(e.dataTransfer.files);
      handleFileUploads(files);
    });
  }
}

// Handle file uploads and generate previews
function handleFileUploads(files) {
  const imageGallery = document.getElementById("image-gallery");
  if (!imageGallery) return;

  const validImageTypes = [
    "image/jpeg",
    "image/jpg",
    "image/png",
    "image/webp",
    "image/gif",
  ];

  files.forEach((file, index) => {
    if (validImageTypes.includes(file.type)) {
      const imageId = "image-" + Date.now() + "-" + index;
      createImagePreview(file, imageId, imageGallery);
    }
  });

  // Update image count and section height after processing all files
  setTimeout(() => {
    updateImageCount();
    updateMediaSectionHeight();
  }, 100);
}

// Create image preview element
function createImagePreview(file, imageId, gallery) {
  const reader = new FileReader();

  reader.onload = function (e) {
    // Create uploaded image item HTML
    const imageItem = document.createElement("div");
    imageItem.id = "uploaded-image-" + imageId;
    imageItem.className = "uploaded-image-item";

    imageItem.innerHTML = `
            <img src="${e.target.result}" alt="${file.name}" class="uploaded-image"/>
            <button type="button" class="image-remove-btn" aria-label="Remove image">
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                    <path d="M10.5 3.5L3.5 10.5" stroke="white" stroke-width="1.16667" stroke-linecap="round" stroke-linejoin="round"/>
                    <path d="M3.5 3.5L10.5 10.5" stroke="white" stroke-width="1.16667" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
            </button>
        `;

    // Add event listener to remove button
    const removeBtn = imageItem.querySelector(".image-remove-btn");
    if (removeBtn) {
      removeBtn.addEventListener("click", function () {
        removeImage(imageId);
      });
    }

    gallery.appendChild(imageItem);
  };

  reader.readAsDataURL(file);
}
