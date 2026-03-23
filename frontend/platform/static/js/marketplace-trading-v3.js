// frontend/platform/static/js/marketplace-trading-v3.js
// V3 Trading — Simple Trade Widget (OP Principle, Variant 3: Market Price Toggle)

(function () {
    'use strict';

    // ── Mock Asset Data ──
    const ASSETS = {};

    const DEFAULT_SLUG = 'bali-villa-canggu-12';

    function getAssetSlug() {
        return new URLSearchParams(window.location.search).get('asset') || DEFAULT_SLUG;
    }

    function fmt(val) {
        return '$' + val.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }
    function fmtInt(val) {
        return 'USD ' + val.toLocaleString('en-US');
    }

    // ── Toast Notification ──
    function showTradeToast(message, type) {
        const existing = document.querySelector('.tv3-toast');
        if (existing) existing.remove();
        const toast = document.createElement('div');
        toast.className = 'tv3-toast tv3-toast--' + type;
        toast.setAttribute('role', 'alert');
        toast.textContent = message;
        Object.assign(toast.style, {
            position: 'fixed', bottom: '24px', right: '24px', zIndex: '10000',
            padding: '12px 20px', borderRadius: '10px', fontWeight: '600', fontSize: '14px',
            color: '#fff', boxShadow: '0 8px 24px rgba(0,0,0,0.25)',
            background: type === 'success' ? '#00c896' : type === 'error' ? '#ef4444' : '#f59e0b',
            opacity: '0', transform: 'translateY(12px)', transition: 'all 0.3s ease'
        });
        document.body.appendChild(toast);
        requestAnimationFrame(() => { toast.style.opacity = '1'; toast.style.transform = 'translateY(0)'; });
        setTimeout(() => {
            toast.style.opacity = '0'; toast.style.transform = 'translateY(12px)';
            setTimeout(() => toast.remove(), 300);
        }, 4000);
    }

    // ── Generate 12-month daily price data ──
    function generatePriceData(startPrice, endPrice) {
        const data = [];
        let price = startPrice;
        const trend = (endPrice - startPrice) / 365;
        for (let i = 0; i < 365; i++) {
            price += trend + (Math.random() - 0.48) * 1.2;
            data.push(parseFloat(Math.max(price, 1).toFixed(2)));
        }
        return data;
    }

    // ── Populate Hero ──
    function populateHero(asset) {
        document.getElementById('tv3-bc-name').textContent = asset.name;
        document.getElementById('tv3-title').textContent = asset.name;
        document.getElementById('tv3-location-text').textContent = asset.location;

        // Financial hero stats
        document.getElementById('tv3-token-price').textContent = fmt(asset.tokenPrice);
        document.getElementById('tv3-yield').textContent = '+' + asset.annualYield + '%';
        document.getElementById('tv3-prop-val').textContent = fmtInt(asset.propertyValue);
        document.getElementById('tv3-net-ret').textContent = asset.netReturn + '%';
        const available = asset.totalSupply - (asset.sellOrders.reduce((s, o) => s + o.tokens, 0) || 0);
        document.getElementById('tv3-available').innerHTML = available.toLocaleString() + ' <small>/ ' + asset.totalSupply.toLocaleString() + '</small>';

        // Gallery — Mosaic layout
        const mainImg = document.getElementById('tv3-main-img');
        mainImg.src = asset.images[0];
        mainImg.alt = asset.name;

        // Fill 4 mosaic grid thumbnails
        const mosaicThumbs = document.querySelectorAll('.tv3-mosaic-thumb img');
        mosaicThumbs.forEach((img, i) => {
            const imgIdx = i + 1;
            if (asset.images[imgIdx]) {
                img.src = asset.images[imgIdx];
                img.alt = asset.name + ' ' + (imgIdx + 1);
            } else {
                img.src = asset.images[imgIdx % asset.images.length];
                img.alt = asset.name;
            }
        });

        // Click on mosaic thumb → swap with main
        document.querySelectorAll('.tv3-mosaic-thumb').forEach((thumb) => {
            thumb.addEventListener('click', () => {
                const thumbImg = thumb.querySelector('img');
                const oldMain = mainImg.src;
                mainImg.src = thumbImg.src;
                thumbImg.src = oldMain;
            });
        });
    }

    // ── Populate Property Details ──
    function populateDetails(asset) {
        document.getElementById('tv3-description').textContent = asset.description;

        // Property info
        document.getElementById('tv3-prop-value').textContent = fmtInt(asset.propertyValue);
        document.getElementById('tv3-gross-yield').textContent = asset.annualYield + '%';
        document.getElementById('tv3-net-return').textContent = asset.netReturn + '%';
        document.getElementById('tv3-price-sqm').textContent = fmtInt(asset.priceSqm);
        document.getElementById('tv3-prop-type').textContent = asset.type;
        document.getElementById('tv3-prop-land').textContent = asset.landSize;
        document.getElementById('tv3-prop-beds').textContent = asset.bedrooms > 0 ? asset.bedrooms : 'N/A';
        document.getElementById('tv3-prop-status').textContent = asset.rentStatus;

        // Info badges
        document.getElementById('tv3-info-country').textContent = asset.country + ', ' + asset.city;
        document.getElementById('tv3-info-status').textContent = asset.rentStatus;
        document.getElementById('tv3-info-yield').textContent = asset.annualYield + '% annual rental yield';
        document.getElementById('tv3-info-growth').textContent = asset.annualYield + '% annual gross yield';
        document.getElementById('tv3-info-net').textContent = 'With a net yield of ' + asset.netReturn + '% and price per m² of USD ' + asset.priceSqm.toLocaleString();

        // Financials
        document.getElementById('tv3-fin-price').textContent = fmtInt(asset.propertyValue);
        document.getElementById('tv3-fin-fee').textContent = '+ ' + fmtInt(asset.platformFee);
        document.getElementById('tv3-fin-total').textContent = '= ' + fmtInt(asset.propertyValue + asset.platformFee);
        document.getElementById('tv3-fin-gross').textContent = asset.annualYield + '%';
        document.getElementById('tv3-fin-proj').textContent = asset.projectedReturn + '%';
        document.getElementById('tv3-fin-net').textContent = asset.netReturn + '%';
        document.getElementById('tv3-fin-note2').textContent = 'Based on ' + asset.annualYield + '% annual rental yield';

        // Location
        document.getElementById('tv3-loc-subtitle').textContent = asset.city + ', ' + asset.country;
        document.getElementById('tv3-loc-desc').textContent = asset.locationDesc || '';
    }

    // ── Trade Widget: Market Data ──
    function getMarketData(asset, side) {
        if (side === 'buy') {
            // Buying: show sell offers (cheapest first)
            const sorted = [...asset.sellOrders].sort((a, b) => a.price - b.price);
            const totalShares = sorted.reduce((s, o) => s + o.tokens, 0);
            const bestPrice = sorted.length > 0 ? sorted[0].price : asset.tokenPrice;
            return { bestPrice, totalShares, count: sorted.length, orders: sorted };
        } else {
            // Selling: show buy bids (highest first)
            const sorted = [...asset.buyBids].sort((a, b) => b.price - a.price);
            const totalShares = sorted.reduce((s, o) => s + o.tokens, 0);
            const bestPrice = sorted.length > 0 ? sorted[0].price : asset.tokenPrice;
            return { bestPrice, totalShares, count: sorted.length, orders: sorted };
        }
    }

    function populateTradeWidget(asset) {
        // Sell-side depth for display
        const sellData = getMarketData(asset, 'buy');
        const buyData = getMarketData(asset, 'sell');

        // Market depth summary
        const depthSell = document.getElementById('tv3-depth-sell');
        const depthBuy = document.getElementById('tv3-depth-buy');
        if (depthSell) {
            depthSell.textContent = sellData.totalShares > 0
                ? sellData.totalShares + ' shares for sale from ' + fmt(sellData.bestPrice)
                : 'No shares currently for sale';
        }
        if (depthBuy) {
            depthBuy.textContent = buyData.totalShares > 0
                ? buyData.totalShares + ' buy offers from ' + fmt(buyData.bestPrice)
                : 'No buy offers placed';
        }
    }

    // ── Build 12-Month Chart ──
    function buildChart(asset) {
        const isPositive = asset.annualYield > 0;
        const color = isPositive ? '#16a34a' : '#dc2626';
        const data = generatePriceData(asset.tokenPrice * 0.92, asset.tokenPrice);

        const now = new Date();
        const categories = data.map((_, i) => {
            const d = new Date(now.getTime() - (data.length - 1 - i) * 86400000);
            return d.toISOString();
        });

        const options = {
            chart: {
                type: 'area',
                height: 250,
                sparkline: { enabled: false },
                toolbar: { show: false },
                zoom: { enabled: false },
                animations: { enabled: true, easing: 'easeinout', speed: 600 },
                background: 'transparent',
                fontFamily: "'TT Norms Pro', sans-serif",
                parentHeightOffset: 0,
            },
            series: [{ name: 'Share Price', data }],
            stroke: { width: 2, curve: 'smooth', colors: [color] },
            colors: [color],
            fill: {
                type: 'gradient',
                gradient: { shadeIntensity: 1, opacityFrom: 0.25, opacityTo: 0.05, stops: [0, 100] },
            },
            xaxis: {
                type: 'datetime',
                categories,
                labels: {
                    show: true,
                    style: { fontSize: '10px', colors: '#9ca3af' },
                    datetimeFormatter: { month: 'MMM', year: 'yyyy' },
                },
                axisBorder: { show: false },
                axisTicks: { show: false },
                tickAmount: 6,
            },
            yaxis: {
                labels: {
                    show: true,
                    style: { fontSize: '10px', colors: '#9ca3af' },
                    formatter: v => '$' + v.toFixed(0),
                },
                tickAmount: 4,
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
                x: { format: 'dd MMM yyyy' },
                y: { formatter: v => '$' + v.toFixed(2) },
                theme: 'light',
            },
            dataLabels: { enabled: false },
        };

        const el = document.getElementById('tv3-chart');
        if (el && typeof ApexCharts !== 'undefined') {
            el.innerHTML = '';
            new ApexCharts(el, options).render();
        }
    }

    // ── Trade Widget State ──
    let currentSide = 'buy';
    let priceMode = 'market'; // 'market' or 'custom'
    let currentAsset = null;

    function getActivePrice() {
        if (priceMode === 'market') {
            const data = getMarketData(currentAsset, currentSide);
            return data.bestPrice;
        } else {
            return parseFloat(document.getElementById('tv3-price').value) || 0;
        }
    }

    // ── Update Summary ──
    function updateSummary() {
        const qty = parseInt(document.getElementById('tv3-qty').value) || 0;
        const price = getActivePrice();
        const subtotal = qty * price;
        const fee = subtotal * 0.05;
        const total = subtotal + fee;

        document.getElementById('tv3-subtotal').textContent = fmt(subtotal);
        document.getElementById('tv3-fee').textContent = fmt(fee);
        document.getElementById('tv3-total').textContent = fmt(total);

        // Update submit button text
        const btn = document.getElementById('tv3-submit-btn');
        const action = currentSide === 'buy' ? 'Buy' : 'Sell';
        const shareText = qty === 1 ? 'Share' : 'Shares';
        btn.textContent = action + ' ' + qty + ' ' + shareText + ' · ' + fmt(total);
    }

    // ── Set Buy/Sell Side ──
    function setSide(side) {
        currentSide = side;
        document.getElementById('tv3-toggle-buy').classList.toggle('active', side === 'buy');
        document.getElementById('tv3-toggle-sell').classList.toggle('active', side === 'sell');

        const btn = document.getElementById('tv3-submit-btn');
        btn.className = side === 'buy'
            ? 'tv3-submit-btn tv3-submit-btn--buy'
            : 'tv3-submit-btn tv3-submit-btn--sell';

        // Update market info for the active side
        updateMarketInfo();
        updateSummary();

        // Update disclaimer
        const disclaimer = document.getElementById('tv3-disclaimer');
        if (disclaimer) {
            disclaimer.textContent = side === 'buy'
                ? "You won't be charged until matched with a seller."
                : "Your shares will be listed and matched with a buyer.";
        }
    }

    // ── Update market info display ──
    function updateMarketInfo() {
        if (!currentAsset) return;
        const data = getMarketData(currentAsset, currentSide);

        // Market info card
        const bestPriceEl = document.getElementById('tv3-best-price');
        const availSharesEl = document.getElementById('tv3-avail-shares');
        const availSellersEl = document.getElementById('tv3-avail-sellers');
        const marketInfoEl = document.getElementById('tv3-market-info');
        const customHint = document.getElementById('tv3-custom-hint');

        if (bestPriceEl) bestPriceEl.textContent = fmt(data.bestPrice);
        if (availSharesEl) availSharesEl.textContent = data.totalShares + ' shares';
        if (availSellersEl) {
            const label = currentSide === 'buy' ? 'sellers' : 'buyers';
            availSellersEl.textContent = data.count + ' ' + label;
        }

        // Ensure market info remains transparent for luxury look
        if (marketInfoEl) {
            marketInfoEl.style.background = 'transparent';
            marketInfoEl.style.borderColor = 'var(--tv3-border)';
            if (bestPriceEl) bestPriceEl.style.color = 'var(--tv3-text)';
        }

        // Update label
        const infoLabel = document.querySelector('.tv3-market-info-label');
        if (infoLabel) {
            infoLabel.textContent = currentSide === 'buy' ? 'Best available price' : 'Highest buy offer';
        }

        // Custom hint
        if (customHint) {
            customHint.textContent = 'Market price: ' + fmt(data.bestPrice) + '/share';
        }
    }

    // ── Set Price Mode ──
    function setPriceMode(mode) {
        priceMode = mode;
        document.getElementById('tv3-mode-market').classList.toggle('active', mode === 'market');
        document.getElementById('tv3-mode-custom').classList.toggle('active', mode === 'custom');

        document.getElementById('tv3-market-info').style.display = mode === 'market' ? 'block' : 'none';
        document.getElementById('tv3-custom-field').style.display = mode === 'custom' ? 'block' : 'none';

        if (mode === 'custom') {
            const data = getMarketData(currentAsset, currentSide);
            const priceInput = document.getElementById('tv3-price');
            if (priceInput && !priceInput.value) {
                priceInput.value = data.bestPrice.toFixed(2);
            }
        }

        updateSummary();
    }

    // ── Init ──
    document.addEventListener('DOMContentLoaded', async () => {
        const slug = getAssetSlug();

        let asset;
        try {
            const res = await fetch('/api/marketplace/secondary/assets');
            if (!res.ok) throw new Error('API fetch failed');
            const secondaryAssets = await res.json();
            const rawAsset = secondaryAssets.find(a => a.slug === slug) || secondaryAssets[0];
            
            if (!rawAsset) {
                document.getElementById('tv3-title').textContent = 'Asset Not Found';
                return;
            }

            // Map backend structure to UI standard
            asset = {
                slug: rawAsset.slug,
                name: rawAsset.name,
                type: rawAsset.type,
                location: rawAsset.location + (rawAsset.country ? ', ' + rawAsset.country : ''),
                country: rawAsset.country || 'N/A',
                city: rawAsset.location || 'N/A',
                description: rawAsset.description || 'No description available for this property.',
                tokenPrice: rawAsset.price / 100,
                annualYield: rawAsset.roi,
                projectedReturn: (rawAsset.roi * 1.5).toFixed(1),
                netReturn: (rawAsset.roi * 0.75).toFixed(1),
                occupancy: rawAsset.occupancy || 100,
                totalSupply: rawAsset.totalSupply,
                propertyValue: rawAsset.propertyValue ? rawAsset.propertyValue / 100 : 0,
                priceSqm: rawAsset.propertyValue && rawAsset.landSize ? Math.round((rawAsset.propertyValue / 100) / parseFloat(rawAsset.landSize || '100')) : 0,
                landSize: rawAsset.landSize || 'N/A',
                bedrooms: rawAsset.bedrooms || 0,
                rentStatus: rawAsset.rentStatus || 'N/A',
                platformFee: rawAsset.propertyValue ? (rawAsset.propertyValue / 100) * 0.05 : 0,
                images: rawAsset.images && rawAsset.images.length > 0 ? rawAsset.images : ['/images/villa1.webp'],
                sellOrders: rawAsset.sellOrders > 0 ? [{ tokens: rawAsset.sellOrders, price: rawAsset.price / 100 }] : [],
                buyBids: rawAsset.buy_interest > 0 ? [{ tokens: rawAsset.buy_interest, price: rawAsset.price / 100 }] : [],
                locationDesc: rawAsset.locationDesc || ''
            };
            
            // Assign to global
            currentAsset = asset;
        } catch (e) {
            console.error('Failed to load asset details:', e);
            document.getElementById('tv3-title').textContent = 'Error Loading Asset';
            return;
        }

        populateHero(asset);
        populateDetails(asset);
        populateTradeWidget(asset);
        buildChart(asset);

        // Initialize trade widget
        updateMarketInfo();
        updateSummary();

        // ── Performance Strip ──
        document.getElementById('tv3-ticker-price').textContent = fmt(asset.tokenPrice);
        document.getElementById('tv3-ticker-yield').textContent = asset.annualYield + '%';

        const current = asset.tokenPrice;
        const price3m = current * 0.96;
        const price6m = current * 0.92;
        const price12m = current * 0.87;
        const pct = (from) => ((current - from) / from * 100).toFixed(1);
        const sign = (v) => parseFloat(v) >= 0 ? '+' + v + '%' : v + '%';
        const color = (v, el) => { el.classList.toggle('tv3-sp-value--green', parseFloat(v) >= 0); el.classList.toggle('tv3-sp-value--red', parseFloat(v) < 0); };

        const el3m = document.getElementById('tv3-ticker-3m');
        const el6m = document.getElementById('tv3-ticker-6m');
        const el12m = document.getElementById('tv3-ticker-12m');
        el3m.textContent = sign(pct(price3m)); color(pct(price3m), el3m);
        el6m.textContent = sign(pct(price6m)); color(pct(price6m), el6m);
        el12m.textContent = sign(pct(price12m)); color(pct(price12m), el12m);

        // ── Mobile footer ──
        const mobilePrice = document.getElementById('tv3-mobile-price');
        const mobileYield = document.getElementById('tv3-mobile-yield');
        if (mobilePrice) mobilePrice.textContent = fmt(asset.tokenPrice);
        if (mobileYield) mobileYield.textContent = '+' + asset.annualYield + '% yield';

        // ── Trade Widget Events ──
        document.getElementById('tv3-qty').addEventListener('input', updateSummary);

        const priceInput = document.getElementById('tv3-price');
        if (priceInput) priceInput.addEventListener('input', updateSummary);

        // Buy/Sell toggle
        document.getElementById('tv3-toggle-buy').addEventListener('click', () => setSide('buy'));
        document.getElementById('tv3-toggle-sell').addEventListener('click', () => setSide('sell'));

        // Price mode toggle
        document.getElementById('tv3-mode-market').addEventListener('click', () => setPriceMode('market'));
        document.getElementById('tv3-mode-custom').addEventListener('click', () => setPriceMode('custom'));

        // Financial tabs
        document.querySelectorAll('.tv3-fin-tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.tv3-fin-tab').forEach(t => t.classList.toggle('active', t === tab));
                document.getElementById('tv3-fin-cost').style.display = tab.dataset.fin === 'cost' ? 'block' : 'none';
                document.getElementById('tv3-fin-rental').style.display = tab.dataset.fin === 'rental' ? 'block' : 'none';
            });
        });

        // Submit — real API call
        document.getElementById('tv3-order-form').addEventListener('submit', async (e) => {
            e.preventDefault();
            const btn = document.getElementById('tv3-submit-btn');
            const orig = btn.textContent;
            btn.textContent = 'Placing Order…';
            btn.disabled = true;
            btn.style.opacity = '0.7';

            const qty = parseInt(document.getElementById('tv3-qty').value) || 0;
            if (qty <= 0) {
                showTradeToast('Please enter a valid quantity', 'error');
                btn.textContent = orig; btn.disabled = false; btn.style.opacity = '1';
                return;
            }

            const orderType = priceMode === 'market' ? 'market' : 'limit';
            const data = getMarketData(currentAsset, currentSide);
            let priceCents = null;
            if (orderType === 'limit') {
                const priceVal = parseFloat(document.getElementById('tv3-price').value);
                if (!priceVal || priceVal <= 0) {
                    showTradeToast('Please enter a valid price', 'error');
                    btn.textContent = orig; btn.disabled = false; btn.style.opacity = '1';
                    return;
                }
                priceCents = Math.round(priceVal * 100);
            } else {
                priceCents = Math.round(data.bestPrice * 100);
            }

            try {
                const res = await fetch('/api/marketplace/orders', {
                    method: 'POST',
                    credentials: 'same-origin',
                    headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': (typeof getCsrfToken === 'function' ? getCsrfToken() : '') },
                    body: JSON.stringify({
                        asset_id: asset.slug,
                        side: currentSide,
                        order_type: orderType,
                        price_cents: priceCents,
                        quantity: qty,
                        idempotency_key: crypto.randomUUID()
                    })
                });
                const result = await res.json();
                if (res.ok) {
                    btn.textContent = '✓ Order Placed Successfully';
                    showTradeToast(result.message || 'Order placed successfully!', 'success');
                    setTimeout(() => { btn.textContent = orig; btn.disabled = false; btn.style.opacity = '1'; }, 2500);
                } else {
                    showTradeToast(result.error || 'Order failed', 'error');
                    btn.textContent = orig; btn.disabled = false; btn.style.opacity = '1';
                }
            } catch (err) {
                console.error('Order submission failed:', err);
                showTradeToast('Network error — please try again', 'error');
                btn.textContent = orig; btn.disabled = false; btn.style.opacity = '1';
            }
        });

        // ══ MOBILE BOTTOM SHEET ══
        const sheetOverlay = document.getElementById('tv3-sheet-overlay');
        const bottomSheet = document.getElementById('tv3-bottom-sheet');
        const mobileTradeBtn = document.getElementById('tv3-mobile-trade-btn');
        const sheetClose = document.getElementById('tv3-sheet-close');
        const sheetPrice = document.getElementById('tv3-sheet-price');
        const sheetQty = document.getElementById('tv3-sheet-qty');
        const sheetTotal = document.getElementById('tv3-sheet-total');

        function openSheet() {
            const data = getMarketData(asset, 'buy');
            if (sheetPrice) sheetPrice.value = data.bestPrice.toFixed(2);
            if (sheetOverlay) sheetOverlay.classList.add('open');
            if (bottomSheet) bottomSheet.classList.add('open');
            updateSheetTotal();
        }
        function closeSheet() {
            if (sheetOverlay) sheetOverlay.classList.remove('open');
            if (bottomSheet) bottomSheet.classList.remove('open');
        }
        function updateSheetTotal() {
            const q = parseInt(sheetQty?.value) || 0;
            const p = parseFloat(sheetPrice?.value) || 0;
            if (sheetTotal) sheetTotal.textContent = fmt(q * p * 1.05);
        }

        if (mobileTradeBtn) mobileTradeBtn.addEventListener('click', openSheet);
        if (sheetClose) sheetClose.addEventListener('click', closeSheet);
        if (sheetOverlay) sheetOverlay.addEventListener('click', closeSheet);
        if (sheetQty) sheetQty.addEventListener('input', updateSheetTotal);
        if (sheetPrice) sheetPrice.addEventListener('input', updateSheetTotal);

        // Sheet buy/sell toggle
        const sheetBuy = document.getElementById('tv3-sheet-buy');
        const sheetSell = document.getElementById('tv3-sheet-sell');
        const sheetSubmit = document.getElementById('tv3-sheet-submit');
        if (sheetBuy) sheetBuy.addEventListener('click', () => {
            sheetBuy.classList.add('active'); sheetSell.classList.remove('active');
            if (sheetSubmit) { sheetSubmit.textContent = 'Place Buy Order'; sheetSubmit.className = 'tv3-submit-btn tv3-submit-btn--buy'; }
        });
        if (sheetSell) sheetSell.addEventListener('click', () => {
            sheetSell.classList.add('active'); sheetBuy.classList.remove('active');
            if (sheetSubmit) { sheetSubmit.textContent = 'Place Sell Order'; sheetSubmit.className = 'tv3-submit-btn tv3-submit-btn--sell'; }
        });
        if (sheetSubmit) sheetSubmit.addEventListener('click', async () => {
            const orig = sheetSubmit.textContent;
            sheetSubmit.textContent = 'Placing…';
            sheetSubmit.disabled = true;

            const sheetSide = sheetBuy?.classList.contains('active') ? 'buy' : 'sell';
            const qty = parseInt(sheetQty?.value) || 0;
            const priceVal = parseFloat(sheetPrice?.value) || 0;
            if (qty <= 0 || priceVal <= 0) {
                showTradeToast('Enter valid price and quantity', 'error');
                sheetSubmit.textContent = orig; sheetSubmit.disabled = false;
                return;
            }

            try {
                const res = await fetch('/api/marketplace/orders', {
                    method: 'POST',
                    credentials: 'same-origin',
                    headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': (typeof getCsrfToken === 'function' ? getCsrfToken() : '') },
                    body: JSON.stringify({
                        asset_id: asset.slug,
                        side: sheetSide,
                        order_type: 'limit',
                        price_cents: Math.round(priceVal * 100),
                        quantity: qty,
                        idempotency_key: crypto.randomUUID()
                    })
                });
                const result = await res.json();
                if (res.ok) {
                    sheetSubmit.textContent = '✓ Order Placed';
                    showTradeToast(result.message || 'Order placed!', 'success');
                    setTimeout(() => { sheetSubmit.textContent = orig; sheetSubmit.disabled = false; closeSheet(); }, 2000);
                } else {
                    showTradeToast(result.error || 'Order failed', 'error');
                    sheetSubmit.textContent = orig; sheetSubmit.disabled = false;
                }
            } catch (err) {
                showTradeToast('Network error', 'error');
                sheetSubmit.textContent = orig; sheetSubmit.disabled = false;
            }
        });

        // ── Staggered Load Animations ──
        const animElements = document.querySelectorAll('.tv3-panel, .tv3-section, .tv3-hiw-card, .tv3-strategy-card');
        let delayCounter = 0;
        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    entry.target.style.animationDelay = `${(delayCounter * 0.08)}s`;
                    entry.target.classList.add('tv3-animate-fade-up');
                    delayCounter++;
                    clearTimeout(window.animationDelayReset);
                    window.animationDelayReset = setTimeout(() => { delayCounter = 0; }, 100);
                    observer.unobserve(entry.target);
                }
            });
        }, { threshold: 0.1 });

        animElements.forEach(el => {
            el.style.animationName = 'none';
            void el.offsetWidth;
            el.style.animationName = '';
            observer.observe(el);
        });
    });
})();

