/**
 * Developer Assets Page
 * Management table filtering and preview panel.
 */

function safeAssetUrl(rawUrl) {
  try {
    const parsed = new URL(String(rawUrl || ""), window.location.origin);
    if (parsed.protocol === "http:" || parsed.protocol === "https:") return parsed.href;
  } catch (_) {
    // Fall through to the placeholder.
  }
  return "/static/images/seed/villa1.webp";
}

function isFundedRow(row) {
  const status = (row.dataset.status || "").toLowerCase();
  const fundedStr = (row.dataset.funded || "").toLowerCase();
  const isFundedStatus = ["funded", "rented", "exited"].includes(status);
  const pct = Number.parseFloat(row.dataset.fundingPct) || 0;
  return isFundedStatus || fundedStr === "true" || pct >= 100;
}

function formatLocationDisplay(value) {
  return String(value || "No location")
    .split(",")
    .map((part) => {
      const trimmed = part.trim();
      if (!trimmed) return "";
      return trimmed.charAt(0).toUpperCase() + trimmed.slice(1);
    })
    .filter(Boolean)
    .join(", ");
}

function rowMatchesFilter(row, filter, query) {
  const statusMatch =
    filter === "all" ||
    (filter === "available" && !isFundedRow(row)) ||
    (filter === "funded" && isFundedRow(row));
  const searchable = `${row.dataset.title || ""} ${row.dataset.location || ""} ${row.dataset.statusLabel || ""}`.toLowerCase();
  return statusMatch && (!query || searchable.includes(query));
}

function updatePreview(row) {
  if (!row) return;
  document.querySelectorAll(".dev-asset-row.is-selected").forEach((el) => el.classList.remove("is-selected"));
  row.classList.add("is-selected");

  const assetId = row.dataset.assetId || "";
  const pct = Number.parseFloat(row.dataset.fundingPct) || 0;
  const cover = safeAssetUrl(row.dataset.coverUrl);

  const image = document.getElementById("dev-assets-preview-image");
  if (image) image.style.backgroundImage = `url("${cover.replace(/"/g, "%22")}")`;

  const setText = (id, value) => {
    const el = document.getElementById(id);
    if (el) el.textContent = value || "—";
  };

  setText("dev-assets-preview-status", row.dataset.statusLabel || "Asset");
  setText("dev-assets-preview-title", row.dataset.title || "Untitled asset");
  setText("dev-assets-preview-location", formatLocationDisplay(row.dataset.location));
  setText("dev-assets-preview-value", row.dataset.value || "—");
  setText("dev-assets-preview-funded", `${pct.toFixed(1)}% funded`);
  setText("dev-assets-preview-duration", row.dataset.duration || "—");
  setText("dev-assets-preview-return", row.dataset.return || "—");
  setText("dev-assets-preview-yield", row.dataset.yield || "—");
  setText("dev-assets-preview-remaining", row.dataset.remaining || "—");

  const fill = document.getElementById("dev-assets-preview-progress-fill");
  if (fill) fill.style.width = `${Math.max(0, Math.min(pct, 100))}%`;

  const view = document.getElementById("dev-assets-preview-view");
  if (view) view.href = `/developer/asset-detail?id=${encodeURIComponent(assetId)}`;
  const edit = document.getElementById("dev-assets-preview-edit");
  if (edit) edit.href = `/developer/property-content?draft_id=${encodeURIComponent(assetId)}`;
}

function clearPreview() {
  document.querySelectorAll(".dev-asset-row.is-selected").forEach((el) => el.classList.remove("is-selected"));

  const image = document.getElementById("dev-assets-preview-image");
  if (image) image.style.backgroundImage = "";

  const values = {
    "dev-assets-preview-status": "No match",
    "dev-assets-preview-title": "No asset selected",
    "dev-assets-preview-location": "Adjust the search or status filter.",
    "dev-assets-preview-value": "—",
    "dev-assets-preview-funded": "0.0% funded",
    "dev-assets-preview-duration": "—",
    "dev-assets-preview-return": "—",
    "dev-assets-preview-yield": "—",
    "dev-assets-preview-remaining": "—",
  };

  Object.entries(values).forEach(([id, value]) => {
    const el = document.getElementById(id);
    if (el) el.textContent = value;
  });

  const fill = document.getElementById("dev-assets-preview-progress-fill");
  if (fill) fill.style.width = "0%";
}

function updateFilterCounts() {
  const rows = Array.from(document.querySelectorAll(".dev-asset-row"));
  const counts = rows.reduce(
    (acc, row) => {
      acc.all += 1;
      acc[isFundedRow(row) ? "funded" : "available"] += 1;
      return acc;
    },
    { all: 0, available: 0, funded: 0 },
  );

  Object.entries(counts).forEach(([key, value]) => {
    const el = document.querySelector(`[data-dev-assets-count="${key}"]`);
    if (el) el.textContent = String(value);
  });
}

function applyAssetFilters() {
  const activeTab = document.querySelector(".dev-assets-tab.active");
  const filter = activeTab?.dataset.devAssetsTab || "all";
  const query = (document.getElementById("dev-assets-search-input")?.value || "").trim().toLowerCase();
  let firstVisible = null;
  let visibleCount = 0;

  document.querySelectorAll(".dev-asset-row").forEach((row) => {
    const visible = rowMatchesFilter(row, filter, query);
    row.hidden = !visible;
    if (visible) visibleCount += 1;
    if (visible && !firstVisible) firstVisible = row;
  });

  const emptyRow = document.getElementById("dev-assets-empty-row");
  if (emptyRow) emptyRow.hidden = visibleCount > 0;

  const current = document.querySelector(".dev-asset-row.is-selected:not([hidden])");
  if (current || firstVisible) {
    updatePreview(current || firstVisible);
  } else {
    clearPreview();
  }
}

function bindStatusTabs() {
  document.querySelectorAll("[data-dev-assets-tab]").forEach((button) => {
    button.setAttribute("aria-pressed", button.classList.contains("active") ? "true" : "false");
    button.addEventListener("click", () => {
      document.querySelectorAll("[data-dev-assets-tab]").forEach((tab) => {
        const isActive = tab === button;
        tab.classList.toggle("active", isActive);
        tab.setAttribute("aria-pressed", isActive ? "true" : "false");
      });
      applyAssetFilters();
    });
  });
}

function bindAssetRows() {
  document.querySelectorAll(".dev-asset-row").forEach((row) => {
    row.addEventListener("click", (event) => {
      if (event.target.closest("a, button")) return;
      updatePreview(row);
    });
    row.addEventListener("keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      updatePreview(row);
    });
  });
}

function bindSearch() {
  const input = document.getElementById("dev-assets-search-input");
  if (!input) return;
  input.addEventListener("input", applyAssetFilters);
}

document.addEventListener("DOMContentLoaded", function () {
  bindStatusTabs();
  bindAssetRows();
  bindSearch();
  updateFilterCounts();
  applyAssetFilters();
});
