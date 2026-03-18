/**
 * pooolConfirm — Custom confirmation dialog
 * Drop-in replacement for window.confirm() that returns a Promise<boolean>.
 *
 * Usage (simple):
 *   if (!await pooolConfirm('Are you sure?')) return;
 *
 * Usage (rich):
 *   if (!await pooolConfirm({
 *     title:       'Delete Image',
 *     message:     'This cannot be undone.',
 *     confirmText: 'Delete',
 *     cancelText:  'Cancel',
 *     type:        'danger',   // 'danger' | 'warning' | 'success' | 'default'
 *   })) return;
 */
(function (window) {
  "use strict";

  var STYLE_ID = "poool-confirm-styles";

  function injectStyles() {
    if (document.getElementById(STYLE_ID)) return;
    var style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = [
      /* Overlay */
      ".pc-overlay{position:fixed;inset:0;z-index:99990;display:flex;align-items:center;justify-content:center;",
      "padding:16px;background:rgba(10,14,26,0.55);backdrop-filter:blur(4px);",
      "animation:pc-fade-in 0.18s ease;}",

      /* Card */
      ".pc-card{background:#fff;border-radius:16px;box-shadow:0 24px 80px rgba(0,0,0,0.22),0 4px 16px rgba(0,0,0,0.08);",
      "max-width:420px;width:100%;padding:28px;position:relative;",
      "animation:pc-slide-up 0.22s cubic-bezier(0.34,1.56,0.64,1);}",

      /* Icon wrapper */
      ".pc-icon{width:48px;height:48px;border-radius:12px;display:flex;align-items:center;justify-content:center;margin-bottom:16px;}",
      ".pc-icon svg{width:24px;height:24px;}",
      ".pc-icon--danger{background:#FEF3F2;}.pc-icon--danger svg{stroke:#D92D20;}",
      ".pc-icon--warning{background:#FFFAEB;}.pc-icon--warning svg{stroke:#B54708;}",
      ".pc-icon--success{background:#ECFDF3;}.pc-icon--success svg{stroke:#027A48;}",
      ".pc-icon--default{background:#EEF4FF;}.pc-icon--default svg{stroke:#2E2EF9;}",

      /* Text */
      ".pc-title{font-family:'TT Norms Pro',system-ui,sans-serif;font-size:17px;font-weight:700;",
      "color:#101828;margin:0 0 8px;line-height:1.3;}",
      ".pc-message{font-size:14px;color:#475467;margin:0 0 24px;line-height:1.6;white-space:pre-wrap;}",

      /* Buttons */
      ".pc-actions{display:flex;gap:10px;justify-content:flex-end;}",
      ".pc-btn{padding:9px 18px;border-radius:8px;font-size:14px;font-weight:600;cursor:pointer;",
      "border:1.5px solid transparent;font-family:inherit;transition:opacity 0.15s,transform 0.1s;outline:none;}",
      ".pc-btn:active{transform:scale(0.97);}",
      ".pc-btn--cancel{background:#fff;border-color:#D0D5DD;color:#344054;}",
      ".pc-btn--cancel:hover{background:#F9FAFB;}",
      ".pc-btn--confirm-danger{background:#D92D20;color:#fff;border-color:#D92D20;}",
      ".pc-btn--confirm-danger:hover{background:#B42318;}",
      ".pc-btn--confirm-warning{background:#B54708;color:#fff;border-color:#B54708;}",
      ".pc-btn--confirm-warning:hover{background:#93370D;}",
      ".pc-btn--confirm-success{background:#027A48;color:#fff;border-color:#027A48;}",
      ".pc-btn--confirm-success:hover{background:#05603A;}",
      ".pc-btn--confirm-default{background:#2E2EF9;color:#fff;border-color:#2E2EF9;}",
      ".pc-btn--confirm-default:hover{background:#1e1ec9;}",

      /* Animations */
      "@keyframes pc-fade-in{from{opacity:0}to{opacity:1}}",
      "@keyframes pc-slide-up{from{opacity:0;transform:translateY(12px) scale(0.97)}to{opacity:1;transform:none}}",
    ].join("");
    document.head.appendChild(style);
  }

  /* Icon SVGs per type */
  var ICONS = {
    danger: '<svg viewBox="0 0 24 24" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
      '<path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>' +
      '<line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>',
    warning: '<svg viewBox="0 0 24 24" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
      '<circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>',
    success: '<svg viewBox="0 0 24 24" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
      '<path d="M22 11.08V12a10 10 0 11-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>',
    default: '<svg viewBox="0 0 24 24" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' +
      '<circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>',
  };

  function pooolConfirm(options) {
    return new Promise(function (resolve) {
      injectStyles();

      /* Parse arguments */
      var title, message, confirmText, cancelText, type;
      if (typeof options === "string") {
        message = options;
        title = "Confirm action";
        confirmText = "Confirm";
        cancelText = "Cancel";
        type = "default";
      } else {
        title       = options.title       || "Confirm action";
        message     = options.message     || "";
        confirmText = options.confirmText || "Confirm";
        cancelText  = options.cancelText  || "Cancel";
        type        = options.type        || "default";
      }

      /* Build DOM */
      var overlay = document.createElement("div");
      overlay.className = "pc-overlay";
      overlay.setAttribute("role", "dialog");
      overlay.setAttribute("aria-modal", "true");
      overlay.setAttribute("aria-labelledby", "pc-title");

      overlay.innerHTML =
        '<div class="pc-card">' +
          '<div class="pc-icon pc-icon--' + type + '">' + (ICONS[type] || ICONS.default) + "</div>" +
          '<p class="pc-title" id="pc-title">' + escHtml(title) + "</p>" +
          '<p class="pc-message">' + escHtml(message) + "</p>" +
          '<div class="pc-actions">' +
            '<button class="pc-btn pc-btn--cancel" id="pc-cancel">' + escHtml(cancelText) + "</button>" +
            '<button class="pc-btn pc-btn--confirm-' + type + '" id="pc-confirm">' + escHtml(confirmText) + "</button>" +
          "</div>" +
        "</div>";

      document.body.appendChild(overlay);

      /* Focus confirm button for keyboard accessibility */
      var confirmBtn = overlay.querySelector("#pc-confirm");
      var cancelBtn  = overlay.querySelector("#pc-cancel");
      setTimeout(function () { confirmBtn.focus(); }, 30);

      function close(result) {
        document.removeEventListener("keydown", onKey);
        /* Fade out */
        overlay.style.animation = "pc-fade-in 0.15s ease reverse forwards";
        setTimeout(function () {
          if (overlay.parentNode) overlay.parentNode.removeChild(overlay);
        }, 150);
        resolve(result);
      }

      confirmBtn.addEventListener("click", function () { close(true); });
      cancelBtn.addEventListener("click",  function () { close(false); });

      /* Click outside card → cancel */
      overlay.addEventListener("click", function (e) {
        if (e.target === overlay) close(false);
      });

      /* Keyboard: Enter = confirm, Escape = cancel */
      function onKey(e) {
        if (e.key === "Escape") { e.preventDefault(); close(false); }
      }
      document.addEventListener("keydown", onKey);
    });
  }

  function escHtml(str) {
    return String(str || "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  window.pooolConfirm = pooolConfirm;

})(window);