// ═══════════════════════════════════════
// INVESTMENT CALCULATOR
// ═══════════════════════════════════════
(function() {
    document.addEventListener('DOMContentLoaded', function() {
        const CHART_HEIGHT = 180;

        // Calculator elements
        const calcMainValue = document.getElementById('tv3-calc-main-value');
        const calcYAxis = document.getElementById('tv3-calc-y-axis');
        const calcChartBars = document.getElementById('tv3-calc-chart-bars');

        // Slider elements
        const investmentSlider = document.getElementById('tv3-calc-slider-1');
        const growthSlider = document.getElementById('tv3-calc-slider-2');
        const yieldSlider = document.getElementById('tv3-calc-slider-3');

        const investmentValue = document.getElementById('tv3-calc-slider-value-1');
        const growthValue = document.getElementById('tv3-calc-slider-value-2');
        const yieldValue = document.getElementById('tv3-calc-slider-value-3');

        if (!investmentSlider) return; // Guard: no calculator on page

        // Update slider track fill
        function updateSliderTrack(slider) {
            const min = parseFloat(slider.min);
            const max = parseFloat(slider.max);
            const val = parseFloat(slider.value);
            const pct = ((val - min) / (max - min)) * 100;
            slider.style.background = `linear-gradient(to right, #0000FF ${pct}%, #e2e2e2 ${pct}%)`;
        }

        function formatSliderValue(val, isUSD) {
            if (isUSD) return 'USD ' + new Intl.NumberFormat('en-US').format(Math.round(val));
            return Number.isInteger(val) ? val + '%' : val.toFixed(1) + '%';
        }

        // Init slider events
        [{ s: investmentSlider, v: investmentValue, usd: true },
         { s: growthSlider, v: growthValue, usd: false },
         { s: yieldSlider, v: yieldValue, usd: false }].forEach(function(cfg) {
            updateSliderTrack(cfg.s);
            cfg.s.addEventListener('input', function() {
                if (cfg.v) cfg.v.textContent = formatSliderValue(parseFloat(this.value), cfg.usd);
                updateSliderTrack(this);
            });
        });

        // Calculate 5-year investment returns using integer cents (BIGINT-safe)
        function calculateReturns(investment, growthRate, yieldRate) {
            var returns = [];
            var investCents = Math.round(investment * 100);
            var propValueCents = investCents;

            for (var y = 1; y <= 5; y++) {
                var appreciationCents = Math.round(propValueCents * (growthRate / 100));
                propValueCents += appreciationCents;
                var rentalCents = Math.round(investCents * (yieldRate / 100));

                returns.push({
                    year: y,
                    investment: investCents / 100,
                    appreciation: appreciationCents / 100,
                    rental: rentalCents / 100,
                    total: (investCents + appreciationCents + rentalCents) / 100
                });
            }
            return returns;
        }

        function formatCurrency(amount) {
            if (amount >= 1000000) return (amount / 1000000).toFixed(1) + 'M';
            if (amount >= 1000) return Math.round(amount / 1000) + 'k';
            return Math.round(amount).toString();
        }

        function formatFullCurrency(amount) {
            return Math.round(amount).toLocaleString();
        }

        function formatTooltipCurrency(amount) {
            return '$' + Math.round(amount).toLocaleString();
        }

        function computeNiceMax(maxValue) {
            var padded = maxValue * 1.15;
            if (padded <= 0) return 1000;
            var mag = Math.pow(10, Math.floor(Math.log10(padded)));
            var norm = padded / mag;
            var nice;
            if (norm <= 1.5) nice = 1.5;
            else if (norm <= 2) nice = 2;
            else if (norm <= 2.5) nice = 2.5;
            else if (norm <= 3) nice = 3;
            else if (norm <= 5) nice = 5;
            else if (norm <= 7.5) nice = 7.5;
            else nice = 10;
            return nice * mag;
        }

        function updateYAxis(maxValue) {
            var yAxisMax = computeNiceMax(maxValue);
            var steps = 6;
            var stepValue = yAxisMax / (steps - 1);
            if (!calcYAxis) return yAxisMax;

            var lines = calcYAxis.querySelectorAll('.tv3-calc-y-axis-line');
            lines.forEach(function(line, i) {
                var value = yAxisMax - stepValue * i;
                var num = line.querySelector('.tv3-calc-y-axis-number');
                if (num) num.textContent = formatCurrency(Math.max(0, value));
            });
            return yAxisMax;
        }

        function updateChartBars(data, yAxisMax) {
            if (!calcChartBars) return;
            var bars = calcChartBars.querySelectorAll('.tv3-calc-bar');
            var currentYear = new Date().getFullYear();

            data.forEach(function(d, i) {
                if (i >= bars.length) return;
                var bar = bars[i];
                var chartBar = bar.querySelector('.tv3-calc-chart-bar');
                var totalH = (d.total / yAxisMax) * CHART_HEIGHT;
                chartBar.style.height = totalH + 'px';
                chartBar.style.bottom = '0px';

                var invH = (d.investment / d.total) * totalH;
                var appH = (d.appreciation / d.total) * totalH;
                var renH = (d.rental / d.total) * totalH;

                var s1 = chartBar.querySelector('.tv3-calc-series.series-1');
                var s2 = chartBar.querySelector('.tv3-calc-series.series-2');
                var s3 = chartBar.querySelector('.tv3-calc-series.series-3');

                if (s1) { s1.style.height = invH + 'px'; s1.style.bottom = '0px'; }
                if (s2) { s2.style.height = appH + 'px'; s2.style.bottom = invH + 'px'; }
                if (s3) { s3.style.height = renH + 'px'; s3.style.bottom = (invH + appH) + 'px'; }

                // Remove old tooltips/labels
                var oldTip = bar.querySelector('.tv3-calc-bar-tooltip');
                var oldLbl = bar.querySelector('.tv3-calc-bar-value-label');
                if (oldTip) oldTip.remove();
                if (oldLbl) oldLbl.remove();

                // Tooltip
                var tip = document.createElement('div');
                tip.className = 'tv3-calc-bar-tooltip';
                tip.innerHTML = '<div style="font-weight:600;margin-bottom:6px;font-size:13px;">' + (currentYear + d.year - 1) + '</div>' +
                    '<div class="tv3-calc-bar-tooltip-row"><span class="tv3-calc-bar-tooltip-dot investment"></span><span class="tv3-calc-bar-tooltip-label">Investment</span><span class="tv3-calc-bar-tooltip-value">' + formatTooltipCurrency(d.investment) + '</span></div>' +
                    '<div class="tv3-calc-bar-tooltip-row"><span class="tv3-calc-bar-tooltip-dot appreciation"></span><span class="tv3-calc-bar-tooltip-label">Appreciation</span><span class="tv3-calc-bar-tooltip-value">' + formatTooltipCurrency(d.appreciation) + '</span></div>' +
                    '<div class="tv3-calc-bar-tooltip-row"><span class="tv3-calc-bar-tooltip-dot rental"></span><span class="tv3-calc-bar-tooltip-label">Rental</span><span class="tv3-calc-bar-tooltip-value">' + formatTooltipCurrency(d.rental) + '</span></div>' +
                    '<div style="border-top:1px solid #333;margin-top:6px;padding-top:6px;display:flex;justify-content:space-between;gap:16px;"><span style="color:#A4A7AE;">Total</span><span style="font-weight:700;">' + formatTooltipCurrency(d.total) + '</span></div>';
                bar.appendChild(tip);

                // Value label
                var lbl = document.createElement('div');
                lbl.className = 'tv3-calc-bar-value-label';
                lbl.textContent = formatCurrency(d.total);
                bar.appendChild(lbl);
            });
        }

        function updateMainTitle(data) {
            if (!calcMainValue) return;
            var cumulative = data.reduce(function(sum, yr) { return sum + yr.appreciation + yr.rental; }, 0);
            calcMainValue.textContent = 'USD ' + formatFullCurrency(cumulative) + ' in 5 years';
        }

        function updateStatsCard(data) {
            var invEl = document.getElementById('tv3-calc-stat-investment');
            var appEl = document.getElementById('tv3-calc-stat-appreciation');
            var renEl = document.getElementById('tv3-calc-stat-rental');

            if (data.length > 0) {
                var totalInv = data[0].investment;
                var totalRen = data.reduce(function(s, y) { return s + y.rental; }, 0);
                var totalApp = data.reduce(function(s, y) { return s + y.appreciation; }, 0);

                if (invEl) invEl.textContent = '$' + formatFullCurrency(totalInv);
                if (renEl) renEl.textContent = '$' + formatFullCurrency(totalRen);
                if (appEl) appEl.textContent = '$' + formatFullCurrency(totalApp);
            }
        }

        function updateCalculator() {
            var inv = parseFloat(investmentSlider.value) || 100000;
            var gro = parseFloat(growthSlider.value) || 10;
            var yld = parseFloat(yieldSlider.value) || 12;
            var data = calculateReturns(inv, gro, yld);
            var maxVal = Math.max.apply(null, data.map(function(d) { return d.total; }));

            var yAxisMax = updateYAxis(maxVal);
            updateChartBars(data, yAxisMax);
            updateMainTitle(data);
            updateStatsCard(data);
        }

        // Attach listeners
        [investmentSlider, growthSlider, yieldSlider].forEach(function(s) {
            if (s) s.addEventListener('input', updateCalculator);
        });

        // Initialize
        updateCalculator();
    });
})();

// ═══════════════════════════════════════
// DOCUMENT TABS
// ═══════════════════════════════════════
(function() {
    document.addEventListener('DOMContentLoaded', function() {
        var tabs = document.querySelectorAll('.tv3-doc-tab');
        if (tabs.length === 0) return;

        tabs.forEach(function(tab) {
            tab.addEventListener('click', function() {
                tabs.forEach(function(t) { t.classList.remove('active'); });
                this.classList.add('active');
            });
        });
    });
})();
