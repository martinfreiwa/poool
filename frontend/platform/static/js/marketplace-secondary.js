// frontend/platform/static/js/marketplace-secondary.js
// Secondary Market Overview — Redesigned with Hero Images
// Matches marketplace card style with toggleable chart

(function () {
    'use strict';

    // ── Mock Assets with image URLs ──
    const MOCK_ASSETS = [
        {
            slug: 'bali-villa-canggu-12',
            name: 'Bali Villa Canggu #12',
            type: 'Villa',
            location: 'Canggu, Bali',
            country: 'ID',
            images: ['/static/images/villa1.webp', '/static/images/villa1_2.webp', '/static/images/villa1_3.webp'],
            price: 10500,
            change24h: 2.3,
            volume24h: 1243000,
            roi: 12.4,
            occupancy: 87,
            sellOrders: 3,
            buyInterest: 1,
            totalSupply: 1000,
            sparkline: generateSparkData(365, 102, 105, true),
        },
        {
            slug: 'vienna-apartment-a3',
            name: 'Vienna City Apartment A3',
            type: 'Apartment',
            location: 'Vienna, Austria',
            country: 'AT',
            images: ['/static/images/villa2_1.webp', '/static/images/villa2_2.webp'],
            price: 8750,
            change24h: -1.1,
            volume24h: 876500,
            roi: 8.2,
            occupancy: 95,
            sellOrders: 2,
            buyInterest: 0,
            totalSupply: 500,
            sparkline: generateSparkData(365, 90, 87.5, false),
        },
        {
            slug: 'dubai-marina-tower-7',
            name: 'Dubai Marina Tower #7',
            type: 'Commercial',
            location: 'Dubai, UAE',
            country: 'AE',
            images: ['/static/images/villa3_1.webp', '/static/images/villa3_2.webp'],
            price: 15200,
            change24h: 0.8,
            volume24h: 2310000,
            roi: 15.1,
            occupancy: 92,
            sellOrders: 5,
            buyInterest: 3,
            totalSupply: 2000,
            sparkline: generateSparkData(365, 150, 152, true),
        },
        {
            slug: 'bali-ubud-retreat-5',
            name: 'Bali Ubud Retreat #5',
            type: 'Villa',
            location: 'Ubud, Bali',
            country: 'ID',
            images: ['/static/images/villa4_1.webp', '/static/images/villa4_2.webp'],
            price: 6200,
            change24h: -0.4,
            volume24h: 540000,
            roi: 9.8,
            occupancy: 72,
            sellOrders: 1,
            buyInterest: 0,
            totalSupply: 400,
            sparkline: generateSparkData(365, 63, 62, false),
        },
        {
            slug: 'lisbon-alfama-loft',
            name: 'Lisbon Alfama Loft',
            type: 'Residential',
            location: 'Lisbon, Portugal',
            country: 'PT',
            images: ['/static/images/villa5.webp', '/static/images/villa6.webp'],
            price: 9400,
            change24h: 3.7,
            volume24h: 1890000,
            roi: 11.3,
            occupancy: 89,
            sellOrders: 4,
            buyInterest: 2,
            totalSupply: 600,
            sparkline: generateSparkData(365, 90, 94, true),
        },
        {
            slug: 'singapore-shophouse-42',
            name: 'Singapore Shophouse #42',
            type: 'Commercial',
            location: 'Singapore',
            country: 'SG',
            images: ['/static/images/villa8.webp', '/static/images/villa1_4.webp'],
            price: 22000,
            change24h: 1.2,
            volume24h: 3120000,
            roi: 7.5,
            occupancy: 100,
            sellOrders: 2,
            buyInterest: 5,
            totalSupply: 1500,
            sparkline: generateSparkData(365, 217, 220, true),
        },
        {
            slug: 'berlin-mitte-penthouse',
            name: 'Berlin Mitte Penthouse',
            type: 'Residential',
            location: 'Berlin, Germany',
            country: 'DE',
            images: ['/static/images/villa6.webp'],
            price: 13500,
            change24h: 0,
            volume24h: 0,
            roi: 6.8,
            occupancy: 100,
            sellOrders: 0,
            buyInterest: 2,
            totalSupply: 800,
            sparkline: generateSparkData(365, 135, 135, true),
        },
        {
            slug: 'tokyo-shibuya-micro',
            name: 'Tokyo Shibuya Micro-Unit',
            type: 'Apartment',
            location: 'Tokyo, Japan',
            country: 'JP',
            images: ['/static/images/villa3_1.webp'],
            price: 4800,
            change24h: 0,
            volume24h: 0,
            roi: 5.2,
            occupancy: 98,
            sellOrders: 0,
            buyInterest: 0,
            totalSupply: 300,
            sparkline: generateSparkData(365, 48, 48, false),
        },
    ];

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
        const images = asset.images || ['/static/images/villa1.webp'];
        const hasMultiple = images.length > 1;

        let imagesHTML = images.map((img, i) =>
            `<div class="mp-sec__card-image ${i === 0 ? 'active' : ''}" style="background-image: url('${img}');" aria-label="${asset.name}"></div>`
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

        let dotsHTML = images.map((_, i) =>
            `<div class="mp-sec__card-dot ${i === 0 ? 'active' : ''}"></div>`
        ).join('');

        return `
            <div class="mp-sec__card-gallery">
                <div class="mp-sec__card-image-container">
                    ${imagesHTML}
                    ${navHTML}
                    <div class="mp-sec__card-dots">${dotsHTML}</div>
                </div>
                <span class="mp-sec__card-badge-overlay">${asset.type}</span>
                ${getStatusOverlay(asset)}
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
        if (!hasOffers) card.classList.add('mp-sec__card--no-offers');
        card.dataset.name = asset.name.toLowerCase();
        card.dataset.price = asset.price;
        card.dataset.volume = asset.volume24h;
        card.dataset.change = asset.change24h;
        card.dataset.roi = asset.roi;
        card.dataset.sellOrders = asset.sellOrders;
        card.dataset.buyInterest = asset.buyInterest;

        // Footer CTA — full-width button
        const footerCTA = hasOffers
            ? `<a href="/marketplace-trading-v3?asset=${asset.slug}" class="mp-sec__footer-btn mp-sec__footer-btn--trade" onclick="event.stopPropagation();">
                    View Property
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5l7 7-7 7"/></svg>
               </a>`
            : `<button class="mp-sec__footer-btn mp-sec__footer-btn--interest" data-asset-slug="${asset.slug}" data-asset-name="${asset.name}" onclick="event.stopPropagation();">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 14l-7 7-7-7"/><path d="M12 3v18"/></svg>
                    Place Buy Interest
               </button>`;

        card.innerHTML = `
            ${buildGalleryHTML(asset)}
            <div class="mp-sec__card-content">
                <div class="mp-sec__card-meta">
                    <div class="mp-sec__card-meta-item">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#414651" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0118 0z"/><circle cx="12" cy="10" r="3"/></svg>
                        <span>${asset.location}</span>
                    </div>
                    <div class="mp-sec__card-meta-divider"></div>
                    <div class="mp-sec__card-meta-item">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#414651" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
                        <span>Secondary</span>
                    </div>
                </div>
                <h3 class="mp-sec__card-title">${asset.name}</h3>
                <div class="mp-sec__price-row">
                    <span class="mp-sec__price">${formatUSD(asset.price)}</span>
                    ${asset.change24h !== 0
                        ? `<span class="mp-sec__change ${changeClass}">${changePrefix}${asset.change24h.toFixed(1)}%</span>`
                        : '<span class="mp-sec__change mp-sec__change--neutral">—</span>'
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
            <div class="mp-sec__card-footer">
                ${footerCTA}
            </div>
        `;

        card.addEventListener('click', () => {
            if (hasOffers) {
                const url = `/marketplace-trading-v3?asset=${asset.slug}`;
                // Tag elements for View Transitions API morph
                const gallery = card.querySelector('.mp-sec__card-gallery');
                if (gallery) gallery.style.viewTransitionName = 'tv3-hero-img';
                const title = card.querySelector('.mp-sec__card-title');
                if (title) title.style.viewTransitionName = 'tv3-title';
                window.location.href = url;
            } else {
                openBuyInterestModal(asset);
            }
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

    // ── Chart Toggle (GLOBAL — all visible cards toggle together) ──
    window.mpSecToggleChart = function (slug) {
        const clickedSection = document.getElementById(`chart-section-${slug}`);
        if (!clickedSection) return;

        // Determine new state from clicked card
        const isCurrentlyHidden = clickedSection.style.display === 'none' || !clickedSection.classList.contains('expanded');

        // Get ALL visible cards' chart sections and toggles
        const allSections = document.querySelectorAll('.mp-sec__chart-section');
        const allToggles = document.querySelectorAll('.mp-sec__chart-toggle');

        if (isCurrentlyHidden) {
            // Show ALL charts
            allSections.forEach(section => {
                section.style.display = 'block';
                section.classList.add('expanded');

                // Lazy-render chart only on first expand
                if (!section.dataset.rendered) {
                    section.dataset.rendered = 'true';
                    const sectionSlug = section.id.replace('chart-section-', '');
                    setTimeout(() => {
                        const asset = MOCK_ASSETS.find(a => a.slug === sectionSlug);
                        if (asset) {
                            const sparkId = `sparkline-${sectionSlug}`;
                            const isPositive = asset.change24h >= 0;
                            buildSparkline(sparkId, asset.sparkline, isPositive, asset.price);
                        }
                    }, 50);
                }
            });
            allToggles.forEach(toggle => {
                toggle.classList.add('expanded');
                toggle.querySelector('span').textContent = 'Hide Chart';
            });
        } else {
            // Hide ALL charts
            allSections.forEach(section => {
                section.style.display = 'none';
                section.classList.remove('expanded');
            });
            allToggles.forEach(toggle => {
                toggle.classList.remove('expanded');
                toggle.querySelector('span').textContent = '12m Chart';
            });
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
        const fee = subtotal * 0.05;
        const total = subtotal + fee;
        if (totalEl) totalEl.textContent = '$' + total.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    // ── Init ──
    document.addEventListener('DOMContentLoaded', () => {
        filterAndSort();

        // Search + Sort
        document.getElementById('mp-search').addEventListener('input', debounce(filterAndSort, 200));
        document.getElementById('mp-sort').addEventListener('change', filterAndSort);

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
                    changeEl.textContent = (isPositive ? '+' : '') + pct.toFixed(1) + '%';
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
