/**
 * Marketplace Orderbook — mp-orderbook.js
 * Fetches orderbook data from the backend API per selected asset.
 * Falls back to mock data if the API is unavailable.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/orderbook';

  // ── Mock Data (unchanged from original) ─────────────────────────
  const MOCK_ORDERBOOKS = {
    'bali-villa': {
      name: 'Bali Villa Resort (BVRT)', lastPrice: 52.40,
      bids: [
        { price: 52.35, qty: 120, user: 'USR-8291' }, { price: 52.30, qty: 250, user: 'USR-3384' },
        { price: 52.25, qty: 85,  user: 'USR-6643' }, { price: 52.20, qty: 400, user: 'USR-1738' },
        { price: 52.15, qty: 175, user: 'USR-5561' }, { price: 52.10, qty: 300, user: 'USR-2201' },
        { price: 52.00, qty: 520, user: 'USR-7829' }, { price: 51.95, qty: 150, user: 'USR-4410' },
        { price: 51.90, qty: 90,  user: 'USR-9203' }, { price: 51.80, qty: 600, user: 'USR-1105' },
      ],
      asks: [
        { price: 52.45, qty: 100, user: 'USR-4410' }, { price: 52.50, qty: 200, user: 'USR-7712' },
        { price: 52.55, qty: 75,  user: 'USR-2290' }, { price: 52.60, qty: 350, user: 'USR-9987' },
        { price: 52.65, qty: 180, user: 'USR-3344' }, { price: 52.70, qty: 420, user: 'USR-6632' },
        { price: 52.80, qty: 90,  user: 'USR-5518' }, { price: 52.90, qty: 250, user: 'USR-1234' },
        { price: 53.00, qty: 500, user: 'USR-5678' }, { price: 53.10, qty: 300, user: 'USR-8845' },
      ]
    },
    'jakarta-office': { name: 'Jakarta Office Tower (JOTX)', lastPrice: 105.00,
      bids: [{ price: 104.90, qty: 30 }, { price: 104.80, qty: 55 }, { price: 104.70, qty: 120 }, { price: 104.50, qty: 80 }, { price: 104.00, qty: 150 }],
      asks: [{ price: 105.10, qty: 45 }, { price: 105.20, qty: 70 }, { price: 105.50, qty: 100 }, { price: 105.80, qty: 150 }, { price: 106.00, qty: 200 }]
    },
    'surabaya-wh': { name: 'Surabaya Warehouse (SWHS)', lastPrice: 23.75,
      bids: [{ price: 23.70, qty: 500 }, { price: 23.65, qty: 300 }, { price: 23.60, qty: 200 }, { price: 23.50, qty: 800 }, { price: 23.40, qty: 250 }],
      asks: [{ price: 23.80, qty: 450 }, { price: 23.85, qty: 200 }, { price: 23.90, qty: 350 }, { price: 24.00, qty: 600 }, { price: 24.10, qty: 250 }]
    },
  };

  let currentAsset = 'bali-villa';
  let usingMockData = false;

  // ── Render Orderbook from API Data ──────────────────────────────
  function renderFromAPI(data) {
    const nameEl = document.getElementById('selected-asset-name');
    if (nameEl) nameEl.textContent = `Asset ${data.asset_id.substring(0, 8)}`;

    renderBids(data.bids.map(l => ({ price: l.price_cents / 100, qty: l.total_quantity, count: l.order_count })));
    renderAsks(data.asks.map(l => ({ price: l.price_cents / 100, qty: l.total_quantity, count: l.order_count })));

    const bestBid = data.bids.length > 0 ? data.bids[0].price_cents / 100 : null;
    const bestAsk = data.asks.length > 0 ? data.asks[0].price_cents / 100 : null;
    renderSpread(bestBid, bestAsk, data.bids, data.asks, true);

    const stats = document.getElementById('ob-stats');
    if (stats) {
      const midPrice = data.mid_price_cents ? (data.mid_price_cents / 100).toFixed(2) : 'N/A';
      const totalOrders = data.bids.reduce((s, l) => s + l.order_count, 0) + data.asks.reduce((s, l) => s + l.order_count, 0);
      stats.innerHTML = `
        <div class="mp-ob-stat"><div class="mp-ob-stat-label">Mid Price</div><div class="mp-ob-stat-value">$${midPrice}</div></div>
        <div class="mp-ob-stat"><div class="mp-ob-stat-label">Orders</div><div class="mp-ob-stat-value">${totalOrders}</div></div>
      `;
    }
  }

  // ── Render Orderbook from Mock Data ─────────────────────────────
  function renderFromMock() {
    const ob = MOCK_ORDERBOOKS[currentAsset];
    if (!ob) return;

    const nameEl = document.getElementById('selected-asset-name');
    if (nameEl) nameEl.textContent = ob.name;

    renderBids(ob.bids.map(b => ({ price: b.price, qty: b.qty, user: b.user })));
    renderAsks(ob.asks.map(a => ({ price: a.price, qty: a.qty, user: a.user })));

    const bestBid = ob.bids[0].price;
    const bestAsk = ob.asks[0].price;
    renderSpread(bestBid, bestAsk, ob.bids, ob.asks, false);

    const stats = document.getElementById('ob-stats');
    if (stats) {
      stats.innerHTML = `
        <div class="mp-ob-stat"><div class="mp-ob-stat-label">Last Price</div><div class="mp-ob-stat-value">$${ob.lastPrice.toFixed(2)}</div></div>
        <div class="mp-ob-stat"><div class="mp-ob-stat-label">Orders</div><div class="mp-ob-stat-value">${ob.bids.length + ob.asks.length}</div></div>
      `;
    }
  }

  // ── Common Renderers ────────────────────────────────────────────
  function renderBids(bids) {
    const bidsBody = document.getElementById('bids-body');
    if (!bidsBody || bids.length === 0) {
      if (bidsBody) bidsBody.innerHTML = '<tr><td colspan="3" style="text-align:center; color:var(--admin-text-muted);">No bids</td></tr>';
      return;
    }
    const maxQty = Math.max(...bids.map(b => b.qty));
    bidsBody.innerHTML = bids.map(b => {
      const pct = (b.qty / maxQty * 100).toFixed(0);
      const userCol = b.user ? `<span class="mp-ob-uid">${b.user}</span>` : (b.count ? `${b.count} orders` : '');
      return `
        <tr style="position:relative;">
          <td>${userCol}</td>
          <td style="text-align:right">${typeof b.qty === 'number' ? b.qty.toLocaleString() : b.qty}</td>
          <td style="text-align:right; position:relative;">
            <div class="mp-ob-depth mp-ob-depth--bid" style="width:${pct}%"></div>
            <span style="position:relative">$${b.price.toFixed(2)}</span>
          </td>
        </tr>
      `;
    }).join('');
  }

  function renderAsks(asks) {
    const asksBody = document.getElementById('asks-body');
    if (!asksBody || asks.length === 0) {
      if (asksBody) asksBody.innerHTML = '<tr><td colspan="3" style="text-align:center; color:var(--admin-text-muted);">No asks</td></tr>';
      return;
    }
    const maxQty = Math.max(...asks.map(a => a.qty));
    asksBody.innerHTML = asks.map(a => {
      const pct = (a.qty / maxQty * 100).toFixed(0);
      const userCol = a.user ? `<span class="mp-ob-uid">${a.user}</span>` : (a.count ? `${a.count} orders` : '');
      return `
        <tr style="position:relative;">
          <td style="position:relative;">
            <div class="mp-ob-depth mp-ob-depth--ask" style="width:${pct}%"></div>
            <span style="position:relative">$${a.price.toFixed(2)}</span>
          </td>
          <td>${typeof a.qty === 'number' ? a.qty.toLocaleString() : a.qty}</td>
          <td style="text-align:right">${userCol}</td>
        </tr>
      `;
    }).join('');
  }

  function renderSpread(bestBid, bestAsk, bids, asks, isCents) {
    const spreadBar = document.getElementById('spread-bar');
    if (!spreadBar) return;

    if (bestBid === null || bestAsk === null) {
      spreadBar.innerHTML = '<div class="mp-ob-spread-item"><span class="mp-ob-spread-label">No active market</span></div>';
      return;
    }

    const spread = bestAsk - bestBid;
    const spreadPct = ((spread / bestBid) * 100).toFixed(2);

    const totalBidVol = isCents
      ? bids.reduce((s, l) => s + l.total_quantity, 0)
      : bids.reduce((s, b) => s + (b.qty || 0), 0);
    const totalAskVol = isCents
      ? asks.reduce((s, l) => s + l.total_quantity, 0)
      : asks.reduce((s, a) => s + (a.qty || 0), 0);

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

  // ── Load Orderbook ──────────────────────────────────────────────
  async function loadOrderbook() {
    // Try to load asset list from selector to get the UUID
    const selector = document.getElementById('asset-selector');
    const selectedValue = selector ? selector.value : currentAsset;

    // If the value looks like a UUID, try the API
    const isUUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(selectedValue);

    if (isUUID) {
      try {
        const res = await fetch(`${API}/${selectedValue}`, { credentials: 'same-origin' });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const data = await res.json();
        usingMockData = false;
        renderFromAPI(data);
        return;
      } catch (err) {
        console.warn('[mp-orderbook] API unavailable:', err);
      }
    }

    // Fall back to mock data
    usingMockData = true;
    currentAsset = selectedValue;
    renderFromMock();
  }

  // ── Init ────────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', () => {
    loadOrderbook();

    const selector = document.getElementById('asset-selector');
    if (selector) {
      selector.addEventListener('change', () => { loadOrderbook(); });
    }

    const rebuildBtn = document.getElementById('btn-rebuild-orderbook');
    if (rebuildBtn) {
      rebuildBtn.addEventListener('click', () => {
        if (typeof mpButtonAction === 'function') {
          mpButtonAction(rebuildBtn, 'Orderbook rebuilt successfully', 1500, () => { loadOrderbook(); });
        }
      });
    }
  });
})();
