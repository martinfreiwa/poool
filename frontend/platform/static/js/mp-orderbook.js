/**
 * Marketplace Orderbook — mp-orderbook.js
 * Mock orderbook data with bids/asks per asset, depth rendering, and rebuild action.
 */
(function () {
  'use strict';

  // ===== MOCK ORDERBOOK DATA PER ASSET =====
  const ORDERBOOKS = {
    'bali-villa': {
      name: 'Bali Villa Resort (BVRT)',
      lastPrice: 52.40,
      bids: [
        { price: 52.35, qty: 120, user: 'USR-8291' },
        { price: 52.30, qty: 250, user: 'USR-3384' },
        { price: 52.25, qty: 85,  user: 'USR-6643' },
        { price: 52.20, qty: 400, user: 'USR-1738' },
        { price: 52.15, qty: 175, user: 'USR-5561' },
        { price: 52.10, qty: 300, user: 'USR-2201' },
        { price: 52.00, qty: 520, user: 'USR-7829' },
        { price: 51.95, qty: 150, user: 'USR-4410' },
        { price: 51.90, qty: 90,  user: 'USR-9203' },
        { price: 51.80, qty: 600, user: 'USR-1105' },
      ],
      asks: [
        { price: 52.45, qty: 100, user: 'USR-4410' },
        { price: 52.50, qty: 200, user: 'USR-7712' },
        { price: 52.55, qty: 75,  user: 'USR-2290' },
        { price: 52.60, qty: 350, user: 'USR-9987' },
        { price: 52.65, qty: 180, user: 'USR-3344' },
        { price: 52.70, qty: 420, user: 'USR-6632' },
        { price: 52.80, qty: 90,  user: 'USR-5518' },
        { price: 52.90, qty: 250, user: 'USR-1234' },
        { price: 53.00, qty: 500, user: 'USR-5678' },
        { price: 53.10, qty: 300, user: 'USR-8845' },
      ]
    },
    'jakarta-office': {
      name: 'Jakarta Office Tower (JOTX)',
      lastPrice: 105.00,
      bids: [
        { price: 104.90, qty: 30,  user: 'USR-2201' },
        { price: 104.80, qty: 55,  user: 'USR-9987' },
        { price: 104.70, qty: 120, user: 'USR-1738' },
        { price: 104.50, qty: 80,  user: 'USR-5561' },
        { price: 104.30, qty: 200, user: 'USR-3384' },
        { price: 104.00, qty: 150, user: 'USR-7829' },
        { price: 103.80, qty: 90,  user: 'USR-6643' },
        { price: 103.50, qty: 300, user: 'USR-8291' },
        { price: 103.00, qty: 400, user: 'USR-4410' },
        { price: 102.50, qty: 250, user: 'USR-1105' },
      ],
      asks: [
        { price: 105.10, qty: 45,  user: 'USR-9203' },
        { price: 105.20, qty: 70,  user: 'USR-2290' },
        { price: 105.50, qty: 100, user: 'USR-7712' },
        { price: 105.80, qty: 150, user: 'USR-3344' },
        { price: 106.00, qty: 200, user: 'USR-6632' },
        { price: 106.20, qty: 80,  user: 'USR-5518' },
        { price: 106.50, qty: 120, user: 'USR-1234' },
        { price: 107.00, qty: 300, user: 'USR-5678' },
        { price: 107.50, qty: 250, user: 'USR-8845' },
        { price: 108.00, qty: 500, user: 'USR-4455' },
      ]
    },
    'surabaya-wh': {
      name: 'Surabaya Warehouse (SWHS)',
      lastPrice: 23.75,
      bids: [
        { price: 23.70, qty: 500, user: 'USR-3384' },
        { price: 23.65, qty: 300, user: 'USR-8291' },
        { price: 23.60, qty: 200, user: 'USR-6643' },
        { price: 23.55, qty: 150, user: 'USR-1738' },
        { price: 23.50, qty: 800, user: 'USR-5561' },
        { price: 23.45, qty: 100, user: 'USR-2201' },
        { price: 23.40, qty: 250, user: 'USR-7829' },
        { price: 23.30, qty: 400, user: 'USR-4410' },
        { price: 23.20, qty: 600, user: 'USR-9203' },
        { price: 23.00, qty: 1000, user: 'USR-1105' },
      ],
      asks: [
        { price: 23.80, qty: 450, user: 'USR-7712' },
        { price: 23.85, qty: 200, user: 'USR-2290' },
        { price: 23.90, qty: 350, user: 'USR-9987' },
        { price: 24.00, qty: 600, user: 'USR-3344' },
        { price: 24.10, qty: 250, user: 'USR-6632' },
        { price: 24.20, qty: 100, user: 'USR-5518' },
        { price: 24.30, qty: 300, user: 'USR-1234' },
        { price: 24.50, qty: 500, user: 'USR-5678' },
        { price: 24.70, qty: 400, user: 'USR-8845' },
        { price: 25.00, qty: 800, user: 'USR-4455' },
      ]
    },
    'bandung-tech': {
      name: 'Bandung Tech Hub (BTHB)',
      lastPrice: 87.20,
      bids: [
        { price: 87.15, qty: 60,  user: 'USR-5561' },
        { price: 87.10, qty: 100, user: 'USR-2290' },
        { price: 87.00, qty: 150, user: 'USR-1738' },
        { price: 86.90, qty: 80,  user: 'USR-3384' },
        { price: 86.80, qty: 200, user: 'USR-8291' },
        { price: 86.70, qty: 120, user: 'USR-6643' },
        { price: 86.50, qty: 250, user: 'USR-7829' },
        { price: 86.30, qty: 300, user: 'USR-2201' },
        { price: 86.00, qty: 180, user: 'USR-4410' },
        { price: 85.50, qty: 400, user: 'USR-9203' },
      ],
      asks: [
        { price: 87.25, qty: 75,  user: 'USR-9987' },
        { price: 87.30, qty: 120, user: 'USR-7712' },
        { price: 87.40, qty: 90,  user: 'USR-3344' },
        { price: 87.50, qty: 200, user: 'USR-6632' },
        { price: 87.60, qty: 150, user: 'USR-5518' },
        { price: 87.80, qty: 100, user: 'USR-1234' },
        { price: 88.00, qty: 300, user: 'USR-5678' },
        { price: 88.20, qty: 250, user: 'USR-8845' },
        { price: 88.50, qty: 400, user: 'USR-4455' },
        { price: 89.00, qty: 500, user: 'USR-1105' },
      ]
    },
    'yogya-hotel': {
      name: 'Yogya Heritage Hotel (YHHT)',
      lastPrice: 34.90,
      bids: [
        { price: 34.85, qty: 200, user: 'USR-6643' },
        { price: 34.80, qty: 150, user: 'USR-1105' },
        { price: 34.75, qty: 300, user: 'USR-8291' },
        { price: 34.70, qty: 100, user: 'USR-3384' },
        { price: 34.60, qty: 250, user: 'USR-1738' },
        { price: 34.50, qty: 400, user: 'USR-5561' },
        { price: 34.40, qty: 180, user: 'USR-2201' },
        { price: 34.30, qty: 120, user: 'USR-7829' },
        { price: 34.20, qty: 350, user: 'USR-4410' },
        { price: 34.00, qty: 500, user: 'USR-9203' },
      ],
      asks: [
        { price: 34.95, qty: 180, user: 'USR-4410' },
        { price: 35.00, qty: 250, user: 'USR-7712' },
        { price: 35.10, qty: 100, user: 'USR-2290' },
        { price: 35.20, qty: 300, user: 'USR-9987' },
        { price: 35.30, qty: 150, user: 'USR-3344' },
        { price: 35.50, qty: 200, user: 'USR-6632' },
        { price: 35.70, qty: 80,  user: 'USR-5518' },
        { price: 35.90, qty: 350, user: 'USR-1234' },
        { price: 36.00, qty: 400, user: 'USR-5678' },
        { price: 36.50, qty: 600, user: 'USR-8845' },
      ]
    }
  };

  let currentAsset = 'bali-villa';

  // ===== RENDER =====
  function render() {
    const ob = ORDERBOOKS[currentAsset];
    if (!ob) return;

    // Update title
    const nameEl = document.getElementById('selected-asset-name');
    if (nameEl) nameEl.textContent = ob.name;

    // Max qty for depth bars
    const maxBidQty = Math.max(...ob.bids.map(b => b.qty));
    const maxAskQty = Math.max(...ob.asks.map(a => a.qty));

    // Bids
    const bidsBody = document.getElementById('bids-body');
    if (bidsBody) {
      bidsBody.innerHTML = ob.bids.map(b => {
        const pct = (b.qty / maxBidQty * 100).toFixed(0);
        return `
          <tr style="position:relative;">
            <td><span class="mp-ob-uid">${b.user}</span></td>
            <td style="text-align:right">${b.qty.toLocaleString()}</td>
            <td style="text-align:right; position:relative;">
              <div class="mp-ob-depth mp-ob-depth--bid" style="width:${pct}%"></div>
              <span style="position:relative">$${b.price.toFixed(2)}</span>
            </td>
          </tr>
        `;
      }).join('');
    }

    // Asks
    const asksBody = document.getElementById('asks-body');
    if (asksBody) {
      asksBody.innerHTML = ob.asks.map(a => {
        const pct = (a.qty / maxAskQty * 100).toFixed(0);
        return `
          <tr style="position:relative;">
            <td style="position:relative;">
              <div class="mp-ob-depth mp-ob-depth--ask" style="width:${pct}%"></div>
              <span style="position:relative">$${a.price.toFixed(2)}</span>
            </td>
            <td>${a.qty.toLocaleString()}</td>
            <td style="text-align:right"><span class="mp-ob-uid">${a.user}</span></td>
          </tr>
        `;
      }).join('');
    }

    // Spread
    const bestBid = ob.bids[0].price;
    const bestAsk = ob.asks[0].price;
    const spread = bestAsk - bestBid;
    const spreadPct = ((spread / bestBid) * 100).toFixed(2);
    const totalBidVol = ob.bids.reduce((s, b) => s + b.qty, 0);
    const totalAskVol = ob.asks.reduce((s, a) => s + a.qty, 0);

    const spreadBar = document.getElementById('spread-bar');
    if (spreadBar) {
      spreadBar.innerHTML = `
        <div class="mp-ob-spread-item">
          <span class="mp-ob-spread-label">Best Bid</span>
          <span class="mp-ob-spread-value mp-ob-spread-value--bid">$${bestBid.toFixed(2)}</span>
        </div>
        <div class="mp-ob-spread-item">
          <span class="mp-ob-spread-label">Spread</span>
          <span class="mp-ob-spread-value">$${spread.toFixed(2)} (${spreadPct}%)</span>
        </div>
        <div class="mp-ob-spread-item">
          <span class="mp-ob-spread-label">Best Ask</span>
          <span class="mp-ob-spread-value mp-ob-spread-value--ask">$${bestAsk.toFixed(2)}</span>
        </div>
        <div class="mp-ob-spread-item">
          <span class="mp-ob-spread-label">Bid Vol</span>
          <span class="mp-ob-spread-value">${totalBidVol.toLocaleString()}</span>
        </div>
        <div class="mp-ob-spread-item">
          <span class="mp-ob-spread-label">Ask Vol</span>
          <span class="mp-ob-spread-value">${totalAskVol.toLocaleString()}</span>
        </div>
      `;
    }

    // Stats
    const stats = document.getElementById('ob-stats');
    if (stats) {
      stats.innerHTML = `
        <div class="mp-ob-stat">
          <div class="mp-ob-stat-label">Last Price</div>
          <div class="mp-ob-stat-value">$${ob.lastPrice.toFixed(2)}</div>
        </div>
        <div class="mp-ob-stat">
          <div class="mp-ob-stat-label">Orders</div>
          <div class="mp-ob-stat-value">${ob.bids.length + ob.asks.length}</div>
        </div>
      `;
    }
  }

  // ===== EVENTS =====
  document.addEventListener('DOMContentLoaded', () => {
    render();

    // Asset selector
    const selector = document.getElementById('asset-selector');
    if (selector) {
      selector.addEventListener('change', (e) => {
        currentAsset = e.target.value;
        render();
      });
    }

    // Rebuild Orderbook button
    const rebuildBtn = document.getElementById('btn-rebuild-orderbook');
    if (rebuildBtn) {
      rebuildBtn.addEventListener('click', () => {
        mpButtonAction(rebuildBtn, 'Orderbook rebuilt successfully', 1500, () => {
          // Simulating rebuild — just re-render
          render();
        });
      });
    }
  });
})();
