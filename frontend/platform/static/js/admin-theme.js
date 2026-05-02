/**
 * Admin Theme Toggle (3-state: light | dark | auto)
 *
 * Persists preference to localStorage. "auto" follows
 * `prefers-color-scheme` and updates live as the OS pref flips.
 *
 * Cycle on click: light → dark → auto → light.
 */
(function () {
  const KEY = "admin-theme";
  const VALID = new Set(["light", "dark", "auto"]);
  const media = window.matchMedia ? window.matchMedia("(prefers-color-scheme: dark)") : null;

  function effective(theme) {
    if (theme === "auto") return media && media.matches ? "dark" : "light";
    return theme === "dark" ? "dark" : "light";
  }

  function applyTheme(theme) {
    const eff = effective(theme);
    if (eff === "dark") {
      document.documentElement.classList.add("admin-dark");
    } else {
      document.documentElement.classList.remove("admin-dark");
    }
    document.documentElement.dataset.themePref = theme;
    updateToggleLabel(theme, eff);
  }

  function updateToggleLabel(theme, eff) {
    const label = document.getElementById("theme-toggle-label");
    if (label) {
      label.textContent =
        theme === "auto" ? `Auto (${eff})` : eff === "dark" ? "Dark" : "Light";
    }
    const btn = document.getElementById("admin-theme-toggle");
    if (btn) {
      const next = nextOf(theme);
      btn.setAttribute("aria-label", `Theme: ${theme}. Click for ${next}.`);
      btn.title = `Theme: ${theme} (click for ${next})`;
    }
  }

  function nextOf(theme) {
    return theme === "light" ? "dark" : theme === "dark" ? "auto" : "light";
  }

  function readSaved() {
    const v = localStorage.getItem(KEY);
    return VALID.has(v) ? v : "auto";
  }

  function toggle() {
    const cur = readSaved();
    const next = nextOf(cur);
    localStorage.setItem(KEY, next);
    applyTheme(next);
  }

  // Apply saved preference on load (before paint if possible).
  const saved = readSaved();
  applyTheme(saved);

  // Live-react to OS pref change while in auto mode.
  if (media && typeof media.addEventListener === "function") {
    media.addEventListener("change", () => {
      if (readSaved() === "auto") applyTheme("auto");
    });
  }

  document.addEventListener("DOMContentLoaded", () => {
    const btn = document.getElementById("admin-theme-toggle");
    if (btn) btn.addEventListener("click", toggle);
    updateToggleLabel(saved, effective(saved));

    // Notification buttons — preserve existing behavior.
    const notifBtns = document.querySelectorAll(".admin-notification-btn");
    notifBtns.forEach((el) => {
      if (!el.getAttribute("onclick")) {
        el.addEventListener("click", () => {
          window.location.href = "/admin/notifications.html";
        });
      }
    });
  });
})();
