/**
 * Admin Shared Utilities
 * Common helpers used across all admin pages.
 * MUST be loaded before any page-specific admin JS.
 */

/**
 * Escape HTML to prevent XSS when inserting user-controlled data via innerHTML.
 * Properly handles both element content AND attribute contexts.
 * @param {string} str - The string to escape
 * @returns {string} HTML-safe string
 */
function escapeHtml(str) {
  if (str == null) return "";
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/**
 * Safe text setter — sets textContent by element ID.
 * @param {string} id - Element ID
 * @param {string} text - Text content
 */
function setTextById(id, text) {
  const el = document.getElementById(id);
  if (el) el.textContent = text;
}

/**
 * Format cents to USD display string (e.g. 485000000 → "$4.85M")
 * @param {number} cents
 * @returns {string}
 */
function formatUSD(cents) {
  if (cents == null) cents = 0;
  const dollars = cents / 100;
  if (dollars >= 1000000) {
    return `$${(dollars / 1000000).toFixed(2)}M`;
  } else if (dollars >= 1000) {
    return `$${(dollars / 1000).toFixed(1)}K`;
  }
  return `$${dollars.toFixed(2)}`;
}

/**
 * Format numbers with comma separators
 * @param {number} num
 * @returns {string}
 */
function formatNumber(num) {
  if (num == null) return "0";
  return num.toLocaleString("en-US");
}

/**
 * Format date from ISO string
 * @param {string} isoString
 * @returns {string}
 */
function formatDate(isoString) {
  if (!isoString) return "—";
  const d = new Date(isoString);
  return d.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}
