/**
 * POOOL — Legal Page Enhancements & Cookie Consent
 * Adds: Table of Contents, Back-to-Top button, Print-friendly styles, Cookie Consent Banner
 * Include this script on all legal pages AND on all platform pages (for cookie consent).
 */
(function () {
  "use strict";

  // ── TABLE OF CONTENTS (Legal pages only) ──────────────────────────────────
  function buildTableOfContents() {
    const main = document.getElementById("legal-main");
    if (!main) return;

    const headings = main.querySelectorAll("h2");
    if (headings.length < 3) return;

    // Add IDs to headings for anchor links
    headings.forEach((h, i) => {
      if (!h.id) h.id = "section-" + (i + 1);
    });

    const tocContainer = document.createElement("div");
    tocContainer.id = "legal-toc";
    tocContainer.style.cssText = `
            background:#fff; border:1px solid #E9EAEB; border-radius:12px; padding:24px 28px;
            margin-bottom:24px; line-height:1.8;
        `;

    const tocTitle = document.createElement("h3");
    tocTitle.textContent = "Table of Contents";
    tocTitle.style.cssText =
      "font-size:15px; font-weight:700; color:#181D27; margin:0 0 12px 0;";
    tocContainer.appendChild(tocTitle);

    const tocList = document.createElement("ol");
    tocList.style.cssText = "padding-left:20px; margin:0; list-style:decimal;";
    headings.forEach((h) => {
      const li = document.createElement("li");
      li.style.marginBottom = "4px";
      const a = document.createElement("a");
      a.href = "#" + h.id;
      a.textContent = h.textContent.replace(/^\d+\.\s*/, "");
      a.style.cssText =
        "color:#4A7DFF; text-decoration:none; font-size:14px; font-weight:500;";
      a.addEventListener(
        "mouseenter",
        () => (a.style.textDecoration = "underline"),
      );
      a.addEventListener("mouseleave", () => (a.style.textDecoration = "none"));
      a.addEventListener("click", (e) => {
        e.preventDefault();
        const target = document.getElementById(h.id);
        if (target) {
          target.scrollIntoView({ behavior: "smooth", block: "start" });
          history.replaceState(null, "", "#" + h.id);
        }
      });
      li.appendChild(a);
      tocList.appendChild(li);
    });
    tocContainer.appendChild(tocList);

    // Insert after the page header (h1 parent div)
    const contentDiv = main.querySelector("div > div:first-child");
    if (contentDiv && contentDiv.nextElementSibling) {
      contentDiv.parentNode.insertBefore(
        tocContainer,
        contentDiv.nextElementSibling,
      );
    }
  }

  // ── BACK TO TOP BUTTON (Legal pages only) ─────────────────────────────────
  function addBackToTopButton() {
    const main = document.getElementById("legal-main");
    if (!main) return;

    const btn = document.createElement("button");
    btn.id = "back-to-top-btn";
    btn.innerHTML = `<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 16V4"/><path d="M4 10l6-6 6 6"/></svg>`;
    btn.setAttribute("aria-label", "Back to top");
    btn.style.cssText = `
            position:fixed; bottom:28px; right:28px; z-index:9990;
            width:44px; height:44px; border-radius:50%; border:none;
            background:#4A7DFF; color:#fff; cursor:pointer;
            display:none; align-items:center; justify-content:center;
            box-shadow:0 4px 16px rgba(74,125,255,0.35);
            transition:opacity 0.3s, transform 0.3s;
            opacity:0; transform:translateY(10px);
        `;
    document.body.appendChild(btn);

    btn.addEventListener("click", () => {
      window.scrollTo({ top: 0, behavior: "smooth" });
    });

    let ticking = false;
    window.addEventListener("scroll", () => {
      if (!ticking) {
        requestAnimationFrame(() => {
          if (window.scrollY > 400) {
            btn.style.display = "flex";
            requestAnimationFrame(() => {
              btn.style.opacity = "1";
              btn.style.transform = "translateY(0)";
            });
          } else {
            btn.style.opacity = "0";
            btn.style.transform = "translateY(10px)";
            setTimeout(() => {
              if (window.scrollY <= 400) btn.style.display = "none";
            }, 300);
          }
          ticking = false;
        });
        ticking = true;
      }
    });
  }

  // ── PRINT-FRIENDLY STYLES ─────────────────────────────────────────────────
  function addPrintStyles() {
    const style = document.createElement("style");
    style.textContent = `
            @media print {
                .sidebar, .mobile-header, .mobile-burger-menu, .mobile-menu-overlay,
                #back-to-top-btn, #cookie-consent-banner, #legal-toc,
                .profile-dropdown, .community-card, template { display:none !important; }
                body { background:#fff !important; }
                main { margin:0 !important; padding:20px !important; }
                main > div { max-width:100% !important; padding:0 !important; }
                a { color:#333 !important; text-decoration:none !important; }
                h1, h2, h3 { page-break-after:avoid; }
                p, li { orphans:3; widows:3; }
            }
        `;
    document.head.appendChild(style);
  }

  // ── COOKIE CONSENT BANNER (All platform pages) ────────────────────────────
  function showCookieConsent() {
    const consentKey = "poool_cookie_consent";
    const savedConsent = localStorage.getItem(consentKey);
    if (savedConsent) return; // Already consented

    const banner = document.createElement("div");
    banner.id = "cookie-consent-banner";
    banner.className = "ds-card";
    banner.style.cssText = `
            position:fixed; bottom:24px; right:24px; z-index:99999;
            max-width:440px; width:calc(100% - 48px);
            padding:20px 24px; display:flex; flex-direction:column; gap:16px;
            transform:translateY(calc(100% + 32px)); opacity:0;
            transition:transform 0.4s cubic-bezier(0.16,1,0.3,1), opacity 0.4s ease;
        `;

    banner.innerHTML = `
            <style>
                @media(max-width:480px){
                    #cookie-consent-banner { left:16px; right:16px; width:auto; max-width:none; }
                    #cookie-consent-banner .cookie-actions { flex-direction:column; }
                    #cookie-consent-banner .cookie-actions button { width:100%; }
                }
                #cookie-consent-banner a { color:#0000FF; text-decoration:none; font-weight:500; }
                #cookie-consent-banner a:hover { text-decoration:underline; }
            </style>
            <div>
                <p style="margin:0 0 4px; font-family:'TT Norms Pro','Segoe UI',system-ui,sans-serif; font-size:14px; font-weight:700; color:#181D27; line-height:1.4;">We use cookies</p>
                <p style="margin:0; font-family:'TT Norms Pro','Segoe UI',system-ui,sans-serif; font-size:13px; color:#535862; line-height:1.6;">
                    We use essential cookies for the platform to function and analytics cookies to understand how you use our services.
                    <a href="/cookies">Learn more</a>
                </p>
            </div>
            <div class="cookie-actions" style="display:flex; gap:8px; flex-shrink:0;">
                <button class="ds-btn ds-btn--secondary ds-btn--sm" id="cookie-reject" style="flex:1;">Essential only</button>
                <button class="ds-btn ds-btn--primary ds-btn--sm" id="cookie-accept" style="flex:1;">Accept all</button>
            </div>
        `;
    document.body.appendChild(banner);

    requestAnimationFrame(() => {
      banner.style.transform = "translateY(0)";
      banner.style.opacity = "1";
    });

    function dismissBanner(preferences) {
      localStorage.setItem(consentKey, JSON.stringify({ ...preferences, ts: Date.now() }));
      // Phase-2 P0: mirror to server-readable cookie so the backend can
      // gate behavioural cookies (affiliate `?ref=` attribution lives in
      // `marketing`). Format matches `cookie-consent.js` + the Rust
      // parser `rewards::attribution::has_marketing_consent`.
      var flags = ["essential"];
      if (preferences && preferences.analytics) flags.push("analytics");
      if (preferences && preferences.marketing) flags.push("marketing");
      var maxAgeSecs = 60 * 60 * 24 * 180;
      var secure = location.protocol === "https:" ? "; Secure" : "";
      document.cookie =
        "poool_consent=" + flags.join("+") + "; Path=/; Max-Age=" + maxAgeSecs +
        "; SameSite=Lax" + secure;
      banner.style.transform = "translateY(calc(100% + 32px))";
      banner.style.opacity = "0";
      setTimeout(() => banner.remove(), 400);
    }

    document.getElementById("cookie-accept").addEventListener("click", () => {
      dismissBanner({ essential: true, analytics: true, marketing: false });
    });

    document.getElementById("cookie-reject").addEventListener("click", () => {
      dismissBanner({ essential: true, analytics: false, marketing: false });
    });
  }

  // ── INITIALIZE ────────────────────────────────────────────────────────────
  function init() {
    buildTableOfContents();
    addBackToTopButton();
    addPrintStyles();
    injectPlatformFooter();
    showCookieConsent();
  }

  // ── PLATFORM FOOTER (Legal links) ─────────────────────────────────────────
  // Injects a minimal legal footer into all platform pages.
  // Skips legal pages (already have full content) and auth pages.
  function injectPlatformFooter() {
    const skipPaths = [
      "/auth/",
      "/terms",
      "/privacy-policy",
      "/currency-policy",
      "/cookies",
      "/signup",
      "/",
    ];
    if (
      skipPaths.some(
        (p) =>
          window.location.pathname.startsWith(p) ||
          window.location.pathname === p,
      )
    )
      return;
    if (document.getElementById("platform-legal-footer")) return;

    // Inject after the main content wrapper closes
    const footer = document.createElement("footer");
    footer.id = "platform-legal-footer";
    footer.style.cssText = `
            padding:16px 24px; text-align:center; font-size:12px; color:#94989F;
            font-family:'TT Norms Pro','Segoe UI',system-ui,sans-serif;
            border-top:1px solid #E9EAEB; margin-top:32px;
            background:#F9FAFB;
        `;
    footer.innerHTML = `
            <nav aria-label="Legal" style="display:flex;flex-wrap:wrap;justify-content:center;gap:4px 16px;margin-bottom:6px;">
                <a href="/terms" style="color:#6B7280;text-decoration:none;font-weight:500;" onmouseover="this.style.color='#374151'" onmouseout="this.style.color='#6B7280'">Terms &amp; Conditions</a>
                <a href="/privacy-policy" style="color:#6B7280;text-decoration:none;font-weight:500;" onmouseover="this.style.color='#374151'" onmouseout="this.style.color='#6B7280'">Privacy Policy</a>
                <a href="/cookies" style="color:#6B7280;text-decoration:none;font-weight:500;" onmouseover="this.style.color='#374151'" onmouseout="this.style.color='#6B7280'">Cookie Policy</a>
                <a href="/currency-policy" style="color:#6B7280;text-decoration:none;font-weight:500;" onmouseover="this.style.color='#374151'" onmouseout="this.style.color='#6B7280'">Currency Policy</a>
                <a href="/support" style="color:#6B7280;text-decoration:none;font-weight:500;" onmouseover="this.style.color='#374151'" onmouseout="this.style.color='#6B7280'">Support</a>
            </nav>
            <span>&copy; ${new Date().getFullYear()} PT. POOOL INTERNATIONAL GROUP &mdash; All rights reserved.</span>
        `;

    // Insert before closing </body> or after last child
    document.body.appendChild(footer);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
