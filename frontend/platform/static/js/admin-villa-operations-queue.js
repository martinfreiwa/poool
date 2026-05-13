/**
 * Admin Villa Operations Approval Queue — Villa-Returns P2.3.
 * URL: /admin/villa-operations-queue
 *
 * Calls:
 *   GET /api/admin/approvals/villa-operations           list submitted rows
 *   PUT /api/admin/villas/:asset_id/operations/:log_id/approve
 *   PUT /api/admin/villas/:asset_id/operations/:log_id/reject
 *
 * 4-eyes is server-enforced. Client also greys out Approve when current user
 * matches the submitter to avoid a guaranteed-409 click.
 */

let currentUserId = null;
let rejectingRow = null;

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
  tbody.innerHTML = `<tr><td colspan="6" style="padding: 24px; text-align: center; color: var(--admin-text-muted, #6b7280);">Loading…</td></tr>`;
  document.getElementById("voq-empty").style.display = "none";
  try {
    const resp = await fetch("/api/admin/villa-operations-queue");
    if (!resp.ok) throw new Error(await responseError(resp));
    const rows = await resp.json();
    render(rows);
  } catch (err) {
    tbody.innerHTML = `<tr><td colspan="6" style="padding: 16px; color: var(--admin-danger, #dc2626);">Failed to load: ${escapeHtml(err.message)}</td></tr>`;
  }
}

function render(rows) {
  const tbody = document.getElementById("voq-tbody");
  const empty = document.getElementById("voq-empty");
  const table = document.getElementById("voq-table");
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
    const isSelfSubmitted = currentUserId && r.submitted_by === currentUserId;
    const tr = document.createElement("tr");

    const tdVilla = document.createElement("td");
    tdVilla.innerHTML = `<a href="/admin/asset-details.html?id=${encodeURIComponent(r.asset_id)}" style="color: inherit; text-decoration: underline;">${escapeHtml(r.asset_title)}</a>`;
    if (r.supersedes_id) {
      tdVilla.innerHTML += `<div class="voq-correction-note">Correction · ${escapeHtml(r.correction_reason || "")}</div>`;
    }
    tr.appendChild(tdVilla);

    const tdPeriod = document.createElement("td");
    tdPeriod.textContent = `${r.period_year}-${String(r.period_month).padStart(2, "0")}`;
    tr.appendChild(tdPeriod);

    const tdDist = document.createElement("td");
    tdDist.className = "voq-num";
    tdDist.textContent = Number(r.distributable_idr_cents).toLocaleString();
    tr.appendChild(tdDist);

    const tdSubmitter = document.createElement("td");
    tdSubmitter.innerHTML = `<div>${escapeHtml(r.submitter_email || "(unknown)")}</div>`;
    if (isSelfSubmitted) {
      tdSubmitter.innerHTML += `<div class="voq-self-note">You submitted this — another admin must approve.</div>`;
    }
    tr.appendChild(tdSubmitter);

    const tdWhen = document.createElement("td");
    tdWhen.textContent = formatDateTime(r.submitted_at || r.recorded_at);
    tr.appendChild(tdWhen);

    const tdActions = document.createElement("td");
    tdActions.className = "voq-actions";
    const reviewBtn = document.createElement("a");
    reviewBtn.className = "voq-btn";
    reviewBtn.href = `/admin/villas/${encodeURIComponent(r.asset_id)}/operations/${r.period_year}/${r.period_month}?log_id=${r.id}&mode=review`;
    reviewBtn.textContent = "Review";
    tdActions.appendChild(reviewBtn);

    const approveBtn = document.createElement("button");
    approveBtn.type = "button";
    approveBtn.className = "voq-btn primary";
    approveBtn.textContent = "Approve";
    approveBtn.disabled = !!isSelfSubmitted;
    approveBtn.addEventListener("click", () => approve(r));
    tdActions.appendChild(approveBtn);

    const rejectBtn = document.createElement("button");
    rejectBtn.type = "button";
    rejectBtn.className = "voq-btn danger";
    rejectBtn.textContent = "Reject";
    rejectBtn.addEventListener("click", () => openRejectModal(r));
    tdActions.appendChild(rejectBtn);

    tr.appendChild(tdActions);
    tbody.appendChild(tr);
  }
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
  document.getElementById("voq-reject-modal").style.display = "flex";
}
function closeRejectModal() {
  rejectingRow = null;
  document.getElementById("voq-reject-modal").style.display = "none";
}

async function confirmReject() {
  if (!rejectingRow) return;
  const reason = document.getElementById("voq-reject-reason").value.trim();
  const errEl = document.getElementById("voq-reject-error");
  errEl.textContent = "";
  if (!reason) {
    errEl.textContent = "Reason is required.";
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

// ─── Helpers ────────────────────────────────────────────────────

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
