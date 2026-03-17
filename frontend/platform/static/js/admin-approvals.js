/**
 * Admin Four-Eyes Approval Queue JS
 * Loads pending approval requests, allows approve/reject actions.
 */
(function () {
  "use strict";

  let allApprovals = [];
  let currentFilter = "";

  document.addEventListener("DOMContentLoaded", () => {
    loadApprovals();

    // Filter tabs
    document.querySelectorAll(".approval-tab").forEach((tab) => {
      tab.addEventListener("click", () => {
        document
          .querySelectorAll(".approval-tab")
          .forEach((t) => t.classList.remove("active"));
        tab.classList.add("active");
        currentFilter = tab.dataset.filter || "";
        renderApprovals();
      });
    });

    // New request form
    document
      .getElementById("new-request-form")
      ?.addEventListener("submit", handleNewRequest);
  });

  async function loadApprovals() {
    try {
      const resp = await fetch("/api/admin/approvals");
      if (!resp.ok) throw new Error("Failed to load approvals");
      const data = await resp.json();
      allApprovals = data.approvals || [];

      // Update KPI counters
      const pending = allApprovals.filter((a) => a.status === "pending").length;
      const approved = allApprovals.filter(
        (a) => a.status === "approved",
      ).length;
      const rejected = allApprovals.filter(
        (a) => a.status === "rejected",
      ).length;
      const expired = allApprovals.filter((a) => a.status === "expired").length;

      setEl("kpi-pending", pending);
      setEl("kpi-approved", approved);
      setEl("kpi-rejected", rejected);
      setEl("kpi-expired", expired);

      // Pending badge in sidebar
      const badge = document.getElementById("approvals-badge");
      if (badge) {
        badge.textContent = pending;
        badge.style.display = pending > 0 ? "inline-flex" : "none";
      }

      renderApprovals();
    } catch (e) {
      console.error("Failed to load approvals", e);
      if (typeof Sentry !== 'undefined') Sentry.captureException(e);
      document.getElementById("approvals-list").innerHTML =
        '<div class="admin-empty-state">Failed to load approval queue. Please refresh.</div>';
    }
  }

  function renderApprovals() {
    const container = document.getElementById("approvals-list");
    if (!container) return;

    let filtered = allApprovals;
    if (currentFilter)
      filtered = allApprovals.filter((a) => a.status === currentFilter);

    if (filtered.length === 0) {
      container.innerHTML = `
                <div class="admin-empty-state">
                    <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#9ca3af" stroke-width="1.5"><path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                    <p style="margin-top:12px;color:#6b7280;">No ${currentFilter || ""} approval requests found.</p>
                </div>`;
      return;
    }

    container.innerHTML = filtered
      .map((a) => {
        const sc = statusConfig(a.status);
        const isExpirable = a.status === "pending" && a.expires_at;
        const timeLeft = isExpirable ? getTimeLeft(a.expires_at) : null;

        return `
                <div class="approval-card" data-id="${esc(a.id)}">
                    <div class="approval-card__header">
                        <div class="approval-card__left">
                            <span class="approval-card__action-badge">${actionLabel(a.action_type)}</span>
                            <span class="approval-card__entity">${esc(a.entity_type)} ${a.entity_id ? `<code>${esc(a.entity_id.substring(0, 8))}…</code>` : ""}</span>
                        </div>
                        <div class="approval-card__right">
                            <span class="approval-status-badge" style="background:${sc.bg};color:${sc.color};">${sc.label}</span>
                            ${timeLeft ? `<span class="approval-card__expiry" title="Expires: ${a.expires_at}">⏳ ${timeLeft}</span>` : ""}
                        </div>
                    </div>

                    <div class="approval-card__body">
                        <div class="approval-card__meta">
                            <div class="approval-card__meta-item">
                                <span class="approval-card__meta-label">Requested by</span>
                                <span class="approval-card__meta-value">${esc(a.requester_name || a.requester_email)}</span>
                            </div>
                            ${a.approver_name
            ? `
                            <div class="approval-card__meta-item">
                                <span class="approval-card__meta-label">${a.status === "approved" ? "Approved by" : "Reviewed by"}</span>
                                <span class="approval-card__meta-value">${esc(a.approver_name || a.approver_email)}</span>
                            </div>`
            : ""
          }
                            <div class="approval-card__meta-item">
                                <span class="approval-card__meta-label">Created</span>
                                <span class="approval-card__meta-value">${formatDate(a.created_at)}</span>
                            </div>
                        </div>

                        ${a.payload
            ? `
                        <details class="approval-card__payload">
                            <summary>View Payload</summary>
                            <pre>${JSON.stringify(a.payload, null, 2)}</pre>
                        </details>`
            : ""
          }

                        ${a.rejection_reason
            ? `
                        <div class="approval-card__rejection">
                            <strong>Rejection reason:</strong> ${esc(a.rejection_reason)}
                        </div>`
            : ""
          }
                    </div>

                    ${a.status === "pending" && timeLeft !== "Expired"
            ? `
                    <div class="approval-card__actions">
                        <button class="approval-btn approval-btn--approve" onclick="window._approveRequest('${esc(a.id)}')">
                            ✅ Approve & Execute
                        </button>
                        <button class="approval-btn approval-btn--reject" onclick="window._rejectRequest('${esc(a.id)}')">
                            ❌ Reject
                        </button>
                    </div>`
            : (a.status === "pending" && timeLeft === "Expired" 
                ? `<div class="approval-card__actions" style="color:var(--admin-danger); font-size:12px; font-weight:600;">Request has expired.</div>` 
                : "")
          }
                </div>
            `;
      })
      .join("");
  }

  // Expose approve/reject actions
  window._approveRequest = async function (id) {
    if (
      !confirm(
        "Are you sure you want to APPROVE this request? The action will be executed immediately.",
      )
    )
      return;

    try {
      const resp = await fetch(`/api/admin/approvals/${id}/approve`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });
      const data = await resp.json();

      if (resp.ok) {
        showToast(
          `✅ Approved: ${data.message || "Action executed successfully."}`,
          "success",
        );
        loadApprovals();
      } else {
        showToast(`⚠️ ${data.error || "Failed to approve."}`, "error");
      }
    } catch (e) {
      console.error("Error approving request", e);
      if (typeof Sentry !== 'undefined') Sentry.captureException(e);
      showToast("Network error. Please try again.", "error");
    }
  };

  window._rejectRequest = async function (id) {
    const reason = prompt("Rejection reason (required):");
    if (!reason) return;

    try {
      const resp = await fetch(`/api/admin/approvals/${id}/reject`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ reason }),
      });
      const data = await resp.json();

      if (resp.ok) {
        showToast("Request rejected.", "success");
        loadApprovals();
      } else {
        showToast(`⚠️ ${data.error || "Failed to reject."}`, "error");
      }
    } catch (e) {
      console.error("Error rejecting request", e);
      if (typeof Sentry !== 'undefined') Sentry.captureException(e);
      showToast("Network error.", "error");
    }
  };

  async function handleNewRequest(e) {
    e.preventDefault();
    const actionType = document.getElementById("req-action-type")?.value;
    const entityType = document.getElementById("req-entity-type")?.value;
    const entityId =
      document.getElementById("req-entity-id")?.value?.trim() || undefined;
    let payload = {};

    try {
      const payloadStr = document.getElementById("req-payload")?.value?.trim();
      if (payloadStr) payload = JSON.parse(payloadStr);
    } catch {
      showToast("Invalid payload JSON.", "error");
      return;
    }

    try {
      const resp = await fetch("/api/admin/approvals", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action_type: actionType,
          entity_type: entityType,
          entity_id: entityId,
          payload,
        }),
      });
      const data = await resp.json();

      if (resp.ok) {
        showToast(
          `✅ Approval request created (${data.approval_id?.substring(0, 8)}…). Awaiting checker review.`,
          "success",
        );
        document.getElementById("new-request-form").reset();
        loadApprovals();
      } else {
        showToast(`⚠️ ${data.error}`, "error");
      }
    } catch (e) {
      console.error("Error creating approval request", e);
      if (typeof Sentry !== 'undefined') Sentry.captureException(e);
      showToast("Network error.", "error");
    }
  }

  function actionLabel(type) {
    const map = {
      "deposit.confirm": "💰 Deposit Confirm",
      "deposit.cancel": "🚫 Deposit Cancel",
      "balance.adjust": "💳 Balance Adjust",
      "user.suspend": "🔒 User Suspend",
      "user.delete": "🗑️ User Delete",
      "kyc.override": "🛡️ KYC Override",
      "kyc.reject": "❌ KYC Reject",
      "treasury.payout": "🏦 Treasury Payout",
      "settings.update": "⚙️ Settings Update",
      "submission.approve": "📋 Submission Approve",
      "submission.reject": "📋 Submission Reject",
    };
    return map[type] || type;
  }

  function statusConfig(status) {
    const m = {
      pending: { bg: "#fef3c7", color: "#d97706", label: "Pending" },
      approved: { bg: "#dcfce7", color: "#16a34a", label: "Approved" },
      rejected: { bg: "#fee2e2", color: "#dc2626", label: "Rejected" },
      expired: { bg: "#f3f4f6", color: "#6b7280", label: "Expired" },
    };
    return m[status] || m.pending;
  }

  function getTimeLeft(expiresAt) {
    const now = new Date();
    const exp = new Date(expiresAt);
    const diff = exp - now;
    if (diff <= 0) return "Expired";
    const hours = Math.floor(diff / 3600000);
    const mins = Math.floor((diff % 3600000) / 60000);
    return hours > 0 ? `${hours}h ${mins}m` : `${mins}m`;
  }

  function formatDate(iso) {
    if (!iso) return "—";
    const d = new Date(iso);
    return d.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
  }

  function setEl(id, val) {
    const el = document.getElementById(id);
    if (el) el.textContent = val;
  }

  function esc(s) {
    if (typeof s !== "string") return s || "";
    const d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }

  function showToast(msg, type) {
    const colors = { success: "#22c55e", error: "#ef4444", info: "#6366f1" };
    const t = document.createElement("div");
    t.style.cssText = `position:fixed;bottom:24px;right:24px;z-index:9999;padding:14px 22px;border-radius:12px;font-size:14px;font-weight:500;background:${colors[type] || colors.info};color:#fff;box-shadow:0 8px 30px rgba(0,0,0,0.2);animation:fadeIn 0.25s ease;max-width:500px;`;
    t.textContent = msg;
    document.body.appendChild(t);
    setTimeout(() => t.remove(), 5000);
  }
})();
