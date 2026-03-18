/**
 * KYC Page Script — Provider-Aware Verification Flow
 *
 * Supports two flows:
 * 1. Redirect Flow (Didit, Sumsub): Calls /api/kyc/initiate → redirects user to provider URL.
 * 2. Manual Flow: Shows the multi-step form for document upload + admin review.
 *
 * The active provider is detected via /api/kyc/provider at page load.
 */
document.addEventListener("alpine:init", () => {
  Alpine.data("kycForm", () => ({
    status: "loading", // not_started, pending, in_review, approved, rejected, expired
    step: 1,
    submitting: false,
    uploading: false,
    uploadProgress: 0,
    dragActive: false,
    provider: "manual", // "didit", "sumsub", or "manual"
    supportsRedirect: false, // true if the provider uses redirect-based verification
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
      // Check if we're returning from provider redirect
      const params = new URLSearchParams(window.location.search);
      if (params.get("completed") === "true") {
        // User was redirected back from Didit/Sumsub — they completed the flow
        this.status = "in_review";
        // Clean URL
        history.replaceState(null, "", "/kyc");
        return;
      }

      try {
        // 1. Fetch current KYC status
        const statusResp = await fetch("/api/kyc/status");
        if (statusResp.ok) {
          const data = await statusResp.json();
          this.status = data.status || "not_started";
          if (data.provider) this.provider = data.provider;
        } else if (statusResp.status === 401) {
          // Not authenticated — redirect to login
          window.location.href = "/auth/login";
          return;
        } else {
          console.warn("KYC status API returned:", statusResp.status);
          this.status = "not_started";
        }

        // 2. Detect the active provider
        const providerResp = await fetch("/api/kyc/provider");
        if (providerResp.ok) {
          const pdata = await providerResp.json();
          this.provider = pdata.provider || "manual";
          this.supportsRedirect = pdata.supports_redirect || false;
        }
      } catch (err) {
        console.error("KYC initialization failed:", err);
        if (typeof Sentry !== 'undefined') Sentry.captureException(err);
        this.status = "error";
      }
    },

    getStatusMessage() {
      switch (this.status) {
        case "not_started":
        case "pending":  // session opened but not yet submitted
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

    /** Whether the user can start a new verification. */
    canStartVerification() {
      return (
        this.status === "not_started" ||
        this.status === "pending" ||   // stale session — backend will clean it up
        this.status === "rejected" ||
        this.status === "expired"
      );
    },

    /** Get the label for the provider badge. */
    getProviderLabel() {
      const labels = {
        didit: "Didit",
        sumsub: "Sumsub",
        manual: "Manual Review",
      };
      return labels[this.provider] || this.provider;
    },

    getStepperClass(stepNumber) {
      if (this.step > stepNumber) return "completed";
      if (this.step === stepNumber) return "active";
      return "";
    },

    nextStep() {
      if (this.step < 5) {
        this.step++;
      }
    },

    prevStep() {
      if (this.step > 1) {
        this.step--;
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

    /** Upload document to GCS. */
    async uploadDocument(file) {
      if (file.size > 10 * 1024 * 1024) {
        alert("File too large. Max 10MB.");
        return;
      }

      this.uploading = true;
      this.uploadProgress = 0;

      const formData = new FormData();
      formData.append("file", file);
      formData.append("document_type", this.formData.documentType);

      try {
        const response = await fetch("/api/upload/kyc", {
          method: "POST",
          body: formData,
        });

        if (response.ok) {
          const data = await response.json();
          this.formData.documentId = data.document_id;
          this.formData.documentName = file.name;
        } else {
          const err = await response.json().catch(() => ({}));
          alert(err.error || "Failed to upload document.");
        }
      } catch (err) {
        console.error("Upload failed:", err);
        alert("Network error during upload.");
      } finally {
        this.uploading = false;
        this.uploadProgress = 100;
      }
    },

    /** Remove uploaded document. */
    removeDocument() {
      this.formData.documentId = null;
      this.formData.documentName = null;
    },

    /**
     * Initiate KYC verification.
     *
     * For redirect providers (Didit/Sumsub):
     *   Calls POST /api/kyc/initiate → gets a verification_url → redirects.
     *
     * For manual provider:
     *   Calls POST /api/kyc/submit (legacy flow with form data).
     */
    async submitKyc() {
      this.submitting = true;
      try {
        if (this.supportsRedirect) {
          // === Redirect-based flow (Didit / Sumsub) ===
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
              // Redirect user to the provider verification page
              window.location.href = data.verification_url;
              return;
            }
            // Fallback: no URL means manual-like flow
            this.status = "pending";
          } else {
            const err = await response.json().catch(() => ({}));
            if (response.status === 409) {
              // Already pending or approved
              this.status = err.error?.includes("approved")
                ? "approved"
                : "pending";
              return;
            }
            alert(
              err.error ||
              "Failed to start KYC verification. Please try again.",
            );
          }
        } else {
          // === Manual submission flow ===
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
            this.status = "pending";
            if (window.showNotification) {
              window.showNotification(
                "Success",
                "KYC application submitted successfully",
                "success",
              );
            }
          } else {
            const errorResponse = await response.text();
            alert("Failed to submit KYC. Please try again.");
          }
        }
      } catch (err) {
        console.error("KYC submission failed:", err);
        if (typeof Sentry !== 'undefined') Sentry.captureException(err);
        alert("Failed to connect to the server.");
      } finally {
        this.submitting = false;
      }
    },
  }));
});
