/**
 * Admin Theme Toggle
 * Persists preference to localStorage.
 * Light = default, Dark = .admin-dark on <html>
 */
(function () {
  const KEY = "admin-theme";

  function applyTheme(theme) {
    if (theme === "dark") {
      document.documentElement.classList.add("admin-dark");
    } else {
      document.documentElement.classList.remove("admin-dark");
    }
    updateToggleLabel(theme);
  }

  function updateToggleLabel(theme) {
    const label = document.getElementById("theme-toggle-label");
    if (label) label.textContent = theme === "dark" ? "Dark" : "Light";
  }

  function toggle() {
    const isDark = document.documentElement.classList.contains("admin-dark");
    const next = isDark ? "light" : "dark";
    localStorage.setItem(KEY, next);
    applyTheme(next);
  }

  // Apply saved preference on load (before paint if possible)
  const saved = localStorage.getItem(KEY) || "light";
  applyTheme(saved);

  // Bind click after DOM ready
  document.addEventListener("DOMContentLoaded", () => {
    const btn = document.getElementById("admin-theme-toggle");
    if (btn) btn.addEventListener("click", toggle);
    updateToggleLabel(saved);

    // Make notification buttons functional across all admin pages
    const notifBtns = document.querySelectorAll(".admin-notification-btn");
    notifBtns.forEach(el => {
      // If it doesn't already have an onclick handler via HTML
      if (!el.getAttribute("onclick")) {
        el.addEventListener("click", () => {
          window.location.href = "/admin/notifications.html";
        });
      }
    });
  });
})();
