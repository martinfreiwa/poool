/**
 * Admin Audit Logs JS
 */

// State Management
let allLogs = [];
let filteredLogs = [];
let currentPage = 1;
const PAGE_SIZE = 20;
let sortField = "created_at";
let sortOrder = "desc";

document.addEventListener("DOMContentLoaded", () => {
  loadLogs();

  // Filters
  document
    .getElementById("audit-search")
    ?.addEventListener("input", debounce(applyFilters, 250));
  document
    .getElementById("filter-entity")
    ?.addEventListener("change", applyFilters);
  document
    .getElementById("filter-action")
    ?.addEventListener("change", applyFilters);

  // Pagination
  document.getElementById("audit-prev")?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      renderTable();
    }
  });
  document.getElementById("audit-next")?.addEventListener("click", () => {
    const totalPages = Math.ceil(filteredLogs.length / PAGE_SIZE);
    if (currentPage < totalPages) {
      currentPage++;
      renderTable();
    }
  });

  // Sorting
  setupSorting();

  // CSV Export
  document
    .getElementById("audit-export-csv")
    ?.addEventListener("click", exportAuditCSV);

  // Modal Close
  document.getElementById("diff-modal-close")?.addEventListener("click", () => {
    document.getElementById("diff-modal").style.display = "none";
  });
  document.getElementById("diff-modal")?.addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeDiff();
  });
});

function setupSorting() {
  const table = document.querySelector(".admin-table");
  if (!table) return;
  table.querySelectorAll("th[data-sort]").forEach((th) => {
    th.style.cursor = "pointer";
    th.addEventListener("click", () => {
      const field = th.dataset.sort;
      if (sortField === field) {
        sortOrder = sortOrder === "asc" ? "desc" : "asc";
      } else {
        sortField = field;
        sortOrder = "asc";
      }
      applyFilters();
    });
  });
}

async function loadLogs() {
  try {
    const resp = await fetch("/api/admin/audit-logs");
    if (resp.ok) {
      const data = await resp.json();
      allLogs = data.logs || [];
    } else {
      console.error('Audit logs API error:', resp.status);
    }
  } catch (e) {
    console.error('Audit logs fetch failed:', e);
    if (window.Sentry) Sentry.captureException(e);
  }

  applyFilters();
}

