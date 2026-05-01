/**
 * Admin Four-Eyes Approval Queue JS
 * Loads pending approval requests, allows approve/reject actions.
 */
(function () {
  "use strict";

  let allApprovals = [];
  let currentFilter = "";
  const inFlight = new Set();

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

      setEl("kpi-pending", pending);
      setEl("kpi-approved", approved);
      setEl("kpi-rejected", rejected);

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
        return `
                <div class="approval-card" data-id="${esc(a.id)}">
                    <div class="approval-card__header">
                        <div class="approval-card__left">
                            <span class="approval-card__action-badge">${esc(actionLabel(a.action_type))}</span>
                            <span class="approval-card__entity">${esc(a.entity_type)} ${a.entity_id ? `<code>${esc(a.entity_id.substring(0, 8))}…</code>` : ""}</span>
                        </div>
                        <div class="approval-card__right">
                            <span class="approval-status-badge admin-badge--${sc.badgeClass}">${sc.label}</span>
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
                            <summary class="approval-card__payload-header">
                                <span>View Payload</span>
                                <button type="button" class="approval-btn--copy" onclick="window._copyPayload(event, '${esc(a.id)}')">Copy JSON</button>
                            </summary>
                            <pre id="payload-${esc(a.id)}">${esc(JSON.stringify(a.payload, null, 2))}</pre>
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

                    ${a.status === "pending"
            ? `
                    <div class="approval-card__actions">
                        <button class="approval-btn approval-btn--approve" onclick="window._approveRequest('${esc(a.id)}')">
                            ✅ Approve & Execute
                        </button>
                        <button class="approval-btn approval-btn--reject" onclick="window._rejectRequest('${esc(a.id)}')">
                            ❌ Reject
                        </button>
                    </div>`
            : ""
          }
                </div>
            `;
      })
      .join("");
  }

  // Expose approve/reject actions
  window._approveRequest = async function (id) {
    if (inFlight.has(id)) return;
    if (
      !await pooolConfirm({
        title: 'Approve & Execute',
        message: 'This action will be executed immediately and cannot be undone.',
        confirmText: 'Approve',
        type: 'success',
      })
    )
      return;

    try {
      setCardBusy(id, true);
      const resp = await fetch(`/api/admin/approvals/${id}/approve`, {
        method: "POST",
        headers: jsonHeaders(),
        body: JSON.stringify({}),
      });
      const data = await parseJson(resp);

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
    } finally {
      setCardBusy(id, false);
    }
  };

    window._rejectRequest = async function (id) {
    if (inFlight.has(id)) return;
    const reason = await requestRejectionReason();
    if (!reason) return;

    try {
      setCardBusy(id, true);
      const resp = await fetch(`/api/admin/approvals/${id}/reject`, {
        method: "POST",
        headers: jsonHeaders(),
        body: JSON.stringify({ reason }),
      });
      const data = await parseJson(resp);

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
    } finally {
      setCardBusy(id, false);
    }
  };

  window._copyPayload = function (e, id) {
    e.preventDefault();
    const pre = document.getElementById(`payload-${id}`);
    if (!pre) return;
    navigator.clipboard.writeText(pre.textContent).then(() => {
      showToast("JSON Payload copied to clipboard", "success");
    }).catch(err => {
      console.error("Could not copy text: ", err);
      showToast("Failed to copy payload.", "error");
    });
  };

  async function handleNewRequest(e) {
    e.preventDefault();
    const submitButton = e.target.querySelector('button[type="submit"]');
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
      if (submitButton) submitButton.disabled = true;
      const resp = await fetch("/api/admin/approvals", {
        method: "POST",
        headers: jsonHeaders(),
        body: JSON.stringify({
          action_type: actionType,
          entity_type: entityType,
          entity_id: entityId,
          payload,
        }),
      });
      const data = await parseJson(resp);

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
    } finally {
      if (submitButton) submitButton.disabled = false;
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
      pending: { bg: "#fef3c7", color: "#d97706", label: "Pending", badgeClass: "warning" },
      processing: { bg: "#dbeafe", color: "#2563eb", label: "Processing", badgeClass: "info" },
      approved: { bg: "#dcfce7", color: "#16a34a", label: "Approved", badgeClass: "success" },
      rejected: { bg: "#fee2e2", color: "#dc2626", label: "Rejected", badgeClass: "danger" },
    };
    return m[status] || m.pending;
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

  function jsonHeaders() {
    const headers = { "Content-Type": "application/json" };
    const token =
      typeof window.getCsrfToken === "function" ? window.getCsrfToken() : "";
    if (token) headers["X-CSRF-Token"] = token;
    return headers;
  }

  async function parseJson(resp) {
    try {
      return await resp.json();
    } catch {
      return {
        error: resp.statusText || `Request failed with status ${resp.status}`,
      };
    }
  }

  function setCardBusy(id, busy) {
    if (busy) {
      inFlight.add(id);
    } else {
      inFlight.delete(id);
    }
    const card = Array.from(document.querySelectorAll(".approval-card")).find(
      (item) => item.dataset.id === id,
    );
    if (!card) return;
    card.querySelectorAll(".approval-btn").forEach((btn) => {
      btn.disabled = busy;
      btn.setAttribute("aria-busy", busy ? "true" : "false");
    });
  }

  function requestRejectionReason() {
    return new Promise((resolve) => {
      const previousFocus = document.activeElement;
      const overlay = document.createElement("div");
      overlay.className = "ds-modal-overlay active";
      overlay.setAttribute("role", "dialog");
      overlay.setAttribute("aria-modal", "true");
      overlay.setAttribute("aria-labelledby", "approval-reject-title");
      overlay.innerHTML = `
        <div class="ds-modal ds-modal--sm">
          <div class="ds-modal__header">
            <div><h3 class="ds-modal__title" id="approval-reject-title">Reject approval request</h3></div>
            <button type="button" class="ds-modal__close" data-action="cancel" aria-label="Close">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
            </button>
          </div>
          <div class="ds-modal__body">
            <div class="ds-modal__error" id="approval-reject-error" aria-live="polite"></div>
            <label for="approval-reject-reason" style="display:block;font-size:14px;font-weight:500;color:#344054;margin-bottom:6px;">Reason</label>
            <textarea id="approval-reject-reason" rows="4" required style="width:100%;padding:10px 14px;border:1px solid #D0D5DD;border-radius:8px;font-size:14px;font-family:inherit;resize:vertical;box-sizing:border-box;"></textarea>
          </div>
          <div class="ds-modal__footer ds-modal__footer--bordered">
            <button type="button" class="ds-btn ds-btn--secondary" data-action="cancel">Cancel</button>
            <button type="button" class="ds-btn ds-btn--danger" data-action="confirm">Reject request</button>
          </div>
        </div>`;
      document.body.appendChild(overlay);

      const textarea = overlay.querySelector("#approval-reject-reason");
      const error = overlay.querySelector(".ds-modal__error");
      const confirm = overlay.querySelector('[data-action="confirm"]');
      const cancel = overlay.querySelector('[data-action="cancel"]');

      function close(value) {
        overlay.remove();
        if (previousFocus && typeof previousFocus.focus === "function") {
          previousFocus.focus();
        }
        resolve(value);
      }

      confirm.addEventListener("click", () => {
        const reason = textarea.value.trim();
        if (!reason) {
          error.textContent = "Enter a rejection reason.";
          textarea.focus();
          return;
        }
        close(reason);
      });
      cancel.addEventListener("click", () => close(""));
      overlay.addEventListener("click", (event) => {
        if (event.target === overlay) close("");
      });
      overlay.addEventListener("keydown", (event) => {
        if (event.key === "Escape") close("");
      });
      textarea.focus();
    });
  }

  function esc(s) {
    if (typeof s !== "string") return s || "";
    const d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }

  function showToast(msg, type) {
  if(window.showPooolToast) {
    window.showPooolToast(null, msg, type);
  }
}
})();
