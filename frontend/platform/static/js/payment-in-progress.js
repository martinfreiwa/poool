/**
 * Payment In Progress Page JS
 * Polls deposit status and auto-redirects when confirmed.
 * Shows bank transfer instructions for manual deposits.
 */
(function () {
  "use strict";

  let pollInterval = null;
  let pollCount = 0;
  const MAX_POLLS = 180; // 30 minutes at 10-second intervals

  document.addEventListener("DOMContentLoaded", async () => {
    const params = new URLSearchParams(window.location.search);
    const depositId = params.get("deposit_id") || params.get("id");

    if (depositId) {
      // Direct deposit lookup
      try {
        const resp = await fetch(`/api/deposits/${depositId}/status`);
        if (!resp.ok) throw new Error("Failed to load deposit status");
        const deposit = await resp.json();
        renderDepositStatus(deposit, depositId);
        if (deposit.status === "pending" || deposit.status === "processing") {
          startPolling(depositId);
        }
      } catch (e) {
        showEmpty(
          "Unable to load deposit details.",
          'Please check your <a href="/wallet">wallet</a> for the latest status.'
        );
      }
    } else {
      // No deposit_id — try loading latest order instead
      try {
        const resp = await fetch("/api/orders/latest");
        if (!resp.ok) throw new Error("No recent order found");
        const order = await resp.json();
        renderOrderStatus(order);
      } catch (e) {
        showEmpty(
          "No deposit reference found.",
          'Navigate to your <a href="/wallet">wallet</a> to check your deposits.'
        );
      }
    }
  });

  /**
   * Render the page based on an order (used when no deposit_id is in the URL).
   * This happens after a bank transfer checkout redirects to /payment-in-progress.
   */
  function renderOrderStatus(order) {
    const dynamicContainer = document.getElementById("deposit-status-dynamic");
    const headerContainer = document.getElementById("pip-status-header");
    const actionsContainer = document.getElementById("pip-actions");
    const pollStatusEl = document.getElementById("poll-status");
    if (!dynamicContainer || !headerContainer) return;

    const currency = order.payment_currency || "USD";
    const symbol = currency === "IDR" ? "Rp" : "$";
    const status = order.status || "pending";
    const method = order.payment_method || "bank";
    const isBankTransfer = method === "bank" || method === "manual";

    const statusConfig = {
      pending:            { label: "Payment In Progress", cssClass: "pending" },
      pending_payment:    { label: "Awaiting Payment", cssClass: "pending" },
      processing:         { label: "Processing", cssClass: "processing" },
      completed:          { label: "Completed", cssClass: "paid" },
      paid:               { label: "Confirmed", cssClass: "paid" },
      failed:             { label: "Failed", cssClass: "failed" },
      cancelled:          { label: "Cancelled", cssClass: "failed" },
    };
    const sc = statusConfig[status] || statusConfig.pending;

    // ── Header ──
    let headerHTML = `
      <div class="pip-status-badge pip-status-badge--${sc.cssClass}">
        <span class="pip-status-badge__dot"></span>
        ${esc(sc.label)}
      </div>
    `;

    if (status === "completed" || status === "paid") {
      headerHTML += `
        <h1 class="pip-title">Payment Confirmed!</h1>
        <p class="pip-subtitle">Your order has been successfully processed and credited to your portfolio.</p>
      `;
    } else if (isBankTransfer) {
      headerHTML += `
        <h1 class="pip-title">Your Payment Is Being Processed</h1>
        <p class="pip-subtitle">
          Your bank transfer has been received. The payment will be activated once the funds arrive
          and credited to your account.
        </p>
      `;
    } else {
      headerHTML += `
        <h1 class="pip-title">Payment In Progress</h1>
        <p class="pip-subtitle">Your order is being processed. This page will update automatically.</p>
      `;
    }

    headerContainer.innerHTML = headerHTML;

    // ── Cards ──
    let cardsHTML = '';

    // Card 1: Order Details
    cardsHTML += `
      <div class="pip-card">
        <div class="pip-card__title">
          <span class="pip-card__title-icon">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="2" y="5" width="20" height="14" rx="2"/><line x1="2" y1="10" x2="22" y2="10"/>
            </svg>
          </span>
          Payment Details
        </div>
        <div class="pip-amount-display">
          <div class="pip-amount-display__value">${symbol}${formatCents(order.total_cents)}</div>
          <div class="pip-amount-display__label">${esc(currency)} via ${esc(methodLabel(method))}</div>
        </div>
        <div class="pip-detail-row">
          <span class="pip-detail-row__label">Order Number</span>
          <span class="pip-detail-row__value"><code>${esc(order.order_number || order.id)}</code></span>
        </div>
        <div class="pip-detail-row">
          <span class="pip-detail-row__label">Payment Method</span>
          <span class="pip-detail-row__value">${esc(methodLabel(method))}</span>
        </div>
        <div class="pip-detail-row">
          <span class="pip-detail-row__label">Status</span>
          <span class="pip-detail-row__value">
            <span class="pip-status-badge pip-status-badge--${sc.cssClass}" style="margin:0; font-size:11px; padding:3px 12px;">
              <span class="pip-status-badge__dot" style="width:6px;height:6px;"></span>
              ${esc(sc.label)}
            </span>
          </span>
        </div>
      </div>
    `;

    // Card 2: Order Items
    if (order.items && order.items.length > 0) {
      let itemsHTML = order.items.map(item => `
        <div class="pip-detail-row">
          <span class="pip-detail-row__label">${esc(item.asset_title || 'Asset')}</span>
          <span class="pip-detail-row__value">
            ${item.tokens_quantity} shares × ${symbol}${formatCents(item.token_price_cents)}
            = <strong>${symbol}${formatCents(item.total_cents || (item.tokens_quantity * item.token_price_cents))}</strong>
          </span>
        </div>
      `).join('');

      cardsHTML += `
        <div class="pip-card">
          <div class="pip-card__title">
            <span class="pip-card__title-icon">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M6 2L3 6v14a2 2 0 002 2h14a2 2 0 002-2V6l-3-4z"/><line x1="3" y1="6" x2="21" y2="6"/><path d="M16 10a4 4 0 01-8 0"/>
              </svg>
            </span>
            Order Items
          </div>
          ${itemsHTML}
        </div>
      `;
    }

    // Card 3: Progress Steps (for pending status)
    if (status !== "completed" && status !== "paid" && status !== "failed" && status !== "cancelled") {
      cardsHTML += `
        <div class="pip-card pip-progress-card">
          <div class="pip-card__title">
            <span class="pip-card__title-icon">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
              </svg>
            </span>
            Payment Progress
          </div>
          <div class="pip-progress-steps">
            <div class="pip-progress-step pip-progress-step--done">
              <div class="pip-progress-step__icon">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6L9 17l-5-5"/></svg>
              </div>
              <div class="pip-progress-step__content">
                <p class="pip-progress-step__title">Order Placed</p>
                <p class="pip-progress-step__desc">Your order has been successfully created</p>
              </div>
            </div>
            <div class="pip-progress-step pip-progress-step--active">
              <div class="pip-progress-step__icon">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/></svg>
              </div>
              <div class="pip-progress-step__content">
                <p class="pip-progress-step__title">Awaiting Payment</p>
                <p class="pip-progress-step__desc">${isBankTransfer ? 'Bank transfer expected (1-3 business days)' : 'Processing your payment'}</p>
              </div>
            </div>
            <div class="pip-progress-step pip-progress-step--upcoming">
              <div class="pip-progress-step__icon">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/></svg>
              </div>
              <div class="pip-progress-step__content">
                <p class="pip-progress-step__title">Payment Activated</p>
                <p class="pip-progress-step__desc">Will be credited to your portfolio upon receipt of payment</p>
              </div>
            </div>
          </div>
        </div>
      `;
    }

    // Card 4: Success (if completed)
    if (status === "completed" || status === "paid") {
      cardsHTML += `
        <div class="pip-card pip-success-card">
          <div class="pip-success-icon">
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#027A48" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M20 6L9 17l-5-5"/>
            </svg>
          </div>
          <p class="pip-success-title">Payment Confirmed!</p>
          <p class="pip-success-desc">Your investment has been added to your portfolio.</p>
        </div>
      `;
    }

    // Card 5: Support Info
    cardsHTML += `
      <div class="pip-support-card">
        <div class="pip-support-card__icon">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 16.92v3a2 2 0 01-2.18 2 19.79 19.79 0 01-8.63-3.07 19.5 19.5 0 01-6-6 19.79 19.79 0 01-3.07-8.67A2 2 0 014.11 2h3a2 2 0 012 1.72 12.84 12.84 0 00.7 2.81 2 2 0 01-.45 2.11L8.09 9.91a16 16 0 006 6l1.27-1.27a2 2 0 012.11-.45 12.84 12.84 0 002.81.7A2 2 0 0122 16.92z"/>
          </svg>
        </div>
        <div class="pip-support-card__content">
          <p class="pip-support-card__title">Questions or Problems?</p>
          <p class="pip-support-card__desc">
            Our support team is happy to help. Give us a call or send us a message via the <a href="/support">Support Portal</a>.
          </p>
        </div>
      </div>
    `;

    dynamicContainer.innerHTML = cardsHTML;

    // ── Actions ──
    if (actionsContainer) {
      actionsContainer.innerHTML = `
        <a href="/marketplace" class="pip-btn pip-btn--primary">
          <span class="button-text">Continue Shopping</span>
          <svg class="btn-arrow" width="20" height="20" viewBox="0 0 20 20" fill="none"><path d="M4.16667 10H15.8333M15.8333 10L10 4.16667M15.8333 10L10 15.8333" stroke="#62F7A4" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </a>
        <a href="/portfolio" class="pip-btn pip-btn--secondary">
          View Portfolio
        </a>
      `;
      actionsContainer.style.display = 'flex';
    }

    if (pollStatusEl) pollStatusEl.textContent = "";
  }

  /**
   * Render the page based on a deposit record (when deposit_id is in the URL).
   */
  function renderDepositStatus(deposit, depositId) {
    const dynamicContainer = document.getElementById("deposit-status-dynamic");
    const headerContainer = document.getElementById("pip-status-header");
    const actionsContainer = document.getElementById("pip-actions");
    const pollStatusEl = document.getElementById("poll-status");
    if (!dynamicContainer || !headerContainer) return;

    const currency = deposit.currency || "USD";
    const symbol = currency === "IDR" ? "Rp" : "$";
    const status = deposit.status || "pending";
    const provider = deposit.provider || "bank";
    const isBankTransfer = provider === "manual" || provider === "bank";

    const statusConfig = {
      pending:     { label: "Payment In Progress", cssClass: "pending" },
      processing:  { label: "Processing", cssClass: "processing" },
      paid:        { label: "Confirmed", cssClass: "paid" },
      failed:      { label: "Failed", cssClass: "failed" },
      expired:     { label: "Expired", cssClass: "failed" },
      cancelled:   { label: "Cancelled", cssClass: "failed" },
    };
    const sc = statusConfig[status] || statusConfig.pending;

    const expectedTime =
      provider === "stripe"
        ? "Usually instant"
        : provider === "xendit"
          ? "1 business day (IDR)"
          : "1-3 business days (bank wire)";

    // ── Header ──
    let headerHTML = `
      <div class="pip-status-badge pip-status-badge--${sc.cssClass}">
        <span class="pip-status-badge__dot"></span>
        ${esc(sc.label)}
      </div>
    `;

    if (status === "pending" || status === "processing") {
      if (isBankTransfer) {
        headerHTML += `
          <h1 class="pip-title">Your Payment Is Being Processed</h1>
          <p class="pip-subtitle">
            Your bank transfer has been received. The payment will be activated once the funds arrive
            and credited to your account.
          </p>
        `;
      } else {
        headerHTML += `
          <h1 class="pip-title">Payment In Progress</h1>
          <p class="pip-subtitle">Your deposit is being processed. This page will update automatically.</p>
        `;
      }
    } else if (status === "paid") {
      headerHTML += `
        <h1 class="pip-title">Payment Confirmed!</h1>
        <p class="pip-subtitle">Your deposit has been successfully processed and credited to your wallet.</p>
      `;
    } else {
      headerHTML += `
        <h1 class="pip-title">Payment ${esc(sc.label)}</h1>
        <p class="pip-subtitle">Please try again or contact our support team.</p>
      `;
    }

    headerContainer.innerHTML = headerHTML;

    // ── Cards ──
    let cardsHTML = '';

    // Payment Details card
    cardsHTML += `
      <div class="pip-card">
        <div class="pip-card__title">
          <span class="pip-card__title-icon">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="2" y="5" width="20" height="14" rx="2"/><line x1="2" y1="10" x2="22" y2="10"/>
            </svg>
          </span>
          Payment Details
        </div>
        <div class="pip-amount-display">
          <div class="pip-amount-display__value">${symbol}${formatCents(deposit.amount_cents)}</div>
          <div class="pip-amount-display__label">${esc(currency)} Deposit via ${esc(providerLabel(provider))}</div>
        </div>
        <div class="pip-detail-row">
          <span class="pip-detail-row__label">Reference Number</span>
          <span class="pip-detail-row__value"><code>${esc(deposit.provider_reference || depositId)}</code></span>
        </div>
        <div class="pip-detail-row">
          <span class="pip-detail-row__label">Expected Duration</span>
          <span class="pip-detail-row__value">${expectedTime}</span>
        </div>
        <div class="pip-detail-row">
          <span class="pip-detail-row__label">Status</span>
          <span class="pip-detail-row__value">
            <span class="pip-status-badge pip-status-badge--${sc.cssClass}" style="margin:0; font-size:11px; padding:3px 12px;">
              <span class="pip-status-badge__dot" style="width:6px;height:6px;"></span>
              ${esc(sc.label)}
            </span>
          </span>
        </div>
      </div>
    `;

    // Bank Transfer Details (if pending bank)
    if (isBankTransfer && (status === "pending" || status === "processing")) {
      const bankDetails = currency === "IDR"
        ? [
            { label: "Bank", value: "BCA (Bank Central Asia)" },
            { label: "Account Name", value: "PT POOOL Indonesia" },
            { label: "Account Number", value: "0987654321" },
          ]
        : [
            { label: "Bank", value: "Chase Bank" },
            { label: "Account Name", value: "POOOL Inc." },
            { label: "Account Number", value: "123456789" },
            { label: "Routing", value: "987654321" },
            { label: "SWIFT", value: "CHASUS33" },
          ];

      let bankRowsHTML = bankDetails.map(d => `
        <div class="pip-detail-row">
          <span class="pip-detail-row__label">${esc(d.label)}</span>
          <span class="pip-detail-row__value">${esc(d.value)}</span>
        </div>
      `).join('');

      cardsHTML += `
        <div class="pip-card pip-bank-card">
          <div class="pip-card__title">
            <span class="pip-card__title-icon">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M3 21h18M3 10h18M5 6l7-3 7 3M4 10v11M20 10v11M8 14v3M12 14v3M16 14v3"/>
              </svg>
            </span>
            Bank Transfer Details
          </div>
          ${bankRowsHTML}
          <div class="pip-bank-ref-row">
            <span class="pip-bank-ref-row__label">Reference</span>
            <span class="pip-bank-ref-row__value">
              ${esc(deposit.provider_reference || depositId)}
              <button class="pip-bank-ref-row__copy" onclick="copyRef(this, '${esc(deposit.provider_reference || depositId)}')" title="Copy">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/>
                </svg>
              </button>
            </span>
          </div>
        </div>
        <div class="pip-bank-warning">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#B54708" stroke-width="2" stroke-linecap="round">
            <path d="M12 9v4M12 17h.01"/><circle cx="12" cy="12" r="10"/>
          </svg>
          <span>⚠️ You MUST include the reference number in your transfer, or it may not be matched.</span>
        </div>
      `;
    }

    // Progress Steps (for pending/processing)
    if (status === "pending" || status === "processing") {
      cardsHTML += `
        <div class="pip-card pip-progress-card">
          <div class="pip-card__title">
            <span class="pip-card__title-icon">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
              </svg>
            </span>
            Payment Progress
          </div>
          <div class="pip-progress-steps">
            <div class="pip-progress-step pip-progress-step--done">
              <div class="pip-progress-step__icon">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6L9 17l-5-5"/></svg>
              </div>
              <div class="pip-progress-step__content">
                <p class="pip-progress-step__title">Order Placed</p>
                <p class="pip-progress-step__desc">Your order has been successfully created</p>
              </div>
            </div>
            <div class="pip-progress-step pip-progress-step--active">
              <div class="pip-progress-step__icon">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/></svg>
              </div>
              <div class="pip-progress-step__content">
                <p class="pip-progress-step__title">Awaiting Payment</p>
                <p class="pip-progress-step__desc">${isBankTransfer ? 'Bank transfer expected (1-3 business days)' : expectedTime}</p>
              </div>
            </div>
            <div class="pip-progress-step pip-progress-step--upcoming">
              <div class="pip-progress-step__icon">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/></svg>
              </div>
              <div class="pip-progress-step__content">
                <p class="pip-progress-step__title">Payment Activated</p>
                <p class="pip-progress-step__desc">Will be credited to your portfolio upon receipt of payment</p>
              </div>
            </div>
          </div>
        </div>
      `;
    }

    // Success card (if paid)
    if (status === "paid") {
      cardsHTML += `
        <div class="pip-card pip-success-card">
          <div class="pip-success-icon">
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#027A48" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M20 6L9 17l-5-5"/>
            </svg>
          </div>
          <p class="pip-success-title">Deposit Confirmed!</p>
          <p class="pip-success-desc">Your funds have been credited to your wallet.</p>
        </div>
      `;
    }

    // Failed/expired/cancelled
    if (status === "failed" || status === "expired" || status === "cancelled") {
      cardsHTML += `
        <div class="pip-card pip-failed-card">
          <p class="pip-failed-title">Deposit ${esc(sc.label)}</p>
          <p class="pip-failed-desc">Please try again or contact support if you've already transferred funds.</p>
        </div>
      `;
    }

    // Support Info
    cardsHTML += `
      <div class="pip-support-card">
        <div class="pip-support-card__icon">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 16.92v3a2 2 0 01-2.18 2 19.79 19.79 0 01-8.63-3.07 19.5 19.5 0 01-6-6 19.79 19.79 0 01-3.07-8.67A2 2 0 014.11 2h3a2 2 0 012 1.72 12.84 12.84 0 00.7 2.81 2 2 0 01-.45 2.11L8.09 9.91a16 16 0 006 6l1.27-1.27a2 2 0 012.11-.45 12.84 12.84 0 002.81.7A2 2 0 0122 16.92z"/>
          </svg>
        </div>
        <div class="pip-support-card__content">
          <p class="pip-support-card__title">Questions or Problems?</p>
          <p class="pip-support-card__desc">
            Our support team is happy to help. Give us a call or send us a message via the <a href="/support">Support Portal</a>.
          </p>
        </div>
      </div>
    `;

    dynamicContainer.innerHTML = cardsHTML;

    // Actions
    if (actionsContainer) {
      actionsContainer.innerHTML = `
        <a href="/marketplace" class="pip-btn pip-btn--primary">
          <span class="button-text">Continue Shopping</span>
          <svg class="btn-arrow" width="20" height="20" viewBox="0 0 20 20" fill="none"><path d="M4.16667 10H15.8333M15.8333 10L10 4.16667M15.8333 10L10 15.8333" stroke="#62F7A4" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </a>
        <a href="/wallet" class="pip-btn pip-btn--secondary">
          Go to Wallet
        </a>
      `;
      actionsContainer.style.display = 'flex';
    }

    // Poll status
    if (pollStatusEl) {
      if (status === "pending" || status === "processing") {
        pollStatusEl.textContent = "Checking for confirmation every 10 seconds…";
      } else {
        pollStatusEl.textContent = "";
      }
    }
  }

  function startPolling(depositId) {
    pollInterval = setInterval(async () => {
      pollCount++;
      if (pollCount >= MAX_POLLS) {
        clearInterval(pollInterval);
        const pollEl = document.getElementById("poll-status");
        if (pollEl) pollEl.textContent = "Still processing — please check your wallet later or contact support.";
        return;
      }

      try {
        const resp = await fetch(`/api/deposits/${depositId}/status`);
        if (!resp.ok) return;
        const deposit = await resp.json();
        if (deposit.status === "paid") {
          clearInterval(pollInterval);
          if (deposit.order_id) {
            window.location.href = `/payment-success?order_id=${deposit.order_id}`;
          } else {
            renderDepositStatus(deposit, depositId);
          }
        } else if (deposit.status === "failed" || deposit.status === "expired" || deposit.status === "cancelled") {
          clearInterval(pollInterval);
          renderDepositStatus(deposit, depositId);
        }
      } catch (e) { /* ignore */ }
    }, 10000);
  }

  function showEmpty(title, body) {
    const dynamicContainer = document.getElementById("deposit-status-dynamic");
    const headerContainer = document.getElementById("pip-status-header");
    if (!dynamicContainer) return;

    if (headerContainer) {
      headerContainer.innerHTML = `<h1 class="pip-title">Payment Not Found</h1>`;
    }

    dynamicContainer.innerHTML = `
      <div class="pip-empty">
        <div class="pip-empty__icon">🔍</div>
        <p class="pip-empty__title">${title}</p>
        <p class="pip-empty__desc">${body}</p>
      </div>
    `;

    const actionsContainer = document.getElementById("pip-actions");
    if (actionsContainer) {
      actionsContainer.innerHTML = `
        <a href="/marketplace" class="pip-btn pip-btn--primary">
          <span class="button-text">Continue Shopping</span>
          <svg class="btn-arrow" width="20" height="20" viewBox="0 0 20 20" fill="none"><path d="M4.16667 10H15.8333M15.8333 10L10 4.16667M15.8333 10L10 15.8333" stroke="#62F7A4" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </a>
        <a href="/wallet" class="pip-btn pip-btn--secondary">Go to Wallet</a>
      `;
      actionsContainer.style.display = 'flex';
    }
  }

  // Copy reference to clipboard
  window.copyRef = function(btn, text) {
    navigator.clipboard.writeText(text).then(() => {
      btn.classList.add('copied');
      setTimeout(() => btn.classList.remove('copied'), 2000);
    });
  };

  function providerLabel(provider) {
    return { bank: "Bank Transfer", manual: "Bank Transfer", stripe: "Stripe", xendit: "Xendit" }[provider] || provider;
  }

  function methodLabel(method) {
    return { bank: "Bank Transfer", manual: "Bank Transfer", wallet: "Wallet", stripe: "Stripe", xendit: "Xendit" }[method] || method;
  }

  function formatCents(cents) {
    if (!cents && cents !== 0) return "0.00";
    return (cents / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  }

  function esc(s) {
    if (typeof s !== "string") return s || "";
    const d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }
})();
