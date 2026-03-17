/**
 * Admin — Asset Change Requests List
 * Fetches from GET /api/admin/change-requests
 */

let allItems = [];

document.addEventListener("DOMContentLoaded", () => {
  loadChangeRequests();

  document.getElementById("search-input")?.addEventListener("input", filterTable);
  document.getElementById("filter-status")?.addEventListener("change", filterTable);
});

async function loadChangeRequests() {
  try {
    const resp = await fetch("/api/admin/change-requests");
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    const data = await resp.json();

    // Update KPIs
    document.getElementById("kpi-pending").textContent = data.pending_count || 0;
    document.getElementById("kpi-approved").textContent = data.approved_count || 0;
    document.getElementById("kpi-rejected").textContent = data.rejected_count || 0;

    allItems = data.items || [];
    filterTable();
  } catch (err) {
    document.getElementById("table-body").innerHTML =
      `<tr><td colspan="6" style="text-align:center; padding:40px; color:#ef4444;">Failed to load: ${esc(err.message)}</td></tr>`;
  }
}

function filterTable() {
  const search = (document.getElementById("search-input")?.value || "").toLowerCase();
  const statusFilter = document.getElementById("filter-status")?.value || "all";

  const filtered = allItems.filter((item) => {
    if (statusFilter !== "all" && item.status !== statusFilter) return false;
    if (search) {
      const haystack = `${item.asset_title} ${item.developer_name}`.toLowerCase();
      if (!haystack.includes(search)) return false;
    }
    return true;
  });

  renderTable(filtered);
}

function renderTable(items) {
  const tbody = document.getElementById("table-body");
  if (!tbody) return;

  if (items.length === 0) {
    tbody.innerHTML = `<tr><td colspan="6" style="text-align:center; padding:40px; color: var(--admin-text-muted, #888);">No change requests found</td></tr>`;
    return;
  }

  tbody.innerHTML = items
    .map((item) => {
      const statusClass =
        item.status === "pending" ? "admin-badge--warning" :
        item.status === "approved" ? "admin-badge--success" : "admin-badge--danger";

      const statusLabel =
        item.status === "pending" ? "Pending" :
        item.status === "approved" ? "Approved" : "Rejected";

      const date = new Date(item.created_at);
      const dateStr = date.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });

      return `
        <tr>
          <td><strong>${item.asset_title ? esc(item.asset_title) : "Unknown Asset"}</strong></td>
          <td>
            <div class="admin-user-inline">
              <div class="admin-user-inline-name">${esc(item.developer_name || "Unknown Developer")}</div>
            </div>
          </td>
          <td><span class="admin-badge admin-badge--info">${item.fields_changed} field${item.fields_changed !== 1 ? 's' : ''} changed</span></td>
          <td><span class="admin-badge ${statusClass}">${statusLabel}</span></td>
          <td style="color: var(--admin-text-muted); font-size: 13px;">${dateStr}</td>
          <td>
            <a href="/admin/asset-change-review.html?id=${item.id}" class="admin-btn admin-btn--sm admin-btn--primary" style="padding: 6px 14px; font-size: 12px; border-radius: 6px; text-decoration: none;">
              Review
            </a>
          </td>
        </tr>
      `;
    })
    .join("");
}

function esc(str) {
  if (!str) return "";
  const d = document.createElement("div");
  d.textContent = str;
  return d.innerHTML;
}
