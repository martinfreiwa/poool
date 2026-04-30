document.addEventListener("DOMContentLoaded", () => {
  initPayoutActions();
  initPayoutModalActions();
  loadPendingPayouts();
});

let currentPayoutAffiliate = null;
let currentPayoutAmount = 0;
let currentPayoutCount = 0;
let previousFocus = null;
const MODAL_FOCUS_SELECTOR = "button, [href], input, select, textarea, [tabindex]:not([tabindex='-1'])";

async function loadPendingPayouts() {
  const tbody = document.getElementById("payouts-body");
  const countEl = document.getElementById("pending-payouts-count");

  try {
    const res = await fetch("/api/admin/rewards/affiliates/payouts/pending");
    if (!res.ok) throw new Error("Failed to load payouts");
    
    const payouts = await res.json();
    countEl.textContent = `${payouts.length} Batches Ready`;

    if (payouts.length === 0) {
      tbody.innerHTML = `
        <tr>
          <td colspan="6" style="text-align: center; padding: 40px; color: var(--admin-text-muted);">
            No mature commissions ready for payout. 
            <br/><span style="font-size: 13px">All commissions are under 30-day holdback or already paid. Payout batches require a $50.00 minimum threshold.</span>
          </td>
        </tr>
      `;
      return;
    }

    tbody.innerHTML = payouts.map(p => {
      const underMinimum = Number(p.total_payable_cents) < 5000;
      const taxBlocked = p.tax_document_uploaded !== true;
      const disabled = underMinimum || taxBlocked;
      const blockedReason = taxBlocked ? (p.payout_blocked_reason || "Tax document required before release") : "";
      const actionTitle = underMinimum ? "Minimum payout is $50.00" : blockedReason;
      const requested = Boolean(p.payout_request_id);
      const requestLabel = requested
        ? `Manual request${p.payout_requested_at ? `: ${new Date(p.payout_requested_at).toLocaleDateString()}` : ""}`
        : "Auto-eligible";
      const requestAmount = p.payout_request_amount_cents
        ? `$${(p.payout_request_amount_cents / 100).toFixed(2)} requested`
        : "";
      return `
      <tr>
        <td>
          <div style="font-weight: 500">${escapeHtml(p.name)}</div>
          <div style="font-size: 12px; color: var(--admin-text-muted)">${escapeHtml(p.email)}</div>
          <div style="font-size: 11px; color: ${requested ? 'var(--admin-accent)' : 'var(--admin-text-muted)'}; margin-top: 4px;">${escapeHtml(requestLabel)}</div>
        </td>
        <td><span class="admin-badge" style="background: rgba(0, 113, 227, 0.1); color: var(--admin-accent); text-transform: uppercase">${escapeHtml(p.referral_code)}</span></td>
        <td><strong>${p.commission_count}</strong><br/><span style="font-size: 11px; color: var(--admin-text-muted)">Mature Commissions</span></td>
        <td style="color: var(--admin-success); font-weight: 600">$${(p.total_payable_cents / 100).toFixed(2)}${requestAmount ? `<br/><span style="font-size: 11px; color: var(--admin-text-muted); font-weight: 400;">${escapeHtml(requestAmount)}</span>` : ""}</td>
        <td>
          <span class="admin-badge" style="${p.tax_document_uploaded ? 'background: rgba(0, 200, 83, 0.1); color: var(--admin-success);' : 'background: rgba(240, 68, 56, 0.1); color: var(--admin-danger);'}">
            ${p.tax_document_uploaded ? 'Tax ready' : 'Tax blocked'}
          </span>
          ${blockedReason ? `<div style="font-size: 11px; color: var(--admin-danger); margin-top: 4px;">${escapeHtml(blockedReason)}</div>` : ''}
        </td>
        <td>
          <button class="admin-btn admin-btn--sm admin-btn--primary release-payout-btn"
             ${disabled ? `disabled title="${escapeHtml(actionTitle)}"` : ''}
             data-affiliate-id="${escapeHtml(p.affiliate_id)}"
             data-affiliate-name="${escapeHtml(p.name)}"
             data-amount-cents="${Number(p.total_payable_cents)}"
             data-commission-count="${Number(p.commission_count)}">
             <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" style="margin-right:4px;"><path d="M5 13l4 4L19 7" /></svg>
             ${taxBlocked ? 'Tax Required' : (underMinimum ? 'Under $50 Min' : 'Release Payout')}
          </button>
        </td>
      </tr>
    `;
    }).join('');

  } catch (err) {
    console.error(err);
    tbody.innerHTML = `
      <tr>
        <td colspan="6" style="text-align: center; padding: 40px; color: var(--admin-danger);">
          Error loading payout data. Check console.
        </td>
      </tr>
    `;
    countEl.textContent = "Error";
  }
}

