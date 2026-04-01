document.addEventListener("DOMContentLoaded", () => {
  loadPendingPayouts();
});

let currentPayoutAffiliate = null;
let currentPayoutAmount = 0;
let currentPayoutCount = 0;

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
          <td colspan="5" style="text-align: center; padding: 40px; color: var(--admin-text-muted);">
            No mature commissions ready for payout. 
            <br/><span style="font-size: 13px">All commissions are under 30-day holdback or already paid. Payout batches require a $50.00 minimum threshold.</span>
          </td>
        </tr>
      `;
      return;
    }

    tbody.innerHTML = payouts.map(p => `
      <tr>
        <td>
          <div style="font-weight: 500">${escapeHtml(p.name)}</div>
          <div style="font-size: 12px; color: var(--admin-text-muted)">${escapeHtml(p.email)}</div>
        </td>
        <td><span class="admin-badge" style="background: rgba(0, 113, 227, 0.1); color: var(--admin-accent); text-transform: uppercase">${escapeHtml(p.referral_code)}</span></td>
        <td><strong>${p.commission_count}</strong><br/><span style="font-size: 11px; color: var(--admin-text-muted)">Mature Commissions</span></td>
        <td style="color: var(--admin-success); font-weight: 600">$${(p.total_payable_cents / 100).toFixed(2)}</td>
        <td>
          <button class="admin-btn admin-btn--sm admin-btn--primary" 
             ${p.total_payable_cents < 5000 ? 'disabled title="Minimum payout is $50.00"' : ''}
             onclick="openPayoutModal('${p.affiliate_id}', '${escapeHtml(p.name)}', ${p.total_payable_cents}, ${p.commission_count})">
             <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" style="margin-right:4px;"><path d="M5 13l4 4L19 7" /></svg>
             ${p.total_payable_cents < 5000 ? 'Under $50 Min' : 'Release Payout'}
          </button>
        </td>
      </tr>
    `).join('');

  } catch (err) {
    console.error(err);
    tbody.innerHTML = `
      <tr>
        <td colspan="5" style="text-align: center; padding: 40px; color: var(--admin-danger);">
          Error loading payout data. Check console.
        </td>
      </tr>
    `;
    countEl.textContent = "Error";
  }
}

function openPayoutModal(id, name, amount_cents, count) {
  currentPayoutAffiliate = id;
  currentPayoutAmount = amount_cents;
  currentPayoutCount = count;

  document.getElementById("payout-modal-details").textContent = `Affiliate: ${name}`;
  document.getElementById("payout-modal-amount").textContent = `$${(amount_cents / 100).toFixed(2)}`;
  document.getElementById("payout-modal-count").textContent = `${count} trades`;
  
  document.getElementById("payout-modal").style.display = "flex";
}

function closePayoutModal() {
  document.getElementById("payout-modal").style.display = "none";
  currentPayoutAffiliate = null;
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

function escapeHtml(unsafe) {
    return String(unsafe)
         .replace(/&/g, "&amp;")
         .replace(/</g, "&lt;")
         .replace(/>/g, "&gt;")
         .replace(/"/g, "&quot;")
         .replace(/'/g, "&#039;");
}
