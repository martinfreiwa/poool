/**
 * marketplace-trading.js
 * 
 * Core orchestration module for the trading page.
 * Initializes all components and wires them together:
 *   1. Event Bus (loaded)
 *   2. WebSocket Client → connect to asset
 *   3. Orderbook Renderer → subscribe to updates
 *   4. Trade Form → submit orders via API
 *   5. My Orders / Trade History → fetch and display
 *   6. Visibility API → pause/resume WebSocket
 * 
 * Requires: marketplace-event-bus.js, marketplace-websocket.js,
 *           marketplace-orderbook.js loaded before this file.
 */

const MarketTrading = (function () {
    'use strict';

    let _assetId = null;
    let _side = 'buy';
    let _priceMode = 'market'; // 'market' | 'limit'
    let _submitting = false;
    let _bestAsk = null;
    let _bestBid = null;
    let _orderbookSnapshot = null;

    // ═══════════════════════════════════════════════════════════════
    // ── FORMATTING HELPERS ────────────────────────────────────────
    // ═══════════════════════════════════════════════════════════════

    function fmtUSD(cents) {
        return '$' + (cents / 100).toFixed(2);
    }

    function fmtUSDFromDollars(dollars) {
        return '$' + dollars.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    // ═══════════════════════════════════════════════════════════════
    // ── API CALLS ─────────────────────────────────────────────────
    // ═══════════════════════════════════════════════════════════════

    /**
     * Fetch the orderbook for an asset.
     */
    async function fetchOrderbook(assetId) {
        try {
            const res = await fetch(`/api/marketplace/${assetId}/orderbook`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            return await res.json();
        } catch (err) {
            console.error('[Trading] Failed to fetch orderbook:', err);
            return null;
        }
    }

    /**
     * Submit a new order.
     */
    async function submitOrder(order) {
        try {
            const res = await fetch('/api/marketplace/orders', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                credentials: 'same-origin',
                body: JSON.stringify(order),
            });

            const data = await res.json();

            if (!res.ok) {
                // Handle 428 (2FA required)
                if (res.status === 428) {
                    window.MarketBus?.emit('order:2fa_required', data);
                    return { success: false, error: '2FA verification required', requires2fa: true };
                }
                return { success: false, error: data.error || data.message || 'Order failed' };
            }

            return { success: true, data };
        } catch (err) {
            console.error('[Trading] Order submission failed:', err);
            return { success: false, error: 'Network error — please try again' };
        }
    }

    /**
     * Cancel an order.
     */
    async function cancelOrder(orderId) {
        try {
            const res = await fetch(`/api/marketplace/orders/${orderId}`, {
                method: 'DELETE',
                credentials: 'same-origin',
            });

            if (!res.ok) {
                const data = await res.json();
                return { success: false, error: data.error || 'Cancel failed' };
            }

            return { success: true };
        } catch (err) {
            console.error('[Trading] Cancel failed:', err);
            return { success: false, error: 'Network error' };
        }
    }

    /**
     * Fetch user's open orders for this asset.
     */
    async function fetchMyOrders() {
        try {
            const res = await fetch('/api/marketplace/orders/mine', {
                credentials: 'same-origin',
            });
            if (!res.ok) return [];
            return await res.json();
        } catch {
            return [];
        }
    }

    /**
     * Fetch recent trades for this asset.
     */
    async function fetchRecentTrades(assetId) {
        try {
            const res = await fetch(`/api/marketplace/${assetId}/trades`);
            if (!res.ok) return [];
            return await res.json();
        } catch {
            return [];
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // ── ORDER FORM HANDLING ───────────────────────────────────────
    // ═══════════════════════════════════════════════════════════════

    function getFormPrice() {
        if (_priceMode === 'market') {
            return _side === 'buy' ? _bestAsk : _bestBid;
        }
        const input = document.getElementById('tv3-price');
        return input ? Math.round(parseFloat(input.value) * 100) : 0;
    }

    function updateOrderSummary() {
        const qty = parseInt(document.getElementById('tv3-qty')?.value) || 0;
        const priceCents = getFormPrice() || 0;
        const priceDollars = priceCents / 100;
        const subtotal = qty * priceDollars;
        const fee = subtotal * 0.05;
        const total = subtotal + fee;

        const subtotalEl = document.getElementById('tv3-subtotal');
        const feeEl = document.getElementById('tv3-fee');
        const totalEl = document.getElementById('tv3-total');
        const btnEl = document.getElementById('tv3-submit-btn');

        if (subtotalEl) subtotalEl.textContent = fmtUSDFromDollars(subtotal);
        if (feeEl) feeEl.textContent = fmtUSDFromDollars(fee);
        if (totalEl) totalEl.textContent = fmtUSDFromDollars(total);

        if (btnEl) {
            const action = _side === 'buy' ? 'Buy' : 'Sell';
            const shareWord = qty === 1 ? 'Share' : 'Shares';
            btnEl.textContent = `${action} ${qty} ${shareWord} · ${fmtUSDFromDollars(total)}`;
        }

        // Update best price display
        const bestPriceEl = document.getElementById('tv3-best-price');
        if (bestPriceEl && priceCents) {
            bestPriceEl.textContent = fmtUSD(priceCents);
        }
    }

    function updateMarketInfo() {
        if (!_orderbookSnapshot) return;

        const bestPriceEl = document.getElementById('tv3-best-price');
        const availSharesEl = document.getElementById('tv3-avail-shares');
        const availSellersEl = document.getElementById('tv3-avail-sellers');

        if (_side === 'buy' && _orderbookSnapshot.asks) {
            const asks = _orderbookSnapshot.asks;
            const totalShares = asks.reduce((s, l) => s + l.total_quantity, 0);
            if (bestPriceEl && asks.length) bestPriceEl.textContent = fmtUSD(asks[0].price_cents);
            if (availSharesEl) availSharesEl.textContent = `${totalShares} shares`;
            if (availSellersEl) {
                const count = asks.reduce((s, l) => s + l.order_count, 0);
                availSellersEl.textContent = `${count} sellers`;
            }
        } else if (_orderbookSnapshot.bids) {
            const bids = _orderbookSnapshot.bids;
            const totalShares = bids.reduce((s, l) => s + l.total_quantity, 0);
            if (bestPriceEl && bids.length) bestPriceEl.textContent = fmtUSD(bids[0].price_cents);
            if (availSharesEl) availSharesEl.textContent = `${totalShares} shares`;
            if (availSellersEl) {
                const count = bids.reduce((s, l) => s + l.order_count, 0);
                availSellersEl.textContent = `${count} buyers`;
            }
        }
    }

    /**
     * Generate a v4-like idempotency key.
     */
    function generateIdempotencyKey() {
        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
            const r = Math.random() * 16 | 0;
            return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
        });
    }

    async function handleSubmit(e) {
        if (e) e.preventDefault();
        if (_submitting || !_assetId) return;

        const btn = document.getElementById('tv3-submit-btn');
        const qty = parseInt(document.getElementById('tv3-qty')?.value) || 0;
        const priceCents = getFormPrice();

        if (qty <= 0) {
            showToast('Please enter a valid quantity', 'error');
            return;
        }

        if (_priceMode === 'limit' && (!priceCents || priceCents <= 0)) {
            showToast('Please enter a valid price', 'error');
            return;
        }

        // Double-click protection
        _submitting = true;
        if (btn) {
            btn.disabled = true;
            btn.textContent = 'Placing Order…';
        }

        const order = {
            asset_id: _assetId,
            side: _side,
            order_type: _priceMode === 'market' ? 'market' : 'limit',
            quantity: qty,
            price_cents: priceCents || undefined,
            idempotency_key: generateIdempotencyKey(),
        };

        const result = await submitOrder(order);

        if (result.success) {
            if (btn) btn.textContent = '✓ Order Placed Successfully';
            showToast(`${_side === 'buy' ? 'Buy' : 'Sell'} order placed for ${qty} shares`, 'success');
            window.MarketBus?.emit('order:submitted', result.data);

            // Refresh orderbook and my orders
            setTimeout(() => {
                fetchAndUpdateOrderbook();
                loadMyOrders();
            }, 500);
        } else {
            if (result.requires2fa) {
                showToast('2FA verification required for trading', 'warning');
            } else {
                showToast(result.error, 'error');
            }
        }

        // Reset button after 2.5s
        setTimeout(() => {
            _submitting = false;
            if (btn) {
                btn.disabled = false;
                updateOrderSummary();
            }
        }, 2500);
    }

    // ═══════════════════════════════════════════════════════════════
    // ── MY ORDERS ─────────────────────────────────────────────────
    // ═══════════════════════════════════════════════════════════════

    async function loadMyOrders() {
        const container = document.getElementById('tv3-my-orders');
        if (!container) return;

        const orders = await fetchMyOrders();
        const assetOrders = orders.filter(o => o.asset_id === _assetId);

        if (!assetOrders.length) {
            container.innerHTML = '<div class="ob-empty">No open orders</div>';
            return;
        }

        container.innerHTML = assetOrders.map(o => `
            <div class="my-order-row my-order-row--${o.side}">
                <span class="my-order-side my-order-side--${o.side}">${o.side.toUpperCase()}</span>
                <span class="my-order-price">${fmtUSD(o.price_cents)}</span>
                <span class="my-order-qty">${o.quantity - o.quantity_filled} / ${o.quantity}</span>
                <span class="my-order-status">${o.status}</span>
                <button class="my-order-cancel" data-order-id="${o.id}" title="Cancel">✕</button>
            </div>
        `).join('');

        // Bind cancel buttons
        container.querySelectorAll('.my-order-cancel').forEach(btn => {
            btn.addEventListener('click', async () => {
                const orderId = btn.dataset.orderId;
                btn.disabled = true;
                btn.textContent = '…';
                const result = await cancelOrder(orderId);
                if (result.success) {
                    showToast('Order cancelled', 'success');
                    loadMyOrders();
                    setTimeout(fetchAndUpdateOrderbook, 300);
                } else {
                    showToast(result.error, 'error');
                    btn.disabled = false;
                    btn.textContent = '✕';
                }
            });
        });
    }

    // ═══════════════════════════════════════════════════════════════
    // ── RECENT TRADES ─────────────────────────────────────────────
    // ═══════════════════════════════════════════════════════════════

    async function loadRecentTrades() {
        const container = document.getElementById('tv3-recent-trades');
        if (!container) return;

        const trades = await fetchRecentTrades(_assetId);

        if (!trades.length) {
            container.innerHTML = '<div class="ob-empty">No trades yet</div>';
            return;
        }

        container.innerHTML = trades.slice(0, 20).map(t => {
            const time = new Date(t.executed_at).toLocaleTimeString();
            const isBuy = t.is_buyer_maker;
            return `
                <div class="trade-row">
                    <span class="trade-price trade-price--${isBuy ? 'bid' : 'ask'}">${fmtUSD(t.price_cents)}</span>
                    <span class="trade-qty">${t.quantity}</span>
                    <span class="trade-time">${time}</span>
                </div>
            `;
        }).join('');
    }

    // ═══════════════════════════════════════════════════════════════
    // ── TOAST NOTIFICATIONS ───────────────────────────────────────
    // ═══════════════════════════════════════════════════════════════

    function showToast(message, type = 'info') {
        // Remove existing toast
        const existing = document.querySelector('.trade-toast');
        if (existing) existing.remove();

        const toast = document.createElement('div');
        toast.className = `trade-toast trade-toast--${type}`;
        toast.setAttribute('role', 'alert');
        toast.textContent = message;

        document.body.appendChild(toast);

        // Trigger animation
        requestAnimationFrame(() => toast.classList.add('show'));

        // Auto-dismiss after 4s
        setTimeout(() => {
            toast.classList.remove('show');
            setTimeout(() => toast.remove(), 300);
        }, 4000);
    }

    // ═══════════════════════════════════════════════════════════════
    // ── INITIAL DATA LOAD ─────────────────────────────────────────
    // ═══════════════════════════════════════════════════════════════

    async function fetchAndUpdateOrderbook() {
        const data = await fetchOrderbook(_assetId);
        if (data) {
            _orderbookSnapshot = data;
            _bestAsk = data.asks?.length ? data.asks[0].price_cents : null;
            _bestBid = data.bids?.length ? data.bids[0].price_cents : null;
            window.MarketOrderbook?.update(data);
            updateMarketInfo();
            updateOrderSummary();
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // ── INITIALIZATION ────────────────────────────────────────────
    // ═══════════════════════════════════════════════════════════════

    function init(assetId) {
        _assetId = assetId;

        console.log('[Trading] Initializing for asset:', assetId);

        // 1. Initialize orderbook renderer
        if (window.MarketOrderbook) {
            window.MarketOrderbook.init('tv3-orderbook', {
                onPriceClick: (price, side) => {
                    // Fill the price into the order form
                    const priceInput = document.getElementById('tv3-price');
                    if (priceInput) {
                        priceInput.value = price.toFixed(2);
                        // Switch to custom/limit mode
                        setPriceMode('custom');
                        updateOrderSummary();
                    }
                },
            });
        }

        // 2. Connect WebSocket
        if (window.MarketWS) {
            window.MarketWS.connect(assetId);
        }

        // 3. Subscribe to live events
        if (window.MarketBus) {
            // Orderbook updates refresh best prices
            window.MarketBus.on('orderbook:best', (data) => {
                _bestAsk = data.bestAsk;
                _bestBid = data.bestBid;
                updateOrderSummary();
            });

            // New trades
            window.MarketBus.on('trade', (data) => {
                showToast(`Trade: ${data.quantity} shares at ${fmtUSD(data.price_cents)}`, 'info');
                loadRecentTrades();
            });

            // Connection state
            window.MarketBus.on('ws:state', (data) => {
                const indicator = document.getElementById('tv3-ws-indicator');
                if (indicator) {
                    indicator.className = `ws-indicator ws-indicator--${data.state}`;
                    indicator.title = `WebSocket: ${data.state}`;
                }
            });
        }

        // 4. Fetch initial data via REST (WebSocket sends snapshot too but REST is backup)
        fetchAndUpdateOrderbook();
        loadMyOrders();
        loadRecentTrades();

        // 5. Bind form events
        const form = document.getElementById('tv3-order-form');
        if (form) form.addEventListener('submit', handleSubmit);

        const qtyInput = document.getElementById('tv3-qty');
        if (qtyInput) qtyInput.addEventListener('input', updateOrderSummary);

        const priceInput = document.getElementById('tv3-price');
        if (priceInput) priceInput.addEventListener('input', updateOrderSummary);

        // Buy/Sell toggle
        const buyBtn = document.getElementById('tv3-toggle-buy');
        const sellBtn = document.getElementById('tv3-toggle-sell');
        if (buyBtn) buyBtn.addEventListener('click', () => setSide('buy'));
        if (sellBtn) sellBtn.addEventListener('click', () => setSide('sell'));

        // Price mode toggle
        const marketBtn = document.getElementById('tv3-mode-market');
        const customBtn = document.getElementById('tv3-mode-custom');
        if (marketBtn) marketBtn.addEventListener('click', () => setPriceMode('market'));
        if (customBtn) customBtn.addEventListener('click', () => setPriceMode('custom'));

        // Periodic refresh (backup for WebSocket)
        setInterval(() => {
            if (document.visibilityState === 'visible') {
                fetchAndUpdateOrderbook();
            }
        }, 30000); // Every 30s
    }

    function setSide(side) {
        _side = side;
        document.getElementById('tv3-toggle-buy')?.classList.toggle('active', side === 'buy');
        document.getElementById('tv3-toggle-sell')?.classList.toggle('active', side === 'sell');

        const btn = document.getElementById('tv3-submit-btn');
        if (btn) {
            btn.className = side === 'buy'
                ? 'tv3-submit-btn tv3-submit-btn--buy'
                : 'tv3-submit-btn tv3-submit-btn--sell';
        }

        const disclaimer = document.getElementById('tv3-disclaimer');
        if (disclaimer) {
            disclaimer.textContent = side === 'buy'
                ? "You won't be charged until matched with a seller."
                : "Your shares will be listed and matched with a buyer.";
        }

        updateMarketInfo();
        updateOrderSummary();
    }

    function setPriceMode(mode) {
        _priceMode = mode === 'custom' ? 'limit' : 'market';

        document.getElementById('tv3-mode-market')?.classList.toggle('active', mode === 'market');
        document.getElementById('tv3-mode-custom')?.classList.toggle('active', mode === 'custom');
        
        const marketInfo = document.getElementById('tv3-market-info');
        const customField = document.getElementById('tv3-custom-field');
        if (marketInfo) marketInfo.style.display = mode === 'market' ? 'block' : 'none';
        if (customField) customField.style.display = mode === 'custom' ? 'block' : 'none';

        if (mode === 'custom') {
            const priceInput = document.getElementById('tv3-price');
            const bestPrice = _side === 'buy' ? _bestAsk : _bestBid;
            if (priceInput && !priceInput.value && bestPrice) {
                priceInput.value = (bestPrice / 100).toFixed(2);
            }
        }

        updateOrderSummary();
    }

    function destroy() {
        if (window.MarketWS) window.MarketWS.disconnect();
        if (window.MarketOrderbook) window.MarketOrderbook.destroy();
    }

    return { init, destroy, setSide, setPriceMode };
})();

window.MarketTrading = MarketTrading;
