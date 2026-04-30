/**
 * KYC Page Script — Provider-Aware Verification Flow
 *
 * Supports two flows:
 * 1. Redirect Flow (Didit, Sumsub): Calls /api/kyc/initiate → redirects user to provider URL.
 * 2. Manual Flow: Shows the multi-step form for document upload + admin review.
 *
 * The active provider is detected via /api/kyc/provider at page load.
 */
function getCookie(name) {
  const prefix = `${name}=`;
  return document.cookie
    .split(";")
    .map((cookie) => cookie.trim())
    .find((cookie) => cookie.startsWith(prefix))
    ?.slice(prefix.length) || "";
}

document.addEventListener("alpine:init", () => {
  Alpine.data("kycForm", () => ({
    status: "loading",
    step: 1,
    submitting: false,
    uploading: false,
    uploadProgress: 0,
    dragActive: false,
    provider: "manual",
    supportsRedirect: false,
    validation: {},
    stepperSteps: ["Personal Info", "Additional Details", "Address", "Identity Doc", "Review & Submit"],
    formData: {
      firstName: "",
      lastName: "",
      dob: "",
      nationality: "",
      addressLine1: "",
      city: "",
      country: "",
      documentType: "passport",
      documentId: null,
      documentName: null,
      isPep: false,
    },

    async initKyc() {
      const returnedFromProvider = localStorage.getItem("poool_kyc_pending") === "true";
      if (returnedFromProvider) {
        localStorage.removeItem("poool_kyc_pending");
      }

      // 1. Fetch current KYC status
      try {
        const statusResp = await fetch("/api/kyc/status");
        if (statusResp.ok) {
          const data = await statusResp.json();
          this.status = data.status || "not_started";
          if (data.provider) this.provider = data.provider;
        } else if (statusResp.status === 401) {
          window.location.href = "/auth/login";
          return;
        } else {
          this.status = "error";
        }
      } catch (err) {
        if (typeof Sentry !== "undefined") Sentry.captureException(err);
        this.status = "error";
      }

      // 2. Detect the active provider
      try {
        const providerResp = await fetch("/api/kyc/provider");
        if (providerResp.ok) {
          const pdata = await providerResp.json();
          this.provider = pdata.provider || "manual";
          this.supportsRedirect = pdata.supports_redirect || false;
        }
      } catch (err) {
        // Provider fetch failed — keep manual default
      }
    },

    getStatusMessage() {
      switch (this.status) {
        case "not_started":
        case "pending":
          return "Please complete the steps below to verify your identity.";
        case "in_review":
          return "Your verification is currently under review.";
        case "approved":
          return "Your identity has been verified!";
        case "rejected":
          return "Your previous verification was rejected. Please try again.";
        case "expired":
          return "Your verification has expired. Please submit new documents.";
        default:
          return "";
      }
    },

    canStartVerification() {
      return (
        this.status === "not_started" ||
        this.status === "pending" ||
        this.status === "rejected" ||
        this.status === "expired"
      );
    },

    getProviderLabel() {
      const labels = {
        didit: "Didit",
        sumsub: "Sumsub",
        manual: "Manual Review",
      };
      return labels[this.provider] || this.provider;
    },

    getDocumentTypeLabel() {
      const labels = {
        passport: "Passport",
        national_id: "National ID",
        driving_licence: "Driver's License",
        driving_license: "Driver's License",
      };
      return labels[this.formData.documentType] || this.formData.documentType;
    },

    formatDate(dateStr) {
      if (!dateStr) return "—";
      try {
        const d = new Date(dateStr + "T00:00:00");
        return d.toLocaleDateString("en-GB", {
          day: "numeric",
          month: "long",
          year: "numeric",
        });
      } catch {
        return dateStr;
      }
    },

    getStepperClass(stepNumber) {
      if (this.step > stepNumber) return "completed";
      if (this.step === stepNumber) return "active";
      return "";
    },

    nextStep() {
      if (this.step < 5) {
        this.step++;
        this.validation = {};
      }
    },

    prevStep() {
      if (this.step > 1) {
        this.step--;
        this.validation = {};
      }
    },

    resetForm() {
      this.status = "not_started";
      this.step = 1;
      this.validation = {};
      this.formData = {
        firstName: "",
        lastName: "",
        dob: "",
        nationality: "",
        addressLine1: "",
        city: "",
        country: "",
        documentType: "passport",
        documentId: null,
        documentName: null,
        isPep: false,
      };
    },

    /** Validate current step fields, then advance. */
    validateAndNext(currentStep) {
      const errors = {};

      if (currentStep === 1) {
        if (!this.formData.firstName.trim()) errors.firstName = "First name is required";
        if (!this.formData.lastName.trim()) errors.lastName = "Last name is required";
      }

      if (currentStep === 2) {
        if (!this.formData.dob) errors.dob = "Date of birth is required";
        if (!this.formData.nationality.trim()) errors.nationality = "Nationality is required";
      }

      if (currentStep === 3) {
        if (!this.formData.addressLine1.trim()) errors.addressLine1 = "Address is required";
        if (!this.formData.city.trim()) errors.city = "City is required";
        if (!this.formData.country.trim()) errors.country = "Country is required";
      }

      if (currentStep === 4) {
        if (!this.formData.documentId) errors.documentId = "Please upload your identity document";
      }

      this.validation = errors;

      if (Object.keys(errors).length === 0) {
        this.nextStep();
      }
    },

    /** Handle file selection from input. */
    handleDocSelect(e) {
      if (e.target.files && e.target.files[0]) {
        this.uploadDocument(e.target.files[0]);
      }
    },

    /** Handle file drop. */
    handleDocDrop(e) {
      this.dragActive = false;
      const file = e.dataTransfer?.files[0];
      if (file) {
        this.uploadDocument(file);
      }
    },

    /** Upload document to GCS with real progress tracking. */
    async uploadDocument(file) {
      if (file.size > 10 * 1024 * 1024) {
        if (typeof showPooolToast === "function") {
          showPooolToast("File too large", "Maximum file size is 10MB.", "error");
        }
        return;
      }

      this.uploading = true;
      this.uploadProgress = 0;
      this.validation.documentId = "";

      const formData = new FormData();
      formData.append("file", file);
      formData.append("document_type", this.formData.documentType);

      try {
        const xhr = new XMLHttpRequest();

        const uploadPromise = new Promise((resolve, reject) => {
          xhr.upload.addEventListener("progress", (e) => {
            if (e.lengthComputable) {
              this.uploadProgress = Math.round((e.loaded / e.total) * 100);
            }
          });

          xhr.addEventListener("load", () => {
            if (xhr.status >= 200 && xhr.status < 300) {
              resolve(JSON.parse(xhr.responseText));
            } else {
              try {
                const err = JSON.parse(xhr.responseText);
                reject(new Error(err.error || "Upload failed"));
              } catch {
                reject(new Error("Upload failed"));
              }
            }
          });

          xhr.addEventListener("error", () => reject(new Error("Network error during upload")));
          xhr.addEventListener("abort", () => reject(new Error("Upload cancelled")));
        });

        xhr.open("POST", "/api/upload/kyc");
        const csrfToken = getCookie("csrf_token");
        if (csrfToken) {
          xhr.setRequestHeader("X-CSRF-Token", decodeURIComponent(csrfToken));
        }
        xhr.send(formData);

        const data = await uploadPromise;
        this.formData.documentId = data.document_id;
        this.formData.documentName = file.name;

        if (typeof showPooolToast === "function") {
          showPooolToast("Uploaded", "Document uploaded successfully.", "success");
        }
      } catch (err) {
        if (typeof showPooolToast === "function") {
          showPooolToast("Upload failed", err.message || "Please try again.", "error");
        }
      } finally {
        this.uploading = false;
      }
    },

    /** Remove uploaded document. */
    removeDocument() {
      this.formData.documentId = null;
      this.formData.documentName = null;
      this.uploadProgress = 0;
    },

    /**
     * Initiate KYC verification.
     */
    async submitKyc() {
      this.submitting = true;
      this.validation = {};
      try {
        if (this.supportsRedirect) {
          // Redirect-based flow (Didit / Sumsub)
          const response = await fetch("/api/kyc/initiate", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              document_type: this.formData.documentType,
            }),
          });

          if (response.ok) {
            const data = await response.json();
            if (data.verification_url) {
              localStorage.setItem("poool_kyc_pending", "true");
              window.location.href = data.verification_url;
              return;
            }
            this.status = "pending";
            if (typeof showPooolToast === "function") {
              showPooolToast("Submitted", "Your verification is being processed.", "success");
            }
          } else {
            const err = await response.json().catch(() => ({}));
            if (response.status === 409) {
              this.status = err.error?.includes("approved") ? "approved" : "pending";
              return;
            }
            if (typeof showPooolToast === "function") {
              showPooolToast("Error", err.error || "Failed to start verification.", "error");
            }
          }
        } else {
          // Manual submission flow
          const payload = {
            first_name: this.formData.firstName,
            last_name: this.formData.lastName,
            date_of_birth: this.formData.dob,
            nationality: this.formData.nationality,
            address_line1: this.formData.addressLine1,
            address_city: this.formData.city,
            address_country: this.formData.country,
            document_type: this.formData.documentType,
            document_id: this.formData.documentId,
            frontend_completed: true,
          };

          const response = await fetch("/api/kyc/submit", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(payload),
          });

          if (response.ok) {
            const data = await response.json().catch(() => ({}));
            this.status = data.status || "in_review";
            if (typeof showPooolToast === "function") {
              showPooolToast("Submitted", "KYC application submitted successfully.", "success");
            }
          } else {
            const err = await response.json().catch(() => ({}));
            if (typeof showPooolToast === "function") {
              showPooolToast("Error", err.error || "Failed to submit KYC. Please try again.", "error");
            }
          }
        }
      } catch (err) {
        if (typeof Sentry !== "undefined") Sentry.captureException(err);
        if (typeof showPooolToast === "function") {
          showPooolToast("Connection Error", "Unable to reach the server. Please try again.", "error");
        }
      } finally {
        this.submitting = false;
      }
    },
  }));
});
