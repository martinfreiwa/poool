// frontend/platform/static/js/marketplace-secondary.js
// Secondary Market Overview – All Assets with Badges, Filters, Buy Interest
// Stakeholder Decisions: E3 (Buy Interest), E4 (All Assets Visible)

(function () {
    'use strict';

    // ── Mock Assets (ALL platform assets — even those without active orders) ──
    const MOCK_ASSETS = [
        {
            slug: 'bali-villa-canggu-12',
            name: 'Bali Villa Canggu #12',
            type: 'Villa',
            location: 'Canggu, Bali',
            price: 10500,           // cents — last traded price
            change24h: 2.3,
            volume24h: 1243000,     // cents
            roi: 12.4,             // annual ROI %
            occupancy: 87,          // occupancy rate %
            sellOrders: 3,          // active sell orders
            buyInterest: 1,         // active buy interest / bid orders
            totalSupply: 1000,      // total tokens
            sparkline: generateSparkData(7, 102, 105, true),
        },
        {
            slug: 'vienna-apartment-a3',
            name: 'Vienna City Apartment A3',
            type: 'Apartment',
            location: 'Vienna, Austria',
            price: 8750,
            change24h: -1.1,
            volume24h: 876500,
            roi: 8.2,
            occupancy: 95,
            sellOrders: 2,
            buyInterest: 0,
            totalSupply: 500,
            sparkline: generateSparkData(7, 90, 87.5, false),
        },
        {
            slug: 'dubai-marina-tower-7',
            name: 'Dubai Marina Tower #7',
            type: 'Commercial',
            location: 'Dubai, UAE',
            price: 15200,
            change24h: 0.8,
            volume24h: 2310000,
            roi: 15.1,
            occupancy: 92,
            sellOrders: 5,
            buyInterest: 3,
            totalSupply: 2000,
            sparkline: generateSparkData(7, 150, 152, true),
        },
        {
            slug: 'bali-ubud-retreat-5',
            name: 'Bali Ubud Retreat #5',
            type: 'Villa',
            location: 'Ubud, Bali',
            price: 6200,
            change24h: -0.4,
            volume24h: 540000,
            roi: 9.8,
            occupancy: 72,
            sellOrders: 1,
            buyInterest: 0,
            totalSupply: 400,
            sparkline: generateSparkData(7, 63, 62, false),
        },
        {
            slug: 'lisbon-alfama-loft',
            name: 'Lisbon Alfama Loft',
            type: 'Residential',
            location: 'Lisbon, Portugal',
            price: 9400,
            change24h: 3.7,
            volume24h: 1890000,
            roi: 11.3,
            occupancy: 89,
            sellOrders: 4,
            buyInterest: 2,
            totalSupply: 600,
            sparkline: generateSparkData(7, 90, 94, true),
        },
        {
            slug: 'singapore-shophouse-42',
            name: 'Singapore Shophouse #42',
            type: 'Commercial',
            location: 'Singapore',
            price: 22000,
            change24h: 1.2,
            volume24h: 3120000,
            roi: 7.5,
            occupancy: 100,
            sellOrders: 2,
            buyInterest: 5,
            totalSupply: 1500,
            sparkline: generateSparkData(7, 217, 220, true),
        },
        // === Assets WITHOUT active sell orders (E4 — all assets visible) ===
        {
            slug: 'berlin-mitte-penthouse',
            name: 'Berlin Mitte Penthouse',
            type: 'Residential',
            location: 'Berlin, Germany',
            price: 13500,
            change24h: 0,
            volume24h: 0,
            roi: 6.8,
            occupancy: 100,
            sellOrders: 0,          // No sell orders!
            buyInterest: 2,         // But there IS buy interest
            totalSupply: 800,
            sparkline: generateSparkData(7, 135, 135, true),
        },
        {
            slug: 'tokyo-shibuya-micro',
            name: 'Tokyo Shibuya Micro-Unit',
            type: 'Apartment',
            location: 'Tokyo, Japan',
            price: 4800,
            change24h: 0,
            volume24h: 0,
            roi: 5.2,
            occupancy: 98,
            sellOrders: 0,          // No sell orders!
            buyInterest: 0,         // No buy interest either
            totalSupply: 300,
            sparkline: generateSparkData(7, 48, 48, false),
        },
    ];

    let currentFilter = 'all';

    // Generate 7-day sparkline data points (hourly → ~168 points)
    function generateSparkData(days, startPrice, endPrice, isPositive) {
        const points = days * 24;
        const data = [];
        let price = startPrice;
        const trend = (endPrice - startPrice) / points;
        for (let i = 0; i < points; i++) {
            price += trend + (Math.random() - 0.48) * 0.8;
            data.push(parseFloat(price.toFixed(2)));
        }
        return data;
    }

    function formatUSD(cents) {
        return '$' + (cents / 100).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    function formatVolume(cents) {
        const dollars = cents / 100;
        if (dollars >= 1000000) return '$' + (dollars / 1000000).toFixed(1) + 'M';
        if (dollars >= 1000) return '$' + (dollars / 1000).toFixed(1) + 'K';
        return '$' + dollars.toFixed(0);
    }

    // ── Determine status badge for asset ──
    function getStatusBadge(asset) {
        if (asset.sellOrders > 0 && asset.buyInterest > 0) {
            return `<span class="mp-sec__status-badge mp-sec__status-badge--active">
                ${asset.sellOrders} offer${asset.sellOrders > 1 ? 's' : ''} · ${asset.buyInterest} interest
            </span>`;
        }
        if (asset.sellOrders > 0) {
            return `<span class="mp-sec__status-badge mp-sec__status-badge--offers">
                <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor"><circle cx="5" cy="5" r="4"/></svg>
                ${asset.sellOrders} offer${asset.sellOrders > 1 ? 's' : ''}
            </span>`;
        }
        if (asset.buyInterest > 0) {
            return `<span class="mp-sec__status-badge mp-sec__status-badge--interest">
                <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor"><circle cx="5" cy="5" r="4"/></svg>
                ${asset.buyInterest} buy interest
            </span>`;
        }
        return `<span class="mp-sec__status-badge mp-sec__status-badge--none">No offers</span>`;
    }

    // ── Render Card ──
    function renderCard(asset, index) {
        const isPositive = asset.change24h >= 0;
        const changeClass = isPositive ? 'mp-sec__change--up' : 'mp-sec__change--down';
        const changePrefix = isPositive ? '+' : '';
        const sparkId = `sparkline-${asset.slug}`;
        const hasOffers = asset.sellOrders > 0;

        const card = document.createElement('div');
        card.className = 'mp-sec__card';
        if (!hasOffers) card.classList.add('mp-sec__card--no-offers');
        card.dataset.name = asset.name.toLowerCase();
        card.dataset.price = asset.price;
        card.dataset.volume = asset.volume24h;
        card.dataset.change = asset.change24h;
        card.dataset.roi = asset.roi;
        card.dataset.sellOrders = asset.sellOrders;
        card.dataset.buyInterest = asset.buyInterest;

        // Footer CTA differs based on whether there are sell orders
        const footerCTA = hasOffers
            ? `<a href="/marketplace-trading-v2?asset=${asset.slug}" class="mp-sec__trade-btn" onclick="event.stopPropagation();">
                    Trade
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5l7 7-7 7"/></svg>
               </a>`
            : `<button class="mp-sec__interest-btn" data-asset-slug="${asset.slug}" data-asset-name="${asset.name}" onclick="event.stopPropagation();">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 14l-7 7-7-7"/><path d="M12 3v18"/></svg>
                    Buy Interest
               </button>`;

        card.innerHTML = `
            <div class="mp-sec__card-body">
                <div class="mp-sec__card-top">
                    <div class="mp-sec__card-top-left">
                        <span class="mp-sec__asset-name">${asset.name}</span>
                        <span class="mp-sec__asset-location">
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0118 0z"/><circle cx="12" cy="10" r="3"/></svg>
                            ${asset.location}
                        </span>
                    </div>
                    <span class="mp-sec__asset-type">${asset.type}</span>
                </div>
                ${getStatusBadge(asset)}
                <div class="mp-sec__price-row">
                    <span class="mp-sec__price">${formatUSD(asset.price)}</span>
                    ${asset.change24h !== 0
                        ? `<span class="mp-sec__change ${changeClass}">${changePrefix}${asset.change24h.toFixed(1)}%</span>`
                        : '<span class="mp-sec__change mp-sec__change--neutral">—</span>'
                    }
                </div>
                <div class="mp-sec__sparkline" id="${sparkId}"></div>
                <div class="mp-sec__metrics">
                    <div class="mp-sec__metric">
                        <span class="mp-sec__metric-label">ROI</span>
                        <span class="mp-sec__metric-value mp-sec__metric-value--${asset.roi >= 10 ? 'high' : 'normal'}">${asset.roi}%</span>
                    </div>
                    <div class="mp-sec__metric">
                        <span class="mp-sec__metric-label">Occupancy</span>
                        <span class="mp-sec__metric-value">${asset.occupancy}%</span>
                    </div>
                    <div class="mp-sec__metric">
                        <span class="mp-sec__metric-label">Supply</span>
                        <span class="mp-sec__metric-value">${asset.totalSupply.toLocaleString()}</span>
                    </div>
                </div>
            </div>
            <div class="mp-sec__card-footer">
                <span class="mp-sec__volume">Vol: <span>${formatVolume(asset.volume24h)}</span></span>
                ${footerCTA}
            </div>
        `;

        card.addEventListener('click', () => {
            if (hasOffers) {
                window.location.href = `/marketplace-trading-v2?asset=${asset.slug}`;
            } else {
                openBuyInterestModal(asset);
            }
        });

        return { card, sparkId, sparkData: asset.sparkline, isPositive };
    }

    // ── Build Sparkline ──
    function buildSparkline(elId, data, isPositive) {
        const color = isPositive ? '#16a34a' : '#dc2626';
        const options = {
            chart: {
                type: 'area',
                sparkline: { enabled: true },
                height: 50,
                animations: { enabled: true, easing: 'easeinout', speed: 600 },
                background: 'transparent',
                fontFamily: "'TT Norms Pro', sans-serif",
            },
            series: [{ data: data }],
            stroke: { width: 1.8, curve: 'smooth', colors: [color] },
            colors: [color],
            fill: {
                type: 'gradient',
                gradient: {
                    shadeIntensity: 1,
                    opacityFrom: 0.35,
                    opacityTo: 0,
                    stops: [0, 100],
                },
            },
            tooltip: { enabled: false },
        };

        const el = document.getElementById(elId);
        if (el && typeof ApexCharts !== 'undefined') {
            const chart = new ApexCharts(el, options);
            chart.render();
        }
    }

    // ── Update Filter Counts ──
    function updateFilterCounts(assets) {
        const all = assets.length;
        const withOffers = assets.filter(a => a.sellOrders > 0).length;
        const withInterest = assets.filter(a => a.buyInterest > 0).length;
        const noOffers = assets.filter(a => a.sellOrders === 0).length;

        const setCount = (id, val) => {
            const el = document.getElementById(id);
            if (el) el.textContent = val;
        };

        setCount('filter-count-all', all);
        setCount('filter-count-offers', withOffers);
        setCount('filter-count-interest', withInterest);
        setCount('filter-count-none', noOffers);
    }

    // ── Filter & Sort ──
    function filterAndSort() {
        const grid = document.getElementById('mp-asset-grid');
        const empty = document.getElementById('mp-empty');
        const query = (document.getElementById('mp-search').value || '').toLowerCase().trim();
        const sortBy = document.getElementById('mp-sort').value;

        let assets = [...MOCK_ASSETS];

        // Search filter
        if (query) {
            assets = assets.filter(a =>
                a.name.toLowerCase().includes(query) ||
                a.type.toLowerCase().includes(query) ||
                a.location.toLowerCase().includes(query)
            );
        }

        // Update counts before category filter
        updateFilterCounts(assets);

        // Category filter (E4)
        if (currentFilter === 'offers') {
            assets = assets.filter(a => a.sellOrders > 0);
        } else if (currentFilter === 'interest') {
            assets = assets.filter(a => a.buyInterest > 0);
        } else if (currentFilter === 'no-offers') {
            assets = assets.filter(a => a.sellOrders === 0);
        }
        // 'all' shows everything

        // Sort
        assets.sort((a, b) => {
            if (sortBy === 'price') return b.price - a.price;
            if (sortBy === 'change') return b.change24h - a.change24h;
            if (sortBy === 'roi') return b.roi - a.roi;
            return b.volume24h - a.volume24h; // default: volume
        });

        // Render
        grid.innerHTML = '';
        const sparklines = [];

        if (assets.length === 0) {
            empty.style.display = 'flex';
        } else {
            empty.style.display = 'none';
            assets.forEach((asset, i) => {
                const { card, sparkId, sparkData, isPositive } = renderCard(asset, i);
                grid.appendChild(card);
                sparklines.push({ sparkId, sparkData, isPositive });
            });
        }

        // Render sparklines after DOM insert
        requestAnimationFrame(() => {
            sparklines.forEach(s => buildSparkline(s.sparkId, s.sparkData, s.isPositive));
        });
    }

    // ── Buy Interest Modal (E3) ──
    function openBuyInterestModal(asset) {
        const modal = document.getElementById('buy-interest-modal');
        const assetLabel = document.getElementById('interest-modal-asset');
        if (assetLabel) {
            assetLabel.innerHTML = `for <strong>${asset.name}</strong>`;
        }
        // Pre-fill with last traded price
        const priceInput = document.getElementById('interest-price');
        if (priceInput) priceInput.value = (asset.price / 100).toFixed(2);
        modal.style.display = 'flex';
        modal.dataset.assetSlug = asset.slug;
    }

    function closeBuyInterestModal() {
        const modal = document.getElementById('buy-interest-modal');
        modal.style.display = 'none';
    }

    function updateInterestTotal() {
        const priceInput = document.getElementById('interest-price');
        const qtyInput = document.getElementById('interest-qty');
        const totalEl = document.getElementById('interest-total');
        const price = parseFloat(priceInput?.value) || 0;
        const qty = parseInt(qtyInput?.value) || 0;
        const subtotal = price * qty;
        const fee = subtotal * 0.05; // 5% fee
        const total = subtotal + fee;
        if (totalEl) totalEl.textContent = '$' + total.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    // ── Init ──
    document.addEventListener('DOMContentLoaded', () => {
        filterAndSort();

        // Search + Sort
        document.getElementById('mp-search').addEventListener('input', debounce(filterAndSort, 200));
        document.getElementById('mp-sort').addEventListener('change', filterAndSort);

        // Filter tabs
        document.querySelectorAll('.mp-sec__filter-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                document.querySelectorAll('.mp-sec__filter-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                currentFilter = btn.dataset.filter;
                filterAndSort();
            });
        });

        // Buy Interest modal — delegate
        document.addEventListener('click', (e) => {
            const btn = e.target.closest('.mp-sec__interest-btn');
            if (btn) {
                const slug = btn.dataset.assetSlug;
                const asset = MOCK_ASSETS.find(a => a.slug === slug);
                if (asset) openBuyInterestModal(asset);
            }
        });

        // Modal close
        const closeBtn = document.getElementById('interest-modal-close');
        const cancelBtn = document.getElementById('interest-cancel-btn');
        if (closeBtn) closeBtn.addEventListener('click', closeBuyInterestModal);
        if (cancelBtn) cancelBtn.addEventListener('click', closeBuyInterestModal);

        // Modal overlay click-to-close
        const modal = document.getElementById('buy-interest-modal');
        if (modal) {
            modal.addEventListener('click', (e) => {
                if (e.target === modal) closeBuyInterestModal();
            });
        }

        // Modal total calculation
        const priceInput = document.getElementById('interest-price');
        const qtyInput = document.getElementById('interest-qty');
        if (priceInput) priceInput.addEventListener('input', updateInterestTotal);
        if (qtyInput) qtyInput.addEventListener('input', updateInterestTotal);

        // Submit interest
        const submitBtn = document.getElementById('interest-submit-btn');
        if (submitBtn) {
            submitBtn.addEventListener('click', () => {
                // Mock: Show success toast
                submitBtn.textContent = '✓ Interest Placed — Holders Notified';
                submitBtn.disabled = true;
                submitBtn.style.background = '#16a34a';
                setTimeout(() => {
                    closeBuyInterestModal();
                    submitBtn.innerHTML = `
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 2L11 13"/><path d="M22 2L15 22 11 13 2 9l20-7z"/></svg>
                        Notify Holders & Place Interest
                    `;
                    submitBtn.disabled = false;
                    submitBtn.style.background = '';
                }, 2000);
            });
        }
    });

    function debounce(fn, ms) {
        let timeout;
        return function (...args) {
            clearTimeout(timeout);
            timeout = setTimeout(() => fn.apply(this, args), ms);
        };
    }
})();
