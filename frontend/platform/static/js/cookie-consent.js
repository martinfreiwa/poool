/**
 * GDPR/ePrivacy Cookie Consent Banner Logic
 * Automatically injects the banner into the page if consent is not yet granted.
 */

document.addEventListener("DOMContentLoaded", () => {
  // Only show before login and if not already consented
  const consent = localStorage.getItem("poool_cookie_consent");
  if (!consent) {
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
  banner.innerHTML = `
        <div class="cookie-text-content">
            <h3 class="cookie-title">We value your privacy</h3>
            <p class="cookie-description">
                We use cookies to enhance your browsing experience, serve personalized ads or content, and analyze our traffic. By clicking "Accept All", you consent to our use of cookies. <a href="/cookies" class="cookie-link">Learn more</a>
            </p>
        </div>
        <div class="cookie-actions">
            <button class="cookie-btn cookie-btn-primary" id="btn-cookie-accept">Accept All</button>
            <button class="cookie-btn cookie-btn-secondary" id="btn-cookie-reject">Reject Non-Essential</button>
            <button class="cookie-btn cookie-btn-secondary" id="btn-cookie-customize" style="background: transparent; border: none; box-shadow: none; color: #535862; text-decoration: underline;">Customize</button>
        </div>
    `;

  document.body.appendChild(banner);

  // Create Modal
  const modal = document.createElement("div");
  modal.id = "cookie-preferences-modal";
  modal.innerHTML = `
        <div class="cookie-modal-content">
            <button class="cookie-modal-close" id="btn-cookie-close">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M18 6L6 18M6 6l12 12"></path>
                </svg>
            </button>
            <h2 style="font-size:20px; font-weight:600; color:#181D27; margin:0 0 16px 0;">Cookie Preferences</h2>
            <p style="font-size:14px; color:#535862; margin-bottom:24px; line-height:1.5;">Manage your cookie preferences below. Essential cookies are required for the website to function.</p>
            
            <div class="cookie-category">
                <div class="cookie-category-header">
                    <h4 class="cookie-category-title">Essential (Required)</h4>
                    <label class="cookie-toggle">
                        <input type="checkbox" checked disabled>
                        <span class="cookie-toggle-slider"></span>
                    </label>
                </div>
                <p class="cookie-category-desc">Necessary for technical functionality, security, and authentication. Cannot be disabled.</p>
            </div>

            <div class="cookie-category">
                <div class="cookie-category-header">
                    <h4 class="cookie-category-title">Analytics</h4>
                    <label class="cookie-toggle">
                        <input type="checkbox" id="chk-analytics" checked>
                        <span class="cookie-toggle-slider"></span>
                    </label>
                </div>
                <p class="cookie-category-desc">Helps us understand how visitors interact with the platform by collecting and reporting information anonymously.</p>
            </div>

            <div class="cookie-category">
                <div class="cookie-category-header">
                    <h4 class="cookie-category-title">Marketing</h4>
                    <label class="cookie-toggle">
                        <input type="checkbox" id="chk-marketing">
                        <span class="cookie-toggle-slider"></span>
                    </label>
                </div>
                <p class="cookie-category-desc">Used to track visitors across websites to display relevant advertisements.</p>
            </div>

            <button class="cookie-btn cookie-btn-primary" id="btn-cookie-save-prefs" style="width:100%;">Save Preferences</button>
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
