(function () {
  "use strict";

  const REFERRALS_API = "/api/affiliate/referrals";
  const EXPORT_API = "/api/affiliate/commissions/export?format=csv&limit=200";
  const FILTERS = new Set(["all", "under_holdback", "payable", "paid"]);

  let referrals = [];
  let currentFilter = "all";

  function $(id) {
    return document.getElementById(id);
  }

  function centsToUsd(cents) {
    const value = Number.isInteger(cents) ? cents : 0;
    const sign = value < 0 ? "-" : "";
    const absolute = Math.abs(value);
    const dollars = Math.floor(absolute / 100);
    const minor = String(absolute % 100).padStart(2, "0");
    return `${sign}$${dollars.toLocaleString()}.${minor}`;
  }

  function formatDate(dateStr) {
    if (!dateStr) return "-";
    const date = new Date(dateStr);
    if (Number.isNaN(date.getTime())) return "-";
    return date.toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }

  function normalizeStatus(status) {
    return String(status || "unknown").replace(/_/g, " ");
  }

  function setStatus(message, kind) {
    const el = $("referrals-status");
    if (!el) return;
    el.textContent = message || "";
    el.classList.toggle("is-error", kind === "error");
    el.classList.toggle("is-success", kind === "success");
  }

  function setLoading(isLoading) {
    $("referrals-loading")?.classList.toggle("ds-hidden", !isLoading);
    $("referrals-content")?.classList.toggle("ds-hidden", isLoading);
  }

  function appendTextCell(row, text, className) {
    const cell = document.createElement("td");
    if (className) cell.className = className;
    cell.textContent = text;
    row.appendChild(cell);
    return cell;
  }

  function renderEmpty(message) {
    const tbody = $("referrals-table-body");
    if (!tbody) return;
    const row = document.createElement("tr");
    const cell = document.createElement("td");
    cell.colSpan = 4;
    cell.className = "affiliate-referrals-empty";
    cell.textContent = message;
    row.appendChild(cell);
    tbody.replaceChildren(row);
  }

  function renderError(message) {
    renderEmpty(message);
    setStatus(message, "error");
  }

  function filteredReferrals() {
    const query = ($("referral-search")?.value || "").trim().toLowerCase();
    return referrals.filter((referral) => {
      const status = String(referral.status || "");
      if (currentFilter !== "all" && status !== currentFilter) return false;
      if (!query) return true;
      const searchText = [
        referral.referral_id,
        referral.email,
        referral.status,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      return searchText.includes(query);
    });
  }

  function renderTable() {
    const tbody = $("referrals-table-body");
    if (!tbody) return;

    const rows = filteredReferrals();
    if (rows.length === 0) {
      const message = referrals.length
        ? "No referrals match the current filters."
        : "No referral data available.";
      renderEmpty(message);
      setStatus(message);
      return;
    }

    const fragment = document.createDocumentFragment();
    rows.forEach((referral) => {
      const row = document.createElement("tr");

      const statusCell = appendTextCell(row, "", "affiliate-referrals-status-cell");
      const rawStatus = String(referral.status || "");
      statusCell.classList.toggle("is-paid", rawStatus === "paid");
      statusCell.classList.toggle("is-payable", rawStatus === "payable");
      statusCell.classList.toggle(
        "is-muted",
        rawStatus !== "paid" && rawStatus !== "payable",
      );
      statusCell.append(document.createTextNode(normalizeStatus(rawStatus)));

      if (referral.email) {
        const email = document.createElement("span");
        email.className = "affiliate-referrals-email";
        email.textContent = String(referral.email);
        statusCell.appendChild(email);
      }

      appendTextCell(row, formatDate(referral.created_at));
      appendTextCell(
        row,
        rawStatus === "under_holdback"
          ? formatDate(referral.holdback_expires_at)
          : "-",
      );
      appendTextCell(
        row,
        centsToUsd(referral.amount_cents),
        "affiliate-referrals-amount",
      );

      fragment.appendChild(row);
    });

    tbody.replaceChildren(fragment);
    setStatus(`${rows.length} referral${rows.length === 1 ? "" : "s"} shown.`);
  }

  function selectFilter(filter, button) {
    if (!FILTERS.has(filter)) return;
    currentFilter = filter;
    document.querySelectorAll("[data-referral-filter]").forEach((tab) => {
      const isActive = tab === button;
      tab.classList.toggle("active", isActive);
      tab.setAttribute("aria-selected", isActive ? "true" : "false");
    });
    renderTable();
  }

  async function loadReferrals() {
    setLoading(true);
    setStatus("");

    try {
      const response = await fetch(REFERRALS_API, {
        headers: { Accept: "application/json" },
        credentials: "same-origin",
      });
      const contentType = response.headers.get("content-type") || "";
      const data = contentType.includes("application/json")
        ? await response.json()
        : {};

      if (!response.ok) {
        const message =
          data.error ||
          (response.status === 403
            ? "Only active affiliates can view referral details."
            : "Referral data could not be loaded.");
        throw new Error(message);
      }

      referrals = Array.isArray(data.data) ? data.data : [];
      renderTable();
    } catch (error) {
      console.error("Failed to load affiliate referrals", error);
      renderError(error.message || "Referral data could not be loaded.");
    } finally {
      setLoading(false);
    }
  }

  function triggerDownload(blob, filename) {
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(url);
  }

  async function exportReferralCSV() {
    const button = $("affiliate-referrals-export-btn");
    const originalText = button?.textContent;
    if (button) {
      button.disabled = true;
      button.dataset.loading = "true";
    }
    setStatus("Preparing CSV export...");

    try {
      const response = await fetch(EXPORT_API, {
        headers: { Accept: "text/csv" },
        credentials: "same-origin",
      });
      if (!response.ok) {
        const data = await response.json().catch(() => ({}));
        throw new Error(data.error || "CSV export failed.");
      }
      const blob = await response.blob();
      triggerDownload(
        blob,
        `poool_affiliate_commissions_${new Date().toISOString().slice(0, 10)}.csv`,
      );
      setStatus("CSV export downloaded.", "success");
    } catch (error) {
      console.error("Failed to export affiliate referrals", error);
      setStatus(error.message || "CSV export failed.", "error");
    } finally {
      if (button) {
        button.disabled = false;
        button.dataset.loading = "false";
        if (originalText) button.lastChild.textContent = originalText.trim();
      }
    }
  }

  function initTabs() {
    const tabs = Array.from(document.querySelectorAll("[data-referral-filter]"));
    tabs.forEach((tab, index) => {
      tab.addEventListener("click", () => {
        selectFilter(tab.dataset.referralFilter, tab);
      });
      tab.addEventListener("keydown", (event) => {
        if (!["ArrowRight", "ArrowLeft", "Home", "End"].includes(event.key)) {
          return;
        }
        event.preventDefault();
        let nextIndex = index;
        if (event.key === "ArrowRight") nextIndex = (index + 1) % tabs.length;
        if (event.key === "ArrowLeft") nextIndex = (index - 1 + tabs.length) % tabs.length;
        if (event.key === "Home") nextIndex = 0;
        if (event.key === "End") nextIndex = tabs.length - 1;
        tabs[nextIndex].focus();
        selectFilter(tabs[nextIndex].dataset.referralFilter, tabs[nextIndex]);
      });
    });
  }

  document.addEventListener("DOMContentLoaded", () => {
    initTabs();
    $("referral-search")?.addEventListener("input", renderTable);
    $("affiliate-referrals-export-btn")?.addEventListener("click", exportReferralCSV);
    loadReferrals();
  });
})();
