/**
 * POOOL – KYC Banner Controller (shared across all investor pages)
 *
 * Behaviour:
 *  - Fetches GET /api/kyc/status once on DOMContentLoaded.
 *  - Hides the banner entirely when KYC is already approved.
 *  - Adjusts the banner colour + message for each status:
 *      not_started → warning  (yellow)  "Complete KYC to invest"
 *      pending     → info     (blue)    "Verification under review"
 *      in_review   → info     (blue)    "Verification under review"
 *      rejected    → warning  (yellow)  "Verification rejected – try again"
 *      expired     → warning  (yellow)  "Verification expired – re-submit"
 *      approved    → banner hidden
 *  - "Complete KYC" button:
 *      · For redirect providers (Didit/Sumsub): calls POST /api/kyc/initiate,
 *        then redirects to the returned verification_url.
 *      · For the manual provider: navigates to /kyc.
 *      · If status is pending/in_review navigates directly to /kyc.
 *  - "Learn more" button: navigates to /kyc.
 *
 * Usage: <script src="/static/js/kyc-banner.js"></script>
 *
 * The script auto-discovers the banner by looking for the first element that
 * matches the selector `.kyc-banner` on the page, so it works on every page
 * without any extra configuration.
 *
 * Cart-specific: if the page has #kyc-summary-box and #payment-summary-box,
 * those are also toggled based on KYC status (kyc-summary-box is shown when
 * KYC is not approved; payment-summary-box is shown when approved).
 */
