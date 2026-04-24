// frontend/platform/static/js/my-trading.js
// My Trading Dashboard — Open Orders, Buy Interests, Trade History, P2P
// Implements Jonas requirements: Dashboard Views, Tax Export

(function () {
    'use strict';

    let MOCK_ORDERS = [];
    let MOCK_INTERESTS = [];
    let MOCK_TRADES = [];

    let PORTFOLIO_ASSETS = [];
    let currentUserEmail = null;

    // Trade History filters
    let TRADE_FILTER_ASSET = '';
    let TRADE_FILTER_PERIOD = '30d';

    function getFilteredTrades() {
        const periods = { '30d': 30, '90d': 90, '1y': 365 };
        const days = periods[TRADE_FILTER_PERIOD];
        const cutoff = days ? Date.now() - days * 86400000 : null;
        return MOCK_TRADES.filter(t => {
            if (TRADE_FILTER_ASSET && t.asset !== TRADE_FILTER_ASSET) return false;
            if (cutoff !== null && new Date(t.date).getTime() < cutoff) return false;
            return true;
        });
    }

    function populateAssetFilter() {
        const sel = document.getElementById('trade-filter-asset');
        if (!sel) return;
        const current = sel.value;
        const assets = [...new Set(MOCK_TRADES.map(t => t.asset))].sort();
        sel.innerHTML = '<option value="">All Assets</option>' +
            assets.map(a => `<option value="${a}">${a}</option>`).join('');
        if (assets.includes(current)) sel.value = current;
    }

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
            tbody.innerHTML = `<tr><td colspan="10" style="padding: 0;">
                <div class="ds-table-empty">
                    <div class="ds-table-empty__icon" style="color: #667085;">
                        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
                            <rect width="20" height="14" x="2" y="3" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>
                        </svg>
                    </div>
                    <h3 class="ds-table-empty__title">No open orders</h3>
                    <p class="ds-table-empty__description">Place a buy or sell order in the resale market.</p>
                    <a href="/marketplace-secondary" class="myt__empty-cta">Go to Resale Market</a>
                </div>
            </td></tr>`;
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
            tbody.innerHTML = `<tr><td colspan="8" style="padding: 0;">
                <div class="ds-table-empty">
                    <div class="ds-table-empty__icon" style="color: #667085;">
                        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
                            <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="16"/><polyline points="8 12 12 16 16 12"/>
                        </svg>
                    </div>
                    <h3 class="ds-table-empty__title">No buy interests</h3>
                    <p class="ds-table-empty__description">Tell holders you want to buy their asset.</p>
                    <a href="/marketplace-secondary" class="myt__empty-cta">Go to Resale Market</a>
                </div>
            </td></tr>`;
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
        const countBadge = document.getElementById('tab-count-history');
        const filterHeader = document.querySelector('#tab-trade-history .myt__table-header');

        if (!tbody) return;

        // Tab badge = total trades across all time (not filter-specific)
        if (countBadge) countBadge.innerText = MOCK_TRADES.length;

        // Hide the filter/summary row when nothing to filter
        if (filterHeader) filterHeader.style.display = MOCK_TRADES.length === 0 ? 'none' : '';

        const trades = getFilteredTrades();

        if (summary) {
            const netPl = trades.reduce((sum, t) => sum + (t.pl || 0), 0);
            const cls = netPl >= 0 ? 'myt__pnl--positive' : 'myt__pnl--negative';
            const prefix = netPl >= 0 ? '+' : '';
            summary.innerHTML = `Net P/L: <strong class="myt__pnl ${cls}">${prefix}${formatUSD(Math.abs(netPl))}</strong>`;
        }

        if (trades.length === 0) {
            const isFiltered = MOCK_TRADES.length > 0;
            const title = isFiltered ? 'No matching trades' : 'No trade history';
            const desc = isFiltered ? 'Adjust the filters to see more results.' : 'Completed trades appear here.';
            tbody.innerHTML = `<tr><td colspan="9" style="padding: 0;">
                <div class="ds-table-empty">
                    <div class="ds-table-empty__icon" style="color: #667085;">
                        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
                            <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
                        </svg>
                    </div>
                    <h3 class="ds-table-empty__title">${title}</h3>
                    <p class="ds-table-empty__description">${desc}</p>
                </div>
            </td></tr>`;
            return;
        }

        tbody.innerHTML = trades.map(t => {
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
            tbody.innerHTML = `<tr><td colspan="8" style="padding: 0;">
                <div class="ds-table-empty">
                    <div class="ds-table-empty__icon" style="color: #667085;">
                        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path><polyline points="9 22 9 12 15 12 15 22"></polyline>
                        </svg>
                    </div>
                    <h3 class="ds-table-empty__title">No assets</h3>
                    <p class="ds-table-empty__description">Invest in a property to start your portfolio.</p>
                    <a href="/marketplace" class="myt__empty-cta">Browse Marketplace</a>
                </div>
            </td></tr>`;
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



    // ── Tab Switching ──────────────────────────────────────────
    function initTabs() {
        document.querySelectorAll('.myt-card-tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.myt-card-tab').forEach(t => t.classList.remove('active'));
                document.querySelectorAll('.myt__tab-content').forEach(c => c.classList.remove('active'));
                tab.classList.add('active');
                const target = document.getElementById('tab-' + tab.dataset.tab);
                if (target) target.classList.add('active');
                
                // Also visually sync the top sub-stat cards if they exist
                document.querySelectorAll('.sub-stat').forEach(s => s.classList.remove('active'));
                let statId = tab.dataset.tab;
                if (statId === 'trade-history') statId = 'trades';
                if (statId === 'buy-interests') statId = 'interests';
                const subStat = document.querySelector(`.sub-stat[data-stat="${statId}"]`);
                if (subStat) subStat.classList.add('active');
            });
        });

        // Trade History filter change handlers — re-render with new filter
        const assetSel = document.getElementById('trade-filter-asset');
        const periodSel = document.getElementById('trade-filter-period');
        if (assetSel) {
            assetSel.addEventListener('change', (e) => {
                TRADE_FILTER_ASSET = e.target.value;
                renderTradeHistory();
            });
        }
        if (periodSel) {
            periodSel.addEventListener('change', (e) => {
                TRADE_FILTER_PERIOD = e.target.value;
                renderTradeHistory();
            });
        }

        // Make the summary cards clickable (acting as tabs)
        document.querySelectorAll('.sub-stat').forEach(statCard => {
            statCard.addEventListener('click', () => {
                let tabName = statCard.dataset.stat;
                // Mapping stat to tab name
                if (tabName === 'trades') tabName = 'trade-history';
                if (tabName === 'interests') tabName = 'buy-interests';

                const associatedTabBtn = document.querySelector(`.myt-card-tab[data-tab="${tabName}"]`);
                if (associatedTabBtn) {
                    associatedTabBtn.click();
                    associatedTabBtn.scrollIntoView({ behavior: 'smooth', block: 'start' });
                }
            });
            // Indicate they are clickable
            statCard.style.cursor = 'pointer';
        });
    }

    // ── Tax Export Modal ───────────────────────────────────────
    function initTaxExport() {
        const btn = document.getElementById('btn-export-tax');
        const modal = document.getElementById('tax-export-modal');
        const closeBtn = document.getElementById('tax-modal-close');
        const downloadBtn = document.getElementById('tax-download-btn');

        const openModal = () => {
            modal.style.display = 'flex';
            modal.setAttribute('aria-hidden', 'false');
        };
        const closeModal = () => {
            modal.style.display = 'none';
            modal.setAttribute('aria-hidden', 'true');
        };

        if (btn && modal) {
            btn.addEventListener('click', openModal);
        }
        if (closeBtn && modal) {
            closeBtn.addEventListener('click', closeModal);
        }
        if (modal) {
            modal.addEventListener('click', (e) => {
                if (e.target === modal) closeModal();
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
                            if (modal) closeModal();
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
        const spanAssets = document.getElementById('summary-assets');
        const spanInterests = document.getElementById('summary-interests');

        const openCount = MOCK_ORDERS.filter(o => ['open', 'partially_filled', 'pending_review'].includes(o.status)).length;

        if (spanOpen) spanOpen.innerText = openCount;
        if (spanTrades) spanTrades.innerText = MOCK_TRADES.length;
        if (spanAssets) spanAssets.innerText = PORTFOLIO_ASSETS.length;
        if (spanInterests) spanInterests.innerText = MOCK_INTERESTS.length;

        // Hide stat row when user has zero activity — promotes empty-state CTA below
        const total = openCount + MOCK_TRADES.length + PORTFOLIO_ASSETS.length + MOCK_INTERESTS.length;
        const row = document.querySelector('.myt-stats-row');
        if (row) row.style.display = total === 0 ? 'none' : '';
    }

    // ── Fetching Data ──────────────────────────────────────────
    async function fetchAllData() {
        try {
            const [ordersRes, tradesRes, portfolioRes, userRes] = await Promise.all([
                fetch('/api/marketplace/orders/mine'),
                fetch('/api/marketplace/trades/mine'),
                fetch('/api/portfolio'),
                fetch('/api/me')
            ]);
            
            if (userRes && userRes.ok) {
                const userData = await userRes.json();
                currentUserEmail = userData.email || userData.user?.email;
            }
            
            if (ordersRes.ok) MOCK_ORDERS = await ordersRes.json();
            if (tradesRes.ok) MOCK_TRADES = await tradesRes.json();

            if (portfolioRes.ok) {
                const p = await portfolioRes.json();
                PORTFOLIO_ASSETS = p.investments || [];
            }

            renderOpenOrders();
            renderBuyInterests();
            populateAssetFilter();
            renderTradeHistory();

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



    // ── Init ──────────────────────────────────────────────────
    document.addEventListener('DOMContentLoaded', () => {
        fetchAllData();
        initTabs();
        initTaxExport();
    });
})();
