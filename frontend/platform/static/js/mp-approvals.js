/**
 * Pending Approvals — mp-approvals.js
 * Owns the admin review queue for marketplace orders with status=pending_review.
 */
(function () {
  "use strict";

  const API = "/api/admin/marketplace/approvals";
  let approvals = [];
  let activeModal = null;

  function el(tag, options = {}) {
    const node = document.createElement(tag);
    if (options.className) node.className = options.className;
    if (options.id) node.id = options.id;
    if (options.text !== undefined) node.textContent = options.text;
    if (options.attrs) {
      Object.entries(options.attrs).forEach(([key, value]) => {
        if (value !== undefined && value !== null) node.setAttribute(key, String(value));
      });
    }
    if (options.style) Object.assign(node.style, options.style);
    return node;
  }

  function getCsrfToken() {
    if (typeof window.getCsrfToken === "function") return window.getCsrfToken();
    const value = `; ${document.cookie}`;
    const parts = value.split("; csrf_token=");
    return parts.length === 2 ? parts.pop().split(";").shift() : "";
  }

  function csrfHeaders(headers = {}) {
    const token = getCsrfToken();
    return token ? { ...headers, "X-CSRF-Token": token } : headers;
  }

  function formatMoney(cents) {
    return ((Number(cents) || 0) / 100).toLocaleString(undefined, {
      style: "currency",
      currency: "USD",
    });
  }

  function formatPrice(cents) {
    return `$${((Number(cents) || 0) / 100).toFixed(2)}`;
  }

  function formatBps(bps) {
    if (bps === null || bps === undefined || Number.isNaN(Number(bps))) return "N/A";
    return `${(Number(bps) / 100).toFixed(2)}%`;
  }

  function shortId(value) {
    return String(value || "").slice(0, 8) || "unknown";
  }

  function userLabel(order) {
    if (!order.user_email) return shortId(order.user_id);
    return order.user_email.split("@")[0] || shortId(order.user_id);
  }

  function timeAgo(dateStr) {
    const timestamp = new Date(dateStr).getTime();
    if (!timestamp) return "unknown";
    const diff = Math.max(0, (Date.now() - timestamp) / 1000);
    if (diff < 60) return `${Math.floor(diff)}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  function setPendingCount(count) {
    const kpi = document.getElementById("kpi-pending-count");
    if (kpi) kpi.textContent = String(count);
  }

  function setEmptyVisible(visible) {
    const empty = document.getElementById("approvals-empty");
    if (empty) empty.style.display = visible ? "block" : "none";
  }

  function clearGrid() {
    const grid = document.getElementById("approvals-grid");
    if (grid) grid.replaceChildren();
    return grid;
  }

  function renderLoading() {
    const grid = clearGrid();
    if (!grid) return;
    setEmptyVisible(false);
    grid.style.display = "grid";
    const card = el("div", { className: "mp-approval-card" });
    card.setAttribute("aria-live", "polite");
    card.appendChild(el("h3", { text: "Loading pending approvals" }));
    card.appendChild(el("p", {
      text: "Checking marketplace orders that require manual review.",
      style: { color: "var(--admin-text-secondary)", margin: "6px 0 0" },
    }));
    grid.appendChild(card);
  }

  function renderError(message) {
    const grid = clearGrid();
    if (!grid) return;
    setPendingCount(0);
    setEmptyVisible(false);
    grid.style.display = "grid";

    const card = el("div", {
      className: "mp-approval-card",
      attrs: { role: "alert" },
      style: { borderColor: "var(--admin-danger)" },
    });
    card.appendChild(el("h3", { text: "Could not load pending approvals" }));
    card.appendChild(el("p", {
      text: message,
      style: { color: "var(--admin-text-secondary)", margin: "6px 0 16px" },
    }));
    const retry = el("button", {
      className: "admin-btn admin-btn--primary",
      text: "Retry",
      attrs: { type: "button" },
    });
    retry.addEventListener("click", loadApprovals);
    card.appendChild(retry);
    grid.appendChild(card);
  }

  function metric(label, value, danger = false) {
    const box = el("div");
    box.appendChild(el("div", {
      text: label,
      style: {
        fontSize: "11px",
        color: "var(--admin-text-muted)",
        textTransform: "uppercase",
        letterSpacing: "0.5px",
        fontWeight: "600",
      },
    }));
    box.appendChild(el("div", {
      text: value,
      style: {
        fontSize: "16px",
        fontWeight: "700",
        color: danger ? "var(--admin-danger)" : "var(--admin-text-primary)",
        marginTop: "2px",
      },
    }));
    return box;
  }

  function renderCard(order, idx) {
    const side = String(order.side || "").toUpperCase();
    const card = el("div", {
      className: "mp-approval-card",
      id: `approval-${idx}`,
      attrs: { "data-idx": idx },
      style: { animationDelay: `${idx * 0.08}s` },
    });

    const header = el("div", {
      style: {
        display: "flex",
        alignItems: "flex-start",
        justifyContent: "space-between",
        gap: "16px",
        flexWrap: "wrap",
      },
    });

    const details = el("div");
    const meta = el("div", {
      style: { display: "flex", alignItems: "center", gap: "10px", marginBottom: "8px", flexWrap: "wrap" },
    });
    meta.appendChild(el("code", {
      text: shortId(order.id),
      style: { fontSize: "12px", padding: "3px 8px", background: "var(--admin-code-bg)", borderRadius: "4px" },
    }));
    const badge = el("span", { className: "admin-badge admin-badge--info" });
    badge.appendChild(el("span", { className: "admin-badge-dot" }));
    badge.appendChild(document.createTextNode("Pending Review"));
    meta.appendChild(badge);
    meta.appendChild(el("span", {
      text: timeAgo(order.created_at),
      style: { fontSize: "12px", color: "var(--admin-text-muted)" },
    }));
    details.appendChild(meta);

    details.appendChild(el("h3", {
      text: order.asset_name || shortId(order.asset_id),
      style: {
        fontSize: "18px",
        fontWeight: "700",
        color: "var(--admin-text-primary)",
        margin: "0 0 4px",
      },
    }));

    const summary = el("p", {
      style: { fontSize: "13px", color: "var(--admin-text-secondary)", margin: "0" },
    });
    summary.appendChild(el("code", {
      text: shortId(order.user_id),
      style: { fontSize: "11px", padding: "2px 6px", background: "var(--admin-code-bg)", borderRadius: "4px" },
    }));
    summary.appendChild(document.createTextNode(` ${userLabel(order)} wants to `));
    summary.appendChild(el("span", {
      className: side === "BUY" ? "mp-side-buy" : "mp-side-sell",
      text: side || "UNKNOWN",
      style: { fontWeight: "700" },
    }));
    summary.appendChild(document.createTextNode(` ${(Number(order.quantity) || 0).toLocaleString()} tokens @ `));
    summary.appendChild(el("strong", { text: formatPrice(order.price_cents) }));
    summary.appendChild(document.createTextNode(" = "));
    summary.appendChild(el("strong", { text: formatMoney(order.total_value_cents) }));
    details.appendChild(summary);

    const actions = el("div", {
      style: { display: "flex", gap: "10px", alignItems: "flex-start" },
    });
    const approve = el("button", {
      className: "admin-btn admin-btn--success btn-approve",
      text: "Approve",
      attrs: { type: "button", "data-idx": idx },
      style: { padding: "10px 24px", fontSize: "14px", fontWeight: "600" },
    });
    const reject = el("button", {
      className: "admin-btn admin-btn--danger btn-reject",
      text: "Reject",
      attrs: { type: "button", "data-idx": idx },
      style: { padding: "10px 24px", fontSize: "14px", fontWeight: "600" },
    });
    approve.addEventListener("click", () => openConfirm(idx, "approve"));
    reject.addEventListener("click", () => openConfirm(idx, "reject"));
    actions.append(approve, reject);

    header.append(details, actions);
    card.appendChild(header);

    const warning = el("div", { className: "mp-approval-warning" });
    warning.appendChild(document.createTextNode(order.review_reason || "Flagged for admin review"));
    card.appendChild(warning);

    const metrics = el("div", {
      style: {
        display: "grid",
        gridTemplateColumns: "repeat(4, minmax(0, 1fr))",
        gap: "12px",
        marginTop: "12px",
      },
    });
    metrics.append(
      metric("Quantity", (Number(order.quantity) || 0).toLocaleString()),
      metric("Unit Price", formatPrice(order.price_cents)),
      metric("Total Value", formatMoney(order.total_value_cents)),
      metric("Supply Impact", formatBps(order.supply_impact_bps), true),
    );
    card.appendChild(metrics);

    return card;
  }

  function render() {
    const grid = clearGrid();
    if (!grid) return;

    setPendingCount(approvals.length);
    if (!approvals.length) {
      grid.style.display = "none";
      setEmptyVisible(true);
      return;
    }

    grid.style.display = "grid";
    setEmptyVisible(false);
    approvals.forEach((approval, idx) => grid.appendChild(renderCard(approval, idx)));
  }

  function setCardBusy(card, busy) {
    if (!card) return;
    card.querySelectorAll("button").forEach((button) => {
      button.disabled = busy;
      button.setAttribute("aria-busy", busy ? "true" : "false");
    });
  }

  function closeModal() {
    if (!activeModal) return;
    const { overlay, returnFocus } = activeModal;
    activeModal = null;
    overlay.classList.remove("active");
    setTimeout(() => overlay.remove(), 200);
    if (returnFocus && typeof returnFocus.focus === "function") returnFocus.focus();
  }

  function trapModalKeydown(event) {
    if (!activeModal) return;
    if (event.key === "Escape") {
      event.preventDefault();
      closeModal();
      return;
    }
    if (event.key !== "Tab") return;

    const focusable = activeModal.overlay.querySelectorAll(
      'button, textarea, input, select, a[href], [tabindex]:not([tabindex="-1"])',
    );
    if (!focusable.length) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  function openConfirm(idx, action) {
    const order = approvals[idx];
    if (!order || activeModal) return;

    const returnFocus = document.activeElement;
    const isReject = action === "reject";
    const overlay = el("div", {
      className: "ds-modal-overlay active",
      attrs: {
        role: "dialog",
        "aria-modal": "true",
        "aria-labelledby": "approval-modal-title",
      },
    });
    const modal = el("div", { className: "ds-modal" });

    // Header
    const header = el("div", { className: "ds-modal__header" });
    const headerText = el("div", {});
    headerText.appendChild(el("h3", {
      id: "approval-modal-title",
      className: "ds-modal__title",
      text: isReject ? "Reject marketplace order" : "Approve marketplace order",
    }));
    headerText.appendChild(el("p", {
      id: "approval-modal-subtitle",
      className: "ds-modal__subtitle",
      text: isReject
        ? "Rejecting releases the held balance or tokens and records the decision."
        : "Approving opens the order for matching and records the decision.",
    }));
    const closeBtn = el("button", {
      className: "ds-modal__close",
      attrs: { type: "button", "aria-label": "Close" },
    });
    closeBtn.innerHTML = '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>';
    closeBtn.addEventListener("click", closeModal);
    header.append(headerText, closeBtn);
    modal.appendChild(header);

    const body = el("div", { className: "ds-modal__body" });
    body.appendChild(el("p", {
      text: `${order.asset_name || shortId(order.asset_id)} • ${String(order.side || "").toUpperCase()} ${(Number(order.quantity) || 0).toLocaleString()} tokens • ${formatMoney(order.total_value_cents)}`,
      style: { marginTop: "0", color: "var(--admin-text-primary)", fontWeight: "600" },
    }));

    const label = el("label", {
      className: "admin-form-label",
      text: isReject ? "Rejection reason" : "Approval note",
      attrs: { for: "approval-reason" },
    });
    const reason = el("textarea", {
      id: "approval-reason",
      className: "admin-textarea",
      attrs: {
        rows: "4",
        maxlength: "500",
        placeholder: isReject ? "Explain why this order is being rejected." : "Optional note for the audit log.",
      },
      style: { width: "100%" },
    });
    if (isReject) reason.required = true;
    const status = el("div", {
      className: "ds-modal__error",
      attrs: { role: "status", "aria-live": "polite" },
    });
    body.append(label, reason, status);
    modal.appendChild(body);

    const footer = el("div", { className: "ds-modal__footer ds-modal__footer--bordered" });
    const cancel = el("button", {
      className: "ds-btn ds-btn--secondary",
      text: "Cancel",
      attrs: { type: "button" },
    });
    const confirm = el("button", {
      className: `ds-btn ${isReject ? "ds-btn--danger" : "ds-btn--primary"}`,
      text: isReject ? "Reject Order" : "Approve Order",
      attrs: { type: "button" },
    });
    cancel.addEventListener("click", closeModal);
    confirm.addEventListener("click", async () => {
      const note = reason.value.trim();
      if (isReject && !note) {
        status.textContent = "A rejection reason is required.";
        reason.focus();
        return;
      }
      confirm.disabled = true;
      cancel.disabled = true;
      confirm.setAttribute("aria-busy", "true");
      status.textContent = isReject ? "Rejecting order..." : "Approving order...";
      try {
        await submitAction(idx, action, note || (isReject ? "Rejected by admin" : "Approved by admin"));
        closeModal();
      } catch (error) {
        confirm.disabled = false;
        cancel.disabled = false;
        confirm.setAttribute("aria-busy", "false");
        status.textContent = error.message;
      }
    });
    footer.append(cancel, confirm);
    modal.appendChild(footer);
    overlay.appendChild(modal);
    overlay.addEventListener("click", (event) => {
      if (event.target === overlay) closeModal();
    });
    overlay.addEventListener("keydown", trapModalKeydown);
    document.body.appendChild(overlay);
    activeModal = { overlay, returnFocus };
    reason.focus();
  }

  async function submitAction(idx, action, reason) {
    const approval = approvals[idx];
    const card = document.getElementById(`approval-${idx}`);
    if (!approval || !card) throw new Error("Approval record is no longer available.");

    setCardBusy(card, true);
    try {
      const res = await fetch(`${API}/${approval.id}/${action}`, {
        method: "POST",
        credentials: "same-origin",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({ reason }),
      });
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.error || `HTTP ${res.status}`);
      }
      const data = await res.json().catch(() => ({}));
      animateRemoval(card, idx, action, data);
    } catch (error) {
      setCardBusy(card, false);
      window.mpToast?.(`Failed: ${error.message}`, "error");
      throw error;
    }
  }

  function animateRemoval(card, idx, action, data) {
    card.style.transition = "all 0.4s ease";
    card.style.opacity = "0";
    card.style.transform = "translateX(40px)";
    card.style.maxHeight = `${card.scrollHeight}px`;

    setTimeout(() => {
      card.style.maxHeight = "0";
      card.style.padding = "0";
      card.style.margin = "0";
      card.style.overflow = "hidden";

      setTimeout(() => {
        approvals.splice(idx, 1);
        render();
      }, 300);
    }, 300);

    if (action === "approve") {
      const syncNote = data && data.orderbook_synced === false ? " DB opened; orderbook sync pending." : "";
      window.mpToast?.(`Order approved.${syncNote}`, "success");
    } else {
      window.mpToast?.("Order rejected and holds released.", "warning");
    }
  }

  async function loadApprovals() {
    renderLoading();
    try {
      const res = await fetch(API, { credentials: "same-origin" });
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.error || `HTTP ${res.status}`);
      }
      const data = await res.json();
      approvals = Array.isArray(data) ? data : [];
      render();
    } catch (error) {
      console.warn("[mp-approvals] API unavailable:", error);
      approvals = [];
      renderError(error.message || "Unexpected load failure.");
    }
  }

  document.addEventListener("DOMContentLoaded", loadApprovals);
})();