function initPayoutActions() {
  const tbody = document.getElementById("payouts-body");
  if (!tbody) return;

  tbody.addEventListener("click", (event) => {
    const button = event.target.closest(".release-payout-btn");
    if (!button || button.disabled) return;

    openPayoutModal(
      button.dataset.affiliateId,
      button.dataset.affiliateName || "Affiliate",
      Number(button.dataset.amountCents),
      Number(button.dataset.commissionCount)
    );
  });
}

function initPayoutModalActions() {
  document.getElementById("payout-cancel-btn")?.addEventListener("click", closePayoutModal);
  document.getElementById("payout-confirm-btn")?.addEventListener("click", confirmPayout);
  document.getElementById("payout-modal")?.addEventListener("click", (event) => {
    if (event.target.id === "payout-modal") {
      closePayoutModal();
    }
  });
  document.getElementById("payout-modal")?.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      closePayoutModal();
      return;
    }

    if (event.key !== "Tab") {
      return;
    }

    const focusable = Array.from(event.currentTarget.querySelectorAll(MODAL_FOCUS_SELECTOR))
      .filter((element) => !element.disabled && element.offsetParent !== null);
    if (!focusable.length) {
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  });
}

function openPayoutModal(id, name, amount_cents, count) {
  currentPayoutAffiliate = id;
  currentPayoutAmount = amount_cents;
  currentPayoutCount = count;
  previousFocus = document.activeElement;

  document.getElementById("payout-modal-details").textContent = `Affiliate: ${name}`;
  document.getElementById("payout-modal-amount").textContent = `$${(amount_cents / 100).toFixed(2)}`;
  document.getElementById("payout-modal-count").textContent = `${count} trades`;
  
  const modal = document.getElementById("payout-modal");
  modal.style.display = "flex";
  document.getElementById("payout-confirm-btn")?.focus();
}

function closePayoutModal() {
  document.getElementById("payout-modal").style.display = "none";
  currentPayoutAffiliate = null;
  if (previousFocus && typeof previousFocus.focus === "function") {
    previousFocus.focus();
  }
  previousFocus = null;
}

async function confirmPayout() {
  if (!currentPayoutAffiliate) return;

  const btn = document.getElementById("payout-confirm-btn");
  btn.disabled = true;
  btn.innerHTML = `<div style="width: 14px; height: 14px; border: 2px solid white; border-top-color: transparent; border-radius: 50%; animation: spin 0.8s linear infinite; margin-right: 6px"></div> Executing…`;

  try {
    const res = await fetch(`/api/admin/rewards/affiliates/${currentPayoutAffiliate}/payout`, {
      method: "POST",
      headers: { "Content-Type": "application/json" }
    });

    const data = await res.json();
    if (!res.ok) throw new Error(data.message || data.error || "Failed to execute payout");

    closePayoutModal();
    
    // Create temporary success toast
    const toast = document.createElement("div");
    toast.style.cssText = `
      position: fixed; bottom: 20px; right: 20px; z-index: 9999;
      background: var(--admin-success); color: white; padding: 12px 20px;
      border-radius: var(--admin-radius-md); font-size: 14px; font-weight: 500;
      box-shadow: 0 4px 12px rgba(0,0,0,0.15); animation: admin-fadeIn 0.3s;
    `;
    toast.textContent = `Success: Executed batch ${data.batch_id} for $${(data.amount_cents / 100).toFixed(2)}`;
    document.body.appendChild(toast);
    setTimeout(() => toast.remove(), 4000);

    // Refresh UI
    loadPendingPayouts();

  } catch (err) {
    console.error(err);
    alert(err.message);
  } finally {
    btn.disabled = false;
    btn.innerHTML = `<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M5 13l4 4L19 7" /></svg> Execute Transfer`;
  }
}

document.querySelector(".admin-notification-btn")?.addEventListener("click", () => {
  window.location.href = "/admin/notifications";
});

function escapeHtml(unsafe) {
    return String(unsafe)
         .replace(/&/g, "&amp;")
         .replace(/</g, "&lt;")
         .replace(/>/g, "&gt;")
         .replace(/"/g, "&quot;")
         .replace(/'/g, "&#039;");
}
