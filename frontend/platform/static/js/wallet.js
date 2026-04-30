/**
 * wallet.js  –  Phase 3 & 4: Frontend UI, State Binding & QA
 *
 * Consumes WalletDataService and manages four UI state layers:
 *   1. loading  – skeleton shimmer shown immediately
 *   2. error    – shown if the API call fails
 *   3. empty    – shown when user has zero balance & no transactions
 *   4. content  – the full wallet UI (already SSR-rendered by Rust)
 *
 * The Rust backend already server-side-renders the full wallet page with
 * real data, so this file's primary job is:
 *   - Live-refresh balance cards without a full page reload (nice-to-have)
 *   - Manage the loading skeleton during that refresh
 *   - Handle error conditions gracefully
 *   - Animate balance changes
 */
(function () {
    "use strict";

    // ─── XSS-safe HTML escaper ───────────────────────────────────
    function escHtml(str) {
        if (typeof str !== "string") return String(str);
        var d = document.createElement("div");
        d.appendChild(document.createTextNode(str));
        return d.innerHTML;
    }

    // ─── State Layer IDs ─────────────────────────────────────────
    const LAYERS = {
        loading: "wallet-loading-layer",
        error: "wallet-error-layer",
        empty: "wallet-empty-layer",
        content: "wallet-content-layer",
        pagination: "wallet-pagination-container",
    };

    let currentPage = 1;
    const pageSize = 10;

    // ─── Helpers ─────────────────────────────────────────────────

    /**
     * Show only the specified layer; hide the others.
     * @param {'loading'|'error'|'empty'|'content'} name
     */
    function switchState(name) {
        Object.entries(LAYERS).forEach(([key, id]) => {
            const el = document.getElementById(id);
            if (!el) return;
            if (key === name) {
                el.classList.remove("hidden");
            } else {
                el.classList.add("hidden");
            }
        });
        if (name === "empty") markEmptyStateSteps();
    }

    /** Mark onboarding steps complete based on user profile + payment methods. */
    function markEmptyStateSteps() {
        const user = window.__POOOL_USER;
        if (!user) return;
        const kycDone = ["verified", "approved", "completed"].includes(
            String(user.kyc_status || user.kycStatus || "").toLowerCase()
        );
        const hasMethod =
            (Array.isArray(user.payment_methods) && user.payment_methods.length > 0) ||
            (Array.isArray(user.paymentMethods) && user.paymentMethods.length > 0);
        const funded = Number(user.wallet_balance ?? user.walletBalance ?? 0) > 0;
        const completed = { verify: kycDone, method: hasMethod, fund: funded };
        document.querySelectorAll(".wallet-empty__step").forEach((el) => {
            const id = el.getAttribute("data-step-id");
            if (completed[id]) el.setAttribute("data-complete", "true");
            else el.removeAttribute("data-complete");
        });
    }

    /**
     * Safely set the textContent of an element by ID.
     * @param {string} id
     * @param {string} text
     */
    function setText(id, text) {
        const el = document.getElementById(id);
        if (el) el.textContent = text;
    }

    /**
     * Animate a balance value change with a brief pulse effect.
     * @param {HTMLElement} el
     */
    function pulseElement(el) {
        if (!el) return;
        el.style.transition = "opacity 0.18s ease";
        el.style.opacity = "0.4";
        requestAnimationFrame(() => {
            setTimeout(() => {
                el.style.opacity = "1";
            }, 180);
        });
    }

    // ─── DOM Updaters ────────────────────────────────────────────

    /**
     * Update the three balance cards with live API data.
     * The SSR values are already correct on first paint; this refreshes
     * them after the client-side fetch resolves.
     * @param {object} balances – result from WalletDataService.getBalances()
     */
    function updateBalanceCards(balances) {
        const cashEl = document.getElementById("wallet-balance-card-cash-amount");
        const rewardsEl = document.getElementById("wallet-balance-card-rewards-amount");
        const assetsEl = document.getElementById("wallet-balance-card-assets-amount");

        if (cashEl) {
            pulseElement(cashEl);
            cashEl.textContent = balances.cashDisplay;
        }
        if (rewardsEl) {
            pulseElement(rewardsEl);
            rewardsEl.textContent = balances.rewardsDisplay;
        }
        if (assetsEl) {
            pulseElement(assetsEl);
            assetsEl.textContent = balances.assetDisplay;
        }

        // Also update mobile balance cards
        const mobileCash = document.querySelector(".mobile-cash-balance-card__amount");
        if (mobileCash) mobileCash.textContent = balances.cashDisplay;

        const mobileRewards = document.querySelector(".mobile-rewards-balance-card__amount");
        if (mobileRewards) mobileRewards.textContent = balances.rewardsDisplay;
    }

    // ─── Icon SVGs (keyed by iconKey) ────────────────────────────
    const ICON_SVGS = {
        deposit: `<svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path d="M10 15V5M10 5L5 10M10 5L15 10" stroke="#717680" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
        withdrawal: `<svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path d="M10 5V15M10 15L5 10M10 15L15 10" stroke="#717680" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
        dividend: `<svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path d="M2.5 10C2.5 10 5 7.5 10 7.5C15 7.5 17.5 10 17.5 10M2.5 10V15C2.5 16.3807 3.61929 17.5 5 17.5H15C16.3807 17.5 17.5 16.3807 17.5 15V10" stroke="#717680" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
      <circle cx="10" cy="5" r="2.5" stroke="#717680" stroke-width="1.66667"/>
    </svg>`,
        purchase: `<svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path d="M2.5 7.5L10 2.5L17.5 7.5V16.25C17.5 16.5815 17.3683 16.8995 17.1339 17.1339C16.8995 17.3683 16.5815 17.5 16.25 17.5H3.75C3.41848 17.5 3.10054 17.3683 2.86612 17.1339C2.6317 16.8995 2.5 16.5815 2.5 16.25V7.5Z" stroke="#717680" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M7.5 17.5V10H12.5V17.5" stroke="#717680" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
    };

    /**
     * Build a single transaction row HTML string from a tx object.
     * @param {object} tx
     * @returns {string}
     */
    function buildTxRowHtml(tx) {
        const icon = ICON_SVGS[tx.iconKey] || ICON_SVGS.purchase;
        // Escape all API-sourced values to prevent XSS
        const safeStatusCss = escHtml(tx.statusCss);
        const safeAmountCss = escHtml(tx.amountCss);
        return `
      <div class="table__row">
        <div class="table__cell table__cell--type" style="width:182px">
          <div class="wallet-transaction-type-icon">
            <div class="featured-icon">${icon}</div>
          </div>
          <span class="wallet-transaction-type-text">${escHtml(tx.typeLabel)}</span>
        </div>
        <div class="table__cell table__cell--status" style="width:163px">
          <div class="wallet-transaction-status-badge ${safeStatusCss}">
            <div class="wallet-transaction-status-dot"></div>
            <span class="wallet-transaction-status-text">${escHtml(tx.statusLabel)}</span>
          </div>
        </div>
        <div class="table__cell table__cell--date" style="width:200px">
          <span class="table__cell-text-value">${escHtml(tx.dateDisplay)}</span>
        </div>
        <div class="table__cell table__cell--wallet" style="width:180px">
          <span class="table__cell-text-value">${escHtml(tx.walletLabel)}</span>
        </div>
        <div class="table__cell table__cell--amount" style="width:183px">
          <span class="${safeAmountCss}">${escHtml(tx.amountPrefix)} ${escHtml(tx.amountDisplay)}</span>
        </div>
        <div class="table__cell table__cell--actions" style="width:188px">
          <button class="wallet-transaction-action-btn">
            View details
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M6 12L10 8L6 4" stroke="#717680" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
        </div>
      </div>`;
    }

    /**
     * Render pagination controls.
     * @param {object} txSummary
     */
    function renderPagination(txSummary) {
        const container = document.getElementById(LAYERS.pagination);
        if (!container) return;

        if (!txSummary.hasData || txSummary.total <= pageSize) {
            container.innerHTML = "";
            return;
        }

        const totalPages = Math.ceil(txSummary.total / pageSize);
        
        let html = `
            <button class="pagination-btn" id="prev-page" ${currentPage === 1 ? "disabled" : ""}>Previous</button>
            <span class="pagination-info">Page ${currentPage} of ${totalPages}</span>
            <button class="pagination-btn" id="next-page" ${currentPage === totalPages ? "disabled" : ""}>Next</button>
        `;

        container.innerHTML = html;

        // Add listeners
        const prevBtn = document.getElementById("prev-page");
        const nextBtn = document.getElementById("next-page");

        if (prevBtn) {
            prevBtn.addEventListener("click", () => {
                if (currentPage > 1) {
                    currentPage--;
                    loadTransactionsPage();
                }
            });
        }

        if (nextBtn) {
            nextBtn.addEventListener("click", () => {
                if (currentPage < totalPages) {
                    currentPage++;
                    loadTransactionsPage();
                }
            });
        }
    }

    /**
     * Load a specific page of transactions.
     */
    async function loadTransactionsPage() {
        try {
            const txSummary = await WalletDataService.getTransactions(currentPage, pageSize);
            refreshTransactionsTable(txSummary);
        } catch (err) {
            console.error("Failed to load transactions page:", err);
        }
    }

    /**
     * Refresh the transaction table body with the latest API data.
     * @param {object} txSummary – result from WalletDataService.getTransactions()
     */
    function refreshTransactionsTable(txSummary) {
        const body = document.getElementById("wallet-transactions-body");
        if (!body) return;

        if (!txSummary.hasData) {
            return;
        }

        body.innerHTML = txSummary.transactions
            .map(buildTxRowHtml)
            .join("");
        
        renderPagination(txSummary);
    }

    // ─── Main Init ───────────────────────────────────────────────

    /**
     * Page initialisation:
     * 1. Show loading state immediately (skeleton overlays content briefly)
     * 2. Fetch live data from API
     * 3. Update DOM with live values
     * 4. Switch to content or empty state
     */
    async function initWalletPage() {
        // The wallet page uses SSR, so the content layer should be visible
        // as the default state. We only run the state machine when the
        // loading/error/empty layer elements are present in the DOM.
        const hasStateLayers = !!document.getElementById(LAYERS.loading);

        if (!hasStateLayers) {
            // No state layers in DOM → pure SSR mode, nothing to orchestrate
            return;
        }

        switchState("loading");

        try {
            // Check if WalletDataService is available (loaded before this script)
            if (typeof WalletDataService === "undefined") {
                console.warn("WalletDataService not loaded – switching to SSR content");
                switchState("content");
                return;
            }

            const { balances, txSummary } = await WalletDataService.getWalletPageData(currentPage, pageSize);

            // Determine page state
            if (balances.isEmpty && !txSummary.hasData) {
                switchState("empty");
            } else {
                switchState("content");
                updateBalanceCards(balances);
                refreshTransactionsTable(txSummary);
            }
        } catch (err) {
            console.error("Wallet page data fetch failed:", err);
            // Fallback: show SSR content rather than an error (SSR data is already good)
            const contentEl = document.getElementById(LAYERS.content);
            if (contentEl && contentEl.querySelector("#wallet-balance-card-cash-amount")) {
                // SSR content already rendered – show it rather than the error state
                switchState("content");
            } else {
                switchState("error");
            }
        }
    }

    // ─── Boot ────────────────────────────────────────────────────
    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", initWalletPage);
    } else {
        initWalletPage();
    }

    // ─── URL Param Notifications ─────────────────────────────────
    /**
     * Handle post-deposit and post-withdraw URL params.
     * Deposit: show bank transfer instructions modal with reference + IBAN.
     * Withdraw: show a toast saying the request is pending admin approval.
     * Error: show a toast with the error description.
     */
    function handleUrlParams() {
        const params = new URLSearchParams(window.location.search);

        if (params.has("deposit_created")) {
            const rawRef = params.get("ref") || "–";
            // Escape HTML to prevent reflected XSS via the ref parameter
            const ref = rawRef.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#x27;');
            const amountCents = parseInt(params.get("amount") || "0", 10);
            const amountFmt = amountCents > 0
                ? "$" + (amountCents / 100).toFixed(2)
                : "the requested amount";
            showDepositInstructionsModal(ref, amountFmt);
            // Clean up URL without reload
            window.history.replaceState({}, "", window.location.pathname);
        } else if (params.has("withdraw_requested")) {
            showToast(
                "Withdrawal Request Submitted",
                "Your withdrawal request is pending admin review. You'll be notified once it's processed.",
                "info"
            );
            window.history.replaceState({}, "", window.location.pathname);
        } else if (params.has("error")) {
            const errMap = {
                insufficient_funds: "Insufficient funds in your wallet.",
                deposit_failed: "We couldn't create the deposit request. Please try again.",
                withdraw_failed: "We couldn't process your withdrawal. Please try again.",
                "2fa_required": "Two-factor authentication required for withdrawals of $100 or more. Please enable 2FA in Settings first.",
                withdrawal_cooldown: "You have reached the hourly withdrawal limit (3 requests). Please try again later.",
                daily_limit_exceeded: "This withdrawal would exceed your daily limit of $250,000.",
                no_payment_method: "No payment method on file. Add a bank account or card first.",
            };
            const msg = errMap[params.get("error")] || "An error occurred. Please try again.";
            showToast("Error", msg, "error");
            window.history.replaceState({}, "", window.location.pathname);
        }
    }

    function showDepositInstructionsModal(ref, amountFmt) {
        // Build modal using DOM construction to prevent XSS via ref/amount params
        const overlay = document.createElement("div");
        overlay.id = "deposit-instructions-modal";
        overlay.style.cssText = "position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(0,0,0,0.5);z-index:99999;display:flex;align-items:center;justify-content:center;";

        // Use static HTML for the structural shell only (no user data)
        overlay.innerHTML = '<div style="background:#fff;border-radius:16px;padding:32px;max-width:520px;width:90%;box-shadow:0 24px 48px rgba(0,0,0,0.18);font-family:inherit;">' +
            '<div style="display:flex;align-items:center;gap:12px;margin-bottom:20px;">' +
            '<div style="width:40px;height:40px;background:#F0FDF4;border-radius:50%;display:flex;align-items:center;justify-content:center;">' +
            '<svg width="20" height="20" viewBox="0 0 20 20" fill="none"><path d="M16.667 5L7.5 14.167 3.333 10" stroke="#16A34A" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg></div>' +
            '<h2 style="margin:0;font-size:18px;font-weight:600;color:#101828;">Deposit Request Created</h2></div>' +
            '<p style="color:#475467;font-size:14px;margin:0 0 20px;">Please wire <strong id="dim-amount"></strong> to the following account. Use the reference number below so we can match your transfer.</p>' +
            '<div style="background:#F9FAFB;border:1px solid #EAECF0;border-radius:12px;padding:16px 20px;margin-bottom:20px;">' +
            '<div style="display:Grid;gap:10px;">' +
            row("Bank", "Deutsche Bank AG") +
            row("Account Name", "POOOL GmbH") +
            row("IBAN", "DE89 3704 0044 0532 0130 00") +
            row("BIC / SWIFT", "DEUTDEDB") +
            '<div style="display:flex;justify-content:space-between;padding:6px 0;border-bottom:1px solid #EAECF0;font-size:13px;">' +
            '<span style="color:#667085;">Reference</span>' +
            '<strong id="dim-ref" style="color:#1570EF;font-family:monospace"></strong></div>' +
            '</div></div>' +
            '<div style="background:#FFFAEB;border:1px solid #FEF0C7;border-radius:8px;padding:12px 16px;margin-bottom:24px;font-size:13px;color:#B45309;">' +
            '⚠️ Include the reference number in your transfer, otherwise we cannot match your deposit.' +
            '</div>' +
            '<button onclick="document.getElementById(\'deposit-instructions-modal\').remove()" ' +
            'style="width:100%;padding:12px;background:#1570EF;color:#fff;border:none;border-radius:8px;font-size:15px;font-weight:600;cursor:pointer;">Got it – I\'ll wire the funds</button></div>';

        document.body.appendChild(overlay);

        // Safely inject user-controlled values via textContent (prevents XSS)
        var amountEl = document.getElementById("dim-amount");
        if (amountEl) amountEl.textContent = amountFmt;
        var refEl = document.getElementById("dim-ref");
        if (refEl) refEl.textContent = ref;

        function row(label, value) {
            return '<div style="display:flex;justify-content:space-between;padding:6px 0;border-bottom:1px solid #EAECF0;font-size:13px;">' +
                '<span style="color:#667085;">' + label + '</span>' +
                '<span style="color:#101828;">' + value + '</span></div>';
        }

        overlay.addEventListener("click", function (e) {
            if (e.target === overlay) overlay.remove();
        });
    }

    function showToast(title, message, type) {
  if(window.showPooolToast) {
    window.showPooolToast(title, message, type);
  }
}

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", handleUrlParams);
    } else {
        handleUrlParams();
    }
})();

