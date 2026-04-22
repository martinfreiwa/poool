// frontend/platform/static/js/marketplace-secondary.js
// Secondary Market Overview — Redesigned with Hero Images
// Matches marketplace card style with toggleable chart

(function () {
    'use strict';

    // ── Assets Data ──
    let MOCK_ASSETS = [];


    let currentFilter = 'all';
    let currentStatus = 'available';

    // Generate 12-month daily data points
    function generateSparkData(days, startPrice, endPrice, isPositive) {
        const data = [];
        let price = startPrice;
        const trend = (endPrice - startPrice) / days;
        for (let i = 0; i < days; i++) {
            price += trend + (Math.random() - 0.48) * 1.2;
            data.push(parseFloat(Math.max(price, 1).toFixed(2)));
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

    // ── Status badge overlay HTML ──
    function getStatusOverlay(asset) {
        if (asset.sellOrders > 0 && asset.buyInterest > 0) {
            return `<span class="mp-sec__card-status-overlay mp-sec__card-status-overlay--active">
                <span class="mp-sec__pulse-dot"></span>
                ${asset.sellOrders} offer${asset.sellOrders > 1 ? 's' : ''} · ${asset.buyInterest} interest
            </span>`;
        }
        if (asset.sellOrders > 0) {
            return `<span class="mp-sec__card-status-overlay mp-sec__card-status-overlay--offers">
                <span class="mp-sec__pulse-dot"></span>
                ${asset.sellOrders} offer${asset.sellOrders > 1 ? 's' : ''}
            </span>`;
        }
        if (asset.buyInterest > 0) {
            return `<span class="mp-sec__card-status-overlay mp-sec__card-status-overlay--interest">
                <span class="mp-sec__pulse-dot"></span>
                ${asset.buyInterest} buy interest
            </span>`;
        }
        return `<span class="mp-sec__card-status-overlay mp-sec__card-status-overlay--none">No offers</span>`;
    }

    // ── Build image gallery HTML ──
    function buildGalleryHTML(asset) {
        let images = asset.images || ['/static/images/seed/villa1.webp'];
        // Limit to 5 images for preview
        images = images.slice(0, 5);
        const hasMultiple = images.length > 1;

        const imagesHTML = images.map((img, i) =>
            `<img src="${img}" class="mp-sec__card-image ${i === 0 ? 'active' : ''}" style="object-fit: cover; object-position: center;" alt="${asset.name}" loading="lazy">`
        ).join('');

        let navHTML = '';
        if (hasMultiple) {
            navHTML = `
                <button class="mp-sec__card-nav mp-sec__card-nav--prev" onclick="event.stopPropagation(); mpSecPrevImage(this)" aria-label="Previous image">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#333" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg>
                </button>
                <button class="mp-sec__card-nav mp-sec__card-nav--next" onclick="event.stopPropagation(); mpSecNextImage(this)" aria-label="Next image">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#333" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
                </button>`;
        }

        const dotsHTML = images.map((_, i) =>
            `<div class="mp-sec__card-dot ${i === 0 ? 'active' : ''}" data-image-index="${i}"></div>`
        ).join('');

        let typeText = 'Standard Leasehold';
        let badgeColorClass = 'ds-badge--leasehold';
        
        if (asset.type && asset.type.toLowerCase() === 'commodity') {
            typeText = 'Agricultural';
            badgeColorClass = 'ds-badge--commodity';
        } else if (asset.leaseType) {
            typeText = asset.leaseType.charAt(0).toUpperCase() + asset.leaseType.slice(1).toLowerCase();
            if (asset.leaseType.toLowerCase() === 'freehold') {
                badgeColorClass = 'ds-badge--freehold';
            }
        }    
        
        const typeBadge = `<div class="property-badge ds-badge ds-badge--overlay ${badgeColorClass}">
            <span class="badge-text">${typeText}</span>
        </div>`;

        return `
            <div class="mp-sec__card-gallery">
                <div class="mp-sec__card-image-container">
                    ${imagesHTML}
                    ${navHTML}
                    <div class="mp-sec__card-dots">${dotsHTML}</div>
                </div>
                ${typeBadge}
            </div>`;
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
        card.dataset.name = asset.name.toLowerCase();
        card.dataset.price = asset.price;
        card.dataset.volume = asset.volume24h;
        card.dataset.change = asset.change24h;
        card.dataset.roi = asset.roi;
        card.dataset.sellOrders = asset.sellOrders;
        card.dataset.buyInterest = asset.buyInterest;

        // No footer button to match normal marketplace style
        card.innerHTML = `
            ${buildGalleryHTML(asset)}
            <div class="mp-sec__card-content">
                <div class="mp-sec__card-meta">
                    ${asset.bedrooms > 0 ? `
                    <div class="mp-sec__card-meta-item">
                        <img src="/static/images/icons/Bed.svg" alt="Beds" width="16" height="16">
                        <span>${asset.bedrooms}</span>
                    </div>
                    <div class="mp-sec__card-meta-divider"></div>
                    ` : ''}
                    <div class="mp-sec__card-meta-item">
                        <img src="/static/images/${asset.country}.webp" onerror="this.style.display='none'" width="16" height="16" style="border-radius:50%;object-fit:cover;flex-shrink:0;" alt="${asset.country}">
                        <span>${asset.location}</span>
                    </div>
                </div>
                <h3 class="mp-sec__card-title">${asset.name}</h3>
                <div class="mp-sec__price-row">
                    <span class="mp-sec__price">${formatUSD(asset.price)}</span>
                    ${asset.change24h !== 0
                        ? `<span class="mp-sec__change ${changeClass}">${changePrefix}${asset.change24h.toFixed(1)}%</span>`
                        : `<span class="mp-sec__change mp-sec__change--neutral">0.0%</span>`
                    }
                </div>
                <div class="mp-sec__details-box">
                    <div class="mp-sec__detail-row">
                        <span class="mp-sec__detail-label">Annual ROI</span>
                        <span class="mp-sec__detail-value ${asset.roi >= 10 ? 'mp-sec__detail-value--green' : ''}">${asset.roi}%</span>
                    </div>
                    <div class="mp-sec__detail-row">
                        <span class="mp-sec__detail-label">Occupancy</span>
                        <span class="mp-sec__detail-value">${asset.occupancy}%</span>
                    </div>
                    <div class="mp-sec__detail-row">
                        <span class="mp-sec__detail-label">Total supply</span>
                        <span class="mp-sec__detail-value">${asset.totalSupply.toLocaleString()} tokens</span>
                    </div>
                </div>
            </div>
            <div class="mp-sec__chart-section" id="chart-section-${asset.slug}">
                <div class="mp-sec__sparkline" id="${sparkId}"></div>
            </div>
            <button class="mp-sec__chart-toggle" data-slug="${asset.slug}" onclick="event.stopPropagation(); mpSecToggleChart('${asset.slug}')">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6"/></svg>
                <span>12m Chart</span>
            </button>
        `;

        card.addEventListener('click', () => {
            const url = `/marketplace-trading-v3?asset=${asset.slug}`;
            // Tag elements for View Transitions API morph
            const gallery = card.querySelector('.mp-sec__card-gallery');
            if (gallery) gallery.style.viewTransitionName = 'tv3-hero-img';
            const title = card.querySelector('.mp-sec__card-title');
            if (title) title.style.viewTransitionName = 'tv3-title';
            window.location.href = url;
        });

        return { card, sparkId, sparkData: asset.sparkline, isPositive };
    }

    // ── Image Gallery Navigation ──
    window.mpSecPrevImage = function (btn) {
        const container = btn.closest('.mp-sec__card-gallery');
        const images = container.querySelectorAll('.mp-sec__card-image');
        const dots = container.querySelectorAll('.mp-sec__card-dot');
        let activeIdx = 0;
        images.forEach((img, i) => { if (img.classList.contains('active')) activeIdx = i; });
        const newIdx = (activeIdx - 1 + images.length) % images.length;
        images.forEach(img => img.classList.remove('active'));
        dots.forEach(dot => dot.classList.remove('active'));
        images[newIdx].classList.add('active');
        if (dots[newIdx]) dots[newIdx].classList.add('active');
    };

    window.mpSecNextImage = function (btn) {
        const container = btn.closest('.mp-sec__card-gallery');
        const images = container.querySelectorAll('.mp-sec__card-image');
        const dots = container.querySelectorAll('.mp-sec__card-dot');
        let activeIdx = 0;
        images.forEach((img, i) => { if (img.classList.contains('active')) activeIdx = i; });
        const newIdx = (activeIdx + 1) % images.length;
        images.forEach(img => img.classList.remove('active'));
        dots.forEach(dot => dot.classList.remove('active'));
        images[newIdx].classList.add('active');
        if (dots[newIdx]) dots[newIdx].classList.add('active');
    };

    // ── Chart Toggle (Per Card) ──
    window.mpSecToggleChart = function (slug) {
        const clickedSection = document.getElementById(`chart-section-${slug}`);
        if (!clickedSection) return;

        // Determine new state from clicked card
        const isCurrentlyHidden = clickedSection.style.display === 'none' || !clickedSection.classList.contains('expanded');

        // Find the specific toggle button for this card
        const toggleBtn = document.querySelector(`.mp-sec__chart-toggle[data-slug="${slug}"]`);

        if (isCurrentlyHidden) {
            // Show this chart
            clickedSection.style.display = 'block';
            clickedSection.classList.add('expanded');

            // Lazy-render chart only on first expand
            if (!clickedSection.dataset.rendered) {
                clickedSection.dataset.rendered = 'true';
                const asset = MOCK_ASSETS.find(a => a.slug === slug);
                if (asset) {
                    const sparkId = `sparkline-${slug}`;
                    const isPositive = asset.change24h >= 0;
                    setTimeout(() => {
                        buildSparkline(sparkId, asset.sparkline, isPositive, asset.price);
                    }, 50);
                }
            }
            if (toggleBtn) {
                toggleBtn.classList.add('expanded');
                toggleBtn.querySelector('span').textContent = 'Hide Chart';
            }
        } else {
            // Hide this chart
            clickedSection.style.display = 'none';
            clickedSection.classList.remove('expanded');
            if (toggleBtn) {
                toggleBtn.classList.remove('expanded');
                toggleBtn.querySelector('span').textContent = '12m Chart';
            }
        }
    };

    // ── Build Chart with Axes ──
    function buildSparkline(elId, data, isPositive, lastPrice) {
        const color = isPositive ? '#16a34a' : '#dc2626';
        // Generate date labels for 12 months (daily data = 365 points)
        const now = new Date();
        const categories = data.map((_, i) => {
            const d = new Date(now.getTime() - (data.length - 1 - i) * 86400000);
            return d;
        });

        const options = {
            chart: {
                type: 'area',
                height: 140,
                sparkline: { enabled: false },
                toolbar: { show: false },
                zoom: { enabled: false },
                animations: { enabled: true, easing: 'easeinout', speed: 600 },
                background: 'transparent',
                fontFamily: "'TT Norms Pro', sans-serif",
                parentHeightOffset: 0,
            },
            series: [{ name: 'Price', data: data }],
            stroke: { width: 2, curve: 'smooth', colors: [color] },
            colors: [color],
            fill: {
                type: 'gradient',
                gradient: {
                    shadeIntensity: 1,
                    opacityFrom: 0.3,
                    opacityTo: 0.05,
                    stops: [0, 100],
                },
            },
            xaxis: {
                type: 'datetime',
                categories: categories.map(d => d.toISOString()),
                labels: {
                    show: true,
                    style: { fontSize: '10px', colors: '#9ca3af', fontFamily: "'TT Norms Pro', sans-serif" },
                    datetimeFormatter: { month: 'MMM', year: 'yyyy' },
                    rotate: 0,
                    maxHeight: 30,
                },
                axisBorder: { show: false },
                axisTicks: { show: false },
                tickAmount: 6,
            },
            yaxis: {
                labels: {
                    show: true,
                    style: { fontSize: '10px', colors: '#9ca3af', fontFamily: "'TT Norms Pro', sans-serif" },
                    formatter: (val) => '$' + val.toFixed(0),
                },
                tickAmount: 3,
            },
            grid: {
                show: true,
                borderColor: '#f0f0f0',
                strokeDashArray: 3,
                xaxis: { lines: { show: false } },
                yaxis: { lines: { show: true } },
                padding: { top: -10, right: 0, bottom: 0, left: 6 },
            },
            tooltip: {
                enabled: true,
                x: { format: 'dd MMM HH:mm' },
                y: { formatter: (val) => '$' + val.toFixed(2) },
                theme: 'light',
            },
            dataLabels: { enabled: false },
        };

        const el = document.getElementById(elId);
        if (el && typeof ApexCharts !== 'undefined') {
            el.innerHTML = ''; // clear any previous
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
        const searchEl = document.getElementById('mp-search');
        const sortEl = document.getElementById('mp-sort');
        const query = (searchEl ? searchEl.value : '').toLowerCase().trim();
        const sortBy = sortEl ? sortEl.value : 'volume';

        let assets = [...MOCK_ASSETS];
        
        // Status Filter (Available / Funded)
        if (currentStatus === 'available') {
            // "Available" means either has secondary sell orders OR is still in primary funding
            assets = assets.filter(a => a.sellOrders > 0 || a.rentStatus !== 'funded');
        } else if (currentStatus === 'funded') {
            // "Funded" means it is fully funded and has NO active sell orders (passive portfolio assets)
            assets = assets.filter(a => a.sellOrders === 0 && a.rentStatus === 'funded');
        }

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

        // Category filter
        if (currentFilter === 'offers') {
            assets = assets.filter(a => a.sellOrders > 0);
        } else if (currentFilter === 'interest') {
            assets = assets.filter(a => a.buyInterest > 0);
        } else if (currentFilter === 'no-offers') {
            assets = assets.filter(a => a.sellOrders === 0);
        }

        // Sort
        assets.sort((a, b) => {
            if (sortBy === 'price') return b.price - a.price;
            if (sortBy === 'change') return b.change24h - a.change24h;
            if (sortBy === 'roi') return b.roi - a.roi;
            return b.volume24h - a.volume24h; // default: volume
        });

        // Render
        grid.innerHTML = '';

        if (assets.length === 0) {
            empty.style.display = 'flex';
        } else {
            empty.style.display = 'none';
            assets.forEach((asset, i) => {
                const { card } = renderCard(asset, i);
                grid.appendChild(card);
            });
        }
    }

    // ── Buy Interest Modal ──
    function openBuyInterestModal(asset) {
        const modal = document.getElementById('buy-interest-modal');
        const assetLabel = document.getElementById('interest-modal-asset');
        if (assetLabel) {
            assetLabel.innerHTML = `for <strong>${asset.name}</strong>`;
        }
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
        const fee = subtotal * ((window.POOOL_FEE_PCT || 5) / 100);
        const total = subtotal + fee;
        if (totalEl) totalEl.textContent = '$' + total.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    // ── Init ──
    document.addEventListener('DOMContentLoaded', async () => {
        try {
            const res = await fetch('/api/marketplace/secondary/assets');
            if (res.ok) {
                MOCK_ASSETS = await res.json();
            }
        } catch (err) {
            console.error('Failed to fetch secondary assets', err);
        }
        
        filterAndSort();

        // Search + Sort (if elements exist)
        const searchInput = document.getElementById('mp-search');
        const sortSelect = document.getElementById('mp-sort');
        if (searchInput) searchInput.addEventListener('input', debounce(filterAndSort, 200));
        if (sortSelect) sortSelect.addEventListener('change', filterAndSort);

        // Status tabs (Available / Funded)
        document.querySelectorAll('.mp-sec__status-tab').forEach(btn => {
            btn.addEventListener('click', () => {
                document.querySelectorAll('.mp-sec__status-tab').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                currentStatus = btn.dataset.status;
                filterAndSort();
            });
        });

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
            const btn = e.target.closest('.mp-sec__footer-btn--interest');
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

    // ═══════════════════════════════════════════════════════════════
    // ── LIVE PRICE UPDATES (Task 5.3) ─────────────────────────────
    // Polls GET /api/marketplace/:asset_id/chart-summary every 30s
    // to update card prices, 24h change, and volume with flash anim.
    // ═══════════════════════════════════════════════════════════════

    async function fetchLiveSummary(assetSlug) {
        try {
            const res = await fetch(`/api/marketplace/${assetSlug}/chart-summary`);
            if (!res.ok) return null;
            return await res.json();
        } catch {
            return null;
        }
    }

    async function updateLivePrices() {
        // Only run if page is visible
        if (document.visibilityState !== 'visible') return;

        for (const asset of MOCK_ASSETS) {
            const summary = await fetchLiveSummary(asset.slug);
            if (!summary || summary.last_price_cents == null) continue;

            // Find the card
            const card = document.querySelector(`.mp-sec__card[data-name="${asset.name.toLowerCase()}"]`);
            if (!card) continue;

            // Update price
            const priceEl = card.querySelector('.mp-sec__price');
            if (priceEl) {
                const oldPrice = parseInt(card.dataset.price);
                const newPrice = summary.last_price_cents;
                if (oldPrice !== newPrice) {
                    priceEl.textContent = formatUSD(newPrice);
                    card.dataset.price = newPrice;
                    // Flash animation
                    priceEl.style.transition = 'color 0.3s';
                    priceEl.style.color = newPrice > oldPrice ? '#16a34a' : '#dc2626';
                    setTimeout(() => { priceEl.style.color = ''; }, 1500);
                }
            }

            // Update 24h change
            if (summary.change_24h_pct != null) {
                const changeEl = card.querySelector('.mp-sec__change');
                if (changeEl) {
                    const pct = summary.change_24h_pct;
                    const isPositive = pct >= 0;
                    changeEl.textContent = (pct === 0 ? '' : (isPositive ? '+' : '')) + pct.toFixed(1) + '%';
                    changeEl.className = 'mp-sec__change ' + (
                        pct === 0 ? 'mp-sec__change--neutral' :
                        isPositive ? 'mp-sec__change--up' : 'mp-sec__change--down'
                    );
                    card.dataset.change = pct;
                }
            }

            // Update volume
            if (summary.volume_24h != null) {
                // Volume is in share count; convert to cents for display
                card.dataset.volume = summary.volume_24h * (summary.last_price_cents || asset.price);
            }
        }
    }

    // Start polling every 30 seconds
    setInterval(updateLivePrices, 30000);
    // Also run once after 2 seconds (initial data)
    setTimeout(updateLivePrices, 2000);
})();
