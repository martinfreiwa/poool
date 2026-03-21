// frontend/platform/static/js/marketplace-trading-v3.js
// V3 Trading — Full property content + trading orderbook

(function () {
    'use strict';

    // ── Mock Asset Data ──
    const ASSETS = {
        'bali-villa-canggu-12': {
            slug: 'bali-villa-canggu-12',
            name: 'Bali Villa Canggu #12',
            type: 'Villa',
            location: 'Canggu, Bali, Indonesia',
            country: 'Indonesia',
            city: 'Bali',
            description: 'Contemporary villa steps from Echo Beach, Canggu\'s most popular surf break. Fully furnished with a private pool, modern kitchen, and outdoor living area. Currently rented and professionally managed with consistent monthly rental yield.',
            tokenPrice: 105.00,
            annualYield: 12.4,
            projectedReturn: 18.5,
            netReturn: 9.2,
            occupancy: 87,
            totalSupply: 1000,
            propertyValue: 350000,
            priceSqm: 2734,
            landSize: '128 m²',
            bedrooms: 4,
            rentStatus: 'Rented',
            platformFee: 17500,
            fiveYearReturn: 121,
            images: [
                '/images/villa1.webp',
                '/images/villa1_2.webp',
                '/images/villa1_3.webp',
                '/images/villa1_4.webp',
            ],
            sellOrders: [
                { seller: 'Investor_A2', tokens: 15, price: 108.50 },
                { seller: 'Investor_C7', tokens: 8, price: 110.00 },
                { seller: 'Investor_F1', tokens: 25, price: 112.00 },
            ],
            buyBids: [
                { buyer: 'Investor_D4', tokens: 20, price: 103.00 },
                { buyer: 'Investor_B9', tokens: 10, price: 101.50 },
            ],
            locationDesc: 'Canggu is one of Bali\'s fastest-growing areas, known for its world-class surf breaks, vibrant food scene, and thriving digital nomad community. The area offers excellent rental returns due to high demand from both tourists and long-term residents.',
        },
        'vienna-apartment-a3': {
            slug: 'vienna-apartment-a3',
            name: 'Vienna City Apartment A3',
            type: 'Apartment',
            location: 'Vienna, Austria',
            country: 'Austria',
            city: 'Vienna',
            description: 'Central Vienna apartment near Stephansplatz. Premium location with stable long-term rental income. Professionally managed with full documentation and high occupancy.',
            tokenPrice: 87.50,
            annualYield: 8.2,
            projectedReturn: 14.1,
            netReturn: 6.1,
            occupancy: 95,
            totalSupply: 500,
            propertyValue: 180000,
            priceSqm: 3200,
            landSize: '56 m²',
            bedrooms: 2,
            rentStatus: 'Rented',
            platformFee: 9000,
            fiveYearReturn: 84,
            images: ['/images/villa2_1.webp', '/images/villa2_2.webp'],
            sellOrders: [
                { seller: 'Investor_E3', tokens: 5, price: 89.00 },
                { seller: 'Investor_G2', tokens: 12, price: 90.50 },
            ],
            buyBids: [],
            locationDesc: 'Vienna consistently ranks among the world\'s most livable cities. The central location near Stephansplatz offers excellent public transport connections and proximity to major cultural attractions.',
        },
        'dubai-marina-tower-7': {
            slug: 'dubai-marina-tower-7',
            name: 'Dubai Marina Tower #7',
            type: 'Commercial',
            location: 'Dubai, UAE',
            country: 'UAE',
            city: 'Dubai',
            description: 'Premium commercial unit in Dubai Marina with high-yield rental income. Fully occupied by a multinational corporate tenant on a 5-year lease with annual rent escalation.',
            tokenPrice: 152.00,
            annualYield: 15.1,
            projectedReturn: 22.3,
            netReturn: 11.8,
            occupancy: 92,
            totalSupply: 2000,
            propertyValue: 980000,
            priceSqm: 5440,
            landSize: '180 m²',
            bedrooms: 0,
            rentStatus: 'Leased (Corporate)',
            platformFee: 49000,
            fiveYearReturn: 152,
            images: ['/images/villa3_1.webp', '/images/villa3_2.webp'],
            sellOrders: [
                { seller: 'Investor_H1', tokens: 30, price: 155.00 },
                { seller: 'Investor_K4', tokens: 10, price: 156.50 },
                { seller: 'Investor_J8', tokens: 50, price: 158.00 },
                { seller: 'Investor_L2', tokens: 15, price: 160.00 },
                { seller: 'Investor_M6', tokens: 5, price: 165.00 },
            ],
            buyBids: [
                { buyer: 'Investor_N3', tokens: 25, price: 150.00 },
                { buyer: 'Investor_P7', tokens: 40, price: 148.50 },
                { buyer: 'Investor_Q1', tokens: 10, price: 147.00 },
            ],
            locationDesc: 'Dubai Marina is one of the most sought-after commercial and residential districts in the UAE, offering premium waterfront views, world-class amenities, and excellent connectivity.',
        },
        'jimbaran-sunset-villa': {
            slug: 'jimbaran-sunset-villa',
            name: 'Jimbaran Sunset Villa',
            type: 'Villa',
            location: 'Jimbaran, Bali, Indonesia',
            country: 'Indonesia',
            city: 'Bali',
            description: 'Stunning 4-bedroom villa with panoramic sunset views over Jimbaran Bay. Experience true Balinese luxury with a private infinity pool, tropical gardens, and walking distance to the beach. Professionally managed with proven high rental occupancy.',
            tokenPrice: 125.00,
            annualYield: 14.2,
            projectedReturn: 21.5,
            netReturn: 10.8,
            occupancy: 92,
            totalSupply: 1200,
            propertyValue: 450000,
            priceSqm: 3100,
            landSize: '240 m²',
            bedrooms: 4,
            rentStatus: 'Rented',
            platformFee: 22500,
            images: [
                '/images/jimbaran_sunset_hero.webp',
                '/images/jimbaran_sunset_pool.webp',
                '/images/jimbaran_sunset_living.webp',
                '/images/jimbaran_sunset_terrace.webp',
                '/images/jimbaran_sunset_master.webp',
            ],
            sellOrders: [
                { seller: 'Investor_X1', tokens: 10, price: 128.00 },
                { seller: 'Investor_Y3', tokens: 5, price: 130.00 },
            ],
            buyBids: [
                { buyer: 'Investor_Z5', tokens: 15, price: 122.00 },
            ],
            locationDesc: 'Jimbaran is world-famous for its sunset seafood grills on the beach and pristine white sand. The area is home to Bali\'s most exclusive resorts, ensuring stable high-end rental demand.',
        },
    };

    const DEFAULT_SLUG = 'bali-villa-canggu-12';
    let currentImgIdx = 0;

    function getAssetSlug() {
        return new URLSearchParams(window.location.search).get('asset') || DEFAULT_SLUG;
    }

    function fmt(val) {
        return '$' + val.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }
    function fmtInt(val) {
        return 'USD ' + val.toLocaleString('en-US');
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
            const imgIdx = i + 1; // images[1] through images[4]
            if (asset.images[imgIdx]) {
                img.src = asset.images[imgIdx];
                img.alt = asset.name + ' ' + (imgIdx + 1);
            } else {
                // Reuse images if less than 5
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

    // ── Populate Order Book ──
    function populateOrderBook(asset) {
        const sellRows = document.getElementById('tv3-sell-rows');
        const buyRows = document.getElementById('tv3-buy-rows');
        document.getElementById('tv3-sell-count').textContent = asset.sellOrders.length;
        document.getElementById('tv3-buy-count').textContent = asset.buyBids.length;

        sellRows.innerHTML = '';
        if (asset.sellOrders.length === 0) {
            document.getElementById('tv3-sell-empty').style.display = 'block';
        } else {
            document.getElementById('tv3-sell-empty').style.display = 'none';
            asset.sellOrders.forEach(o => {
                const row = document.createElement('tr');
                row.innerHTML = `<td>${o.seller}</td><td>${o.tokens}</td><td>${fmt(o.price)}</td><td>${fmt(o.tokens * o.price)}</td><td><button class="tv3-ob-action tv3-ob-action--buy" data-price="${o.price}" data-qty="${o.tokens}">Buy</button></td>`;
                sellRows.appendChild(row);
            });
        }

        buyRows.innerHTML = '';
        if (asset.buyBids.length === 0) {
            document.getElementById('tv3-buy-empty').style.display = 'block';
        } else {
            document.getElementById('tv3-buy-empty').style.display = 'none';
            asset.buyBids.forEach(o => {
                const row = document.createElement('tr');
                row.innerHTML = `<td>${o.buyer}</td><td>${o.tokens}</td><td>${fmt(o.price)}</td><td>${fmt(o.tokens * o.price)}</td><td><button class="tv3-ob-action tv3-ob-action--sell" data-price="${o.price}" data-qty="${o.tokens}">Sell</button></td>`;
                buyRows.appendChild(row);
            });
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

    // ── Order Summary ──
    function updateSummary() {
        const qty = parseInt(document.getElementById('tv3-qty').value) || 0;
        const price = parseFloat(document.getElementById('tv3-price').value) || 0;
        const subtotal = qty * price;
        const fee = subtotal * 0.05;
        document.getElementById('tv3-subtotal').textContent = fmt(subtotal);
        document.getElementById('tv3-fee').textContent = fmt(fee);
        document.getElementById('tv3-total').textContent = fmt(subtotal + fee);
    }

    // ── Buy/Sell Toggle ──
    let currentSide = 'buy';
    function setSide(side) {
        currentSide = side;
        document.getElementById('tv3-toggle-buy').classList.toggle('active', side === 'buy');
        document.getElementById('tv3-toggle-sell').classList.toggle('active', side === 'sell');
        const btn = document.getElementById('tv3-submit-btn');
        if (side === 'buy') {
            btn.textContent = 'Place Buy Order';
            btn.className = 'tv3-submit-btn tv3-submit-btn--buy';
        } else {
            btn.textContent = 'Place Sell Order';
            btn.className = 'tv3-submit-btn tv3-submit-btn--sell';
        }
    }

    // ── Init ──
    document.addEventListener('DOMContentLoaded', () => {
        const slug = getAssetSlug();
        const asset = ASSETS[slug] || ASSETS[DEFAULT_SLUG];

        populateHero(asset);
        populateDetails(asset);
        populateOrderBook(asset);
        buildChart(asset);

        document.getElementById('tv3-price').value = asset.tokenPrice.toFixed(2);
        updateSummary();

        // ── Performance Strip ──
        document.getElementById('tv3-ticker-price').textContent = fmt(asset.tokenPrice);
        document.getElementById('tv3-ticker-yield').textContent = asset.annualYield + '%';

        // Simulate 3M/6M/12M price changes from chart data
        const chartData = asset.chartData || [];
        const current = asset.tokenPrice;
        const price3m = chartData[Math.max(0, chartData.length - 90)] || current * 0.96;
        const price6m = chartData[Math.max(0, chartData.length - 180)] || current * 0.92;
        const price12m = chartData[0] || current * 0.87;
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

        // Inputs
        document.getElementById('tv3-qty').addEventListener('input', updateSummary);
        document.getElementById('tv3-price').addEventListener('input', updateSummary);

        // Buy/Sell toggle
        document.getElementById('tv3-toggle-buy').addEventListener('click', () => setSide('buy'));
        document.getElementById('tv3-toggle-sell').addEventListener('click', () => setSide('sell'));

        // Order Book tabs
        document.querySelectorAll('.tv3-tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.tv3-tab').forEach(t => t.classList.toggle('active', t === tab));
                const v = tab.dataset.tab;
                document.getElementById('tv3-sell-section').style.display = v === 'buy' ? 'none' : 'block';
                document.getElementById('tv3-buy-section').style.display = v === 'sell' ? 'none' : 'block';
            });
        });

        // Financial tabs
        document.querySelectorAll('.tv3-fin-tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.tv3-fin-tab').forEach(t => t.classList.toggle('active', t === tab));
                document.getElementById('tv3-fin-cost').style.display = tab.dataset.fin === 'cost' ? 'block' : 'none';
                document.getElementById('tv3-fin-rental').style.display = tab.dataset.fin === 'rental' ? 'block' : 'none';
            });
        });

        // ══ ORDER BOOK VOLUME BARS ══
        function applyVolumeBars() {
            // Sell rows
            const sellRows = document.querySelectorAll('#tv3-sell-rows tr');
            if (sellRows.length > 0) {
                const maxSell = Math.max(...asset.sellOrders.map(o => o.tokens));
                sellRows.forEach((row, i) => {
                    if (asset.sellOrders[i]) {
                        const pct = (asset.sellOrders[i].tokens / maxSell) * 100;
                        row.style.setProperty('--volume-pct', pct + '%');
                    }
                });
            }
            // Buy rows
            const buyRows = document.querySelectorAll('#tv3-buy-rows tr');
            if (buyRows.length > 0) {
                const maxBuy = Math.max(...asset.buyBids.map(o => o.tokens));
                buyRows.forEach((row, i) => {
                    if (asset.buyBids[i]) {
                        const pct = (asset.buyBids[i].tokens / maxBuy) * 100;
                        row.style.setProperty('--volume-pct', pct + '%');
                    }
                });
            }
        }
        applyVolumeBars();

        // Quick fill from order book
        document.addEventListener('click', e => {
            const btn = e.target.closest('.tv3-ob-action');
            if (btn) {
                document.getElementById('tv3-price').value = parseFloat(btn.dataset.price).toFixed(2);
                document.getElementById('tv3-qty').value = btn.dataset.qty;
                updateSummary();
                setSide(btn.classList.contains('tv3-ob-action--buy') ? 'buy' : 'sell');
                document.querySelector('.tv3-order-panel').scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        });

        // Submit
        document.getElementById('tv3-order-form').addEventListener('submit', e => {
            e.preventDefault();
            const btn = document.getElementById('tv3-submit-btn');
            const orig = btn.textContent;
            btn.textContent = '✓ Order Placed Successfully';
            btn.disabled = true;
            btn.style.opacity = '0.7';
            setTimeout(() => { btn.textContent = orig; btn.disabled = false; btn.style.opacity = '1'; }, 2500);
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
            if (sheetPrice) sheetPrice.value = asset.tokenPrice.toFixed(2);
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
        if (sheetSubmit) sheetSubmit.addEventListener('click', () => {
            const orig = sheetSubmit.textContent;
            sheetSubmit.textContent = '✓ Order Placed';
            sheetSubmit.disabled = true;
            setTimeout(() => { sheetSubmit.textContent = orig; sheetSubmit.disabled = false; closeSheet(); }, 2000);
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
