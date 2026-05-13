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
    document.getElementById("dev-empty").style.display = "block";
  }
}

function render(entries) {
  const tbody = document.getElementById("dev-tbody");
  const empty = document.getElementById("dev-empty");
  tbody.innerHTML = "";
  if (!entries.length) {
    empty.style.display = "block";
    return;
  }
  empty.style.display = "none";

  const now = new Date();
  const targetYear = now.getMonth() === 0 ? now.getFullYear() - 1 : now.getFullYear();
  const targetMonth = now.getMonth() === 0 ? 12 : now.getMonth(); // most-recent fully-closed month

  for (const e of entries) {
    const tr = document.createElement("tr");

    const tdName = document.createElement("td");
    tdName.textContent = e.asset_title || "(untitled)";
    tr.appendChild(tdName);

    const tdPeriod = document.createElement("td");
    tdPeriod.textContent =
      e.latest_period_year && e.latest_period_month
        ? `${e.latest_period_year}-${String(e.latest_period_month).padStart(2, "0")}`
        : "—";
    tr.appendChild(tdPeriod);

    const tdStatus = document.createElement("td");
    const status = e.latest_status || "none";
    const badge = document.createElement("span");
    badge.className = `dev-status ${status}`;
    badge.textContent = status === "none" ? "Not started" : status;
    tdStatus.appendChild(badge);
    if (status === "rejected" || (status === "draft" && e.latest_rejected_reason)) {
      const note = document.createElement("span");
      note.className = "dev-rejected-note";
      note.textContent = `Reason: ${e.latest_rejected_reason || ""}`;
      tdStatus.appendChild(note);
    }
    tr.appendChild(tdStatus);

    const tdAction = document.createElement("td");
    const a = document.createElement("a");
    a.className = "dev-btn primary";
    a.href = `/developer/villas/${encodeURIComponent(e.asset_id)}/operations/new?year=${targetYear}&month=${targetMonth}`;
    a.textContent = `Submit ${targetYear}-${String(targetMonth).padStart(2, "0")}`;
    tdAction.appendChild(a);
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
