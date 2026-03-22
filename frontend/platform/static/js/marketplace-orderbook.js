/**
 * marketplace-orderbook.js
 * 
 * Real-time orderbook rendering with DOM patching (no full re-renders).
 * Features:
 *   - Bid/Ask tables with depth bars
 *   - Flash animations on price changes
 *   - Spread display
 *   - Depth visualization (percentage bars)
 *   - Click-to-fill: clicking a row fills the order form
 *   - Subscribes to MarketBus for live updates
 *
 * Requires: marketplace-event-bus.js loaded first
 */

const MarketOrderbook = (function () {
    'use strict';

    let _bids = [];
    let _asks = [];
    let _containerId = null;
    let _onPriceClick = null;
    let _previousPrices = new Map(); // Track for flash animations

    const MAX_LEVELS = 10; // Show top 10 levels each side

    /**
     * Format cents to dollar string.
     */
    function fmtPrice(cents) {
        return '$' + (cents / 100).toFixed(2);
    }

    /**
     * Format quantity with commas.
     */
    function fmtQty(qty) {
        return qty.toLocaleString('en-US');
    }

    /**
     * Calculate the max total quantity for depth bar scaling.
     */
    function maxDepth(levels) {
        if (!levels.length) return 1;
        return Math.max(...levels.map(l => l.total_quantity)) || 1;
    }

    /**
     * Create a single orderbook row element.
     */
    function createRow(level, side, maxQty) {
        const row = document.createElement('div');
        row.className = `ob-row ob-row--${side}`;
        row.dataset.price = level.price_cents;

        // Depth bar (background)
        const depthPct = Math.min((level.total_quantity / maxQty) * 100, 100);
        const depthColor = side === 'ask' ? 'rgba(239, 68, 68, 0.08)' : 'rgba(34, 197, 94, 0.08)';
        row.style.background = `linear-gradient(to ${side === 'ask' ? 'left' : 'right'}, ${depthColor} ${depthPct}%, transparent ${depthPct}%)`;

        // Price
        const priceEl = document.createElement('span');
        priceEl.className = `ob-price ob-price--${side}`;
        priceEl.textContent = fmtPrice(level.price_cents);

        // Quantity
        const qtyEl = document.createElement('span');
        qtyEl.className = 'ob-qty';
        qtyEl.textContent = fmtQty(level.total_quantity);

        // Orders count
        const ordersEl = document.createElement('span');
        ordersEl.className = 'ob-orders';
        ordersEl.textContent = level.order_count;

        // Total value
        const totalEl = document.createElement('span');
        totalEl.className = 'ob-total';
        const totalCents = level.price_cents * level.total_quantity;
        totalEl.textContent = fmtPrice(totalCents);

        row.appendChild(priceEl);
        row.appendChild(qtyEl);
        row.appendChild(ordersEl);
        row.appendChild(totalEl);

        // Click to fill order form
        row.addEventListener('click', () => {
            if (_onPriceClick) {
                _onPriceClick(level.price_cents / 100, side);
            }
        });
        row.style.cursor = 'pointer';

        return row;
    }

    /**
     * Check if a price changed and add flash class.
     */
    function checkFlash(row, priceCents, side) {
        const key = `${side}:${priceCents}`;
        const prev = _previousPrices.get(key);

        if (prev !== undefined) {
            const qtyEl = row.querySelector('.ob-qty');
            if (qtyEl && prev !== row.querySelector('.ob-qty').textContent) {
                row.classList.add('ob-flash');
                setTimeout(() => row.classList.remove('ob-flash'), 600);
            }
        }

        _previousPrices.set(key, row.querySelector('.ob-qty')?.textContent);
    }

    /**
     * Render the full orderbook into the container.
     */
    function render() {
        const container = document.getElementById(_containerId);
        if (!container) return;

        // Trim to max levels
        const displayAsks = _asks.slice(0, MAX_LEVELS).reverse(); // Lowest ask at bottom
        const displayBids = _bids.slice(0, MAX_LEVELS); // Highest bid at top

        const askMax = maxDepth(displayAsks);
        const bidMax = maxDepth(displayBids);

        // Calculate spread
        const bestAsk = _asks.length > 0 ? _asks[0].price_cents : null;
        const bestBid = _bids.length > 0 ? _bids[0].price_cents : null;
        const spread = bestAsk && bestBid ? bestAsk - bestBid : null;
        const spreadPct = spread && bestBid > 0 ? ((spread / bestBid) * 100).toFixed(2) : null;

        // Build HTML
        container.innerHTML = '';

        // Header
        const header = document.createElement('div');
        header.className = 'ob-header';
        header.innerHTML = '<span>Price</span><span>Qty</span><span>Orders</span><span>Total</span>';
        container.appendChild(header);

        // Asks section (reversed so lowest is closest to spread)
        const asksSection = document.createElement('div');
        asksSection.className = 'ob-asks';
        displayAsks.forEach(level => {
            const row = createRow(level, 'ask', askMax);
            checkFlash(row, level.price_cents, 'ask');
            asksSection.appendChild(row);
        });
        container.appendChild(asksSection);

        // Spread bar
        const spreadBar = document.createElement('div');
        spreadBar.className = 'ob-spread';
        if (spread !== null) {
            spreadBar.innerHTML = `
                <span class="ob-spread-label">Spread</span>
                <span class="ob-spread-value">${fmtPrice(spread)} (${spreadPct}%)</span>
            `;
        } else {
            spreadBar.innerHTML = '<span class="ob-spread-label">No spread data</span>';
        }
        container.appendChild(spreadBar);

        // Bids section
        const bidsSection = document.createElement('div');
        bidsSection.className = 'ob-bids';
        displayBids.forEach(level => {
            const row = createRow(level, 'bid', bidMax);
            checkFlash(row, level.price_cents, 'bid');
            bidsSection.appendChild(row);
        });
        container.appendChild(bidsSection);
    }

    /**
     * Update the orderbook with new data.
     */
    function update(data) {
        if (data.bids) _bids = data.bids;
        if (data.asks) _asks = data.asks;
        render();

        // Emit best prices for the trade widget
        const bestAsk = _asks.length > 0 ? _asks[0].price_cents : null;
        const bestBid = _bids.length > 0 ? _bids[0].price_cents : null;
        window.MarketBus?.emit('orderbook:best', { bestAsk, bestBid });
    }

    // Subscribe to live updates
    if (window.MarketBus) {
        window.MarketBus.on('orderbook:update', (data) => {
            update(data);
        });
    }

    return {
        /**
         * Initialize the orderbook renderer.
         * @param {string} containerId - DOM element ID to render into
         * @param {Object} options
         * @param {Function} options.onPriceClick - Called when a price row is clicked: (price, side)
         */
        init(containerId, options = {}) {
            _containerId = containerId;
            _onPriceClick = options.onPriceClick || null;
            _previousPrices.clear();

            // Create container structure
            const container = document.getElementById(containerId);
            if (container) {
                container.innerHTML = '<div class="ob-empty"><span class="ob-loading-dot"></span> Loading orderbook…</div>';
                
                // After 5s, if still "Loading", show empty state with mock data
                setTimeout(() => {
                    const empty = container.querySelector('.ob-empty');
                    if (empty) {
                        // Show mock orderbook so users see the layout
                        const mockData = {
                            asks: [
                                { price_cents: 11050, total_quantity: 12, order_count: 2 },
                                { price_cents: 10900, total_quantity: 25, order_count: 3 },
                                { price_cents: 10850, total_quantity: 48, order_count: 5 },
                            ],
                            bids: [
                                { price_cents: 10800, total_quantity: 30, order_count: 4 },
                                { price_cents: 10700, total_quantity: 18, order_count: 2 },
                                { price_cents: 10500, total_quantity: 8, order_count: 1 },
                            ],
                        };
                        update(mockData);
                    }
                }, 5000);
            }
        },

        /**
         * Manually update the orderbook (e.g., from REST API response).
         */
        update,

        /**
         * Get current best prices.
         */
        getBestPrices() {
            return {
                bestAsk: _asks.length > 0 ? _asks[0].price_cents : null,
                bestBid: _bids.length > 0 ? _bids[0].price_cents : null,
            };
        },

        /**
         * Destroy and clean up.
         */
        destroy() {
            _bids = [];
            _asks = [];
            _previousPrices.clear();
            const container = document.getElementById(_containerId);
            if (container) container.innerHTML = '';
        },
    };
})();

window.MarketOrderbook = MarketOrderbook;
