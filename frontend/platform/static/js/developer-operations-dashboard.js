/**
 * Developer Operations Dashboard — Villa-Returns P2.
 * Calls GET /api/developer/operations/dashboard and renders one row per assigned villa.
 */

document.addEventListener("DOMContentLoaded", () => {
  load();
});

async function load() {
  try {
    const resp = await fetch("/api/developer/operations/dashboard");
    if (!resp.ok) throw new Error(await responseError(resp));
    const entries = await resp.json();
    render(entries);
  } catch (err) {
    document.getElementById("dev-empty").textContent = `Failed to load: ${err.message}`;
    document.getElementById("dev-empty").style.display = "flex";
  }
}

// villa_operations_log.status → ds-badge variant.
const STATUS_VARIANT = {
  published: "success",
  approved: "success",
  submitted: "info",
  draft: "neutral",
  rejected: "danger",
  none: "warning",
};

function render(entries) {
  const tbody = document.getElementById("dev-tbody");
  const empty = document.getElementById("dev-empty");
  tbody.innerHTML = "";
  if (!entries.length) {
    empty.style.display = "flex";
    return;
  }
  empty.style.display = "none";

  const now = new Date();
  const targetYear = now.getMonth() === 0 ? now.getFullYear() - 1 : now.getFullYear();
  const targetMonth = now.getMonth() === 0 ? 12 : now.getMonth(); // most-recent fully-closed month

  for (const e of entries) {
    const tr = document.createElement("tr");

    const tdName = document.createElement("td");
    tdName.className = "ds-table-td--bold";
    tdName.textContent = e.asset_title || "(untitled)";
    tr.appendChild(tdName);

    const tdPeriod = document.createElement("td");
    tdPeriod.className = "ds-table-td--mono";
    tdPeriod.textContent =
      e.latest_period_year && e.latest_period_month
        ? `${e.latest_period_year}-${String(e.latest_period_month).padStart(2, "0")}`
        : "—";
    tr.appendChild(tdPeriod);

    const tdStatus = document.createElement("td");
    const status = e.latest_status || "none";
    const badge = document.createElement("span");
    badge.className = `ds-badge ds-badge--${STATUS_VARIANT[status] || "neutral"}`;
    badge.textContent = status === "none" ? "Not started" : status;
    tdStatus.appendChild(badge);
    if (status === "rejected" || (status === "draft" && e.latest_rejected_reason)) {
      const note = document.createElement("div");
      note.className = "ds-text-caption ds-text--danger";
      note.textContent = `Reason: ${e.latest_rejected_reason || ""}`;
      tdStatus.appendChild(note);
    }
    tr.appendChild(tdStatus);

    const tdAction = document.createElement("td");
    const actions = document.createElement("div");
    actions.className = "ds-flex ds-gap-8";

    const a = document.createElement("a");
    a.className = "ds-btn ds-btn--primary ds-btn--sm";
    a.href = `/developer/villas/${encodeURIComponent(e.asset_id)}/operations/new?year=${targetYear}&month=${targetMonth}`;
    a.textContent = `Submit ${targetYear}-${String(targetMonth).padStart(2, "0")}`;
    actions.appendChild(a);

    const annual = document.createElement("a");
    annual.className = "ds-btn ds-btn--secondary ds-btn--sm";
    annual.href = `/developer/villas/${encodeURIComponent(e.asset_id)}/annual/${targetYear}`;
    annual.textContent = `Annual ${targetYear}`;
    actions.appendChild(annual);

    tdAction.appendChild(actions);
    tr.appendChild(tdAction);

    tbody.appendChild(tr);
  }
}

async function responseError(resp) {
  try {
    const b = await resp.json();
    return b.error || b.message || `HTTP ${resp.status}`;
  } catch {
    return `HTTP ${resp.status}`;
  }
}
