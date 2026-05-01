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

  /* Icon SVGs per type */
  var ICONS = {
    danger:
      '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="22" height="22">' +
      '<circle cx="12" cy="12" r="10"/>' +
      '<line x1="12" y1="8" x2="12" y2="12"/>' +
      '<line x1="12" y1="16" x2="12.01" y2="16"/></svg>',
    warning:
      '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="22" height="22">' +
      '<path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>' +
      '<line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>',
    success:
      '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="22" height="22">' +
      '<path d="M22 11.08V12a10 10 0 11-5.93-9.14"/>' +
      '<polyline points="22 4 12 14.01 9 11.01"/></svg>',
    default:
      '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="22" height="22">' +
      '<circle cx="12" cy="12" r="10"/>' +
      '<line x1="12" y1="8" x2="12" y2="12"/>' +
      '<line x1="12" y1="16" x2="12.01" y2="16"/></svg>',
  };

  /* Icon background/stroke colours per type */
  var ICON_STYLES = {
    danger:  { bg: "#FEF3F2", stroke: "#D92D20" },
    warning: { bg: "#FFFAEB", stroke: "#B54708" },
    success: { bg: "#ECFDF3", stroke: "#027A48" },
    default: { bg: "#EEF4FF", stroke: "#0000FF" },
  };

  /* Confirm button class per type */
  var CONFIRM_BTN_CLASS = {
    danger:  "ds-btn ds-btn--danger",
    warning: "ds-btn ds-btn--danger",
    success: "ds-btn ds-btn--primary",
    default: "ds-btn ds-btn--primary",
  };

  function pooolConfirm(options) {
    return new Promise(function (resolve) {
      var title, message, confirmText, cancelText, type;
      if (typeof options === "string") {
        message     = options;
        title       = "Confirm action";
        confirmText = "Confirm";
        cancelText  = "Cancel";
        type        = "default";
      } else {
        title       = options.title       || "Confirm action";
        message     = options.message     || "";
        confirmText = options.confirmText || "Confirm";
        cancelText  = options.cancelText  || "Cancel";
        type        = options.type        || "default";
      }

      var iconStyle = ICON_STYLES[type] || ICON_STYLES.default;
      var iconSvg   = ICONS[type] || ICONS.default;
      /* Re-colour the SVG stroke inline */
      iconSvg = iconSvg.replace('stroke="currentColor"', 'stroke="' + iconStyle.stroke + '"');

      var overlay = document.createElement("div");
      overlay.className = "ds-modal-overlay active";
      overlay.setAttribute("role", "dialog");
      overlay.setAttribute("aria-modal", "true");
      overlay.setAttribute("aria-labelledby", "pc-title");

      overlay.innerHTML =
        '<div class="ds-modal ds-modal--sm">' +
          '<div class="ds-modal__header">' +
            '<div>' +
              '<h3 class="ds-modal__title" id="pc-title">' + escHtml(title) + '</h3>' +
            '</div>' +
          '</div>' +
          '<div class="ds-modal__body">' +
            '<div style="width:44px;height:44px;border-radius:10px;display:flex;align-items:center;justify-content:center;margin-bottom:16px;flex-shrink:0;background:' + iconStyle.bg + ';">' +
              iconSvg +
            '</div>' +
            '<p style="font-size:14px;color:#475467;margin:0;line-height:1.6;white-space:pre-wrap;">' + escHtml(message) + '</p>' +
          '</div>' +
          '<div class="ds-modal__footer ds-modal__footer--bordered">' +
            '<button class="ds-btn ds-btn--secondary" id="pc-cancel">' + escHtml(cancelText) + '</button>' +
            '<button class="' + (CONFIRM_BTN_CLASS[type] || "ds-btn ds-btn--primary") + '" id="pc-confirm">' + escHtml(confirmText) + '</button>' +
          '</div>' +
        '</div>';

      document.body.appendChild(overlay);

      var confirmBtn = overlay.querySelector("#pc-confirm");
      var cancelBtn  = overlay.querySelector("#pc-cancel");
      setTimeout(function () { confirmBtn.focus(); }, 30);

      function close(result) {
        document.removeEventListener("keydown", onKey);
        overlay.classList.remove("active");
        setTimeout(function () {
          if (overlay.parentNode) overlay.parentNode.removeChild(overlay);
        }, 200);
        resolve(result);
      }

      confirmBtn.addEventListener("click", function () { close(true); });
      cancelBtn.addEventListener("click",  function () { close(false); });

      overlay.addEventListener("click", function (e) {
        if (e.target === overlay) close(false);
      });

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
