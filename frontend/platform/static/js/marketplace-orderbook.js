// frontend/platform/static/js/marketplace-orderbook.js
// Live Orderbook with depth bars, flash animation, and mock updates

class MarketplaceOrderbook {
    constructor() {
        this.bidsContainer = document.getElementById('orderbook-bids');
        this.asksContainer = document.getElementById('orderbook-asks');
        this.spreadDisplay = document.getElementById('orderbook-spread');
        this.lastBids = [];
        this.lastAsks = [];
        this.maxRows = 10;
        this.mockInterval = null;
    }

    init() {
        if (!this.bidsContainer || !this.asksContainer) return;

        this._initEmptyRows(this.bidsContainer, 'bid');
        this._initEmptyRows(this.asksContainer, 'ask');

        // Listen for orderbook updates from event bus
        if (window.marketBus) {
            window.marketBus.on('orderbook:updated', (data) => {
                if (data.bids) this.renderBids(data.bids);
                if (data.asks) this.renderAsks(data.asks);
                if (data.spread !== undefined) this.updateSpread(data.spread, data.spreadPct);
            });
        }

        this._startMockUpdates();
    }

    renderBids(bids) {
        this._renderSide(this.bidsContainer, bids, this.lastBids, 'bid');
        this.lastBids = [...bids];
    }

    renderAsks(asks) {
        const bestAsks = [...asks].slice(0, this.maxRows).reverse();
        this._renderSide(this.asksContainer, bestAsks, this.lastAsks, 'ask');
        this.lastAsks = [...bestAsks];
    }

    updateSpread(spreadCents, spreadPct) {
        if (this.spreadDisplay) {
            const spreadStr = '$' + (spreadCents / 100).toFixed(2);
            const pctStr = spreadPct ? ` (${spreadPct.toFixed(2)}%)` : '';
            this.spreadDisplay.textContent = `Spread: ${spreadStr}${pctStr}`;
        }
    }

    _initEmptyRows(container, side) {
        container.innerHTML = '';
        for (let i = 0; i < this.maxRows; i++) {
            container.appendChild(this._createRow(side));
        }
    }

    _createRow(side) {
        const row = document.createElement('div');
        row.className = `orderbook-row orderbook-row--${side}`;
        row.innerHTML = `
            <div class="ob-col ob-price">–</div>
            <div class="ob-col ob-qty">–</div>
            <div class="ob-col ob-total">–</div>
            <div class="ob-bg"></div>
        `;
        // Click on row to populate price field
        row.addEventListener('click', () => {
            const priceText = row.querySelector('.ob-price').textContent;
            if (priceText && priceText !== '–') {
                const priceInput = document.getElementById('order-price');
                if (priceInput) {
                    priceInput.value = priceText;
                    priceInput.dispatchEvent(new Event('input'));
                }
            }
        });
        return row;
    }

    _renderSide(container, levels, previousLevels, side) {
        const rows = container.children;
        let cumulativeTotal = 0;
        const maxQty = Math.max(...levels.map(l => l.quantity), 1);
        const maxTotal = maxQty * this.maxRows * 0.5;

        for (let i = 0; i < this.maxRows; i++) {
            const row = rows[i];
            if (!row) break;

            const level = levels[i];
            const prevLevel = previousLevels[i];

            if (!level) {
                this._patchRow(row, '–', '–', '–', 0);
                continue;
            }

            cumulativeTotal += level.quantity;
            const priceStr = (level.price / 100).toFixed(2);
            const qtyStr = level.quantity.toString();
            const totalStr = cumulativeTotal.toString();
            const depthPct = Math.min((cumulativeTotal / (maxTotal || 1)) * 100, 100);

            if (!prevLevel || prevLevel.price !== level.price || prevLevel.quantity !== level.quantity) {
                this._patchRow(row, priceStr, qtyStr, totalStr, depthPct);

                // Flash animation
                row.classList.remove('ob-flash');
                void row.offsetWidth;
                row.classList.add('ob-flash');
            }
        }
    }

    _patchRow(row, price, qty, total, depthPct) {
        row.children[0].textContent = price;
        row.children[1].textContent = qty;
        row.children[2].textContent = total;
        row.children[3].style.width = `${depthPct}%`;
    }

    _startMockUpdates() {
        let basePrice = 10500; // $105.00 in cents

        const update = () => {
            const mockBids = [];
            const mockAsks = [];

            basePrice += Math.floor((Math.random() - 0.5) * 20);

            for (let i = 1; i <= this.maxRows; i++) {
                mockBids.push({
                    price: basePrice - i * 12 - Math.floor(Math.random() * 8),
                    quantity: Math.floor(Math.random() * 60) + 5
                });
                mockAsks.push({
                    price: basePrice + i * 12 + Math.floor(Math.random() * 8),
                    quantity: Math.floor(Math.random() * 60) + 5
                });
            }

            const spread = mockAsks[0].price - mockBids[0].price;
            const spreadPct = (spread / basePrice) * 100;

            if (window.marketBus) {
                window.marketBus.emit('orderbook:updated', {
                    bids: mockBids,
                    asks: mockAsks,
                    spread: spread,
                    spreadPct: spreadPct
                });
            }

            // Occasionally fire live trades for the chart
            if (Math.random() > 0.55) {
                const isBuy = Math.random() > 0.5;
                if (window.marketBus) {
                    window.marketBus.emit('trade:executed', {
                        price: isBuy ? mockAsks[0].price : mockBids[0].price,
                        quantity: Math.floor(Math.random() * 25) + 1,
                        timestamp: new Date().toISOString()
                    });
                }
            }
        };

        // Initial update
        update();

        // Update every 3 seconds
        this.mockInterval = setInterval(update, 3000);
    }

    destroy() {
        if (this.mockInterval) {
            clearInterval(this.mockInterval);
            this.mockInterval = null;
        }
    }
}