function applyFilters() {
  const search = (
    document.getElementById("audit-search")?.value || ""
  ).toLowerCase();
  const entity = document.getElementById("filter-entity")?.value || "";
  const action = document.getElementById("filter-action")?.value || "";

  let result = allLogs.filter((log) => {
    if (entity && log.entity_type !== entity) return false;
    if (action && log.action !== action) return false;
    if (search) {
      const text =
        `${log.action} ${log.entity_type} ${log.actor_email} ${log.ip_address} ${log.id}`.toLowerCase();
      if (!text.includes(search)) return false;
    }
    return true;
  });

  // Sort Result
  result.sort((a, b) => {
    let valA = a[sortField];
    let valB = b[sortField];
    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredLogs = result;
  currentPage = 1;
  const auditCountEl = document.getElementById("audit-count-label");
  if (auditCountEl) auditCountEl.textContent = `${filteredLogs.length} entries`;
  renderTable();
}

function renderTable() {
  const tbody = document.getElementById("audit-table-body");
  if (!tbody) return;

  const totalPages = Math.max(1, Math.ceil(filteredLogs.length / PAGE_SIZE));
  currentPage = Math.min(currentPage, totalPages);
  const start = (currentPage - 1) * PAGE_SIZE;
  const slice = filteredLogs.slice(start, start + PAGE_SIZE);

  if (slice.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="7" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No logs match your filters.</td></tr>';
    return;
  }

  // Update Pagination UI
  const info = document.getElementById("audit-page-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${filteredLogs.length} total)`;
  const prevBtn = document.getElementById("audit-prev");
  const nextBtn = document.getElementById("audit-next");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;

  tbody.innerHTML = slice
    .map(
      (log) => `
            <tr>
                <td style="font-family:monospace;font-size:11px;color:var(--admin-text-muted);">#${log.id}</td>
                <td><span style="font-size:12px;font-weight:500;color:var(--admin-text-primary);">${esc(formatAction(log))}</span></td>
                <td>${entityBadge(log.entity_type)}${log.entity_id ? `<span style="font-size:10px;color:var(--admin-text-muted);margin-left:4px;">${esc(log.entity_id.substring(0, 8))}…</span>` : ""}</td>
                <td style="font-size:12px;">${esc(log.actor_email || "system")}</td>
                <td style="font-size:11px;font-family:monospace;color:var(--admin-text-muted);">${esc(log.ip_address || "—")}</td>
                <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${fmtDateTime(log.created_at)}</td>
                <td>${log.previous_state || log.new_state
          ? `<button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="showDiff(${log.id})" title="View state diff">
                    <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M5 2H3a1 1 0 00-1 1v10a1 1 0 001 1h2"/><path d="M11 2h2a1 1 0 011 1v10a1 1 0 01-1 1h-2"/><path d="M8 4v8"/></svg>
                </button>`
          : '<span style="font-size:11px;color:var(--admin-text-muted);">—</span>'
        }</td>
            </tr>
        `,
    )
    .join("");
}

function formatUSD(c) {
  return (
    (c < 0 ? "-" : "") +
    "$" +
    (Math.abs(c || 0) / 100).toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  );
}

function formatAction(log) {
  const action = log.action || "";
  const state = log.new_state || {};

  if (action === "admin.balance_update") {
    const sign = (state.amount_cents || 0) >= 0 ? "+" : "";
    return `Adjusted balance by ${sign}${formatUSD(state.amount_cents)} [${state.category || "cash"}] (New: ${formatUSD(state.new_balance)})`;
  }
  if (action === "admin.user_status_update") {
    return `Changed user account status to "${state.status || "unknown"}"`;
  }
  if (action === "admin.roles_update" || action === "admin.user_role_update") {
    return `Updated user roles: ${state.new_roles || "modified"}`;
  }
  if (action === "admin.revoke_sessions") {
    return `Revoked all active sessions (${state.deleted_count || 0} removed)`;
  }
  if (action === "user.tier_override") {
    return `Manually overrode loyalty tier to "${state.new_tier || "unknown"}"`;
  }
  if (action === "admin.settings_update") {
    return `Updated system settings: ${Object.keys(state).join(", ")}`;
  }

  // Generic Diff Parser for "Updated x"
  if (
    log.previous_state &&
    log.new_state &&
    typeof log.previous_state === "object"
  ) {
    const changes = [];
    for (const k in state) {
      if (JSON.stringify(log.previous_state[k]) !== JSON.stringify(state[k])) {
        const from = typeof log.previous_state[k] === 'object' ? JSON.stringify(log.previous_state[k]) : log.previous_state[k];
        const to = typeof state[k] === 'object' ? JSON.stringify(state[k]) : state[k];
        changes.push(`${k}: ${from} → ${to}`);
      }
    }
    if (changes.length > 0) return `Updated: ${changes.join(", ")}`;
  }

  return action.replace(/\./g, " ").replace(/_/g, " ");
}

function showDiff(id) {
  const l = allLogs.find((x) => x.id === id);
  if (!l) return;
  document.getElementById("diff-modal-title").textContent =
    `${l.action} — ${l.entity_type}`;
  const prev = l.previous_state
    ? JSON.stringify(l.previous_state, null, 2)
    : null;
  const next = l.new_state ? JSON.stringify(l.new_state, null, 2) : null;
  document.getElementById("diff-modal-body").innerHTML = `
        <div style="display:flex;gap:12px;margin-bottom:12px;font-size:12px;color:var(--admin-text-muted);">
            <span>Entity: <strong>${esc(l.entity_type)}</strong></span>
            <span>ID: <strong style="font-family:monospace;">${esc(l.entity_id || "—")}</strong></span>
            <span>Actor: <strong>${esc(l.actor_email || "system")}</strong></span>
        </div>
        ${prev ? `<div style="margin-bottom:12px;"><div style="font-size:11px;font-weight:600;color:var(--admin-danger);margin-bottom:4px;">Previous State</div><pre style="background:var(--admin-bg);border:1px solid var(--admin-border);border-radius:var(--admin-radius-md);padding:12px;font-size:11px;overflow-x:auto;max-height:200px;color:var(--admin-text-secondary);margin:0;">${esc(prev)}</pre></div>` : ""}
        ${next ? `<div><div style="font-size:11px;font-weight:600;color:var(--admin-success);margin-bottom:4px;">New State</div><pre style="background:var(--admin-bg);border:1px solid var(--admin-border);border-radius:var(--admin-radius-md);padding:12px;font-size:11px;overflow-x:auto;max-height:200px;color:var(--admin-text-secondary);margin:0;">${esc(next)}</pre></div>` : ""}
    `;
  document.getElementById("diff-modal").style.display = "flex";
}

function closeDiff() {
  document.getElementById("diff-modal").style.display = "none";
}

function esc(s) {
  if (s === null || s === undefined) return "";
  if (typeof s === "object") s = JSON.stringify(s);
  if (typeof s !== "string") s = String(s);
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
function fmtDateTime(iso) {
  if (!iso) return "—";
  const d = new Date(iso);
  return (
    d.toLocaleDateString("en-US", { month: "short", day: "numeric" }) +
    " " +
    d.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit" })
  );
}
function debounce(fn, ms) {
  let t;
  return function (...a) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, a), ms);
  };
}

function entityBadge(e) {
  const m = {
    user: "admin-badge--info",
    investment: "admin-badge--success",
    wallet_transaction: "admin-badge--warning",
    order: "admin-badge--neutral",
    asset: "admin-badge--info",
    kyc_record: "admin-badge--warning",
    support_ticket: "admin-badge--neutral",
  };
  return `<span class="admin-badge ${m[e] || "admin-badge--neutral"}" style="text-transform:capitalize;">${(e || "").replace(/_/g, " ")}</span>`;
}

function exportAuditCSV() {
  if (filteredLogs.length === 0) {
    alert("No logs to export.");
    return;
  }
  const headers = [
    "ID",
    "Action",
    "Entity Type",
    "Entity ID",
    "Actor Email",
    "IP Address",
    "User Agent",
    "Timestamp",
    "Previous State",
    "New State",
  ];
  const rows = filteredLogs.map((log) => [
    log.id,
    log.action || "",
    log.entity_type || "",
    log.entity_id || "",
    log.actor_email || "system",
    log.ip_address || "",
    log.user_agent || "",
    log.created_at || "",
    log.previous_state ? JSON.stringify(log.previous_state) : "",
    log.new_state ? JSON.stringify(log.new_state) : "",
  ]);
  const csv = [
    headers.join(","),
    ...rows.map((r) =>
      r.map((v) => `"${String(v).replace(/"/g, '""')}"`).join(","),
    ),
  ].join("\n");
  const blob = new Blob([csv], { type: "text/csv" });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  const now = new Date().toISOString().split("T")[0];
  a.download = `poool_audit-logs_${now}.csv`;
  a.click();
}
