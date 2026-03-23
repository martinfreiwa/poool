/**
 * GDPR/ePrivacy Cookie Consent Banner Logic
 * Automatically injects the banner into the page if consent is not yet granted.
 */

document.addEventListener("DOMContentLoaded", () => {
  // Only show before login and if not already consented
  const consent = localStorage.getItem("poool_cookie_consent");
  const isAuthPage = window.location.pathname.includes('/auth/') || window.location.pathname.includes('login');
  
  // FIX BUG-002: Do not show cookie banner on auth pages as it blocks login
  if (!consent && !isAuthPage) {
    initCookieBanner();
  }
});

function initCookieBanner() {
  // Inject CSS
  const link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = "/static/css/cookie-consent.css";
  document.head.appendChild(link);

  // Create Banner
  const banner = document.createElement("div");
  banner.id = "cookie-consent-banner";
  banner.className = "ds-card";
  banner.innerHTML = `
        <div class="cookie-text-content">
            <h3 class="ds-text-md" style="margin: 0; color: var(--page-title-color, #181D27);">We value your privacy</h3>
            <p class="ds-text-body" style="color: var(--text-secondary, #535862); margin: 0;">
                We use cookies to enhance your browsing experience, serve personalized ads or content, and analyze our traffic. By clicking "Accept All", you consent to our use of cookies. <a href="/cookies" style="color: var(--primary-color, #0000FF); text-decoration: none; font-weight: 500;">Learn more</a>
            </p>
        </div>
        <div class="cookie-actions">
            <button class="ds-btn ds-btn--primary" id="btn-cookie-accept" style="flex: 1;">Accept All</button>
            <button class="ds-btn ds-btn--secondary" id="btn-cookie-reject" style="flex: 1;">Reject Non-Essential</button>
            <button class="ds-btn ds-btn--ghost" id="btn-cookie-customize" style="flex: 1;">Customize</button>
        </div>
    `;

  document.body.appendChild(banner);

  // Create Modal
  const modal = document.createElement("div");
  modal.id = "cookie-preferences-modal";
  modal.innerHTML = `
        <div class="cookie-modal-container">
            <button class="cookie-modal-close" id="btn-cookie-close">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M18 6L6 18M6 6l12 12"></path>
                </svg>
            </button>
            <h2 class="ds-text-md" style="margin: 0 0 16px 0; color: var(--page-title-color, #181D27);">Cookie Preferences</h2>
            <p class="ds-text-body" style="color: var(--text-secondary, #535862); margin-bottom: 24px; line-height: 1.5;">Manage your cookie preferences below. Essential cookies are required for the website to function.</p>
            
            <div class="cookie-category">
                <div class="cookie-category-header">
                    <h4 class="ds-text-sm-heading" style="margin: 0; color: var(--page-title-color, #181D27);">Essential (Required)</h4>
                    <label class="cookie-toggle">
                        <input type="checkbox" checked disabled>
                        <span class="cookie-toggle-slider"></span>
                    </label>
                </div>
                <p class="ds-text-caption" style="color: var(--text-secondary, #535862); margin: 0; line-height: 1.5;">Necessary for technical functionality, security, and authentication. Cannot be disabled.</p>
            </div>

            <div class="cookie-category">
                <div class="cookie-category-header">
                    <h4 class="ds-text-sm-heading" style="margin: 0; color: var(--page-title-color, #181D27);">Analytics</h4>
                    <label class="cookie-toggle">
                        <input type="checkbox" id="chk-analytics" checked>
                        <span class="cookie-toggle-slider"></span>
                    </label>
                </div>
                <p class="ds-text-caption" style="color: var(--text-secondary, #535862); margin: 0; line-height: 1.5;">Helps us understand how visitors interact with the platform by collecting and reporting information anonymously.</p>
            </div>

            <div class="cookie-category" style="border-bottom: none; margin-bottom: 24px;">
                <div class="cookie-category-header">
                    <h4 class="ds-text-sm-heading" style="margin: 0; color: var(--page-title-color, #181D27);">Marketing</h4>
                    <label class="cookie-toggle">
                        <input type="checkbox" id="chk-marketing">
                        <span class="cookie-toggle-slider"></span>
                    </label>
                </div>
                <p class="ds-text-caption" style="color: var(--text-secondary, #535862); margin: 0; line-height: 1.5;">Used to track visitors across websites to display relevant advertisements.</p>
            </div>

            <button class="ds-btn ds-btn--primary" id="btn-cookie-save-prefs" style="width:100%;">Save Preferences</button>
        </div>
    `;

  document.body.appendChild(modal);

  // Add listeners
  setTimeout(() => {
    banner.classList.add("visible");
  }, 100);

  document.getElementById("btn-cookie-accept").addEventListener("click", () => {
    saveConsent({ essential: true, analytics: true, marketing: true });
  });

  document.getElementById("btn-cookie-reject").addEventListener("click", () => {
    saveConsent({ essential: true, analytics: false, marketing: false });
  });

  document
    .getElementById("btn-cookie-customize")
    .addEventListener("click", () => {
      modal.classList.add("active");
    });

  document.getElementById("btn-cookie-close").addEventListener("click", () => {
    modal.classList.remove("active");
  });

  document
    .getElementById("btn-cookie-save-prefs")
    .addEventListener("click", () => {
      saveConsent({
        essential: true,
        analytics: document.getElementById("chk-analytics").checked,
        marketing: document.getElementById("chk-marketing").checked,
      });
      modal.classList.remove("active");
    });
}

function saveConsent(preferences) {
  // 1. Save to Local Storage
  const consentObj = {
    granted_at: new Date().toISOString(),
    preferences: preferences,
  };
  localStorage.setItem("poool_cookie_consent", JSON.stringify(consentObj));

  // 2. Hide Banner
  const banner = document.getElementById("cookie-consent-banner");
  if (banner) {
    banner.classList.remove("visible");
    setTimeout(() => banner.remove(), 400);
  }

  // 3. Send async audit payload to backend if needed
  // fetch('/api/audit/cookie-consent', { ... })
}