(function () {
    "use strict";

    // ── Configuration ─────────────────────────────────────────────────────────

    var BANNER_SELECTOR = ".kyc-banner";
    var STATUS_API = "/api/kyc/status";
    var INITIATE_API = "/api/kyc/initiate";
    var PROVIDER_API = "/api/kyc/provider";
    var KYC_PAGE = "/kyc";

    // CSS classes applied to the banner to change its colour scheme
    var CLASS_WARNING = "kyc-banner-warning";
    var CLASS_INFO = "kyc-banner-info";
    var CLASS_SUCCESS = "kyc-banner-success";

    // ── Helpers ───────────────────────────────────────────────────────────────

    /**
     * Find the first matching child of `banner` whose id *ends with* `suffix`.
     * This lets the script work with page-prefixed IDs like
     * `rewards-kyc-banner-icon`, `portfolio-kyc-banner-icon`, etc.
     */
    function findChild(banner, suffix) {
        // Try class selector first (modern standard)
        var cls = suffix;
        if (cls.startsWith("-")) cls = "kyc" + cls;
        var el = banner.querySelector("." + cls);
        if (el) return el;

        // Fallback to ID suffix (legacy)
        return banner.querySelector("[id$='" + suffix + "']");
    }

    function setColour(banner, colourClass) {
        banner.classList.remove(CLASS_WARNING, CLASS_INFO, CLASS_SUCCESS);
        banner.classList.add(colourClass);
    }

    function showBanner(banner) {
        banner.style.display = "";          // restore CSS-defined display
        banner.style.visibility = "visible";
        document.body.classList.remove("kyc-hidden");
    }

    function hideBanner(banner) {
        // Keep the space in the layout intact (content area relies on it) —
        // we simply make it invisible and zero-height so it doesn't push content.
        banner.style.display = "none";
        document.body.classList.add("kyc-hidden");
    }

    // ── KYC Banner Initialisation ─────────────────────────────────────────────

    function initBanner() {
        var banner = document.querySelector(BANNER_SELECTOR);
        if (!banner) {
            document.body.classList.add("kyc-hidden");
            return;   // Page has no banner
        }

        // Start hidden until we know status (prevents flash of banner on approved users)
        banner.style.display = "none";

        // Fetch KYC status
        fetch(STATUS_API, { credentials: "same-origin" })
            .then(function (res) {
                if (!res.ok) throw new Error("status " + res.status);
                return res.json();
            })
            .then(function (data) {
                applyStatus(banner, data);
            })
            .catch(function (err) {
                // Report to Sentry
                console.error("KYC status fetch failed:", err);
                if (typeof Sentry !== 'undefined') Sentry.captureException(err);

                // On error (e.g. not logged in), show the default warning banner
                // so we don't silently block users from seeing the KYC prompt.
                showBanner(banner);
                document.body.classList.remove("kyc-hidden");
                wireButtons(banner, "not_started", false);
            });
    }

    function applyStatus(banner, data) {
        var status = (data.status || "not_started").toLowerCase();

        // Build content based on status
        var iconSvgPath = "";
        var iconColour = "";
        var messageHtml = "";
        var colourClass = CLASS_WARNING;
        var showActions = true;

        // Cart-specific sidebar boxes
        var kycSidebarBox = document.getElementById("kyc-summary-box");
        var paymentSidebarBox = document.getElementById("payment-summary-box");

        switch (status) {
            case "approved":
                // Fully verified – hide banner completely
                hideBanner(banner);
                // Cart: show payment box, hide KYC box
                if (kycSidebarBox) kycSidebarBox.style.display = "none";
                if (paymentSidebarBox) paymentSidebarBox.style.display = "";
                // Expose for other scripts
                window.kycBannerStatus = "approved";
                return;

            case "pending":
            case "in_review":
                colourClass = CLASS_INFO;
                iconColour = "#1570EF";
                iconSvgPath = buildClockIcon(iconColour);
                messageHtml =
                    "Your identity verification is <strong>under review</strong>. " +
                    "We'll notify you once it's complete.";
                showActions = false;   // Nothing to do while pending
                break;

            case "rejected":
                colourClass = CLASS_WARNING;
                iconColour = "#DC6803";
                iconSvgPath = buildFlagIcon(iconColour);
                var reason = data.rejection_reason
                    ? " Reason: " + escapeHtml(data.rejection_reason) + "."
                    : "";
                messageHtml =
                    "Your verification was <strong>declined</strong>." + reason +
                    " Please <a href=\"" + KYC_PAGE + "\" class=\"kyc-banner-link\">" +
                    "re-submit your documents</a>.";
                break;

            case "expired":
                colourClass = CLASS_WARNING;
                iconColour = "#DC6803";
                iconSvgPath = buildFlagIcon(iconColour);
                messageHtml =
                    "Your identity verification has <strong>expired</strong>. " +
                    "Please <a href=\"" + KYC_PAGE + "\" class=\"kyc-banner-link\">" +
                    "re-submit your documents</a> to keep investing.";
                break;

            default: // not_started
                colourClass = CLASS_WARNING;
                iconColour = "#DC6803";
                iconSvgPath = buildFlagIcon(iconColour);
                messageHtml =
                    "You have to <a id=\"kyc-banner-inline-link\" href=\"" + KYC_PAGE +
                    "\" class=\"kyc-banner-link\">complete identity verification (KYC)</a>" +
                    " to buy or get property tokens. It takes 2 mins.";
                break;
        }

        // Apply colour scheme
        setColour(banner, colourClass);

        // Update icon (only if there's a discoverable icon element)
        var iconEl = findChild(banner, "-banner-icon");
        if (iconEl && iconSvgPath) {
            iconEl.innerHTML = iconSvgPath;
            // Adjust icon background for info colour
            if (colourClass === CLASS_INFO) {
                iconEl.style.background = "#D1E9FF";
            } else if (colourClass === CLASS_SUCCESS) {
                iconEl.style.background = "#D1FADF";
            } else {
                iconEl.style.background = "#FEF0C7";
            }
        }

        // Update message text
        var msgEl = findChild(banner, "-banner-message");
        if (msgEl) {
            msgEl.innerHTML = messageHtml;
        }

        // Show / hide actions panel
        var actionsEl = findChild(banner, "-banner-actions");
        if (actionsEl) {
            actionsEl.style.display = showActions ? "" : "none";
        }

        showBanner(banner);

        // Cart: show KYC box, hide payment box (not approved)
        if (kycSidebarBox) kycSidebarBox.style.display = "";
        if (paymentSidebarBox) paymentSidebarBox.style.display = "none";

        // Expose for other scripts
        window.kycBannerStatus = status;

        // Wire buttons (only relevant for actionable states)
        if (showActions) {
            wireButtons(banner, status, false);
        }
    }

    // ── Button Wiring ─────────────────────────────────────────────────────────

    function wireButtons(banner, status, supportsRedirect) {
        var completeBtn = findChild(banner, "-banner-complete-btn");
        var learnMoreBtn = findChild(banner, "-banner-learn-more-btn");

        if (learnMoreBtn) {
            learnMoreBtn.addEventListener("click", function () {
                window.location.href = KYC_PAGE;
            });
        }

        if (completeBtn) {
            completeBtn.addEventListener("click", function () {
                handleCompleteKyc(completeBtn, status);
            });
        }
    }

    function handleCompleteKyc(btn, status) {
        // If already pending/in-review, just navigate to the KYC status page
        if (status === "pending" || status === "in_review") {
            window.location.href = KYC_PAGE;
            return;
        }

        // Disable button while loading
        btn.disabled = true;
        var originalText = btn.querySelector(".kyc-banner-btn-text");
        var originalContent = originalText ? originalText.textContent : btn.textContent;

        if (originalText) {
            originalText.textContent = "Loading…";
        } else {
            btn.textContent = "Loading…";
        }

        // First check which provider is active
        fetch(PROVIDER_API, { credentials: "same-origin" })
            .then(function (r) { return r.ok ? r.json() : null; })
            .then(function (providerData) {
                var usesRedirect = providerData && providerData.supports_redirect;

                if (usesRedirect) {
                    // Call initiate to get a redirect URL
                    return fetch(INITIATE_API, {
                        method: "POST",
                        credentials: "same-origin",
                        headers: { "Content-Type": "application/json" },
                        body: JSON.stringify({ document_type: "passport" }),
                    })
                        .then(function (res) {
                            if (!res.ok) {
                                // 409 = already submitted
                                if (res.status === 409) {
                                    window.location.href = KYC_PAGE;
                                    return;
                                }
                                throw new Error("initiate failed: " + res.status);
                            }
                            return res.json();
                        })
                        .then(function (data) {
                            if (data && data.verification_url) {
                                window.location.href = data.verification_url;
                            } else {
                                // Fallback
                                window.location.href = KYC_PAGE;
                            }
                        });
                } else {
                    // Manual provider — just navigate to the KYC page
                    window.location.href = KYC_PAGE;
                }
            })
            .catch(function (err) {
                console.error("KYC initiate process failed:", err);
                if (typeof Sentry !== 'undefined') Sentry.captureException(err);

                // Re-enable on error
                btn.disabled = false;
                if (originalText) {
                    originalText.textContent = originalContent;
                } else {
                    btn.textContent = originalContent;
                }
                // Fallback navigation
                window.location.href = KYC_PAGE;
            });
    }

    // ── SVG Icon builders ─────────────────────────────────────────────────────

    function buildFlagIcon(colour) {
        return (
            '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" ' +
            'xmlns="http://www.w3.org/2000/svg">' +
            '<path d="M14.0914 6.72222H20.0451C20.5173 6.72222 20.7534 6.72222 ' +
            "20.8914 6.82149C21.0119 6.9081 21.0903 7.04141 21.1075 7.18877C21.1272 " +
            "7.35767 21.0126 7.56403 20.7833 7.97677L19.3624 10.5343C19.2793 10.684 " +
            "19.2377 10.7589 19.2214 10.8381C19.207 10.9083 19.207 10.9806 19.2214 " +
            "11.0508C19.2377 11.13 19.2793 11.2049 19.3624 11.3545L20.7833 13.9121C" +
            "21.0126 14.3248 21.1272 14.5312 21.1075 14.7001C21.0903 14.8475 21.0119 " +
            "14.9808 20.8914 15.0674C20.7534 15.1667 20.5173 15.1667 20.0451 15.1667H" +
            "12.6136C12.0224 15.1667 11.7269 15.1667 11.5011 15.0516C11.3024 14.9504 " +
            "11.141 14.7889 11.0398 14.5903C10.9247 14.3645 10.9247 14.0689 10.9247 " +
            "13.4778V10.9444M7.23027 21.5L3.00805 4.61111M4.59143 10.9444H12.4025C" +
            "12.9937 10.9444 13.2892 10.9444 13.515 10.8294C13.7137 10.7282 13.8751 " +
            "10.5667 13.9763 10.3681C14.0914 10.1423 14.0914 9.84672 14.0914 9.25556" +
            "V4.18889C14.0914 3.59772 14.0914 3.30214 13.9763 3.07634C13.8751 2.87773 " +
            "13.7137 2.71625 13.515 2.61505C13.2892 2.5 12.9937 2.5 12.4025 2.5H4.64335" +
            "C3.90602 2.5 3.53735 2.5 3.2852 2.65278C3.0642 2.78668 2.89999 2.99699 " +
            '2.82369 3.24387C2.73663 3.52555 2.82605 3.88321 3.00489 4.59852L4.59143 10.9444Z"' +
            ' stroke="' + colour + '" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>' +
            "</svg>"
        );
    }

    function buildClockIcon(colour) {
        return (
            '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" ' +
            'xmlns="http://www.w3.org/2000/svg">' +
            '<circle cx="12" cy="12" r="9" stroke="' + colour + '" stroke-width="2"/>' +
            '<path d="M12 7v5l3 3" stroke="' + colour + '" stroke-width="2" ' +
            'stroke-linecap="round" stroke-linejoin="round"/>' +
            "</svg>"
        );
    }

    function escapeHtml(str) {
        return String(str)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;");
    }

    // ── Bootstrap ─────────────────────────────────────────────────────────────

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", initBanner);
    } else {
        // DOM already ready (e.g. script loaded at bottom of body)
        initBanner();
    }
})();
