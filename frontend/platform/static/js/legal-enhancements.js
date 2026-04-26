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
    banner.style.cssText = `
            position:fixed; bottom:0; left:0; right:0; z-index:99999;
            background:rgba(24,29,39,0.97); backdrop-filter:blur(12px);
            color:#E9EAEB; padding:20px 24px;
            font-family:'TT Norms Pro','Segoe UI',system-ui,sans-serif; font-size:14px; line-height:1.6;
            display:flex; flex-wrap:wrap; align-items:center; gap:16px;
            box-shadow:0 -4px 24px rgba(0,0,0,0.25);
            animation:cookieSlideUp 0.4s ease-out;
        `;

    banner.innerHTML = `
            <style>
                @keyframes cookieSlideUp { from { transform:translateY(100%); opacity:0; } to { transform:translateY(0); opacity:1; } }
                #cookie-consent-banner a { color:#7BA3FF; text-decoration:underline; }
                #cookie-consent-banner a:hover { color:#A3C0FF; }
                .cookie-btn {
                    padding:8px 20px; border-radius:8px; font-size:13px; font-weight:600;
                    cursor:pointer; border:none; font-family:inherit; transition:all 0.2s;
                }
                .cookie-btn--accept { background:#4A7DFF; color:#fff; }
                .cookie-btn--accept:hover { background:#3A6DE8; }
                .cookie-btn--reject { background:transparent; color:#A4A7AE; border:1px solid #535862; }
                .cookie-btn--reject:hover { background:rgba(255,255,255,0.05); color:#E9EAEB; }
                .cookie-btn--customize { background:transparent; color:#A4A7AE; border:1px solid #535862; }
                .cookie-btn--customize:hover { background:rgba(255,255,255,0.05); color:#E9EAEB; }
            </style>
            <div style="flex:1; min-width:280px;">
                <strong style="color:#fff;">🍪 We use cookies</strong>
                <p style="margin:4px 0 0; color:#A4A7AE; font-size:13px;">
                    We use essential cookies for the platform to function and analytics cookies to understand how you use our services.
                    <a href="/cookies">Learn more</a>
                </p>
            </div>
            <div style="display:flex; gap:8px; flex-shrink:0;">
                <button class="cookie-btn cookie-btn--reject" id="cookie-reject">Essential only</button>
                <button class="cookie-btn cookie-btn--accept" id="cookie-accept">Accept all</button>
            </div>
        `;
    document.body.appendChild(banner);

    document.getElementById("cookie-accept").addEventListener("click", () => {
      localStorage.setItem(
        consentKey,
        JSON.stringify({
          essential: true,
          analytics: true,
          marketing: false,
          ts: Date.now(),
        }),
      );
      banner.style.animation = "none";
      banner.style.transform = "translateY(100%)";
      banner.style.opacity = "0";
      banner.style.transition = "transform 0.3s, opacity 0.3s";
      setTimeout(() => banner.remove(), 300);
    });

    document.getElementById("cookie-reject").addEventListener("click", () => {
      localStorage.setItem(
        consentKey,
        JSON.stringify({
          essential: true,
          analytics: false,
          marketing: false,
          ts: Date.now(),
        }),
      );
      banner.style.animation = "none";
      banner.style.transform = "translateY(100%)";
      banner.style.opacity = "0";
      banner.style.transition = "transform 0.3s, opacity 0.3s";
      setTimeout(() => banner.remove(), 300);
    });
  }

  // ── TERMS RE-ACCEPTANCE BANNER (All platform pages) ───────────────────────
  // Called on every platform page. Checks /api/user/legal-status — if the
  // user hasn't accepted the current Terms version, a top banner is shown
  // that they must accept before it dismisses. Non-blocking but persistent.
  async function checkTermsReacceptance() {
    // Don't show on auth pages (not logged in)
    if (
      window.location.pathname.startsWith("/auth/") ||
      window.location.pathname.startsWith("/p/") ||
      window.location.pathname === "/signup" ||
      window.location.pathname === "/"
    )
      return;

    // Check if already dismissed for this version in sessionStorage
    const dismissKey = "poool_terms_dismissed_v";
    try {
      const res = await fetch("/api/user/legal-status");
      if (!res.ok) return; // Not logged in or error — skip silently
      const data = await res.json();

      if (!data.needs_reaccept) return; // Up to date

      const sessionDismissed = sessionStorage.getItem(dismissKey);
      if (sessionDismissed === data.current_version) return; // Already dismissed this session

      showTermsBanner(data.current_version, dismissKey);
    } catch (_) {
      // Silently ignore — never block the user experience
    }
  }

  function showTermsBanner(currentVersion, dismissKey) {
    if (document.getElementById("terms-reaccept-banner")) return; // Already showing

    const banner = document.createElement("div");
    banner.id = "terms-reaccept-banner";
    banner.style.cssText = `
            position:sticky; top:0; left:0; right:0; z-index:99998;
            background:linear-gradient(135deg, #1D3557 0%, #1a2a50 100%);
            color:#fff; padding:14px 24px;
            font-family:'TT Norms Pro','Segoe UI',system-ui,sans-serif; font-size:14px; line-height:1.5;
            display:flex; flex-wrap:wrap; align-items:center; gap:12px;
            box-shadow:0 2px 12px rgba(0,0,0,0.18);
            animation:termsSlideDown 0.35s ease-out;
        `;

    banner.innerHTML = `
            <style>
                @keyframes termsSlideDown { from { transform:translateY(-100%); opacity:0; } to { transform:translateY(0); opacity:1; } }
                .terms-accept-btn {
                    padding:7px 18px; border-radius:8px; font-size:13px; font-weight:600;
                    cursor:pointer; border:none; font-family:inherit; transition:all 0.2s;
                    background:#4A7DFF; color:#fff;
                }
                .terms-accept-btn:hover { background:#3A6DE8; transform:translateY(-1px); }
                .terms-view-link { color:#A3C0FF; font-weight:500; text-decoration:underline; }
                .terms-view-link:hover { color:#fff; }
            </style>
            <svg width="18" height="18" viewBox="0 0 20 20" fill="none" stroke="#A3C0FF" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="flex-shrink:0;">
                <path d="M4 5h12M4 10h12M4 15h8"/>
            </svg>
            <span style="flex:1; min-width:260px;">
                <strong style="color:#fff;">Our Terms have been updated</strong> — 
                Please <a href="/terms" target="_blank" class="terms-view-link">review the changes</a> 
                and accept the updated Terms &amp; Conditions (v${currentVersion}).
            </span>
            <button class="terms-accept-btn" id="terms-accept-btn">I Accept</button>
        `;
    document.body.insertBefore(banner, document.body.firstChild);

    document
      .getElementById("terms-accept-btn")
      .addEventListener("click", async () => {
        try {
          const res = await fetch("/api/user/legal-accept", {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              "X-CSRF-Token": typeof window.getCsrfToken === "function" ? window.getCsrfToken() : "",
            },
          });
          if (res.ok) {
            banner.style.animation = "none";
            banner.style.transform = "translateY(-100%)";
            banner.style.transition = "transform 0.3s";
            setTimeout(() => banner.remove(), 300);
          }
        } catch (_) {
          // Dismiss anyway on error — don't trap users
          sessionStorage.setItem(dismissKey, currentVersion);
          banner.remove();
        }
      });
  }

  // ── INITIALIZE ────────────────────────────────────────────────────────────
  function init() {
    buildTableOfContents();
    addBackToTopButton();
    addPrintStyles();
    injectPlatformFooter();
    showCookieConsent();
    checkTermsReacceptance();
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
