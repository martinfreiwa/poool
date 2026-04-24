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

    function formatSharePrice(cents) {
        const dollars = Math.round((Number(cents) || 0) / 100).toLocaleString('en-US');
        return `USD ${dollars}<small style="font-size: 0.5em; font-weight: 500; color: #667085; margin-left: 4px;">/ share</small>`;
    }

    function formatVolume(cents) {
        const dollars = cents / 100;
        if (dollars >= 1000000) return '$' + (dollars / 1000000).toFixed(1) + 'M';
        if (dollars >= 1000) return '$' + (dollars / 1000).toFixed(1) + 'K';
        return '$' + dollars.toFixed(0);
    }

    function formatPct(value) {
        const num = Number(value || 0);
        return `${num >= 10 ? num.toFixed(0) : num.toFixed(1)}%`;
    }

    function formatCardPct(value) {
        const num = Number(value || 0);
        return `${Number.isInteger(num) ? num.toFixed(0) : num.toFixed(1)}%`;
    }

    function formatBps(value) {
        if (value === null || value === undefined || value === '') return '—';
        const pct = Number(value) / 100;
        return `${Number.isInteger(pct) ? pct.toFixed(0) : pct.toFixed(1)}%`;
    }

    function escapeHTML(value) {
        return String(value ?? '').replace(/[&<>"']/g, (char) => ({
            '&': '&amp;',
            '<': '&lt;',
            '>': '&gt;',
            '"': '&quot;',
            "'": '&#39;'
        }[char]));
    }

    function escapeAttr(value) {
        return escapeHTML(value);
    }

    function metricValue(value) {
        return value === null || value === undefined || value === '' ? '—' : escapeHTML(value);
    }

    function sizeMetric(asset) {
        return asset.buildingSizeSqm || asset.landSize || '—';
    }

    function buildPropertyMetaHTML(asset) {
        return `
            <div class="card-meta-item">
                <img src="/static/images/icons/Bed.svg" alt="Bedrooms" width="16" height="16">
                <span>${metricValue(asset.bedrooms)}</span>
            </div>
            <div class="card-meta-divider"></div>
            <div class="card-meta-item">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#414651" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M22 13a3 3 0 0 0-3-3H5a3 3 0 0 0-3 3v2a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-2Z"/><path d="M7 17v2"/><path d="M17 17v2"/><path d="M21 10V6a2 2 0 0 0-2-2"/></svg>
                <span>${metricValue(asset.bathrooms)}</span>
            </div>
            <div class="card-meta-divider"></div>
            <div class="card-meta-item">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#414651" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 4h16v16H4z"/><path d="M9 4v16"/><path d="M4 9h16"/></svg>
                <span>${metricValue(sizeMetric(asset))}</span>
            </div>`;
    }

    function leaseBadge(asset) {
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

        return `<div class="property-badge ds-badge ds-badge--overlay ${badgeColorClass}">
            <span class="badge-text">${escapeHTML(typeText)}</span>
        </div>`;
    }

    function getFundingProgress(asset) {
        if (typeof asset.fundingProgressPct === 'number') {
            return Math.min(100, Math.max(0, asset.fundingProgressPct));
        }
        return asset.totalSupply > 0
            ? Math.min(100, Math.max(0, ((asset.totalSupply - (asset.tokensAvailable || 0)) / asset.totalSupply) * 100))
            : 0;
    }

    // ── Build shared property-card gallery HTML ──
    function buildGalleryHTML(asset) {
        let images = (asset.images || []).filter((img) => typeof img === 'string' && img.trim() !== '');
        if (images.length === 0) {
            images = ['/static/images/seed/villa1.webp'];
        }
        // Limit to 5 images for preview
        images = images.slice(0, 5);
        const hasMultiple = images.length > 1;

        const imagesHTML = images.map((img, i) =>
            `<img src="${escapeAttr(img)}" class="property-image ${i === 0 ? 'active' : ''}" style="object-fit: cover; object-position: center;" alt="${escapeAttr(asset.name)}" loading="lazy" onerror="this.onerror=null;this.src='/static/images/seed/villa1.webp';">`
        ).join('');

        let navHTML = '';
        if (hasMultiple) {
            navHTML = `
                <button class="property-nav-arrow property-nav-prev" onclick="event.stopPropagation(); cardPrevImage(this)" aria-label="Previous image">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#333" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg>
                </button>
                <button class="property-nav-arrow property-nav-next" onclick="event.stopPropagation(); cardNextImage(this)" aria-label="Next image">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#333" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
                </button>`;
        }

        const dotsHTML = images.map((_, i) =>
            `<div class="property-dot ${i === 0 ? 'active' : ''}" data-property-id="${escapeAttr(asset.slug)}" data-image-index="${i}"></div>`
        ).join('');

        return `
            <div class="property-gallery">
                <div class="property-image-container">
                    ${imagesHTML}
                    ${navHTML}
                    <div class="property-dots">${dotsHTML}</div>
                </div>
                ${leaseBadge(asset)}
            </div>`;
    }

    // ── Render Card ──
    function renderCard(asset) {
        const fundingPct = getFundingProgress(asset);
        const isFullyFunded = fundingPct >= 100;
        const investmentDuration = asset.termMonths ? `${asset.termMonths} months` : '—';
        const annualizedReturn = asset.roi === null || asset.roi === undefined ? '—' : `${formatCardPct(asset.roi)}`;

        const card = document.createElement('div');
        card.className = 'property-card';
        card.dataset.propertyId = asset.slug;
        card.dataset.name = asset.name.toLowerCase();
        card.dataset.location = asset.location;
        card.dataset.assetType = asset.type;
        card.dataset.fundingStatus = asset.fundingStatus || asset.rentStatus || '';
        card.dataset.price = asset.price;
        card.dataset.volume = asset.volume24h;
        card.dataset.change = asset.change24h;
        card.dataset.roi = asset.roi;
        card.dataset.sellOrders = asset.sellOrders;
        card.dataset.buyInterest = asset.buyInterest;

        card.innerHTML = `
            ${buildGalleryHTML(asset)}
            <div class="property-content">
                <div class="card-meta-row">
                    ${buildPropertyMetaHTML(asset)}
                </div>

                <div class="property-heading">
                    <h3 class="property-title">${escapeHTML(asset.name)}</h3>
                </div>

                <div class="property-pricing">
                    <div class="price-wrapper">
                        <span class="property-price">${formatSharePrice(asset.price)}</span>
                        <span class="funded-percentage${isFullyFunded ? ' funded-percentage--complete' : ''}">${formatCardPct(fundingPct)} funded</span>
                    </div>
                    <div class="property-progress ds-progress">
                        <div class="ds-progress__fill${isFullyFunded ? ' ds-progress__fill--complete' : ''}" style="width: ${fundingPct}%"></div>
                    </div>
                </div>

                <div class="investment-details">
                    <div class="investment-row">
                        <span class="investment-label">Investment duration</span>
                        <span class="investment-value">${escapeHTML(investmentDuration)}</span>
                    </div>
                    <div class="investment-row">
                        <span class="investment-label">Projected return</span>
                        <span class="investment-value">${formatBps(asset.capitalAppreciationBps)}</span>
                    </div>
                    <div class="investment-row">
                        <span class="investment-label">Projected annualised net return</span>
                        <span class="investment-value">${escapeHTML(annualizedReturn)}</span>
                    </div>
                </div>
            </div>
        `;

        card.addEventListener('click', () => {
            const url = `/marketplace-trading-v3?asset=${asset.slug}`;
            const gallery = card.querySelector('.property-gallery');
            if (gallery) gallery.style.viewTransitionName = 'tv3-hero-img';
            const title = card.querySelector('.property-title');
            if (title) title.style.viewTransitionName = 'tv3-title';
            window.location.href = url;
        });

        return { card };
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

        // Sort cards
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
            if (window.initializePropertyCards) {
                window.initializePropertyCards(grid);
            }
        }
    }

    // ── Buy Interest Modal ──
    function openBuyInterestModal(asset) {
        const modal = document.getElementById('buy-interest-modal');
        const assetLabel = document.getElementById('interest-modal-asset');
        if (assetLabel) {
            const prefix = document.createTextNode('for ');
            const strong = document.createElement('strong');
            strong.textContent = asset.name || 'this asset';
            assetLabel.replaceChildren(prefix, strong);
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

    function setInterestFeedback(message, type = 'info') {
        const totalEl = document.getElementById('interest-total');
        if (!totalEl) return;
        totalEl.textContent = message;
        totalEl.style.color = type === 'error' ? '#d92d20' : type === 'success' ? '#027a48' : '';
    }

    function newIdempotencyKey() {
        if (window.crypto && typeof window.crypto.randomUUID === 'function') {
            return window.crypto.randomUUID();
        }
        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
            const r = Math.random() * 16 | 0;
            const v = c === 'x' ? r : (r & 0x3 | 0x8);
            return v.toString(16);
        });
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
        if (totalEl) totalEl.style.color = '';
    }

    // ── Init ──
    document.addEventListener('DOMContentLoaded', async () => {
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

        try {
            const res = await fetch('/api/marketplace/secondary/assets');
            if (res.ok) {
                MOCK_ASSETS = await res.json();
            }
        } catch (err) {
            console.error('Failed to fetch secondary assets', err);
        }

        filterAndSort();

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
            submitBtn.addEventListener('click', async () => {
                const modal = document.getElementById('buy-interest-modal');
                const priceInput = document.getElementById('interest-price');
                const qtyInput = document.getElementById('interest-qty');
                const assetSlug = modal?.dataset.assetSlug;
                const price = parseFloat(priceInput?.value);
                const quantity = parseInt(qtyInput?.value, 10);

                if (!assetSlug || !price || price <= 0 || !quantity || quantity <= 0) {
                    setInterestFeedback('Enter a valid price and quantity.', 'error');
                    return;
                }

                const original = submitBtn.innerHTML;
                submitBtn.textContent = 'Placing order...';
                submitBtn.disabled = true;

                try {
                    const res = await fetch('/api/marketplace/orders', {
                        method: 'POST',
                        credentials: 'same-origin',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            asset_id: assetSlug,
                            side: 'buy',
                            order_type: 'limit',
                            price_cents: Math.round(price * 100),
                            quantity,
                            idempotency_key: newIdempotencyKey()
                        })
                    });
                    const data = await res.json().catch(() => ({}));
                    if (!res.ok) {
                        throw new Error(data.error || data.message || 'Failed to place buy order.');
                    }
                    setInterestFeedback('Buy order placed.', 'success');
                    setTimeout(() => {
                        closeBuyInterestModal();
                        window.location.href = `/trade-success?side=buy&qty=${quantity}&price=${price.toFixed(2)}&order_id=${encodeURIComponent(data.order_id || '')}&slug=${encodeURIComponent(assetSlug)}`;
                    }, 600);
                } catch (err) {
                    setInterestFeedback(err.message || 'Failed to place buy order.', 'error');
                    submitBtn.innerHTML = original;
                    submitBtn.disabled = false;
                }
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
            const card = document.querySelector(`.property-card[data-name="${CSS.escape(asset.name.toLowerCase())}"]`);
            if (!card) continue;

            // Update price
            const priceEl = card.querySelector('.property-price');
            if (priceEl) {
                const oldPrice = parseInt(card.dataset.price);
                const newPrice = summary.last_price_cents;
                if (oldPrice !== newPrice) {
                    priceEl.innerHTML = formatSharePrice(newPrice);
                    card.dataset.price = newPrice;
                    // Flash animation
                    priceEl.style.transition = 'color 0.3s';
                    priceEl.style.color = newPrice > oldPrice ? '#16a34a' : '#dc2626';
                    setTimeout(() => { priceEl.style.color = ''; }, 1500);
                }
            }

            // Keep the shared card funded badge stable; live 24h movement remains sortable via dataset.
            if (summary.change_24h_pct != null) {
                card.dataset.change = summary.change_24h_pct;
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
