// File Upload Functionality
// Alpine.js component for file upload handling

document.addEventListener("alpine:init", () => {
  Alpine.data("fileUpload", (type) => ({
    files: [],
    dragActive: false,

    init() {
      // Set up drag and drop event listeners
      this.setupDragAndDrop();
    },

    setupDragAndDrop() {
      const uploadArea = this.$el;
      const dragOverlay = uploadArea.querySelector(".drag-overlay");

      // Prevent default drag behaviors
      ["dragenter", "dragover", "dragleave", "drop"].forEach((eventName) => {
        uploadArea.addEventListener(eventName, this.preventDefaults, false);
        document.body.addEventListener(eventName, this.preventDefaults, false);
      });

      // Highlight drop area when item is dragged over it
      ["dragenter", "dragover"].forEach((eventName) => {
        uploadArea.addEventListener(
          eventName,
          () => {
            this.dragActive = true;
            if (dragOverlay) dragOverlay.style.display = "flex";
          },
          false,
        );
      });

      ["dragleave", "drop"].forEach((eventName) => {
        uploadArea.addEventListener(
          eventName,
          () => {
            this.dragActive = false;
            if (dragOverlay) dragOverlay.style.display = "none";
          },
          false,
        );
      });

      // Handle dropped files
      uploadArea.addEventListener("drop", this.handleDrop.bind(this), false);
    },

    preventDefaults(e) {
      e.preventDefault();
      e.stopPropagation();
    },

    handleDrop(e) {
      const dt = e.dataTransfer;
      const files = dt.files;
      this.handleFiles(files);
    },

    handleFileSelect(e) {
      const files = e.target.files;
      this.handleFiles(files);
    },

    handleFiles(fileList) {
      const files = Array.from(fileList);
      const maxFiles = type === "images" ? 16 : 10;

      // Check file limits
      if (this.files.length + files.length > maxFiles) {
        this.showNotification(`Maximum ${maxFiles} files allowed`, "error");
        return;
      }

      files.forEach((file) => {
        if (this.validateFile(file, type)) {
          this.addFile(file);
        }
      });
    },

    validateFile(file, type) {
      const maxSize = type === "images" ? 2 * 1024 * 1024 : 10 * 1024 * 1024; // 2MB for images, 10MB for documents
      const allowedTypes = {
        images: ["image/webp", "image/jpg", "image/webp"],
        documents: [
          "application/pdf",
          "application/msword",
          "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ],
      };

      // Check file size
      if (file.size > maxSize) {
        this.showNotification(
          `File too large. Maximum size: ${this.formatFileSize(maxSize)}`,
          "error",
        );
        return false;
      }

      // Check file type
      if (!allowedTypes[type].includes(file.type)) {
        this.showNotification(
          `Invalid file type. Allowed: ${this.getAllowedExtensions(type)}`,
          "error",
        );
        return false;
      }

      return true;
    },

    addFile(file) {
      const fileId = `${type}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      const fileObj = {
        id: fileId,
        file: file,
        name: file.name,
        size: this.formatFileSize(file.size),
        type: this.getFileExtension(file.name),
        progress: 0,
        status: "uploading",
      };

      this.files.push(fileObj);

      // Simulate upload progress
      this.simulateUpload(fileObj);

      // Add to DOM if it's an image
      if (type === "images") {
        this.addImageToGallery(fileObj);
      } else {
        this.addDocumentToList(fileObj);
      }
    },

    simulateUpload(fileObj) {
      const progressInterval = setInterval(() => {
        fileObj.progress += Math.random() * 20;

        if (fileObj.progress >= 100) {
          fileObj.progress = 100;
          fileObj.status = "completed";
          clearInterval(progressInterval);
          this.updateProgressDisplay(fileObj);
        } else {
          this.updateProgressDisplay(fileObj);
        }
      }, 200);
    },

    updateProgressDisplay(fileObj) {
      const progressBar = document.querySelector(
        `#${fileObj.id} .ds-progress__fill`,
      );
      const progressText = document.querySelector(
        `#${fileObj.id} .progress-percentage`,
      );

      if (progressBar) {
        progressBar.style.width = `${fileObj.progress}%`;
      }
      if (progressText) {
        progressText.textContent = `${Math.round(fileObj.progress)}%`;
      }
    },

    addImageToGallery(fileObj) {
      const gallery = document.getElementById("image-gallery");
      if (!gallery) return;

      const imageContainer = document.createElement("div");
      imageContainer.id = fileObj.id;
      imageContainer.className = "uploaded-image-item";

      const reader = new FileReader();
      reader.onload = (e) => {
        imageContainer.innerHTML = `
                    <img src="${e.target.result}" alt="${fileObj.name}" class="uploaded-image"/>
                    <button type="button" class="image-remove-btn" onclick="removeImage('${fileObj.id}')" aria-label="Remove image">
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                            <path d="M10.5 3.5L3.5 10.5" stroke="white" stroke-width="1.16667" stroke-linecap="round" stroke-linejoin="round"/>
                            <path d="M3.5 3.5L10.5 10.5" stroke="white" stroke-width="1.16667" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                    </button>
                `;
      };
      reader.readAsDataURL(fileObj.file);

      gallery.appendChild(imageContainer);
    },

    addDocumentToList(fileObj) {
      const documentList = document.getElementById("document-list");
      if (!documentList) return;

      const docContainer = document.createElement("div");
      docContainer.id = fileObj.id;
      docContainer.className = "document-item";
      docContainer.innerHTML = `
                <div class="document-icon">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                        <path d="M14 2H6C5.46957 2 4.96086 2.21071 4.58579 2.58579C4.21071 2.96086 4 3.46957 4 4V20C4 20.5304 4.21071 21.0391 4.58579 21.4142C4.96086 21.7893 5.46957 22 6 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V8L14 2Z" stroke="#D92D20" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                        <path d="M14 2V8H20" stroke="#D92D20" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                        <text x="12" y="16" text-anchor="middle" font-family="TT Norms Pro" font-size="6" font-weight="700" fill="#D92D20">${fileObj.type.toUpperCase()}</text>
                    </svg>
                </div>
                <div class="document-info">
                    <span class="document-name">${fileObj.name}</span>
                    <span class="document-size">${fileObj.size}</span>
                </div>
                <div class="document-progress">
                    <div class="ds-progress ds-progress--sm">
                        <div class="ds-progress__fill" style="width: ${fileObj.progress}%"></div>
                    </div>
                    <span class="progress-percentage">${Math.round(fileObj.progress)}%</span>
                </div>
                <button type="button" class="document-delete" onclick="removeDocument('${fileObj.id}')" aria-label="Delete document">
                    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                        <path d="M2.5 5H4.16667H17.5" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
                        <path d="M6.66667 5V3.33333C6.66667 2.89131 6.84226 2.46738 7.15482 2.15482C7.46738 1.84226 7.89131 1.66667 8.33333 1.66667H11.6667C12.1087 1.66667 12.5326 1.84226 12.8452 2.15482C13.1577 2.46738 13.3333 2.89131 13.3333 3.33333V5M15.8333 5V16.6667C15.8333 17.1087 15.6577 17.5326 15.3452 17.8452C15.0326 18.1577 14.6087 18.3333 14.1667 18.3333H5.83333C5.39131 18.3333 4.96738 18.1577 4.65482 17.8452C4.34226 17.5326 4.16667 17.1087 4.16667 16.6667V5H15.8333Z" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
                        <path d="M8.33333 9.16667V14.1667" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
                        <path d="M11.6667 9.16667V14.1667" stroke="currentColor" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                </button>
            `;

      documentList.appendChild(docContainer);
    },

    removeFile(fileId) {
      this.files = this.files.filter((file) => file.id !== fileId);
      const element = document.getElementById(fileId);
      if (element) {
        element.remove();
      }
    },

    formatFileSize(bytes) {
      if (bytes === 0) return "0 Bytes";
      const k = 1024;
      const sizes = ["Bytes", "KB", "MB", "GB"];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
    },

    getFileExtension(filename) {
      return filename.split(".").pop().toLowerCase();
    },

    getAllowedExtensions(type) {
      const extensions = {
        images: "JPG, JPEG, PNG",
        documents: "PDF, DOC, DOCX",
      };
      return extensions[type] || "";
    },

    showNotification(message, type) {
      // Use the notification function from property-content.js
      if (window.showNotification) {
        window.showNotification(message, type);
      } else {
        alert(message);
      }
    },
  }));
});

// Global functions for remove buttons (called from template)
window.removeImage = function (imageId) {
  const element = document.getElementById(imageId);
  if (element) {
    element.remove();
    updateImageCount();
  }
};

window.removeDocument = function (docId) {
  const element = document.getElementById(docId);
  if (element) {
    element.remove();
  }
};

window.removeFile = function (fileId) {
  const element = document.getElementById(fileId);
  if (element) {
    element.remove();
  }
};

// Update image count display
function updateImageCount() {
  const images = document.querySelectorAll(".uploaded-image-item");
  const subtitle = document.getElementById("media-subtitle");
  if (subtitle) {
    const count = images.length;
    if (count < 8) {
      subtitle.style.color = "#F04438";
      subtitle.textContent = `Please upload 8-16 photos (Currently: ${count})`;
    } else if (count > 16) {
      subtitle.style.color = "#F04438";
      subtitle.textContent = `Please upload 8-16 photos (Currently: ${count} - Too many)`;
    } else {
      subtitle.style.color = "#535862";
      subtitle.textContent = `Please upload 8-16 photos (Currently: ${count})`;
    }
  }
}
