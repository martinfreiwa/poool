/**
 * marketplace-p2p.js — P2P/OTC Offer UI (Task 5.9)
 *
 * Provides:
 * - Incoming offers panel with accept/decline/counter actions
 * - Outgoing offers panel with cancel action
 * - Create offer modal (send offer to a specific user)
 * - Notification badge for pending incoming offers
 * - Per-asset P2P offer list
 *
 * Requires: marketplace-event-bus.js
 */
const MarketP2P = (function () {
  "use strict";

  let _assetId = null;
  let _container = null;

  // ═══════════════════════════════════════════════════════════════
  // ── API CALLS ──────────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  async function apiFetch(url, options = {}) {
    try {
      const res = await fetch(url, {
        credentials: "same-origin",
        headers: { "Content-Type": "application/json" },
        ...options,
      });
      const data = await res.json();
      if (!res.ok) return { ok: false, error: data.error || data.message || `HTTP ${res.status}` };
      return { ok: true, data };
    } catch (err) {
      console.error("[P2P] API error:", err);
      return { ok: false, error: "Network error" };
    }
  }

  async function createOffer(offerData) {
    return apiFetch("/api/marketplace/p2p/offers", {
      method: "POST",
      body: JSON.stringify(offerData),
    });
  }

  async function respondToOffer(offerId, action, extra = {}) {
    return apiFetch(`/api/marketplace/p2p/offers/${offerId}/respond`, {
      method: "POST",
      body: JSON.stringify({ action, ...extra }),
    });
  }

  async function cancelOffer(offerId) {
    return apiFetch(`/api/marketplace/p2p/offers/${offerId}`, {
      method: "DELETE",
    });
  }

  async function getIncoming() {
    return apiFetch("/api/marketplace/p2p/offers/incoming");
  }

  async function getOutgoing() {
    return apiFetch("/api/marketplace/p2p/offers/outgoing");
  }

  async function getAssetOffers(assetId) {
    return apiFetch(`/api/marketplace/${assetId}/p2p`);
  }

  // ═══════════════════════════════════════════════════════════════
  // ── FORMATTING HELPERS ─────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function fmtUSD(cents) {
    return "$" + (cents / 100).toFixed(2);
  }

  function fmtTime(isoStr) {
    const d = new Date(isoStr);
    const now = new Date();
    const diffMs = now - d;
    if (diffMs < 60000) return "just now";
    if (diffMs < 3600000) return Math.floor(diffMs / 60000) + "m ago";
    if (diffMs < 86400000) return Math.floor(diffMs / 3600000) + "h ago";
    return d.toLocaleDateString();
  }

  function fmtExpiry(isoStr) {
    const d = new Date(isoStr);
    const now = new Date();
    const diffMs = d - now;
    if (diffMs <= 0) return "expired";
    if (diffMs < 3600000) return Math.floor(diffMs / 60000) + "m left";
    if (diffMs < 86400000) return Math.floor(diffMs / 3600000) + "h left";
    return Math.floor(diffMs / 86400000) + "d left";
  }

  function statusBadge(status) {
    const colors = {
      pending: "#f59e0b",
      accepted: "#00c896",
      declined: "#ef4444",
      expired: "#7a7f87",
      cancelled: "#7a7f87",
      countered: "#818cf8",
      admin_cancelled: "#ef4444",
    };
    const color = colors[status] || "#7a7f87";
    return `<span class="p2p-status" style="color:${color};font-size:11px;font-weight:600;text-transform:uppercase;">${status}</span>`;
  }

  // ═══════════════════════════════════════════════════════════════
  // ── NOTIFICATION BADGE ─────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  async function updateNotificationBadge() {
    const result = await getIncoming();
    if (!result.ok) return;

    const pendingCount = result.data.filter(
      (o) => o.status === "pending"
    ).length;

    // Update badge on the P2P tab/button
    const badge = document.getElementById("p2p-badge");
    if (badge) {
      if (pendingCount > 0) {
        badge.textContent = pendingCount;
        badge.style.display = "inline-flex";
      } else {
        badge.style.display = "none";
      }
    }
  }

  // ═══════════════════════════════════════════════════════════════
  // ── OFFER CARD ─────────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function renderOfferCard(offer, isIncoming) {
    const total = (offer.price_cents * offer.quantity) / 100;
    const counterParty = isIncoming
      ? offer.maker_user_id.slice(0, 8)
      : offer.taker_user_id.slice(0, 8);

    let actions = "";
    if (offer.status === "pending") {
      if (isIncoming) {
        actions = `
          <div class="p2p-actions">
            <button class="p2p-btn p2p-btn--accept" data-id="${offer.id}" data-action="accept"
              title="Accept this offer">✓ Accept</button>
            <button class="p2p-btn p2p-btn--decline" data-id="${offer.id}" data-action="decline"
              title="Decline this offer">✕ Decline</button>
            <button class="p2p-btn p2p-btn--counter" data-id="${offer.id}" data-action="counter"
              title="Send a counter-offer">↩ Counter</button>
          </div>
        `;
      } else {
        actions = `
          <div class="p2p-actions">
            <button class="p2p-btn p2p-btn--cancel" data-id="${offer.id}" data-action="cancel"
              title="Cancel this offer">Cancel</button>
          </div>
        `;
      }
    }

    return `
      <div class="p2p-card" data-offer-id="${offer.id}">
        <div class="p2p-card-header">
          <span class="p2p-side p2p-side--${offer.side}">${offer.side.toUpperCase()}</span>
          ${statusBadge(offer.status)}
          <span class="p2p-time">${fmtTime(offer.created_at)}</span>
        </div>
        <div class="p2p-card-body">
          <div class="p2p-detail">
            <span class="p2p-label">${isIncoming ? "From" : "To"}</span>
            <span class="p2p-value">…${counterParty}</span>
          </div>
          <div class="p2p-detail">
            <span class="p2p-label">Price</span>
            <span class="p2p-value">${fmtUSD(offer.price_cents)}</span>
          </div>
          <div class="p2p-detail">
            <span class="p2p-label">Qty</span>
            <span class="p2p-value">${offer.quantity} shares</span>
          </div>
          <div class="p2p-detail">
            <span class="p2p-label">Total</span>
            <span class="p2p-value p2p-total">${fmtUSD(offer.price_cents * offer.quantity)}</span>
          </div>
          ${offer.status === "pending" ? `
            <div class="p2p-detail">
              <span class="p2p-label">Expires</span>
              <span class="p2p-value p2p-expiry" title="${new Date(offer.expires_at).toLocaleString()}">${fmtExpiry(offer.expires_at)}</span>
            </div>
          ` : ""}
          ${offer.message ? `
            <div class="p2p-message">"${offer.message}"</div>
          ` : ""}
        </div>
        ${actions}
      </div>
    `;
  }

  // ═══════════════════════════════════════════════════════════════
  // ── OFFER LISTS ────────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  async function loadIncomingOffers(container) {
    const result = await getIncoming();
    if (!result.ok) {
      container.innerHTML =
        '<div class="p2p-empty">Unable to load offers</div>';
      return;
    }

    const filtered = _assetId
      ? result.data.filter((o) => o.asset_id === _assetId)
      : result.data;

    if (!filtered.length) {
      container.innerHTML =
        '<div class="p2p-empty">No incoming offers</div>';
      return;
    }

    container.innerHTML = filtered
      .map((o) => renderOfferCard(o, true))
      .join("");
    bindActions(container);
  }

  async function loadOutgoingOffers(container) {
    const result = await getOutgoing();
    if (!result.ok) {
      container.innerHTML =
        '<div class="p2p-empty">Unable to load offers</div>';
      return;
    }

    const filtered = _assetId
      ? result.data.filter((o) => o.asset_id === _assetId)
      : result.data;

    if (!filtered.length) {
      container.innerHTML =
        '<div class="p2p-empty">No outgoing offers</div>';
      return;
    }

    container.innerHTML = filtered
      .map((o) => renderOfferCard(o, false))
      .join("");
    bindActions(container);
  }

  // ═══════════════════════════════════════════════════════════════
  // ── ACTION HANDLERS ────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function bindActions(container) {
    container.querySelectorAll(".p2p-btn").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const id = btn.dataset.id;
        const action = btn.dataset.action;
        btn.disabled = true;
        btn.textContent = "…";

        let result;

        if (action === "cancel") {
          result = await cancelOffer(id);
        } else if (action === "counter") {
          showCounterModal(id);
          btn.disabled = false;
          btn.textContent = "↩ Counter";
          return;
        } else {
          result = await respondToOffer(id, action);
        }

        if (result.ok) {
          showToast(
            action === "accept"
              ? "Offer accepted — trade settled!"
              : action === "decline"
                ? "Offer declined"
                : "Offer cancelled",
            action === "accept" ? "success" : "info"
          );
          refreshAll();
          window.MarketBus?.emit("p2p:updated", result.data);
        } else {
          showToast(result.error, "error");
          btn.disabled = false;
          btn.textContent =
            action === "accept" ? "✓ Accept" :
            action === "decline" ? "✕ Decline" : "Cancel";
        }
      });
    });
  }

  // ═══════════════════════════════════════════════════════════════
  // ── CREATE OFFER MODAL ─────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function showCreateOfferModal() {
    removeExistingModal();

    const overlay = document.createElement("div");
    overlay.className = "p2p-modal-overlay";
    overlay.innerHTML = `
      <div class="p2p-modal">
        <div class="p2p-modal-header">
          <h3>Send P2P Offer</h3>
          <button class="p2p-modal-close" title="Close">✕</button>
        </div>
        <form id="p2p-create-form" class="p2p-modal-body">
          <div class="p2p-form-group">
            <label>Recipient User ID</label>
            <input type="text" id="p2p-taker-id" placeholder="Enter user UUID" required />
          </div>
          <div class="p2p-form-row">
            <div class="p2p-form-group">
              <label>Side</label>
              <select id="p2p-side">
                <option value="sell">Sell</option>
                <option value="buy">Buy</option>
              </select>
            </div>
            <div class="p2p-form-group">
              <label>Price (USD)</label>
              <input type="number" id="p2p-price" min="0.01" step="0.01" placeholder="105.00" required />
            </div>
          </div>
          <div class="p2p-form-row">
            <div class="p2p-form-group">
              <label>Quantity</label>
              <input type="number" id="p2p-qty" min="1" step="1" value="1" required />
            </div>
            <div class="p2p-form-group">
              <label>Expires (hours)</label>
              <input type="number" id="p2p-expires" min="1" max="168" value="48" />
            </div>
          </div>
          <div class="p2p-form-group">
            <label>Message (optional)</label>
            <textarea id="p2p-message" rows="2" maxlength="500" placeholder="Add a message…"></textarea>
          </div>
          <div class="p2p-form-summary" id="p2p-form-summary">
            Total: $0.00
          </div>
          <button type="submit" class="p2p-btn p2p-btn--submit">Send Offer</button>
        </form>
      </div>
    `;

    document.body.appendChild(overlay);

    // Close handlers
    overlay
      .querySelector(".p2p-modal-close")
      .addEventListener("click", () => overlay.remove());
    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) overlay.remove();
    });

    // Price/qty → summary
    const priceInput = overlay.querySelector("#p2p-price");
    const qtyInput = overlay.querySelector("#p2p-qty");
    const summaryEl = overlay.querySelector("#p2p-form-summary");

    function updateSummary() {
      const p = parseFloat(priceInput.value) || 0;
      const q = parseInt(qtyInput.value) || 0;
      summaryEl.textContent = `Total: $${(p * q).toFixed(2)}`;
    }
    priceInput.addEventListener("input", updateSummary);
    qtyInput.addEventListener("input", updateSummary);

    // Submit
    overlay
      .querySelector("#p2p-create-form")
      .addEventListener("submit", async (e) => {
        e.preventDefault();
        const submitBtn = overlay.querySelector(".p2p-btn--submit");
        submitBtn.disabled = true;
        submitBtn.textContent = "Sending…";

        const result = await createOffer({
          asset_id: _assetId,
          taker_user_id: overlay.querySelector("#p2p-taker-id").value.trim(),
          side: overlay.querySelector("#p2p-side").value,
          price_cents: Math.round(
            parseFloat(overlay.querySelector("#p2p-price").value) * 100
          ),
          quantity: parseInt(overlay.querySelector("#p2p-qty").value),
          message: overlay.querySelector("#p2p-message").value.trim() || null,
          expires_in_hours:
            parseInt(overlay.querySelector("#p2p-expires").value) || 48,
        });

        if (result.ok) {
          showToast("Offer sent successfully!", "success");
          overlay.remove();
          refreshAll();
          window.MarketBus?.emit("p2p:created", result.data);
        } else {
          showToast(result.error, "error");
          submitBtn.disabled = false;
          submitBtn.textContent = "Send Offer";
        }
      });
  }

  function showCounterModal(offerId) {
    removeExistingModal();

    const overlay = document.createElement("div");
    overlay.className = "p2p-modal-overlay";
    overlay.innerHTML = `
      <div class="p2p-modal p2p-modal--counter">
        <div class="p2p-modal-header">
          <h3>Counter Offer</h3>
          <button class="p2p-modal-close" title="Close">✕</button>
        </div>
        <form id="p2p-counter-form" class="p2p-modal-body">
          <div class="p2p-form-row">
            <div class="p2p-form-group">
              <label>Counter Price (USD)</label>
              <input type="number" id="p2p-counter-price" min="0.01" step="0.01" required />
            </div>
            <div class="p2p-form-group">
              <label>Counter Quantity</label>
              <input type="number" id="p2p-counter-qty" min="1" step="1" value="1" required />
            </div>
          </div>
          <div class="p2p-form-group">
            <label>Message (optional)</label>
            <textarea id="p2p-counter-msg" rows="2" maxlength="500" placeholder="Add a message…"></textarea>
          </div>
          <button type="submit" class="p2p-btn p2p-btn--submit">Send Counter Offer</button>
        </form>
      </div>
    `;

    document.body.appendChild(overlay);

    overlay
      .querySelector(".p2p-modal-close")
      .addEventListener("click", () => overlay.remove());
    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) overlay.remove();
    });

    overlay
      .querySelector("#p2p-counter-form")
      .addEventListener("submit", async (e) => {
        e.preventDefault();
        const submitBtn = overlay.querySelector(".p2p-btn--submit");
        submitBtn.disabled = true;
        submitBtn.textContent = "Sending…";

        const result = await respondToOffer(offerId, "counter", {
          counter_price_cents: Math.round(
            parseFloat(overlay.querySelector("#p2p-counter-price").value) * 100
          ),
          counter_quantity: parseInt(
            overlay.querySelector("#p2p-counter-qty").value
          ),
          message: overlay.querySelector("#p2p-counter-msg").value.trim() || null,
        });

        if (result.ok) {
          showToast("Counter offer sent!", "success");
          overlay.remove();
          refreshAll();
        } else {
          showToast(result.error, "error");
          submitBtn.disabled = false;
          submitBtn.textContent = "Send Counter Offer";
        }
      });
  }

  function removeExistingModal() {
    document.querySelector(".p2p-modal-overlay")?.remove();
  }

  // ═══════════════════════════════════════════════════════════════
  // ── TOAST ──────────────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function showToast(message, type = "info") {
    // Use MarketTrading's toast if available
    if (window.MarketTrading?.showToast) {
      window.MarketTrading.showToast(message, type);
      return;
    }
    // Fallback
    const existing = document.querySelector(".trade-toast");
    if (existing) existing.remove();
    const toast = document.createElement("div");
    toast.className = `trade-toast trade-toast--${type}`;
    toast.setAttribute("role", "alert");
    toast.textContent = message;
    document.body.appendChild(toast);
    requestAnimationFrame(() => toast.classList.add("show"));
    setTimeout(() => {
      toast.classList.remove("show");
      setTimeout(() => toast.remove(), 300);
    }, 4000);
  }

  // ═══════════════════════════════════════════════════════════════
  // ── MAIN PANEL RENDER ──────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function renderPanel(container) {
    container.innerHTML = `
      <div class="p2p-panel">
        <div class="p2p-panel-header">
          <h3>P2P Offers</h3>
          <span id="p2p-badge" class="p2p-badge" style="display:none;">0</span>
          <button class="p2p-btn p2p-btn--new" id="p2p-create-btn" title="Create a new P2P offer">
            + New Offer
          </button>
        </div>
        <div class="p2p-tabs">
          <button class="p2p-tab active" data-tab="incoming">
            Incoming <span id="p2p-incoming-count" class="p2p-tab-count"></span>
          </button>
          <button class="p2p-tab" data-tab="outgoing">
            Outgoing
          </button>
        </div>
        <div class="p2p-tab-content" id="p2p-incoming" style="display:block;">
          <div class="p2p-loading">Loading offers…</div>
        </div>
        <div class="p2p-tab-content" id="p2p-outgoing" style="display:none;">
          <div class="p2p-loading">Loading offers…</div>
        </div>
      </div>
    `;

    // Tab switching
    container.querySelectorAll(".p2p-tab").forEach((tab) => {
      tab.addEventListener("click", () => {
        container.querySelectorAll(".p2p-tab").forEach((t) => t.classList.remove("active"));
        container.querySelectorAll(".p2p-tab-content").forEach((c) => (c.style.display = "none"));
        tab.classList.add("active");
        const target = container.querySelector(`#p2p-${tab.dataset.tab}`);
        if (target) target.style.display = "block";
      });
    });

    // New offer button
    container
      .querySelector("#p2p-create-btn")
      .addEventListener("click", showCreateOfferModal);

    // Load data
    loadIncomingOffers(container.querySelector("#p2p-incoming"));
    loadOutgoingOffers(container.querySelector("#p2p-outgoing"));
    updateNotificationBadge();
  }

  function refreshAll() {
    const incEl = document.getElementById("p2p-incoming");
    const outEl = document.getElementById("p2p-outgoing");
    if (incEl) loadIncomingOffers(incEl);
    if (outEl) loadOutgoingOffers(outEl);
    updateNotificationBadge();
  }

  // ═══════════════════════════════════════════════════════════════
  // ── CSS INJECTION ──────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function injectStyles() {
    if (document.getElementById("p2p-styles")) return;

    const style = document.createElement("style");
    style.id = "p2p-styles";
    style.textContent = `
      .p2p-panel { margin-top: 16px; }
      .p2p-panel-header {
        display: flex; align-items: center; gap: 10px;
        margin-bottom: 12px;
      }
      .p2p-panel-header h3 {
        font-size: 15px; font-weight: 600; color: #e1e3e6;
        margin: 0; flex: 1;
      }
      .p2p-badge {
        background: #ef4444; color: white; font-size: 11px; font-weight: 700;
        min-width: 18px; height: 18px; border-radius: 9px;
        display: inline-flex; align-items: center; justify-content: center;
        padding: 0 5px;
      }
      .p2p-tabs {
        display: flex; gap: 0; margin-bottom: 12px;
        border-bottom: 1px solid rgba(255,255,255,0.1);
      }
      .p2p-tab {
        padding: 8px 16px; font-size: 13px; cursor: pointer;
        background: none; border: none; color: #7a7f87;
        border-bottom: 2px solid transparent; transition: all 0.2s;
        font-family: inherit;
      }
      .p2p-tab:hover { color: #e1e3e6; }
      .p2p-tab.active {
        color: #00c896; border-bottom-color: #00c896;
      }
      .p2p-tab-count {
        font-size: 11px; background: rgba(0,200,150,0.15);
        color: #00c896; border-radius: 8px; padding: 1px 6px;
        margin-left: 4px;
      }

      .p2p-card {
        background: rgba(255,255,255,0.03);
        border: 1px solid rgba(255,255,255,0.08);
        border-radius: 10px; padding: 12px 14px;
        margin-bottom: 8px; transition: border-color 0.2s;
      }
      .p2p-card:hover { border-color: rgba(255,255,255,0.15); }
      .p2p-card-header {
        display: flex; align-items: center; gap: 8px; margin-bottom: 8px;
      }
      .p2p-side {
        font-size: 11px; font-weight: 700; padding: 2px 8px;
        border-radius: 4px; text-transform: uppercase;
      }
      .p2p-side--buy { background: rgba(0,200,150,0.12); color: #00c896; }
      .p2p-side--sell { background: rgba(239,68,68,0.12); color: #ef4444; }
      .p2p-time { font-size: 11px; color: #7a7f87; margin-left: auto; }
      .p2p-card-body { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 12px; }
      .p2p-detail { display: flex; justify-content: space-between; }
      .p2p-label { font-size: 12px; color: #7a7f87; }
      .p2p-value { font-size: 12px; color: #e1e3e6; font-weight: 500; }
      .p2p-total { font-weight: 600; color: #ffffff; }
      .p2p-expiry { color: #f59e0b; }
      .p2p-message {
        grid-column: 1 / -1; font-size: 12px; color: #a0a4ab;
        font-style: italic; margin-top: 4px;
        padding-top: 4px; border-top: 1px solid rgba(255,255,255,0.05);
      }

      .p2p-actions {
        display: flex; gap: 6px; margin-top: 10px;
        padding-top: 10px; border-top: 1px solid rgba(255,255,255,0.06);
      }
      .p2p-btn {
        padding: 6px 14px; border-radius: 6px; border: none;
        font-size: 12px; font-weight: 600; cursor: pointer;
        transition: all 0.2s; font-family: inherit;
      }
      .p2p-btn:disabled { opacity: 0.5; cursor: not-allowed; }
      .p2p-btn--accept {
        background: rgba(0,200,150,0.15); color: #00c896;
      }
      .p2p-btn--accept:hover:not(:disabled) {
        background: rgba(0,200,150,0.25);
      }
      .p2p-btn--decline {
        background: rgba(239,68,68,0.1); color: #ef4444;
      }
      .p2p-btn--decline:hover:not(:disabled) {
        background: rgba(239,68,68,0.2);
      }
      .p2p-btn--counter {
        background: rgba(129,140,248,0.1); color: #818cf8;
      }
      .p2p-btn--counter:hover:not(:disabled) {
        background: rgba(129,140,248,0.2);
      }
      .p2p-btn--cancel {
        background: rgba(122,127,135,0.15); color: #7a7f87;
      }
      .p2p-btn--cancel:hover:not(:disabled) {
        background: rgba(122,127,135,0.25); color: #e1e3e6;
      }
      .p2p-btn--new {
        background: rgba(0,200,150,0.1); color: #00c896;
        padding: 5px 12px; font-size: 12px;
      }
      .p2p-btn--new:hover { background: rgba(0,200,150,0.2); }
      .p2p-btn--submit {
        width: 100%; background: #00c896; color: white;
        padding: 10px; font-size: 14px; border-radius: 8px;
        margin-top: 8px;
      }
      .p2p-btn--submit:hover:not(:disabled) { background: #00b085; }

      .p2p-empty {
        text-align: center; color: #7a7f87; padding: 24px;
        font-size: 13px;
      }
      .p2p-loading {
        text-align: center; color: #7a7f87; padding: 24px;
        font-size: 13px;
      }

      /* ── Modal ── */
      .p2p-modal-overlay {
        position: fixed; inset: 0; z-index: 9999;
        background: rgba(0,0,0,0.7); backdrop-filter: blur(4px);
        display: flex; align-items: center; justify-content: center;
        animation: p2p-fade-in 0.2s ease;
      }
      @keyframes p2p-fade-in { from { opacity: 0; } to { opacity: 1; } }
      .p2p-modal {
        background: #1a1d23; border: 1px solid rgba(255,255,255,0.1);
        border-radius: 14px; width: 420px; max-width: 95vw;
        box-shadow: 0 20px 60px rgba(0,0,0,0.5);
        animation: p2p-slide-up 0.25s ease;
      }
      @keyframes p2p-slide-up {
        from { transform: translateY(20px); opacity: 0; }
        to { transform: translateY(0); opacity: 1; }
      }
      .p2p-modal-header {
        display: flex; align-items: center; justify-content: space-between;
        padding: 16px 20px; border-bottom: 1px solid rgba(255,255,255,0.08);
      }
      .p2p-modal-header h3 {
        font-size: 16px; font-weight: 600; color: #e1e3e6; margin: 0;
      }
      .p2p-modal-close {
        background: none; border: none; color: #7a7f87;
        font-size: 18px; cursor: pointer; padding: 4px 8px;
        border-radius: 6px; transition: all 0.2s;
      }
      .p2p-modal-close:hover { background: rgba(255,255,255,0.05); color: #e1e3e6; }
      .p2p-modal-body { padding: 16px 20px; }
      .p2p-form-group { margin-bottom: 12px; }
      .p2p-form-group label {
        display: block; font-size: 12px; color: #7a7f87;
        margin-bottom: 4px; font-weight: 500;
      }
      .p2p-form-group input, .p2p-form-group textarea, .p2p-form-group select {
        width: 100%; padding: 8px 12px; border-radius: 8px;
        border: 1px solid rgba(255,255,255,0.12); background: rgba(255,255,255,0.04);
        color: #e1e3e6; font-size: 13px; font-family: inherit;
        transition: border-color 0.2s; box-sizing: border-box;
      }
      .p2p-form-group input:focus, .p2p-form-group textarea:focus, .p2p-form-group select:focus {
        outline: none; border-color: #00c896;
      }
      .p2p-form-row { display: flex; gap: 12px; }
      .p2p-form-row .p2p-form-group { flex: 1; }
      .p2p-form-summary {
        text-align: center; font-size: 16px; font-weight: 600;
        color: #e1e3e6; padding: 8px; margin-top: 4px;
        border-radius: 8px; background: rgba(255,255,255,0.03);
      }
    `;
    document.head.appendChild(style);
  }

  // ═══════════════════════════════════════════════════════════════
  // ── INIT ───────────────────────────────────────────────────
  // ═══════════════════════════════════════════════════════════════

  function init(containerId, assetId) {
    _assetId = assetId;
    _container = document.getElementById(containerId);

    if (!_container) {
      console.warn("[P2P] Container not found:", containerId);
      return;
    }

    injectStyles();
    renderPanel(_container);

    // Refresh on trade events
    if (window.MarketBus) {
      window.MarketBus.on("trade", refreshAll);
      window.MarketBus.on("p2p:updated", refreshAll);
    }

    // Periodic refresh for expiry countdown
    _refreshInterval = setInterval(() => {
      if (document.visibilityState === "visible") {
        refreshAll();
      }
    }, 60000);
  }

  let _refreshInterval = null;

  function destroy() {
    clearInterval(_refreshInterval);
  }

  return { init, destroy, refresh: refreshAll, showCreateOfferModal };
})();

window.MarketP2P = MarketP2P;
