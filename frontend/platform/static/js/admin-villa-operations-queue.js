/**
 * Admin Villa Operations Approval Queue — Villa-Returns P2.3.
 * URL: /admin/villa-operations-queue
 *
 * Calls:
 *   GET /api/admin/villa-operations-queue              list submitted rows
 *   PUT /api/admin/villas/:asset_id/operations/:log_id/approve
 *   PUT /api/admin/villas/:asset_id/operations/:log_id/reject
 *
 * 4-eyes is server-enforced. Client also disables Approve when current user
 * matches the submitter to avoid a guaranteed-409 click.
 */

let currentUserId = null;
let rejectingRow  = null;

document.addEventListener("DOMContentLoaded", async () => {
  currentUserId = await loadCurrentUserId();
  await load();

  document.getElementById("btn-refresh")?.addEventListener("click", load);
  document.getElementById("voq-reject-cancel")?.addEventListener("click", closeRejectModal);
  document.getElementById("voq-reject-confirm")?.addEventListener("click", confirmReject);
});

async function loadCurrentUserId() {
  try {
    const r = await fetch("/api/me");
    if (!r.ok) return null;
    const b = await r.json();
    return b?.id || b?.user?.id || null;
  } catch {
    return null;
  }
}

async function load() {
  const tbody = document.getElementById("voq-tbody");
  tbody.innerHTML = `<tr><td colspan="6" style="padding: 36px; text-align: center; color: var(--admin-text-muted);">Loading…</td></tr>`;
  document.getElementById("voq-empty").style.display = "none";
  document.getElementById("voq-table").style.display  = "";

  try {
    const resp = await fetch("/api/admin/villa-operations-queue");
    if (!resp.ok) throw new Error(await responseError(resp));
    const rows = await resp.json();
    render(rows);
  } catch (err) {
    tbody.innerHTML = `<tr><td colspan="6" style="padding: 24px 16px; color: var(--admin-danger);">Failed to load: ${escapeHtml(err.message)}</td></tr>`;
  }
}

function render(rows) {
  const tbody  = document.getElementById("voq-tbody");
  const empty  = document.getElementById("voq-empty");
  const table  = document.getElementById("voq-table");

  updateStats(rows);

  if (!rows.length) {
    tbody.innerHTML = "";
    table.style.display = "none";
    empty.style.display = "block";
    return;
  }

  table.style.display = "";
  empty.style.display = "none";
  tbody.innerHTML = "";

  for (const r of rows) {
    const isSelf = !!(currentUserId && r.submitted_by === currentUserId);
    const tr = document.createElement("tr");

    // ── Villa ──────────────────────────────────────────────────
    const tdVilla = document.createElement("td");
    tdVilla.innerHTML = `<a href="/admin/asset-details.html?id=${encodeURIComponent(r.asset_id)}" style="font-weight: 600; color: var(--admin-text-primary); text-decoration: none;" onmouseover="this.style.textDecoration='underline'" onmouseout="this.style.textDecoration='none'">${escapeHtml(r.asset_title)}</a>`;
    if (r.supersedes_id) {
      tdVilla.innerHTML += `<div style="font-size: 11px; color: var(--admin-warn, #d97706); margin-top: 3px; font-weight: 500;">↩ Correction · ${escapeHtml(r.correction_reason || "")}</div>`;
    }
    tr.appendChild(tdVilla);

    // ── Period ─────────────────────────────────────────────────
    const tdPeriod = document.createElement("td");
    tdPeriod.style.cssText = "font-variant-numeric: tabular-nums; color: var(--admin-text-muted); font-size: 13px;";
    tdPeriod.textContent = `${r.period_year}-${String(r.period_month).padStart(2, "0")}`;
    tr.appendChild(tdPeriod);

    // ── Distributable ──────────────────────────────────────────
    const tdDist = document.createElement("td");
    tdDist.style.cssText = "text-align: right; font-variant-numeric: tabular-nums; font-weight: 600; font-size: 13px;";
    tdDist.textContent = Number(r.distributable_idr_cents).toLocaleString("en-US");
    tr.appendChild(tdDist);

    // ── Submitter ──────────────────────────────────────────────
    const tdSubmitter = document.createElement("td");
    tdSubmitter.innerHTML = `<div style="font-size: 13px;">${escapeHtml(r.submitter_email || "(unknown)")}</div>`;
    if (isSelf) {
      tdSubmitter.innerHTML += `<div style="font-size: 11px; color: var(--admin-text-muted); font-style: italic; margin-top: 3px;">You submitted this — another admin must approve.</div>`;
    }
    tr.appendChild(tdSubmitter);

    // ── Submitted at ───────────────────────────────────────────
    const tdWhen = document.createElement("td");
    tdWhen.style.cssText = "font-size: 12px; color: var(--admin-text-muted); white-space: nowrap;";
    tdWhen.textContent = formatDateTime(r.submitted_at || r.recorded_at);
    tr.appendChild(tdWhen);

    // ── Actions ────────────────────────────────────────────────
    const tdActions = document.createElement("td");
    tdActions.style.cssText = "text-align: right;";

    const wrap = document.createElement("div");
    wrap.style.cssText = "display: flex; gap: 6px; justify-content: flex-end; align-items: center;";

    // Review link
    const reviewBtn = document.createElement("a");
    reviewBtn.className = "admin-btn admin-btn--secondary admin-btn--sm";
    reviewBtn.href = `/admin/villas/${encodeURIComponent(r.asset_id)}/operations/${r.period_year}/${r.period_month}?log_id=${r.id}&mode=review`;
    reviewBtn.textContent = "Review";
    wrap.appendChild(reviewBtn);

    // Approve button
    const approveBtn = document.createElement("button");
    approveBtn.type = "button";
    approveBtn.className = "admin-btn admin-btn--primary admin-btn--sm";
    approveBtn.textContent = "Approve";
    approveBtn.disabled = isSelf;
    if (isSelf) approveBtn.title = "You submitted this row — another admin must approve (4-eyes rule).";
    approveBtn.addEventListener("click", () => approve(r));
    wrap.appendChild(approveBtn);

    // Reject button
    const rejectBtn = document.createElement("button");
    rejectBtn.type = "button";
    rejectBtn.className = "admin-btn admin-btn--danger admin-btn--sm";
    rejectBtn.textContent = "Reject";
    rejectBtn.addEventListener("click", () => openRejectModal(r));
    wrap.appendChild(rejectBtn);

    tdActions.appendChild(wrap);
    tr.appendChild(tdActions);
    tbody.appendChild(tr);
  }
}

