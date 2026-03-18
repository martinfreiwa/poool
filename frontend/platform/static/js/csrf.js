/**
 * CSRF Token Helper
 * Standardized across the platform to provide the X-CSRF-Token header.
 *
 * NOTE: The global fetch() interceptor lives in components/head.html (loaded on every page).
 * This file only exports getCsrfToken() for legacy callers and handles
 * HTML form submissions on pages that include it directly.
 */

(function(window) {
    /**
     * Reads the csrf_token from the cookie.
     * @returns {string} The CSRF token or an empty string.
     */
    function getCsrfToken() {
        const value = `; ${document.cookie}`;
        const parts = value.split(`; csrf_token=`);
        if (parts.length === 2) return parts.pop().split(';').shift();
        return "";
    }

    // Export to global scope
    window.getCsrfToken = getCsrfToken;

    // Legacy alias
    window.csrfToken = getCsrfToken;

    // Auto-inject CSRF token into standard HTML form submissions
    // (Backup for pages that load csrf.js but not head.html)
    document.addEventListener("submit", function(e) {
        if (e.target && e.target.tagName && e.target.tagName.toLowerCase() === "form") {
            const method = (e.target.getAttribute("method") || "GET").toUpperCase();
            if (["POST", "PUT", "DELETE", "PATCH"].includes(method)) {
                const token = getCsrfToken();
                if (token && !e.target.querySelector('input[name="csrf_token"]')) {
                    const input = document.createElement("input");
                    input.type = "hidden";
                    input.name = "csrf_token";
                    input.value = token;
                    e.target.appendChild(input);
                }
            }
        }
    });

})(window);
