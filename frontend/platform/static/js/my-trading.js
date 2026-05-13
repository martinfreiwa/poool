// frontend/platform/static/js/my-trading.js
// My Trading Dashboard — Open Orders, Buy Interests, My Assets, Trade History.
// Optimized: ARIA tabs + keyboard nav, sortable columns, search/filter/bulk,
// drill-down drawer, skeleton + error states, dynamic tax modal, CSV exports
// per tab, keyboard shortcuts, anomaly badges, last-updated indicator.

(function () {
    'use strict';

    // ── State ───────────────────────────────────────────────────
    const state = {
        orders: [],
        interests: [],
        trades: [],
        assets: [],
        userEmail: null,
        userId: null,
        loaded: { orders: false, interests: false, trades: false, assets: false, summary: false },
        errors: { orders: null, interests: null, trades: null, assets: null, summary: null },
        sort: {
            orders: { key: 'createdAt', dir: 'desc' },
            interests: { key: 'expires', dir: 'asc' },
            assets: { key: 'current_value_cents', dir: 'desc' },
            trades: { key: 'date', dir: 'desc' },
        },
        page: { orders: 1, interests: 1, trades: 1, assets: 1 },
        pageSize: 25,
        filter: {
            orders: { search: '', side: '', status: '' },
            interests: { search: '', status: '' },
            assets: { search: '', status: '' },
            trades: { search: '', side: '', period: '1y', asset: '' },
        },
        selected: new Set(),
        portfolio: null,
        feeBps: Math.round((parseFloat(window.POOOL_FEE_PCT || 5.0)) * 100),
    };
    state.feeBps = isFinite(state.feeBps) ? state.feeBps : 500;

    const TAB_IDS = ['open-orders', 'buy-interests', 'my-assets', 'trade-history'];

    // ── Utils ───────────────────────────────────────────────────
    const $ = (id) => document.getElementById(id);
    const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));

    function escapeHtml(value) {
        if (value == null) return '';
        return String(value)
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;')
            .replace(/'/g, '&#39;');
    }

    function safeUrl(url) {
        if (typeof url !== 'string') return '';
        const trimmed = url.trim().toLowerCase();
        if (trimmed.startsWith('javascript:') || trimmed.startsWith('data:') || trimmed.startsWith('vbscript:')) {
            return '';
        }
        return escapeHtml(url);
    }

    const usdFormatter = new Intl.NumberFormat('en-US', {
        style: 'currency', currency: 'USD', minimumFractionDigits: 2, maximumFractionDigits: 2,
    });

    function formatUSD(cents) {
        if (cents == null || !isFinite(cents)) return '—';
        return usdFormatter.format(cents / 100);
    }

    function formatPL(cents, opts = {}) {
        if (cents == null || !isFinite(cents)) return '<span class="myt__pnl">—</span>';
        const cls = cents >= 0 ? 'myt__pnl--positive' : 'myt__pnl--negative';
        const sign = cents > 0 ? '+' : (cents < 0 ? '−' : '');
        const abs = usdFormatter.format(Math.abs(cents) / 100);
        return `<span class="myt__pnl ${cls}">${sign}${abs}</span>`;
    }

    function formatPctBps(bps) {
        if (bps == null || !isFinite(bps)) return '—';
        const pct = bps / 100;
        const sign = pct > 0 ? '+' : (pct < 0 ? '−' : '');
        return `${sign}${Math.abs(pct).toFixed(2)}%`;
    }

    function formatDate(iso) {
        if (!iso) return '—';
        const d = new Date(iso);
        if (isNaN(d.getTime())) return '—';
        return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
    }

    function formatTime(iso) {
        if (!iso) return '';
        const d = new Date(iso);
        if (isNaN(d.getTime())) return '';
        return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
    }

    function relativeTime(iso) {
        if (!iso) return '';
        const d = new Date(iso);
        if (isNaN(d.getTime())) return '';
        const ms = Date.now() - d.getTime();
        const min = Math.floor(ms / 60000);
        if (min < 1) return 'just now';
        if (min < 60) return `${min}m ago`;
        const hr = Math.floor(min / 60);
        if (hr < 24) return `${hr}h ago`;
        const days = Math.floor(hr / 24);
        if (days < 7) return `${days}d ago`;
        const weeks = Math.floor(days / 7);
        if (weeks < 4) return `${weeks}w ago`;
        const months = Math.floor(days / 30);
        if (months < 12) return `${months}mo ago`;
        return `${Math.floor(days / 365)}y ago`;
    }

    function relativeFuture(iso) {
        if (!iso) return '';
        const d = new Date(iso);
        if (isNaN(d.getTime())) return '';
        const ms = d.getTime() - Date.now();
        if (ms <= 0) return 'expired';
        const min = Math.floor(ms / 60000);
        if (min < 60) return `in ${min}m`;
        const hr = Math.floor(min / 60);
        if (hr < 24) return `in ${hr}h`;
        const days = Math.floor(hr / 24);
        if (days < 30) return `in ${days}d`;
        return `in ${Math.floor(days / 30)}mo`;
    }

    function getCsrfToken() {
        if (typeof window.getCsrfToken === 'function') return window.getCsrfToken();
        const meta = document.querySelector('meta[name="csrf-token"]');
        return meta ? meta.getAttribute('content') : '';
    }

    function authedFetch(url, opts = {}) {
        const headers = Object.assign({}, opts.headers || {});
        if (opts.method && opts.method !== 'GET') {
            headers['X-CSRF-Token'] = getCsrfToken();
        }
        return fetch(url, Object.assign({ credentials: 'same-origin' }, opts, { headers }));
    }

    function debounce(fn, ms) {
        let t;
        return function () {
            clearTimeout(t);
            const args = arguments;
            t = setTimeout(() => fn.apply(null, args), ms);
        };
    }

    // ── Skeleton & error renderers ──────────────────────────────
    function skeletonRows(cols, rows = 5) {
        const cells = Array(cols).fill(0).map(() =>
            `<td><span class="myt__skeleton-cell myt__skeleton-cell--mid"></span></td>`
        ).join('');
        return Array(rows).fill(0).map(() => `<tr>${cells}</tr>`).join('');
    }

    function renderError(tbody, cols, message, retryFn) {
        const id = 'retry-' + Math.random().toString(36).slice(2, 8);
        tbody.innerHTML = `<tr><td colspan="${cols}" style="padding:0;">
            <div class="myt__error-banner" role="alert">
                <span>${escapeHtml(message)}</span>
                <button type="button" id="${id}">Retry</button>
            </div>
        </td></tr>`;
        const btn = $(id);
        if (btn && retryFn) btn.addEventListener('click', retryFn);
    }

    // ── Sorting ─────────────────────────────────────────────────
    function sortRows(rows, key, dir) {
        const factor = dir === 'asc' ? 1 : -1;
        return rows.slice().sort((a, b) => {
            const av = a[key], bv = b[key];
            if (av == null && bv == null) return 0;
            if (av == null) return 1;
            if (bv == null) return -1;
            if (typeof av === 'number' && typeof bv === 'number') return (av - bv) * factor;
            // try date parse
            const ad = Date.parse(av), bd = Date.parse(bv);
            if (!isNaN(ad) && !isNaN(bd)) return (ad - bd) * factor;
            return String(av).localeCompare(String(bv)) * factor;
        });
    }

    function bindSortableHeaders(table, tabKey, render) {
        const ths = $$('thead th[data-sort]', table);
        function paint() {
            const cur = state.sort[tabKey];
            ths.forEach(th => {
                const k = th.dataset.sort;
                th.setAttribute('aria-sort', k === cur.key ? (cur.dir === 'asc' ? 'ascending' : 'descending') : 'none');
            });
        }
        ths.forEach(th => {
            th.addEventListener('click', () => {
                const k = th.dataset.sort;
                const cur = state.sort[tabKey];
                if (cur.key === k) {
                    cur.dir = cur.dir === 'asc' ? 'desc' : 'asc';
                } else {
                    cur.key = k;
                    cur.dir = th.dataset.sortDefault || 'asc';
                }
                paint();
                render();
            });
        });
        paint();
    }

    // ── Pagination footer ───────────────────────────────────────
    function paginate(rows, tabKey) {
        const total = rows.length;
        const size = state.pageSize;
        const pages = Math.max(1, Math.ceil(total / size));
        if (state.page[tabKey] > pages) state.page[tabKey] = pages;
        const start = (state.page[tabKey] - 1) * size;
        return { slice: rows.slice(start, start + size), total, pages, page: state.page[tabKey] };
    }

    function paginationFooter(tabKey, info, render) {
        if (info.total <= state.pageSize) return '';
        return `<tr><td colspan="99" style="padding:0;">
            <div class="myt__pagination">
                <span>Showing ${(info.page - 1) * state.pageSize + 1}–${Math.min(info.page * state.pageSize, info.total)} of ${info.total}</span>
                <span>
                    <button type="button" data-page="prev" ${info.page === 1 ? 'disabled' : ''}>Prev</button>
                    Page ${info.page} / ${info.pages}
                    <button type="button" data-page="next" ${info.page === info.pages ? 'disabled' : ''}>Next</button>
                </span>
            </div>
        </td></tr>`;
    }

    function bindPaginationFooter(tbody, tabKey, render) {
        $$('button[data-page]', tbody).forEach(btn => {
            btn.addEventListener('click', () => {
                if (btn.dataset.page === 'prev') state.page[tabKey] = Math.max(1, state.page[tabKey] - 1);
                else state.page[tabKey] += 1;
                render();
            });
        });
    }

    // ── Empty states ────────────────────────────────────────────
    function emptyState(cols, opts) {
        const reset = opts.onReset
            ? `<button type="button" class="myt__empty-reset" data-action="reset-filters">Reset filters</button>`
            : (opts.cta
                ? `<a href="${escapeHtml(opts.cta.href)}" class="myt__empty-cta ds-btn ds-btn--primary">${escapeHtml(opts.cta.label)}</a>`
                : '');
        return `<tr><td colspan="${cols}" style="padding: 0;">
            <div class="ds-table-empty myt-branded-empty">
                <img src="/static/images/logos/logo-blue.svg" alt="POOOL" class="myt-branded-empty__logo">
                <h3 class="ds-table-empty__title">${escapeHtml(opts.title)}</h3>
                <p class="ds-table-empty__description">${escapeHtml(opts.description)}</p>
                ${reset}
            </div>
        </td></tr>`;
    }

    // ── Open Orders ─────────────────────────────────────────────
    function getFilteredOrders() {
        const f = state.filter.orders;
        const q = f.search.trim().toLowerCase();
        return state.orders.filter(o => {
            if (q && !(o.asset || '').toLowerCase().includes(q)) return false;
            if (f.side && o.side !== f.side) return false;
            if (f.status && o.status !== f.status) return false;
            return true;
        });
    }

    function renderOpenOrders() {
        const tbody = $('open-orders-body');
        const countBadge = $('tab-count-orders');
        if (!tbody) return;

        if (state.errors.orders) {
            if (countBadge) countBadge.textContent = '!';
            renderError(tbody, 11, state.errors.orders, () => fetchOrders());
            return;
        }
        if (!state.loaded.orders) {
            tbody.innerHTML = skeletonRows(11);
            return;
        }

        if (countBadge) countBadge.textContent = state.orders.length;

        const filtered = getFilteredOrders();
        const sorted = sortRows(filtered, state.sort.orders.key, state.sort.orders.dir);
        const info = paginate(sorted, 'orders');

        if (state.orders.length === 0) {
            tbody.innerHTML = emptyState(11, {
                title: 'No open orders',
                description: 'Place buy or sell orders once you own assets, then track fills, fees, and activity here.',
                cta: { href: '/marketplace-secondary', label: 'Go to Resale Market' },
            });
            return;
        }
        if (sorted.length === 0) {
            tbody.innerHTML = emptyState(11, {
                title: 'No matching orders',
                description: 'Adjust filters to see more results.',
                onReset: true,
            });
            bindResetFilters(tbody, 'orders', renderOpenOrders);
            return;
        }

        const rows = info.slice.map(o => {
            const id = escapeHtml(o.id || '');
            const idShort = id.substring(0, 8);
            const ageDays = o.createdAt ? Math.floor((Date.now() - Date.parse(o.createdAt)) / 86400000) : null;
            let ageBadge = '';
            if (ageDays != null && ageDays >= 14) ageBadge = `<span class="myt__age myt__age--danger" title="Order is ${ageDays} days old">${ageDays}d</span>`;
            else if (ageDays != null && ageDays >= 7) ageBadge = `<span class="myt__age myt__age--warn" title="Order is ${ageDays} days old">${ageDays}d</span>`;
            const checked = state.selected.has(o.id) ? 'checked' : '';
            return `<tr data-row-id="${id}" data-row-type="order">
                <td class="myt__row-check"><input type="checkbox" data-select="${id}" ${checked} aria-label="Select order ${idShort}"></td>
                <td class="myt__cell--id">${idShort}</td>
                <td class="myt__cell--asset">${escapeHtml(o.asset || '—')}</td>
                <td><span class="myt__side myt__side--${escapeHtml((o.side || '').toLowerCase())}">${escapeHtml((o.side || '').toUpperCase())}</span></td>
                <td class="myt__cell--num">${formatUSD(o.priceCents)}</td>
                <td class="myt__cell--num">${escapeHtml(o.qty || 0)}</td>
                <td class="myt__cell--num">${escapeHtml(o.filled || 0)}/${escapeHtml(o.qty || 0)}</td>
                <td class="myt__cell--num">${formatUSD(o.fee)}</td>
                <td><span class="myt__status myt__status--${escapeHtml((o.status || '').replace(/\s+/g, '_'))}">${escapeHtml(o.status || '—')}</span></td>
                <td class="myt__cell--meta">${formatDate(o.createdAt)} · ${ageBadge || `<span class="myt__age">${relativeTime(o.createdAt)}</span>`}</td>
                <td><button type="button" class="myt__cancel-btn" data-action="cancel-order" data-id="${id}" aria-label="Cancel order ${idShort}">Cancel</button></td>
            </tr>`;
        }).join('');

        tbody.innerHTML = rows + paginationFooter('orders', info, renderOpenOrders);
        bindPaginationFooter(tbody, 'orders', renderOpenOrders);
    }

    // ── Buy Interests ───────────────────────────────────────────
    function getFilteredInterests() {
        const f = state.filter.interests;
        const q = f.search.trim().toLowerCase();
        return state.interests.filter(bi => {
            if (q && !(bi.asset || '').toLowerCase().includes(q)) return false;
            if (f.status && bi.status !== f.status) return false;
            return true;
        });
    }

    function renderBuyInterests() {
        const tbody = $('buy-interests-body');
        const countBadge = $('tab-count-interests');
        if (!tbody) return;

        if (state.errors.interests) {
            if (countBadge) countBadge.textContent = '!';
            renderError(tbody, 8, state.errors.interests, () => fetchInterests());
            return;
        }
        if (!state.loaded.interests) {
            tbody.innerHTML = skeletonRows(8);
            return;
        }

        if (countBadge) countBadge.textContent = state.interests.length;

        const filtered = getFilteredInterests();
        const sorted = sortRows(filtered, state.sort.interests.key, state.sort.interests.dir);
        const info = paginate(sorted, 'interests');

        if (state.interests.length === 0) {
            tbody.innerHTML = emptyState(8, {
                title: 'No buy interests',
                description: 'Tell holders you want to buy their asset and track responses here.',
                cta: { href: '/marketplace-secondary', label: 'Go to Resale Market' },
            });
            return;
        }
        if (sorted.length === 0) {
            tbody.innerHTML = emptyState(8, {
                title: 'No matching interests',
                description: 'Adjust filters to see more results.',
                onReset: true,
            });
            bindResetFilters(tbody, 'interests', renderBuyInterests);
            return;
        }

        const rows = info.slice.map(bi => {
            const id = escapeHtml(bi.id || '');
            const total = (bi.price || 0) * (bi.qty || 0) + (bi.fee || 0);
            const expSoon = bi.expires && (Date.parse(bi.expires) - Date.now()) < 86400000 && (Date.parse(bi.expires) - Date.now()) > 0;
            const expBadge = expSoon ? `<span class="myt__expiry-soon">${escapeHtml(relativeFuture(bi.expires))}</span>` : `<span class="myt__age">${escapeHtml(formatDate(bi.expires))}</span>`;
            const cancellable = bi.status === 'pending' || bi.status === 'countered';
            return `<tr data-row-id="${id}" data-row-type="interest">
                <td class="myt__cell--asset">${escapeHtml(bi.asset || '—')}</td>
                <td class="myt__cell--num">${formatUSD(bi.price)}</td>
                <td class="myt__cell--num">${escapeHtml(bi.qty || 0)}</td>
                <td class="myt__cell--num">${formatUSD(total)}</td>
                <td class="myt__cell--num">${escapeHtml(bi.holders || 0)}</td>
                <td class="myt__cell--meta">${expBadge}</td>
                <td><span class="myt__status myt__status--${escapeHtml(bi.status || '')}">${escapeHtml(bi.status || '—')}</span></td>
                <td>${cancellable ? `<button type="button" class="myt__cancel-btn" data-action="cancel-interest" data-id="${id}" aria-label="Cancel buy interest for ${escapeHtml(bi.asset || '')}">Cancel</button>` : ''}</td>
            </tr>`;
        }).join('');

        tbody.innerHTML = rows + paginationFooter('interests', info, renderBuyInterests);
        bindPaginationFooter(tbody, 'interests', renderBuyInterests);
    }

    // ── My Assets ───────────────────────────────────────────────
    function getFilteredAssets() {
        const f = state.filter.assets;
        const q = f.search.trim().toLowerCase();
        return state.assets.filter(a => {
            if (q && !(a.asset_title || '').toLowerCase().includes(q)) return false;
            if (f.status && (a.status || '').toLowerCase() !== f.status) return false;
            return true;
        }).map(a => Object.assign({}, a, {
            pl: (a.current_value_cents || 0) - (a.purchase_value_cents || 0),
            yield: a.appreciation_pct_bps || 0,
        }));
    }

    function renderMyAssets() {
        const tbody = $('my-assets-body');
        const countBadge = $('tab-count-assets');
        if (!tbody) return;

        if (state.errors.assets) {
            if (countBadge) countBadge.textContent = '!';
            renderError(tbody, 8, state.errors.assets, () => fetchPortfolio());
            return;
        }
        if (!state.loaded.assets) {
            tbody.innerHTML = skeletonRows(8);
            return;
        }

        if (countBadge) countBadge.textContent = state.assets.length;

        const filtered = getFilteredAssets();
        const sorted = sortRows(filtered, state.sort.assets.key, state.sort.assets.dir);
        const info = paginate(sorted, 'assets');

        if (state.assets.length === 0) {
            tbody.innerHTML = emptyState(8, {
                title: 'No assets',
                description: 'Invest in your first asset to unlock resale-market trading from this page.',
                cta: { href: '/marketplace', label: 'Browse Marketplace' },
            });
            return;
        }
        if (sorted.length === 0) {
            tbody.innerHTML = emptyState(8, {
                title: 'No matching assets',
                description: 'Adjust filters to see more results.',
                onReset: true,
            });
            bindResetFilters(tbody, 'assets', renderMyAssets);
            return;
        }

        const rows = info.slice.map(a => {
            const slug = escapeHtml(a.asset_slug || '');
            const title = escapeHtml(a.asset_title || '—');
            const cover = safeUrl(a.cover_image || '');
            const tradable = (a.funding_status === 'funded');
            const tradeUrl = `/marketplace-trading-v3?asset=${encodeURIComponent(a.asset_slug || '')}`;
            return `<tr data-row-id="${escapeHtml(a.id || '')}" data-row-type="asset" data-asset-slug="${slug}">
                <td class="myt__cell--asset">
                    <a href="${escapeHtml(tradeUrl)}" style="text-decoration:none; color:inherit; display:flex; align-items:center; gap:8px;">
                        ${cover ? `<img src="${cover}" alt="" style="width:32px; height:32px; border-radius:4px; object-fit:cover;">` : `<div style="width:32px; height:32px; border-radius:4px; background:var(--myt-bg-tertiary, #eef0f4);"></div>`}
                        ${title}
                    </a>
                </td>
                <td class="myt__cell--num">${escapeHtml(a.tokens_owned || 0)}</td>
                <td class="myt__cell--num">${formatUSD(a.purchase_value_cents)}</td>
                <td class="myt__cell--num">${formatUSD(a.current_value_cents)}</td>
                <td class="myt__cell--num">${a.nav_token_usd_cents != null ? formatUSD(a.nav_token_usd_cents) : '<span style="color:var(--myt-text-tertiary,#999);">—</span>'}</td>
                <td class="myt__cell--num">${a.market_token_usd_cents != null ? formatUSD(a.market_token_usd_cents) : '<span style="color:var(--myt-text-tertiary,#999);">—</span>'}</td>
                <td class="myt__cell--num">${formatPL(a.pl)}</td>
                <td class="myt__cell--num"><span class="${a.yield >= 0 ? 'myt__pnl--positive' : 'myt__pnl--negative'}">${escapeHtml(formatPctBps(a.yield))}</span></td>
                <td><span class="myt__status myt__status--${escapeHtml((a.status || '').toLowerCase())}">${escapeHtml(a.status || '—')}</span></td>
                <td class="myt__cell--action">
                    ${tradable
                        ? `<a href="${escapeHtml(tradeUrl)}" class="myt__pill myt__pill--trade">Trade</a>`
                        : `<span class="myt__pill myt__pill--locked" title="Asset is in primary funding phase and cannot yet be traded.">Locked</span>`
                    }
                </td>
            </tr>`;
        }).join('');

        tbody.innerHTML = rows + paginationFooter('assets', info, renderMyAssets);
        bindPaginationFooter(tbody, 'assets', renderMyAssets);
    }

    // ── Trade History ───────────────────────────────────────────
    function getFilteredTrades() {
        const f = state.filter.trades;
        const q = f.search.trim().toLowerCase();
        const periodMap = { '30d': 30, '90d': 90, '1y': 365 };
        const days = periodMap[f.period];
        let cutoff = null;
        if (days) cutoff = Date.now() - days * 86400000;
        if (f.period === 'ytd') {
            const now = new Date();
            cutoff = new Date(now.getFullYear(), 0, 1).getTime();
        }
        return state.trades.filter(t => {
            const ts = t.date ? Date.parse(t.date) : NaN;
            if (cutoff != null && (isNaN(ts) || ts < cutoff)) return false;
            if (q && !(t.asset || '').toLowerCase().includes(q)) return false;
            if (f.side && (t.side || '').toLowerCase() !== f.side) return false;
            if (f.asset && t.asset !== f.asset) return false;
            return true;
        });
    }

    function renderTradeHistory() {
        const tbody = $('trade-history-body');
        const summary = $('trade-summary');
        const countBadge = $('tab-count-history');
        if (!tbody) return;

        if (state.errors.trades) {
            if (countBadge) countBadge.textContent = '!';
            renderError(tbody, 9, state.errors.trades, () => fetchTrades());
            return;
        }
        if (!state.loaded.trades) {
            tbody.innerHTML = skeletonRows(9);
            return;
        }

        if (countBadge) countBadge.textContent = state.trades.length;

        const trades = getFilteredTrades();
        const sorted = sortRows(trades, state.sort.trades.key, state.sort.trades.dir);
        const info = paginate(sorted, 'trades');

        if (summary) {
            const netPl = trades.reduce((sum, t) => sum + (typeof t.pl === 'number' ? t.pl : 0), 0);
            const periodLabel = ({
                '30d': 'Last 30 days', '90d': 'Last 90 days', '1y': 'Last year',
                'ytd': 'Year to date', 'all': 'All time',
            })[state.filter.trades.period] || '';
            summary.innerHTML = `Net P/L${periodLabel ? ` (${escapeHtml(periodLabel)})` : ''}: ${formatPL(netPl)}`;
        }

        if (state.trades.length === 0) {
            tbody.innerHTML = emptyState(9, {
                title: 'No trade history',
                description: 'Completed trades will appear here.',
            });
            return;
        }
        if (sorted.length === 0) {
            tbody.innerHTML = emptyState(9, {
                title: 'No matching trades',
                description: 'Adjust filters to see more results.',
                onReset: true,
            });
            bindResetFilters(tbody, 'trades', renderTradeHistory);
            return;
        }

        const rows = info.slice.map(t => `<tr data-row-id="${escapeHtml(t.id || '')}" data-row-type="trade">
            <td class="myt__cell--meta">${escapeHtml(formatDate(t.date))}</td>
            <td class="myt__cell--asset">${escapeHtml(t.asset || '—')}</td>
            <td><span class="myt__side myt__side--${escapeHtml((t.side || '').toLowerCase())}">${escapeHtml((t.side || '').toUpperCase())}</span></td>
            <td class="myt__cell--num">${formatUSD(t.price)}</td>
            <td class="myt__cell--num">${escapeHtml(t.qty || 0)}</td>
            <td class="myt__cell--num">${formatUSD(t.total)}</td>
            <td class="myt__cell--num myt__cell--meta">${formatUSD(t.fee)}</td>
            <td class="myt__cell--num">${formatUSD(t.net)}</td>
            <td class="myt__cell--num">${formatPL(typeof t.pl === 'number' ? t.pl : null)}</td>
        </tr>`).join('');

        tbody.innerHTML = rows + paginationFooter('trades', info, renderTradeHistory);
        bindPaginationFooter(tbody, 'trades', renderTradeHistory);
    }

    function bindResetFilters(tbody, tabKey, render) {
        const btn = tbody.querySelector('[data-action="reset-filters"]');
        if (!btn) return;
        btn.addEventListener('click', () => {
            const f = state.filter[tabKey];
            Object.keys(f).forEach(k => { f[k] = (k === 'period' ? '1y' : ''); });
            // reset DOM controls
            const map = {
                orders: ['search-orders', 'filter-orders-side', 'filter-orders-status'],
                interests: ['search-interests', 'filter-interests-status'],
                assets: ['search-assets', 'filter-assets-status'],
                trades: ['search-trades', 'trade-filter-asset', 'trade-filter-side', 'trade-filter-period'],
            };
            (map[tabKey] || []).forEach(id => {
                const el = $(id);
                if (el) {
                    if (el.tagName === 'SELECT') el.value = id === 'trade-filter-period' ? '1y' : '';
                    else el.value = '';
                }
            });
            state.page[tabKey] = 1;
            render();
        });
    }

    // ── Hero P/L ────────────────────────────────────────────────
    function renderHero() {
        const p = state.portfolio;
        const v = $('hero-portfolio-value');
        const at = $('hero-all-time');
        const yieldEl = $('hero-yield');
        const monthly = $('hero-monthly');
        if (!p) {
            if (state.errors.summary) {
                if (v) v.textContent = '—';
                return;
            }
            if (v) v.innerHTML = '<span class="myt__skeleton-cell" style="display:inline-block;width:140px;height:30px;"></span>';
            return;
        }
        if (v) v.textContent = formatUSD(p.total_value_cents);
        if (at) at.innerHTML = formatPL(p.total_appreciation_cents);
        if (yieldEl) yieldEl.textContent = formatPctBps(p.annual_yield_bps);
        if (monthly) monthly.textContent = formatUSD(p.monthly_income_cents);
    }

    function renderSummaryCards() {
        const openCount = state.orders.filter(o => ['open', 'partially_filled', 'pending_review'].includes(o.status)).length;
        if ($('summary-open-orders')) $('summary-open-orders').textContent = state.loaded.orders ? openCount : '—';
        if ($('summary-trades')) $('summary-trades').textContent = state.loaded.trades ? state.trades.length : '—';
        if ($('summary-assets')) $('summary-assets').textContent = state.loaded.assets ? state.assets.length : '—';
        if ($('summary-interests')) $('summary-interests').textContent = state.loaded.interests ? state.interests.length : '—';
    }

    function setLastUpdated() {
        const el = $('myt-last-updated');
        if (!el) return;
        const time = new Date().toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
        el.innerHTML = `<span class="myt__live-dot" aria-hidden="true"></span>Last updated ${escapeHtml(time)}`;
    }

    // ── Tabs (ARIA + keyboard) ──────────────────────────────────
    function activateTab(name, focus = false) {
        $$('.myt-card-tab').forEach(t => {
            const active = t.dataset.tab === name;
            t.classList.toggle('active', active);
            t.setAttribute('aria-selected', active ? 'true' : 'false');
            t.setAttribute('tabindex', active ? '0' : '-1');
            if (active && focus) t.focus();
        });
        $$('.myt__tab-content').forEach(c => {
            const active = c.id === 'tab-' + name;
            c.classList.toggle('active', active);
            if (active) c.removeAttribute('hidden'); else c.setAttribute('hidden', '');
        });
        $$('.sub-stat--btn').forEach(b => {
            const active = b.getAttribute('aria-controls') === 'tab-' + name;
            b.classList.toggle('active', active);
        });
    }

    function initTabs() {
        $$('.myt-card-tab').forEach(tab => {
            tab.addEventListener('click', () => activateTab(tab.dataset.tab));
            tab.addEventListener('keydown', (e) => {
                const tabs = $$('.myt-card-tab');
                const i = tabs.indexOf(tab);
                if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
                    e.preventDefault();
                    const next = tabs[(i + 1) % tabs.length];
                    activateTab(next.dataset.tab, true);
                } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
                    e.preventDefault();
                    const prev = tabs[(i - 1 + tabs.length) % tabs.length];
                    activateTab(prev.dataset.tab, true);
                } else if (e.key === 'Home') {
                    e.preventDefault();
                    activateTab(tabs[0].dataset.tab, true);
                } else if (e.key === 'End') {
                    e.preventDefault();
                    activateTab(tabs[tabs.length - 1].dataset.tab, true);
                }
            });
        });
        $$('.sub-stat--btn').forEach(card => {
            card.addEventListener('click', () => {
                const target = card.getAttribute('aria-controls') || '';
                const name = target.replace(/^tab-/, '');
                if (name) activateTab(name);
            });
        });
    }

    // ── Filters wiring ──────────────────────────────────────────
    function wireFilters() {
        const debouncedRender = (fn) => debounce(fn, 200);

        const so = $('search-orders');
        if (so) so.addEventListener('input', debouncedRender(() => { state.filter.orders.search = so.value; state.page.orders = 1; renderOpenOrders(); }));
        const fos = $('filter-orders-side');
        if (fos) fos.addEventListener('change', () => { state.filter.orders.side = fos.value; state.page.orders = 1; renderOpenOrders(); });
        const fost = $('filter-orders-status');
        if (fost) fost.addEventListener('change', () => { state.filter.orders.status = fost.value; state.page.orders = 1; renderOpenOrders(); });

        const si = $('search-interests');
        if (si) si.addEventListener('input', debouncedRender(() => { state.filter.interests.search = si.value; state.page.interests = 1; renderBuyInterests(); }));
        const fis = $('filter-interests-status');
        if (fis) fis.addEventListener('change', () => { state.filter.interests.status = fis.value; state.page.interests = 1; renderBuyInterests(); });

        const sa = $('search-assets');
        if (sa) sa.addEventListener('input', debouncedRender(() => { state.filter.assets.search = sa.value; state.page.assets = 1; renderMyAssets(); }));
        const fas = $('filter-assets-status');
        if (fas) fas.addEventListener('change', () => { state.filter.assets.status = fas.value; state.page.assets = 1; renderMyAssets(); });

        const st = $('search-trades');
        if (st) st.addEventListener('input', debouncedRender(() => { state.filter.trades.search = st.value; state.page.trades = 1; renderTradeHistory(); }));
        const tfa = $('trade-filter-asset');
        if (tfa) tfa.addEventListener('change', () => { state.filter.trades.asset = tfa.value; state.page.trades = 1; renderTradeHistory(); });
        const tfs = $('trade-filter-side');
        if (tfs) tfs.addEventListener('change', () => { state.filter.trades.side = tfs.value; state.page.trades = 1; renderTradeHistory(); });
        const tfp = $('trade-filter-period');
        if (tfp) {
            tfp.value = state.filter.trades.period;
            tfp.addEventListener('change', () => { state.filter.trades.period = tfp.value; state.page.trades = 1; renderTradeHistory(); });
        }
    }

    function populateAssetFilter() {
        const sel = $('trade-filter-asset');
        if (!sel) return;
        const current = sel.value;
        const assets = [...new Set(state.trades.map(t => t.asset).filter(Boolean))].sort();
        sel.innerHTML = '<option value="">All Assets</option>' +
            assets.map(a => `<option value="${escapeHtml(a)}">${escapeHtml(a)}</option>`).join('');
        if (assets.includes(current)) sel.value = current;
    }

    // ── Bulk select & cancel ────────────────────────────────────
    function updateBulkUI() {
        const btn = $('bulk-cancel-orders');
        const cnt = $('bulk-orders-count');
        if (cnt) cnt.textContent = state.selected.size;
        if (btn) btn.disabled = state.selected.size === 0;
        const all = $('select-all-orders');
        if (all) {
            const visibleIds = getFilteredOrders().map(o => o.id);
            const allSel = visibleIds.length > 0 && visibleIds.every(id => state.selected.has(id));
            all.checked = allSel;
            all.indeterminate = !allSel && visibleIds.some(id => state.selected.has(id));
        }
    }

    function wireBulk() {
        const tbody = $('open-orders-body');
        if (tbody) {
            tbody.addEventListener('change', (e) => {
                const cb = e.target.closest('input[data-select]');
                if (!cb) return;
                const id = cb.dataset.select;
                if (cb.checked) state.selected.add(id);
                else state.selected.delete(id);
                updateBulkUI();
            });
        }
        const all = $('select-all-orders');
        if (all) {
            all.addEventListener('change', () => {
                const ids = getFilteredOrders().map(o => o.id);
                ids.forEach(id => {
                    if (all.checked) state.selected.add(id);
                    else state.selected.delete(id);
                });
                renderOpenOrders();
                updateBulkUI();
            });
        }
        const bulkBtn = $('bulk-cancel-orders');
        if (bulkBtn) {
            bulkBtn.addEventListener('click', async () => {
                if (state.selected.size === 0) return;
                if (!confirm(`Cancel ${state.selected.size} order(s)?`)) return;
                bulkBtn.disabled = true;
                const ids = Array.from(state.selected);
                const results = await Promise.allSettled(ids.map(id =>
                    authedFetch(`/api/marketplace/orders/${encodeURIComponent(id)}`, { method: 'DELETE' })
                ));
                const failed = results.filter(r => r.status === 'rejected' || (r.value && !r.value.ok));
                state.selected.clear();
                await fetchOrders();
                if (failed.length > 0) alert(`${failed.length} order(s) could not be cancelled.`);
            });
        }
    }

    // ── Cancel order / interest (delegated) ─────────────────────
    function wireRowActions() {
        document.addEventListener('click', async (e) => {
            const btn = e.target.closest('button[data-action]');
            if (!btn) return;
            const action = btn.dataset.action;
            const id = btn.dataset.id;

            if (action === 'cancel-order' && id) {
                if (btn.disabled) return;
                if (!confirm('Cancel this order?')) return;
                btn.disabled = true;
                try {
                    const r = await authedFetch(`/api/marketplace/orders/${encodeURIComponent(id)}`, { method: 'DELETE' });
                    if (!r.ok) {
                        const text = await r.text();
                        alert('Failed to cancel: ' + text);
                    }
                } catch (err) {
                    alert('Network error cancelling order.');
                } finally {
                    state.selected.delete(id);
                    fetchOrders();
                }
            } else if (action === 'cancel-interest' && id) {
                if (btn.disabled) return;
                if (!confirm('Cancel this buy interest?')) return;
                btn.disabled = true;
                try {
                    const r = await authedFetch(`/api/marketplace/p2p/offers/${encodeURIComponent(id)}`, { method: 'DELETE' });
                    if (!r.ok) {
                        const text = await r.text();
                        alert('Failed to cancel: ' + text);
                    }
                } catch (err) {
                    alert('Network error.');
                } finally {
                    fetchInterests();
                }
            }
        });

        // Row-click → drawer
        document.addEventListener('click', (e) => {
            const row = e.target.closest('tr[data-row-type]');
            if (!row) return;
            // ignore if clicked on interactive child
            if (e.target.closest('button, a, input, select')) return;
            openDrawer(row.dataset.rowType, row.dataset.rowId);
        });
    }

    // ── Drill-down drawer ──────────────────────────────────────
    let lastFocused = null;
    function openDrawer(type, id) {
        const drawer = $('myt-drawer');
        const title = $('myt-drawer-title');
        const body = $('myt-drawer-body');
        if (!drawer || !body) return;

        let item, html = '';
        if (type === 'order') {
            item = state.orders.find(o => o.id === id);
            if (!item) return;
            title.textContent = `Order ${(item.id || '').substring(0, 8)}`;
            html = `
                <div class="myt-drawer__row"><span>Asset</span><strong>${escapeHtml(item.asset || '—')}</strong></div>
                <div class="myt-drawer__row"><span>Side</span><strong><span class="myt__side myt__side--${escapeHtml((item.side||'').toLowerCase())}">${escapeHtml((item.side||'').toUpperCase())}</span></strong></div>
                <div class="myt-drawer__row"><span>Price</span><strong>${formatUSD(item.priceCents)}</strong></div>
                <div class="myt-drawer__row"><span>Quantity</span><strong>${escapeHtml(item.qty || 0)}</strong></div>
                <div class="myt-drawer__row"><span>Filled</span><strong>${escapeHtml(item.filled || 0)} / ${escapeHtml(item.qty || 0)}</strong></div>
                <div class="myt-drawer__row"><span>Status</span><strong>${escapeHtml(item.status || '—')}</strong></div>
                <div class="myt-drawer__row"><span>Fee</span><strong>${formatUSD(item.fee)}</strong></div>
                <div class="myt-drawer__row"><span>Created</span><strong>${escapeHtml(formatDate(item.createdAt))} ${escapeHtml(formatTime(item.createdAt))}</strong></div>
                <div class="myt-drawer__row"><span>Age</span><strong>${escapeHtml(relativeTime(item.createdAt))}</strong></div>
                <h4 class="myt-drawer__section-title">Fee breakdown</h4>
                <div class="myt-drawer__row"><span>Fee rate</span><strong>${(state.feeBps/100).toFixed(2)}%</strong></div>
                <div class="myt-drawer__row"><span>Notional</span><strong>${formatUSD((item.priceCents || 0) * (item.qty || 0))}</strong></div>
            `;
        } else if (type === 'interest') {
            item = state.interests.find(b => b.id === id);
            if (!item) return;
            title.textContent = `Buy Interest`;
            const total = (item.price || 0) * (item.qty || 0) + (item.fee || 0);
            html = `
                <div class="myt-drawer__row"><span>Asset</span><strong>${escapeHtml(item.asset || '—')}</strong></div>
                <div class="myt-drawer__row"><span>Offer Price</span><strong>${formatUSD(item.price)}</strong></div>
                <div class="myt-drawer__row"><span>Quantity</span><strong>${escapeHtml(item.qty || 0)}</strong></div>
                <div class="myt-drawer__row"><span>Fee</span><strong>${formatUSD(item.fee)}</strong></div>
                <div class="myt-drawer__row"><span>Total (incl. fee)</span><strong>${formatUSD(total)}</strong></div>
                <div class="myt-drawer__row"><span>Holders Notified</span><strong>${escapeHtml(item.holders || 0)}</strong></div>
                <div class="myt-drawer__row"><span>Status</span><strong>${escapeHtml(item.status || '—')}</strong></div>
                <div class="myt-drawer__row"><span>Expires</span><strong>${escapeHtml(formatDate(item.expires))} (${escapeHtml(relativeFuture(item.expires))})</strong></div>
                ${item.message ? `<h4 class="myt-drawer__section-title">Message</h4><p style="font-size:13px;line-height:1.5;">${escapeHtml(item.message)}</p>` : ''}
            `;
        } else if (type === 'trade') {
            item = state.trades.find(t => t.id === id);
            if (!item) return;
            title.textContent = 'Trade';
            html = `
                <div class="myt-drawer__row"><span>Asset</span><strong>${escapeHtml(item.asset || '—')}</strong></div>
                <div class="myt-drawer__row"><span>Side</span><strong><span class="myt__side myt__side--${escapeHtml((item.side||'').toLowerCase())}">${escapeHtml((item.side||'').toUpperCase())}</span></strong></div>
                <div class="myt-drawer__row"><span>Price</span><strong>${formatUSD(item.price)}</strong></div>
                <div class="myt-drawer__row"><span>Quantity</span><strong>${escapeHtml(item.qty || 0)}</strong></div>
                <div class="myt-drawer__row"><span>Total</span><strong>${formatUSD(item.total)}</strong></div>
                <div class="myt-drawer__row"><span>Fee</span><strong>${formatUSD(item.fee)}</strong></div>
                <div class="myt-drawer__row"><span>Net</span><strong>${formatUSD(item.net)}</strong></div>
                <div class="myt-drawer__row"><span>P/L</span><strong>${formatPL(typeof item.pl === 'number' ? item.pl : null)}</strong></div>
                <div class="myt-drawer__row"><span>Date</span><strong>${escapeHtml(formatDate(item.date))} ${escapeHtml(formatTime(item.date))}</strong></div>
                <h4 class="myt-drawer__section-title">Settlement</h4>
                ${item.tx_hash
                    ? `<div class="myt-drawer__row"><span>Tx hash</span><strong class="myt-drawer__tx">${escapeHtml(item.tx_hash.slice(0, 10))}…</strong></div>`
                    : `<div class="myt-drawer__row"><span>On-chain</span><strong>Off-chain settled</strong></div>`}
            `;
        } else if (type === 'asset') {
            item = state.assets.find(a => a.id === id);
            if (!item) return;
            title.textContent = escapeHtml(item.asset_title || 'Asset');
            const pl = (item.current_value_cents || 0) - (item.purchase_value_cents || 0);
            html = `
                <div class="myt-drawer__row"><span>Tokens owned</span><strong>${escapeHtml(item.tokens_owned || 0)}</strong></div>
                <div class="myt-drawer__row"><span>Total invested</span><strong>${formatUSD(item.purchase_value_cents)}</strong></div>
                <div class="myt-drawer__row"><span>Current value</span><strong>${formatUSD(item.current_value_cents)}</strong></div>
                <div class="myt-drawer__row"><span>Unrealized P/L</span><strong>${formatPL(pl)}</strong></div>
                <div class="myt-drawer__row"><span>Yield</span><strong>${escapeHtml(formatPctBps(item.appreciation_pct_bps))}</strong></div>
                <div class="myt-drawer__row"><span>Status</span><strong>${escapeHtml(item.status || '—')}</strong></div>
                <div class="myt-drawer__row"><span>Funding</span><strong>${escapeHtml(item.funding_status || '—')}</strong></div>
                <div class="myt-drawer__row"><span>Total rental income</span><strong>${formatUSD(item.total_rental_cents)}</strong></div>
                <div class="myt-drawer__row"><span>Purchased</span><strong>${escapeHtml(formatDate(item.purchased_at))}</strong></div>
                ${item.chain_tx_hash ? `<h4 class="myt-drawer__section-title">On-chain</h4><div class="myt-drawer__row"><span>Tx hash</span><strong class="myt-drawer__tx">${escapeHtml(item.chain_tx_hash.slice(0, 10))}…</strong></div>` : ''}
            `;
        } else {
            return;
        }

        body.innerHTML = html;
        lastFocused = document.activeElement;
        drawer.setAttribute('aria-hidden', 'false');
        const closeBtn = drawer.querySelector('.myt-drawer__close');
        if (closeBtn) closeBtn.focus();
    }

    function closeDrawer() {
        const drawer = $('myt-drawer');
        if (!drawer) return;
        drawer.setAttribute('aria-hidden', 'true');
        if (lastFocused && lastFocused.focus) lastFocused.focus();
    }

    function wireDrawer() {
        document.addEventListener('click', (e) => {
            if (e.target.closest('[data-drawer-close]')) closeDrawer();
        });
    }

    // ── Modals (tax + shortcuts) ────────────────────────────────
    function openModal(modalEl) {
        if (!modalEl) return;
        modalEl.classList.add('active');
        modalEl.removeAttribute('aria-hidden');
        const focusable = modalEl.querySelector('button, input, select, textarea, [tabindex]');
        if (focusable) focusable.focus();
    }

    function closeModal(modalEl) {
        if (!modalEl) return;
        modalEl.classList.remove('active');
        modalEl.setAttribute('aria-hidden', 'true');
    }

    function trapFocus(modalEl, e) {
        if (e.key !== 'Tab') return;
        const focusable = $$('button, input, select, textarea, [tabindex]:not([tabindex="-1"])', modalEl)
            .filter(el => !el.hasAttribute('disabled') && el.offsetParent !== null);
        if (focusable.length === 0) return;
        const first = focusable[0], last = focusable[focusable.length - 1];
        if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
        else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
    }

    // ── Tax Export ──────────────────────────────────────────────
    function populateTaxYearSelect() {
        const sel = $('tax-year');
        if (!sel) return;
        const years = new Set();
        const currentYear = new Date().getFullYear();
        years.add(currentYear);
        years.add(currentYear - 1);
        state.trades.forEach(t => {
            const y = new Date(t.date).getFullYear();
            if (!isNaN(y)) years.add(y);
        });
        const sorted = [...years].sort((a, b) => b - a);
        const cur = sel.value;
        sel.innerHTML = sorted.map(y => `<option value="${y}">${y}</option>`).join('');
        if (sorted.includes(parseInt(cur, 10))) sel.value = cur;
    }

    function updateTaxPreview() {
        const yearEl = $('tax-year');
        if (!yearEl) return;
        const year = parseInt(yearEl.value, 10);
        if (!year) return;
        const yearTrades = state.trades.filter(t => {
            const d = new Date(t.date);
            return !isNaN(d.getTime()) && d.getFullYear() === year;
        });
        const buyVol = yearTrades.filter(t => (t.side || '').toLowerCase() === 'buy').reduce((s, t) => s + (t.total || 0), 0);
        const sellVol = yearTrades.filter(t => (t.side || '').toLowerCase() === 'sell').reduce((s, t) => s + (t.total || 0), 0);
        const realizedPL = yearTrades.reduce((s, t) => s + (typeof t.pl === 'number' ? t.pl : 0), 0);
        const fees = yearTrades.reduce((s, t) => s + (t.fee || 0), 0);

        $('tax-preview-trades').textContent = String(yearTrades.length);
        $('tax-preview-buy').textContent = formatUSD(buyVol);
        $('tax-preview-sell').textContent = formatUSD(sellVol);
        $('tax-preview-pl').innerHTML = formatPL(realizedPL);
        $('tax-preview-fees').textContent = formatUSD(fees);
    }

    function initTaxExport() {
        const triggerBtn = $('btn-export-tax');
        const modal = $('tax-export-modal');
        const closeBtn = $('tax-modal-close');
        const downloadBtn = $('tax-download-btn');
        const yearEl = $('tax-year');

        if (triggerBtn && modal) {
            triggerBtn.addEventListener('click', () => {
                populateTaxYearSelect();
                updateTaxPreview();
                openModal(modal);
            });
        }
        if (closeBtn && modal) closeBtn.addEventListener('click', () => closeModal(modal));
        if (modal) {
            modal.addEventListener('click', (e) => { if (e.target === modal) closeModal(modal); });
            modal.addEventListener('keydown', (e) => {
                if (e.key === 'Escape') closeModal(modal);
                else trapFocus(modal, e);
            });
        }
        if (yearEl) yearEl.addEventListener('change', updateTaxPreview);

        if (downloadBtn) {
            const restore = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg> Download Report`;
            downloadBtn.addEventListener('click', async () => {
                const year = $('tax-year').value;
                const format = $('tax-format').value;
                downloadBtn.disabled = true;
                downloadBtn.innerHTML = 'Generating…';
                try {
                    if (format === 'pdf') {
                        window.open(`/tax-report?year=${encodeURIComponent(year)}&format=pdf`, '_blank', 'noopener');
                        downloadBtn.innerHTML = '✓ Report Opened';
                        downloadBtn.style.background = '#16a34a';
                        setTimeout(() => { closeModal(modal); downloadBtn.innerHTML = restore; downloadBtn.style.background = ''; downloadBtn.disabled = false; }, 1600);
                        return;
                    }
                    const r = await authedFetch(`/api/marketplace/tax-export?year=${encodeURIComponent(year)}&format=${encodeURIComponent(format)}`);
                    if (!r.ok) {
                        const errText = await r.text();
                        throw new Error(errText || 'Export failed');
                    }
                    const blob = await r.blob();
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement('a');
                    a.href = url;
                    a.download = `tax_report_${year}.csv`;
                    document.body.appendChild(a);
                    a.click();
                    document.body.removeChild(a);
                    URL.revokeObjectURL(url);
                    downloadBtn.innerHTML = '✓ Report Downloaded';
                    downloadBtn.style.background = '#16a34a';
                    setTimeout(() => { closeModal(modal); downloadBtn.innerHTML = restore; downloadBtn.style.background = ''; downloadBtn.disabled = false; }, 1600);
                } catch (err) {
                    console.error('Tax export failed', err);
                    alert('Failed to generate report: ' + err.message);
                    downloadBtn.innerHTML = restore;
                    downloadBtn.disabled = false;
                }
            });
        }
    }

    // ── Generic CSV export (per tab) ────────────────────────────
    function csvEscape(value) {
        if (value == null) return '';
        const s = String(value);
        return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
    }
    function downloadCSV(filename, header, rows) {
        const csv = [header.map(csvEscape).join(','), ...rows.map(r => r.map(csvEscape).join(','))].join('\n');
        const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }
    function exportOrders() {
        downloadCSV('open_orders.csv',
            ['ID', 'Asset', 'Side', 'Price', 'Qty', 'Filled', 'Fee', 'Status', 'Created'],
            getFilteredOrders().map(o => [o.id, o.asset, (o.side||'').toUpperCase(), (o.priceCents||0)/100, o.qty, o.filled, (o.fee||0)/100, o.status, o.createdAt]));
    }
    function exportInterests() {
        downloadCSV('buy_interests.csv',
            ['Asset', 'Price', 'Qty', 'Fee', 'Holders Notified', 'Expires', 'Status'],
            getFilteredInterests().map(b => [b.asset, (b.price||0)/100, b.qty, (b.fee||0)/100, b.holders, b.expires, b.status]));
    }
    function exportAssets() {
        downloadCSV('my_assets.csv',
            ['Asset', 'Slug', 'Tokens Owned', 'Total Invested', 'Current Value', 'P/L', 'Yield %', 'Status'],
            getFilteredAssets().map(a => [a.asset_title, a.asset_slug, a.tokens_owned, (a.purchase_value_cents||0)/100, (a.current_value_cents||0)/100, (a.pl||0)/100, (a.yield||0)/100, a.status]));
    }
    function exportTrades() {
        downloadCSV('trades.csv',
            ['Date', 'Asset', 'Side', 'Price', 'Qty', 'Total', 'Fee', 'Net', 'P/L'],
            getFilteredTrades().map(t => [t.date, t.asset, (t.side||'').toUpperCase(), (t.price||0)/100, t.qty, (t.total||0)/100, (t.fee||0)/100, (t.net||0)/100, (t.pl||0)/100]));
    }
    function wireExports() {
        const map = {
            'export-orders-btn': exportOrders,
            'export-interests-btn': exportInterests,
            'export-assets-btn': exportAssets,
            'export-trades-btn': exportTrades,
        };
        Object.entries(map).forEach(([id, fn]) => {
            const el = $(id);
            if (el) el.addEventListener('click', fn);
        });
    }

    // ── Keyboard shortcuts ──────────────────────────────────────
    function getActiveTabName() {
        const t = document.querySelector('.myt-card-tab.active');
        return t ? t.dataset.tab : 'open-orders';
    }

    function focusActiveSearch() {
        const map = {
            'open-orders': 'search-orders', 'buy-interests': 'search-interests',
            'my-assets': 'search-assets', 'trade-history': 'search-trades',
        };
        const el = $(map[getActiveTabName()]);
        if (el) { el.focus(); el.select(); }
    }

    function exportActive() {
        const map = { 'open-orders': exportOrders, 'buy-interests': exportInterests, 'my-assets': exportAssets, 'trade-history': exportTrades };
        const fn = map[getActiveTabName()];
        if (fn) fn();
    }

    function wireShortcuts() {
        const sc = $('myt-shortcuts');
        document.addEventListener('keydown', (e) => {
            // ignore when typing
            if (e.target.matches('input, textarea, select') && e.key !== 'Escape' && e.key !== '?') return;
            if (e.key === '/' && !e.ctrlKey && !e.metaKey) { e.preventDefault(); focusActiveSearch(); }
            else if (e.key >= '1' && e.key <= '4' && !e.ctrlKey && !e.metaKey) {
                const idx = parseInt(e.key, 10) - 1;
                if (TAB_IDS[idx]) { e.preventDefault(); activateTab(TAB_IDS[idx]); }
            }
            else if (e.key === 'r' && !e.ctrlKey && !e.metaKey) { e.preventDefault(); fetchAllData(); }
            else if (e.key === 'e' && !e.ctrlKey && !e.metaKey) { e.preventDefault(); exportActive(); }
            else if (e.key === '?' || (e.shiftKey && e.key === '/')) {
                e.preventDefault();
                if (sc) {
                    if (sc.getAttribute('aria-hidden') === 'false') closeShortcuts();
                    else openShortcuts();
                }
            } else if (e.key === 'Escape') {
                closeDrawer();
                closeShortcuts();
                const tax = $('tax-export-modal');
                if (tax && tax.classList.contains('active')) closeModal(tax);
            }
        });

        if (sc) {
            sc.addEventListener('click', (e) => {
                if (e.target.closest('[data-shortcuts-close]') || e.target.classList.contains('myt-shortcuts__backdrop')) closeShortcuts();
            });
        }
    }

    function openShortcuts() {
        const sc = $('myt-shortcuts');
        if (sc) sc.setAttribute('aria-hidden', 'false');
    }
    function closeShortcuts() {
        const sc = $('myt-shortcuts');
        if (sc) sc.setAttribute('aria-hidden', 'true');
    }

    // ── Fetching ────────────────────────────────────────────────
    async function fetchOrders() {
        state.errors.orders = null;
        try {
            const r = await authedFetch('/api/marketplace/orders/mine');
            if (!r.ok) throw new Error(`HTTP ${r.status}`);
            const data = await r.json();
            state.orders = Array.isArray(data) ? data : (data.orders || []);
            state.loaded.orders = true;
        } catch (err) {
            state.errors.orders = 'Failed to load orders. ' + (err.message || '');
        }
        renderOpenOrders();
        renderSummaryCards();
        updateBulkUI();
    }

    async function fetchTrades() {
        state.errors.trades = null;
        try {
            const r = await authedFetch('/api/marketplace/trades/mine');
            if (!r.ok) throw new Error(`HTTP ${r.status}`);
            const data = await r.json();
            state.trades = Array.isArray(data) ? data : (data.trades || []);
            state.loaded.trades = true;
            populateAssetFilter();
        } catch (err) {
            state.errors.trades = 'Failed to load trade history. ' + (err.message || '');
        }
        renderTradeHistory();
        renderSummaryCards();
    }

    async function fetchInterests() {
        state.errors.interests = null;
        try {
            const r = await authedFetch('/api/marketplace/p2p/offers/outgoing');
            if (r.status === 404) {
                state.interests = [];
                state.loaded.interests = true;
            } else if (!r.ok) {
                throw new Error(`HTTP ${r.status}`);
            } else {
                const data = await r.json();
                const list = Array.isArray(data) ? data : (data.offers || []);
                state.interests = list.map(o => ({
                    id: o.id,
                    asset: o.asset_title || o.asset || o.asset_id,
                    price: o.price_cents,
                    qty: o.quantity,
                    fee: o.fee_cents || 0,
                    holders: o.holders_notified || 0,
                    expires: o.expires_at,
                    status: o.status,
                    message: o.message,
                }));
                state.loaded.interests = true;
            }
        } catch (err) {
            state.errors.interests = 'Failed to load buy interests. ' + (err.message || '');
        }
        renderBuyInterests();
        renderSummaryCards();
    }

    async function fetchPortfolio() {
        state.errors.assets = null;
        state.errors.summary = null;
        try {
            const [r, navResp] = await Promise.all([
                authedFetch('/api/portfolio'),
                authedFetch('/api/investors/me/positions-nav').catch(() => null),
            ]);
            if (!r.ok) throw new Error(`HTTP ${r.status}`);
            const data = await r.json();
            state.portfolio = data;
            // Villa-Returns A5: merge NAV/Market snapshot into each investment row.
            let navByAsset = new Map();
            if (navResp && navResp.ok) {
                const navList = await navResp.json();
                for (const n of navList) navByAsset.set(n.asset_id, n);
            }
            state.assets = (data.investments || []).map((inv) => {
                const nav = navByAsset.get(inv.asset_id);
                return {
                    ...inv,
                    nav_token_usd_cents: nav ? nav.nav_token_usd_cents : null,
                    market_token_usd_cents: nav ? nav.market_token_usd_cents : null,
                };
            });
            state.loaded.assets = true;
            state.loaded.summary = true;
        } catch (err) {
            state.errors.assets = 'Failed to load assets. ' + (err.message || '');
            state.errors.summary = state.errors.assets;
        }
        renderHero();
        renderMyAssets();
        renderSummaryCards();
    }

    async function fetchUser() {
        try {
            const r = await authedFetch('/api/me');
            if (r.ok) {
                const u = await r.json();
                state.userEmail = u.email || (u.user && u.user.email) || null;
                state.userId = u.id || (u.user && u.user.id) || null;
            }
        } catch { /* non-fatal */ }
    }

    async function fetchAllData() {
        await Promise.all([fetchUser()]);
        await Promise.all([fetchOrders(), fetchTrades(), fetchInterests(), fetchPortfolio()]);
        setLastUpdated();
    }

    // ── Auto-refresh (polling) ─────────────────────────────────
    let refreshTimer = null;
    function startAutoRefresh() {
        stopAutoRefresh();
        refreshTimer = setInterval(() => {
            fetchOrders();
            fetchInterests();
            setLastUpdated();
        }, 30000);
    }
    function stopAutoRefresh() {
        if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = null; }
    }

    // ── Init ────────────────────────────────────────────────────
    document.addEventListener('DOMContentLoaded', () => {
        initTabs();
        wireFilters();
        wireBulk();
        wireRowActions();
        wireDrawer();
        wireExports();
        wireShortcuts();
        initTaxExport();

        // Bind sortable headers per table
        const tables = {
            orders: document.querySelector('table[data-table="orders"]'),
            interests: document.querySelector('table[data-table="interests"]'),
            assets: document.querySelector('table[data-table="assets"]'),
            trades: document.querySelector('table[data-table="trades"]'),
        };
        if (tables.orders) bindSortableHeaders(tables.orders, 'orders', renderOpenOrders);
        if (tables.interests) bindSortableHeaders(tables.interests, 'interests', renderBuyInterests);
        if (tables.assets) bindSortableHeaders(tables.assets, 'assets', renderMyAssets);
        if (tables.trades) bindSortableHeaders(tables.trades, 'trades', renderTradeHistory);

        // Initial skeleton render
        renderOpenOrders(); renderBuyInterests(); renderMyAssets(); renderTradeHistory(); renderHero();

        fetchAllData().then(startAutoRefresh);

        // Pause polling when tab not visible
        document.addEventListener('visibilitychange', () => {
            if (document.hidden) stopAutoRefresh();
            else { fetchAllData(); startAutoRefresh(); }
        });
    });

    // Public hook for legacy callers
    window.cancelOrder = async function (id) {
        if (!confirm('Cancel this order?')) return;
        try {
            const r = await authedFetch(`/api/marketplace/orders/${encodeURIComponent(id)}`, { method: 'DELETE' });
            if (!r.ok) {
                const text = await r.text();
                alert('Failed to cancel: ' + text);
            }
        } catch { /* swallow */ }
        fetchOrders();
    };
})();
