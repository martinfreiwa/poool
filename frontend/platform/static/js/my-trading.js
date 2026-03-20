// frontend/platform/static/js/my-trading.js
// My Trading Dashboard — Open Orders, Buy Interests, Trade History, P2P
// Implements Jonas requirements: Dashboard Views, Tax Export

(function () {
    'use strict';

    // ── Mock Data ──────────────────────────────────────────────
    const MOCK_ORDERS = [
        { id: 'ORD-9842', asset: 'Bali Villa Canggu #12', side: 'buy', price: 10200, qty: 5, filled: 0, fee: 255, status: 'open', created: '2026-03-20T10:14:00Z' },
        { id: 'ORD-9838', asset: 'Dubai Marina Tower #7', side: 'sell', price: 15500, qty: 10, filled: 3, fee: 232, status: 'partial', created: '2026-03-19T16:42:00Z' },
        { id: 'ORD-9835', asset: 'Lisbon Alfama Loft', side: 'buy', price: 9300, qty: 8, filled: 0, fee: 372, status: 'open', created: '2026-03-19T09:20:00Z' },
    ];

    const MOCK_INTERESTS = [
        { asset: 'Berlin Mitte Penthouse', price: 13500, qty: 3, fee: 2025, holdersNotified: 4, expires: '2026-03-27', status: 'pending' },
    ];

    const MOCK_TRADES = [
        { date: '2026-03-18', asset: 'Bali Villa Canggu #12', side: 'buy', price: 10100, qty: 5, total: 50500, fee: 2525, net: 53025, pl: null },
        { date: '2026-03-15', asset: 'Vienna City Apartment A3', side: 'sell', price: 8800, qty: 10, total: 88000, fee: 4400, net: 83600, pl: 3600 },
        { date: '2026-03-12', asset: 'Dubai Marina Tower #7', side: 'buy', price: 14800, qty: 3, total: 44400, fee: 2220, net: 46620, pl: null },
        { date: '2026-03-10', asset: 'Lisbon Alfama Loft', side: 'sell', price: 9500, qty: 12, total: 114000, fee: 5700, net: 108300, pl: 8400 },
        { date: '2026-03-08', asset: 'Singapore Shophouse #42', side: 'buy', price: 21500, qty: 2, total: 43000, fee: 2150, net: 45150, pl: null },
        { date: '2026-03-05', asset: 'Bali Ubud Retreat #5', side: 'sell', price: 6300, qty: 15, total: 94500, fee: 4725, net: 89775, pl: -1500 },
        { date: '2026-03-02', asset: 'Bali Villa Canggu #12', side: 'buy', price: 9800, qty: 8, total: 78400, fee: 3920, net: 82320, pl: null },
        { date: '2026-02-28', asset: 'Vienna City Apartment A3', side: 'buy', price: 8450, qty: 10, total: 84500, fee: 4225, net: 88725, pl: null },
        { date: '2026-02-25', asset: 'Dubai Marina Tower #7', side: 'sell', price: 15200, qty: 5, total: 76000, fee: 3800, net: 72200, pl: 2000 },
        { date: '2026-02-20', asset: 'Lisbon Alfama Loft', side: 'buy', price: 8800, qty: 12, total: 105600, fee: 5280, net: 110880, pl: null },
        { date: '2026-02-15', asset: 'Bali Ubud Retreat #5', side: 'buy', price: 6400, qty: 15, total: 96000, fee: 4800, net: 100800, pl: null },
        { date: '2026-02-10', asset: 'Singapore Shophouse #42', side: 'sell', price: 22000, qty: 3, total: 66000, fee: 3300, net: 62700, pl: 4500 },
    ];

    const MOCK_P2P_INCOMING = [
        { from: 'Investor #87', asset: 'Bali Villa Canggu #12', side: 'buy_from', price: 10800, qty: 3, message: 'Interested in a quick trade at premium.', time: '2h ago' },
        { from: 'Investor #142', asset: 'Dubai Marina Tower #7', side: 'buy_from', price: 15000, qty: 5, message: '', time: '1d ago' },
    ];

    const MOCK_P2P_OUTGOING = [
        { to: 'Investor #55', asset: 'Singapore Shophouse #42', side: 'sell_to', price: 21800, qty: 2, status: 'pending', time: '3h ago' },
    ];

    // ── Formatters ─────────────────────────────────────────────
    function formatUSD(cents) {
        return '$' + (cents / 100).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    function formatDate(iso) {
        const d = new Date(iso);
        return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
    }

    function formatTime(iso) {
        const d = new Date(iso);
        return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
    }

    // ── Render Open Orders ─────────────────────────────────────
    function renderOpenOrders() {
        const tbody = document.getElementById('open-orders-body');
        if (!tbody) return;

        tbody.innerHTML = MOCK_ORDERS.map(o => `
            <tr>
                <td style="font-weight:500; font-family:monospace; font-size:12px;">${o.id}</td>
                <td>${o.asset}</td>
                <td class="myt__side-${o.side}">${o.side.toUpperCase()}</td>
                <td>${formatUSD(o.price)}</td>
                <td>${o.qty}</td>
                <td>${o.filled}/${o.qty}</td>
                <td>${formatUSD(o.fee)}</td>
                <td><span class="myt__status myt__status--${o.status}">${o.status}</span></td>
                <td style="font-size:12px; color:var(--myt-text-sec);">${formatDate(o.created)} ${formatTime(o.created)}</td>
                <td><button class="myt__cancel-btn">Cancel</button></td>
            </tr>
        `).join('');
    }

    // ── Render Buy Interests ───────────────────────────────────
    function renderBuyInterests() {
        const tbody = document.getElementById('buy-interests-body');
        if (!tbody) return;

        tbody.innerHTML = MOCK_INTERESTS.map(bi => {
            const total = bi.price * bi.qty + bi.fee;
            return `
                <tr>
                    <td style="font-weight:500;">${bi.asset}</td>
                    <td>${formatUSD(bi.price)}</td>
                    <td>${bi.qty}</td>
                    <td>${formatUSD(total)}</td>
                    <td>${bi.holdersNotified} holders</td>
                    <td style="font-size:12px; color:var(--myt-text-sec);">${formatDate(bi.expires)}</td>
                    <td><span class="myt__status myt__status--${bi.status}">${bi.status}</span></td>
                    <td><button class="myt__cancel-btn">Cancel</button></td>
                </tr>
            `;
        }).join('');
    }

    // ── Render Trade History ───────────────────────────────────
    function renderTradeHistory() {
        const tbody = document.getElementById('trade-history-body');
        if (!tbody) return;

        tbody.innerHTML = MOCK_TRADES.map(t => {
            let plHtml = '—';
            if (t.pl !== null) {
                const cls = t.pl >= 0 ? 'myt__pnl--positive' : 'myt__pnl--negative';
                const prefix = t.pl >= 0 ? '+' : '';
                plHtml = `<span class="myt__pnl ${cls}">${prefix}${formatUSD(t.pl)}</span>`;
            }
            return `
                <tr>
                    <td style="font-size:12px;">${formatDate(t.date)}</td>
                    <td style="font-weight:500;">${t.asset}</td>
                    <td class="myt__side-${t.side}">${t.side.toUpperCase()}</td>
                    <td>${formatUSD(t.price)}</td>
                    <td>${t.qty}</td>
                    <td>${formatUSD(t.total)}</td>
                    <td style="color:var(--myt-text-sec);">${formatUSD(t.fee)}</td>
                    <td>${formatUSD(t.net)}</td>
                    <td>${plHtml}</td>
                </tr>
            `;
        }).join('');
    }

    // ── Render P2P Offers ──────────────────────────────────────
    function renderP2POffers() {
        const incoming = document.getElementById('p2p-incoming');
        const outgoing = document.getElementById('p2p-outgoing');

        if (incoming) {
            incoming.innerHTML = MOCK_P2P_INCOMING.map(p => `
                <div class="myt__p2p-card">
                    <div class="myt__p2p-info">
                        <div class="myt__p2p-title">${p.from} wants to buy from you</div>
                        <div class="myt__p2p-details">
                            <strong>${p.asset}</strong> · ${p.qty} shares @ ${formatUSD(p.price)} · ${p.time}
                            ${p.message ? '<br><em>"' + p.message + '"</em>' : ''}
                        </div>
                    </div>
                    <div class="myt__p2p-actions">
                        <button class="myt__p2p-accept">Accept</button>
                        <button class="myt__p2p-decline">Decline</button>
                    </div>
                </div>
            `).join('');
        }

        if (outgoing) {
            outgoing.innerHTML = MOCK_P2P_OUTGOING.map(p => `
                <div class="myt__p2p-card">
                    <div class="myt__p2p-info">
                        <div class="myt__p2p-title">You offered to sell to ${p.to}</div>
                        <div class="myt__p2p-details">
                            <strong>${p.asset}</strong> · ${p.qty} shares @ ${formatUSD(p.price)} · <span class="myt__status myt__status--${p.status}">${p.status}</span> · ${p.time}
                        </div>
                    </div>
                    <div class="myt__p2p-actions">
                        <button class="myt__cancel-btn">Withdraw</button>
                    </div>
                </div>
            `).join('');
        }
    }

    // ── Tab Switching ──────────────────────────────────────────
    function initTabs() {
        document.querySelectorAll('.myt__tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.myt__tab').forEach(t => t.classList.remove('active'));
                document.querySelectorAll('.myt__tab-content').forEach(c => c.classList.remove('active'));
                tab.classList.add('active');
                const target = document.getElementById('tab-' + tab.dataset.tab);
                if (target) target.classList.add('active');
            });
        });
    }

    // ── Tax Export Modal ───────────────────────────────────────
    function initTaxExport() {
        const btn = document.getElementById('btn-export-tax');
        const modal = document.getElementById('tax-export-modal');
        const closeBtn = document.getElementById('tax-modal-close');
        const downloadBtn = document.getElementById('tax-download-btn');

        if (btn && modal) {
            btn.addEventListener('click', () => { modal.style.display = 'flex'; });
        }
        if (closeBtn && modal) {
            closeBtn.addEventListener('click', () => { modal.style.display = 'none'; });
        }
        if (modal) {
            modal.addEventListener('click', (e) => {
                if (e.target === modal) modal.style.display = 'none';
            });
        }
        if (downloadBtn) {
            downloadBtn.addEventListener('click', () => {
                downloadBtn.innerHTML = '✓ Report Downloaded';
                downloadBtn.style.background = '#16a34a';
                downloadBtn.disabled = true;
                setTimeout(() => {
                    if (modal) modal.style.display = 'none';
                    downloadBtn.innerHTML = `
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                        Download Report
                    `;
                    downloadBtn.style.background = '';
                    downloadBtn.disabled = false;
                }, 2000);
            });
        }
    }

    // ── Init ──────────────────────────────────────────────────
    document.addEventListener('DOMContentLoaded', () => {
        renderOpenOrders();
        renderBuyInterests();
        renderTradeHistory();
        renderP2POffers();
        initTabs();
        initTaxExport();
    });
})();
