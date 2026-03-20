// frontend/platform/static/js/marketplace-trading.js
// Orchestrator for the Marketplace Trading page

document.addEventListener('DOMContentLoaded', () => {
    'use strict';

    // ── 1. Global Variables ──────────────────────────────────
    window.assetId = window.assetId || 'mock-asset-123';
    const MOCK_BALANCE = 2500000; // $25,000 in cents
    const MOCK_MAX_QTY = 500;
    const TWO_FA_THRESHOLD = 50000; // $500 in cents

    // ── 2. EventBus Check ────────────────────────────────────
    if (!window.marketBus) {
        console.warn('[Trading] marketBus missing, creating fallback');
        window.marketBus = new EventTarget();
        window.marketBus.on = function(name, cb) { this.addEventListener(name, e => cb(e.detail)); };
        window.marketBus.emit = function(name, data) { this.dispatchEvent(new CustomEvent(name, { detail: data })); };
    }

    // ── 3. Init Chart ────────────────────────────────────────
    if (typeof MarketplaceChart !== 'undefined') {
        window.marketChart = new MarketplaceChart('chart-container', window.assetId);
        window.marketChart.init();
    } else {
        console.warn('[Trading] MarketplaceChart not loaded');
    }

    // ── 4. Init Orderbook ────────────────────────────────────
    if (typeof MarketplaceOrderbook !== 'undefined') {
        window.marketOrderbook = new MarketplaceOrderbook();
        window.marketOrderbook.init();
    } else {
        console.warn('[Trading] MarketplaceOrderbook not loaded');
    }

    // ── 5. WebSocket (Mock) ──────────────────────────────────
    if (typeof MarketplaceWebSocket !== 'undefined') {
        window.marketWs = new MarketplaceWebSocket(window.assetId);
        // Don't actually connect for mock
        setTimeout(() => {
            const status = document.getElementById('ws-status');
            if (status) {
                status.className = 'mkt-ws-badge status--live';
                status.innerHTML = '🟢 Live';
            }
        }, 800);
    }

    // ── 6. Interval Buttons ──────────────────────────────────
    document.querySelectorAll('.mkt-interval-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            if (window.marketChart) {
                window.marketChart.switchInterval(btn.dataset.interval);
            }
        });
    });

    // ── 7. Trading Form ──────────────────────────────────────
    const form = document.getElementById('trading-form');
    const priceInput = document.getElementById('order-price');
    const qtyInput = document.getElementById('order-qty');
    const subtotalDisplay = document.getElementById('order-subtotal');
    const feeDisplay = document.getElementById('order-fee');
    const totalDisplay = document.getElementById('order-total');
    const balanceDisplay = document.getElementById('available-balance');
    const submitBtn = document.getElementById('order-submit-btn');
    const maxBtn = document.getElementById('max-qty-btn');
    const sideBuy = document.getElementById('side-buy');
    const sideSell = document.getElementById('side-sell');

    // Fee config (Stakeholder Decision E2: 5% base, tier discount)
    const FEE_RATE = 0.05; // 5.0%
    const TIER_DISCOUNT = 0; // 0% for standard user (mock)

    function getSelectedSide() {
        return document.querySelector('input[name="order_side"]:checked')?.value || 'buy';
    }

    function updateTotal() {
        const price = parseFloat(priceInput?.value) || 0;
        const qty = parseInt(qtyInput?.value) || 0;
        const subtotal = price * qty;
        const effectiveRate = Math.max(0, FEE_RATE - TIER_DISCOUNT);
        const fee = subtotal * effectiveRate;
        const total = subtotal + fee;

        const fmt = (v) => '$' + v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

        if (subtotalDisplay) subtotalDisplay.textContent = fmt(subtotal);
        if (feeDisplay) feeDisplay.textContent = fmt(fee);
        if (totalDisplay) totalDisplay.textContent = fmt(total);

        // Check balance
        const totalCents = Math.round(total * 100);
        if (balanceDisplay) {
            if (totalCents > MOCK_BALANCE && total > 0) {
                balanceDisplay.classList.add('over-limit');
            } else {
                balanceDisplay.classList.remove('over-limit');
            }
        }
    }

    function updateButtonState() {
        const side = getSelectedSide();
        if (submitBtn) {
            if (side === 'buy') {
                submitBtn.textContent = 'Place Buy Order';
                submitBtn.className = 'mkt-submit-btn mkt-submit-btn--buy';
            } else {
                submitBtn.textContent = 'Place Sell Order';
                submitBtn.className = 'mkt-submit-btn mkt-submit-btn--sell';
            }
        }
    }

    if (priceInput) priceInput.addEventListener('input', updateTotal);
    if (qtyInput) qtyInput.addEventListener('input', updateTotal);
    if (sideBuy) sideBuy.addEventListener('change', updateButtonState);
    if (sideSell) sideSell.addEventListener('change', updateButtonState);

    // Max button
    if (maxBtn) {
        maxBtn.addEventListener('click', () => {
            if (qtyInput) {
                qtyInput.value = MOCK_MAX_QTY;
                qtyInput.dispatchEvent(new Event('input'));
            }
        });
    }

    // Form submit with 2FA check
    if (form) {
        form.addEventListener('submit', (e) => {
            e.preventDefault();
            if (!submitBtn || submitBtn.disabled) return;

            const side = getSelectedSide();
            const priceDollars = parseFloat(priceInput?.value);
            const qty = parseInt(qtyInput?.value);

            if (isNaN(qty) || qty <= 0 || isNaN(priceDollars) || priceDollars <= 0) {
                if (window.showMarketToast) window.showMarketToast('Please enter valid positive values.', 'error');
                return;
            }

            const totalCents = Math.round(priceDollars * 100) * qty;

            // Check if total > $500 → trigger 2FA
            if (totalCents > TWO_FA_THRESHOLD && typeof window.open2FAModal === 'function') {
                window.open2FAModal(() => {
                    executeOrder(side, priceDollars, qty);
                });
                return;
            }

            executeOrder(side, priceDollars, qty);
        });
    }

    function executeOrder(side, priceDollars, qty) {
        if (!submitBtn) return;

        // Double-click protection
        submitBtn.disabled = true;
        const orgHTML = submitBtn.innerHTML;
        submitBtn.innerHTML = '<span class="mkt-spinner"></span> Processing…';

        const priceCents = Math.round(priceDollars * 100);

        setTimeout(() => {
            window.marketBus.emit('order:submitted', {
                side: side,
                price: priceCents,
                quantity: qty,
                status: 'pending'
            });

            if (window.showMarketToast) {
                window.showMarketToast(
                    `✅ Order placed: ${side.toUpperCase()} ${qty} Shares @ $${priceDollars.toFixed(2)}`,
                    'success'
                );
            }

            submitBtn.disabled = false;
            submitBtn.innerHTML = orgHTML;
            if (form) form.reset();
            updateTotal();
            updateButtonState();
        }, 1000);
    }

    // ── 8. Listen for order:submitted ────────────────────────
    window.marketBus.on('order:submitted', (order) => {
        console.log('[Trading] Order submitted:', order);
    });

    // Initialize button state
    updateButtonState();
});
