// frontend/platform/static/js/my-trading.js
// My Trading Dashboard — Open Orders, Buy Interests, Trade History, P2P
// Implements Jonas requirements: Dashboard Views, Tax Export

(function () {
    'use strict';

    let MOCK_ORDERS = [];
    let MOCK_INTERESTS = [];
    let MOCK_TRADES = [];
    let MOCK_P2P_INCOMING = [];
    let MOCK_P2P_OUTGOING = [];

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
                <td style="font-weight:500; font-family:monospace; font-size:12px;">${o.id.substring(0, 8)}</td>
                <td>${o.asset}</td>
                <td class="myt__side-${o.side}">${o.side.toUpperCase()}</td>
                <td>${formatUSD(o.price)}</td>
                <td>${o.qty}</td>
                <td>${o.filled}/${o.qty}</td>
                <td>${formatUSD(o.fee)}</td>
                <td><span class="myt__status myt__status--${o.status}">${o.status}</span></td>
                <td style="font-size:12px; color:var(--myt-text-sec);">${formatDate(o.createdAt)} ${formatTime(o.createdAt)}</td>
                <td><button class="myt__cancel-btn" onclick="cancelOrder('${o.id}')">Cancel</button></td>
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
                                <strong>${p.asset_id}</strong> · ${p.quantity} shares @ ${formatUSD(p.price_cents)} · <span style="font-size: 11px;">${formatDate(p.created_at)}</span>
                            ${p.message ? '<br><em>"' + p.message + '"</em>' : ''}
                        </div>
                    </div>
                    <div class="myt__p2p-actions">
                        <button class="myt__p2p-accept" onclick="respondP2POffer('${p.id}', 'accept')">Accept</button>
                        <button class="myt__p2p-decline" onclick="respondP2POffer('${p.id}', 'decline')">Decline</button>
                    </div>
                </div>
            `).join('');
        }

        if (outgoing) {
            outgoing.innerHTML = MOCK_P2P_OUTGOING.map(p => `
                <div class="myt__p2p-card">
                    <div class="myt__p2p-info">
                        <div class="myt__p2p-title">You offered to sell to target user</div>
                        <div class="myt__p2p-details">
                            <strong>${p.asset_id}</strong> · ${p.quantity} shares @ ${formatUSD(p.price_cents)} · <span class="myt__status myt__status--${p.status}">${p.status}</span> · <span style="font-size:11px;">${formatDate(p.created_at)}</span>
                        </div>
                    </div>
                    <div class="myt__p2p-actions">
                        <button class="myt__cancel-btn" onclick="cancelP2POffer('${p.id}')">Withdraw</button>
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

    // ── Fetching Data ──────────────────────────────────────────
    async function fetchAllData() {
        try {
            const [ordersRes, tradesRes, incomingRes, outgoingRes] = await Promise.all([
                fetch('/api/marketplace/orders/mine'),
                fetch('/api/marketplace/trades/mine'),
                fetch('/api/marketplace/p2p/offers/incoming'),
                fetch('/api/marketplace/p2p/offers/outgoing')
            ]);
            
            if (ordersRes.ok) MOCK_ORDERS = await ordersRes.json();
            if (tradesRes.ok) MOCK_TRADES = await tradesRes.json();
            if (incomingRes.ok) MOCK_P2P_INCOMING = await incomingRes.json();
            if (outgoingRes.ok) MOCK_P2P_OUTGOING = await outgoingRes.json();

            renderOpenOrders();
            renderBuyInterests();
            renderTradeHistory();
            renderP2POffers();
        } catch (err) {
            console.error('Failed to load dashboard data:', err);
        }
    }

    // ── Actions ────────────────────────────────────────────────
    window.cancelOrder = async function(id) {
        if (!confirm('Cancel this order?')) return;
        try {
            const res = await fetch(`/api/marketplace/orders/${id}`, { method: 'DELETE' });
            if (res.ok) {
                alert('Order cancelled');
                fetchAllData();
            } else {
                const text = await res.text();
                alert('Failed to cancel: ' + text);
            }
        } catch (e) { console.error(e); }
    };

    window.respondP2POffer = async function(id, action) {
        if (!confirm(`Are you sure you want to ${action} this offer?`)) return;
        try {
            const res = await fetch(`/api/marketplace/p2p/offers/${id}/respond`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ action: action })
            });

            if (res.ok) {
                alert('Offer ' + action + 'ed');
                fetchAllData();
            } else {
                const text = await res.text();
                alert('Failed: ' + text);
            }
        } catch (e) { console.error(e); }
    };

    window.cancelP2POffer = async function(id) {
        if (!confirm('Withdraw this offer?')) return;
        try {
            const res = await fetch(`/api/marketplace/p2p/offers/${id}`, { method: 'DELETE' });
            if (res.ok) {
                alert('Offer withdrawn');
                fetchAllData();
            } else {
                const text = await res.text();
                alert('Failed: ' + text);
            }
        } catch (e) { console.error(e); }
    };

    // ── Init ──────────────────────────────────────────────────
    document.addEventListener('DOMContentLoaded', () => {
        fetchAllData();
        initTabs();
        initTaxExport();
    });
})();
