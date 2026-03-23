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
    let PORTFOLIO_ASSETS = [];
    let currentUserEmail = null;

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
        const countBadge = document.getElementById('tab-count-orders');
        if (!tbody) return;

        if (countBadge) countBadge.innerText = MOCK_ORDERS.length;

        if (MOCK_ORDERS.length === 0) {
            tbody.innerHTML = `<tr><td colspan="10" style="text-align:center; padding: 32px; color: var(--myt-text-sec);">No open orders.</td></tr>`;
            return;
        }

        tbody.innerHTML = MOCK_ORDERS.map(o => `
            <tr>
                <td style="font-weight:500; font-family:monospace; font-size:12px;">${o.id.substring(0, 8)}</td>
                <td>${o.asset}</td>
                <td class="myt__side-${o.side}">${o.side.toUpperCase()}</td>
                <td>${formatUSD(o.priceCents)}</td>
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
        const countBadge = document.getElementById('tab-count-interests');
        if (!tbody) return;

        if (countBadge) countBadge.innerText = MOCK_INTERESTS.length;

        if (MOCK_INTERESTS.length === 0) {
            tbody.innerHTML = `<tr><td colspan="8" style="text-align:center; padding: 32px; color: var(--myt-text-sec);">No buy interests found.</td></tr>`;
            return;
        }

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
        const summary = document.getElementById('trade-summary');
        
        if (!tbody) return;

        if (summary) {
            const netPl = MOCK_TRADES.reduce((sum, t) => sum + (t.pl || 0), 0);
            const cls = netPl >= 0 ? 'myt__pnl--positive' : 'myt__pnl--negative';
            const prefix = netPl >= 0 ? '+' : '';
            summary.innerHTML = `Net P/L: <strong class="myt__pnl ${cls}">${prefix}${formatUSD(Math.abs(netPl))}</strong>`;
        }

        if (MOCK_TRADES.length === 0) {
            tbody.innerHTML = `<tr><td colspan="9" style="text-align:center; padding: 32px; color: var(--myt-text-sec);">No trade history found.</td></tr>`;
            return;
        }

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

    // ── Render My Assets ─────────────────────────────────────
    function renderMyAssets() {
        const tbody = document.getElementById('my-assets-body');
        const countBadge = document.getElementById('tab-count-assets');
        
        if (!tbody) return;
        
        if (countBadge) {
            countBadge.innerText = PORTFOLIO_ASSETS.length;
        }

        if (PORTFOLIO_ASSETS.length === 0) {
            tbody.innerHTML = `<tr><td colspan="8" style="text-align:center; padding: 32px; color: var(--myt-text-sec);">No purchased assets found.</td></tr>`;
            return;
        }

        tbody.innerHTML = PORTFOLIO_ASSETS.map(a => {
            const pl = a.current_value_cents - a.purchase_value_cents;
            const cls = pl >= 0 ? 'myt__pnl--positive' : 'myt__pnl--negative';
            const prefix = pl >= 0 ? '+' : '';
            const plHtml = `<span class="myt__pnl ${cls}">${prefix}${formatUSD(Math.abs(pl))}</span>`;
            
            const yieldPct = (a.appreciation_pct_bps / 100).toFixed(2);
            const yieldCls = yieldPct >= 0 ? 'myt__pnl--positive' : 'myt__pnl--negative';
            
            return `
                <tr>
                    <td style="font-weight:500;">
                        <a href="/marketplace-trading-v3?asset=${a.asset_slug}" style="text-decoration:none; color:inherit; display:flex; align-items:center; gap:8px;">
                            ${a.cover_image ? `<img src="${a.cover_image}" alt="cover" style="width:32px; height:32px; border-radius:4px; object-fit:cover;">` : `<div style="width:32px; height:32px; border-radius:4px; background:var(--myt-bg-tertiary);"></div>`}
                            ${a.asset_title}
                        </a>
                    </td>
                    <td>${a.tokens_owned}</td>
                    <td>${formatUSD(a.purchase_value_cents)}</td>
                    <td>${formatUSD(a.current_value_cents)}</td>
                    <td>${plHtml}</td>
                    <td><span class="${yieldCls}">${yieldPct}%</span></td>
                    <td><span class="myt__status myt__status--${a.status.toLowerCase()}">${a.status}</span></td>
                    <td>
                        ${(a.funding_status === 'funded' || currentUserEmail === 'support@traffic-creator.com') 
                            ? `<a href="/marketplace-trading-v3?asset=${a.asset_slug}" class="myt__action-btn myt__action-btn--outline" style="padding: 4px 8px; font-size: 12px; height:auto;">Trade</a>`
                            : `<span class="myt__action-btn myt__action-btn--outline" style="padding: 4px 8px; font-size: 11px; height:auto; opacity:0.5; cursor:not-allowed; background:#f2f4f7; border-color:#d0d5dd; color:#667085;" title="This asset is currently in its primary funding phase and cannot yet be traded.">Not Tradable</span>`
                        }
                    </td>
                </tr>
            `;
        }).join('');
    }

    // ── Render P2P Offers ──────────────────────────────────────
    function renderP2POffers() {
        const incoming = document.getElementById('p2p-incoming');
        const outgoing = document.getElementById('p2p-outgoing');
        const countBadge = document.getElementById('tab-count-p2p');

        const pendingIncoming = MOCK_P2P_INCOMING.filter(p => p.status === 'pending');
        if (countBadge) countBadge.innerText = pendingIncoming.length;

        if (incoming) {
            if (MOCK_P2P_INCOMING.length === 0) {
                incoming.innerHTML = `<div style="text-align:center; padding: 32px; color: var(--myt-text-sec);">No incoming offers.</div>`;
            } else {
                incoming.innerHTML = MOCK_P2P_INCOMING.map(p => `
                    <div class="myt__p2p-card">
                        <div class="myt__p2p-info">
                            <div class="myt__p2p-title">User ${p.maker_user_id.substring(0,8)} wants to buy from you</div>
                            <div class="myt__p2p-details">
                                    <strong>Asset ${p.asset_id.substring(0,8)}</strong> · ${p.quantity} shares @ ${formatUSD(p.price_cents)} · <span class="myt__status myt__status--${p.status}">${p.status}</span> · <span style="font-size: 11px;">${formatDate(p.created_at)}</span>
                                ${p.message ? '<br><em>"' + p.message + '"</em>' : ''}
                            </div>
                        </div>
                        ${p.status === 'pending' ? `
                        <div class="myt__p2p-actions">
                            <button class="myt__p2p-accept" onclick="respondP2POffer('${p.id}', 'accept')">Accept</button>
                            <button class="myt__p2p-decline" onclick="respondP2POffer('${p.id}', 'decline')">Decline</button>
                        </div>
                        ` : ''}
                    </div>
                `).join('');
            }
        }

        if (outgoing) {
            if (MOCK_P2P_OUTGOING.length === 0) {
                outgoing.innerHTML = `<div style="text-align:center; padding: 32px; color: var(--myt-text-sec);">No sent offers.</div>`;
            } else {
                outgoing.innerHTML = MOCK_P2P_OUTGOING.map(p => `
                    <div class="myt__p2p-card">
                        <div class="myt__p2p-info">
                            <div class="myt__p2p-title">You offered to sell to User ${p.taker_user_id.substring(0,8)}</div>
                            <div class="myt__p2p-details">
                                <strong>Asset ${p.asset_id.substring(0,8)}</strong> · ${p.quantity} shares @ ${formatUSD(p.price_cents)} · <span class="myt__status myt__status--${p.status}">${p.status}</span> · <span style="font-size:11px;">${formatDate(p.created_at)}</span>
                            </div>
                        </div>
                        ${p.status === 'pending' ? `
                        <div class="myt__p2p-actions">
                            <button class="myt__cancel-btn" onclick="cancelP2POffer('${p.id}')">Withdraw</button>
                        </div>
                        ` : ''}
                    </div>
                `).join('');
            }
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
            downloadBtn.addEventListener('click', async () => {
                const year = document.getElementById('tax-year').value;
                const format = document.getElementById('tax-format').value;
                
                downloadBtn.innerHTML = 'Generating...';
                downloadBtn.disabled = true;
                
                try {
                    // PDF: open branded print-ready page in new tab
                    if (format === 'pdf') {
                        window.open(`/tax-report?year=${year}&format=pdf`, '_blank');
                        downloadBtn.innerHTML = '✓ Report Opened';
                        downloadBtn.style.background = '#16a34a';
                        setTimeout(() => {
                            if (modal) modal.style.display = 'none';
                            downloadBtn.innerHTML = `
                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                                Download Report
                            `;
                            downloadBtn.style.background = '';
                            downloadBtn.disabled = false;
                        }, 2000);
                        return;
                    }

                    // CSV: fetch and download directly
                    const response = await fetch(`/api/marketplace/tax-export?year=${year}&format=${format}`);
                    if (!response.ok) {
                        const errText = await response.text();
                        alert('Failed to generate report: ' + errText);
                        throw new Error('Export failed');
                    }
                    
                    const blob = await response.blob();
                    const url = window.URL.createObjectURL(blob);
                    const a = document.createElement('a');
                    a.href = url;
                    a.download = `tax_report_${year}.csv`;
                    document.body.appendChild(a);
                    a.click();
                    document.body.removeChild(a);
                    window.URL.revokeObjectURL(url);
                    
                    downloadBtn.innerHTML = '✓ Report Downloaded';
                    downloadBtn.style.background = '#16a34a';
                    
                    setTimeout(() => {
                        if (modal) modal.style.display = 'none';
                        downloadBtn.innerHTML = `
                            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                            Download Report
                        `;
                        downloadBtn.style.background = '';
                        downloadBtn.disabled = false;
                    }, 2000);
                } catch (err) {
                    console.error('Export Error:', err);
                    downloadBtn.innerHTML = `
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                        Download Report
                    `;
                    downloadBtn.disabled = false;
                }
            });
        }
    }

    function renderSummaryCards() {
        const spanOpen = document.getElementById('summary-open-orders');
        const spanTrades = document.getElementById('summary-trades');
        const spanFees = document.getElementById('summary-fees');
        const spanInterests = document.getElementById('summary-interests');

        if (spanOpen) {
            spanOpen.innerText = MOCK_ORDERS.filter(o => ['open', 'partially_filled', 'pending_review'].includes(o.status)).length;
        }
        if (spanTrades) {
            spanTrades.innerText = MOCK_TRADES.length;
        }
        if (spanFees) {
            const totalFees = MOCK_TRADES.reduce((sum, t) => sum + (t.fee || 0), 0);
            spanFees.innerText = formatUSD(totalFees);
        }
        if (spanInterests) {
            spanInterests.innerText = MOCK_INTERESTS.length;
        }
    }

    // ── Fetching Data ──────────────────────────────────────────
    async function fetchAllData() {
        try {
            const [ordersRes, tradesRes, incomingRes, outgoingRes, portfolioRes, userRes] = await Promise.all([
                fetch('/api/marketplace/orders/mine'),
                fetch('/api/marketplace/trades/mine'),
                fetch('/api/marketplace/p2p/offers/incoming'),
                fetch('/api/marketplace/p2p/offers/outgoing'),
                fetch('/api/portfolio'),
                fetch('/api/me')
            ]);
            
            if (userRes && userRes.ok) {
                const userData = await userRes.json();
                currentUserEmail = userData.email || userData.user?.email;
            }
            
            if (ordersRes.ok) MOCK_ORDERS = await ordersRes.json();
            if (tradesRes.ok) MOCK_TRADES = await tradesRes.json();
            if (incomingRes.ok) MOCK_P2P_INCOMING = await incomingRes.json();
            if (outgoingRes.ok) MOCK_P2P_OUTGOING = await outgoingRes.json();
            if (portfolioRes.ok) {
                const p = await portfolioRes.json();
                PORTFOLIO_ASSETS = p.investments || [];
            }

            renderOpenOrders();
            renderBuyInterests();
            renderTradeHistory();
            renderP2POffers();
            renderMyAssets();
            renderSummaryCards();
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