function updateStats(rows) {
  const selfCount    = rows.filter(r => currentUserId && r.submitted_by === currentUserId).length;
  const totalDistrib = rows.reduce((s, r) => s + Number(r.distributable_idr_cents || 0), 0);

  const elPending = document.getElementById("stat-pending");
  const elSelf    = document.getElementById("stat-self");
  const elDistrib = document.getElementById("stat-distributable");

  if (elPending) elPending.textContent = rows.length;
  if (elSelf)    elSelf.textContent    = selfCount;
  if (elDistrib) elDistrib.textContent = totalDistrib.toLocaleString("en-US");
}

async function approve(row) {
  try {
    const resp = await fetch(
      `/api/admin/villas/${encodeURIComponent(row.asset_id)}/operations/${row.id}/approve`,
      { method: "PUT", headers: csrfHeaders() }
    );
    if (!resp.ok) throw new Error(await responseError(resp));
    await load();
  } catch (err) {
    alert(`Approve failed: ${err.message}`);
  }
}

function openRejectModal(row) {
  rejectingRow = row;
  document.getElementById("voq-reject-reason").value = "";
  document.getElementById("voq-reject-error").textContent = "";
  const modal = document.getElementById("voq-reject-modal");
  modal.style.display = "flex";
}

function closeRejectModal() {
  rejectingRow = null;
  document.getElementById("voq-reject-modal").style.display = "none";
}

async function confirmReject() {
  if (!rejectingRow) return;
  const reason = document.getElementById("voq-reject-reason").value.trim();
  const errEl  = document.getElementById("voq-reject-error");
  errEl.textContent = "";
  if (!reason) {
    errEl.textContent = "A reason is required.";
    return;
  }
  try {
    const resp = await fetch(
      `/api/admin/villas/${encodeURIComponent(rejectingRow.asset_id)}/operations/${rejectingRow.id}/reject`,
      {
        method: "PUT",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({ reason }),
      }
    );
    if (!resp.ok) throw new Error(await responseError(resp));
    closeRejectModal();
    await load();
  } catch (err) {
    errEl.textContent = err.message;
  }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

function csrfHeaders(headers = {}) {
  const value = `; ${document.cookie}`;
  const parts = value.split("; csrf_token=");
  const token = parts.length === 2 ? parts.pop().split(";").shift() : null;
  return token ? { ...headers, "X-CSRF-Token": token } : headers;
}

async function responseError(resp) {
  try {
    const b = await resp.json();
    return b.error || b.message || `HTTP ${resp.status}`;
  } catch {
    return `HTTP ${resp.status}`;
  }
}

function escapeHtml(s) {
  return String(s == null ? "" : s).replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  }[c]));
}

function formatDateTime(iso) {
  if (!iso) return "—";
  try {
    const d = new Date(iso);
    return `${d.toISOString().slice(0, 10)} ${d.toISOString().slice(11, 16)} UTC`;
  } catch {
    return iso;
  }
}
