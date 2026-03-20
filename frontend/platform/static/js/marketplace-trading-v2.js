// frontend/platform/static/js/marketplace-trading-v2.js
// V2 Trading View — Property-focused, no charts, "Oma Prinzip"

(function () {
    'use strict';

    // ── Mock Asset Data ──
    const ASSETS = {
        'bali-villa-canggu-12': {
            slug: 'bali-villa-canggu-12',
            name: 'Bali Villa Canggu #12',
            type: 'Villa',
            location: 'Canggu, Bali, Indonesia',
            description: 'Contemporary villa steps from Echo Beach, Canggu\'s most popular surf break. Fully furnished with a private pool, modern kitchen, and outdoor living area. Currently rented and professionally managed.',
            tokenPrice: 105.00,
            annualYield: 12.4,
            occupancy: 87,
            totalSupply: 1000,
            propertyValue: 'USD 350,000',
            grossYield: '12.4%',
            netReturn: '9.2%',
            priceSqm: 'USD 2,734',
            landSize: '128 m²',
            bedrooms: 4,
            rentStatus: 'Rented',
            images: [
                '/static/images/villa1.webp',
                '/static/images/villa1.webp',
                '/static/images/villa1.webp',
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
        },
        'vienna-apartment-a3': {
            slug: 'vienna-apartment-a3',
            name: 'Vienna City Apartment A3',
            type: 'Apartment',
            location: 'Vienna, Austria',
            description: 'Central Vienna apartment near Stephansplatz. Premium location with stable long-term rental income. Professionally managed with full documentation.',
            tokenPrice: 87.50,
            annualYield: 8.2,
            occupancy: 95,
            totalSupply: 500,
            propertyValue: 'USD 180,000',
            grossYield: '8.2%',
            netReturn: '6.1%',
            priceSqm: 'USD 3,200',
            landSize: '56 m²',
            bedrooms: 2,
            rentStatus: 'Rented',
            images: ['/static/images/villa1.webp'],
            sellOrders: [
                { seller: 'Investor_E3', tokens: 5, price: 89.00 },
                { seller: 'Investor_G2', tokens: 12, price: 90.50 },
            ],
            buyBids: [],
        },
        'dubai-marina-tower-7': {
            slug: 'dubai-marina-tower-7',
            name: 'Dubai Marina Tower #7',
            type: 'Commercial',
            location: 'Dubai, UAE',
            description: 'Premium commercial unit in Dubai Marina with high-yield rental income. Fully occupied by a multinational corporate tenant on a 5-year lease.',
            tokenPrice: 152.00,
            annualYield: 15.1,
            occupancy: 92,
            totalSupply: 2000,
            propertyValue: 'USD 980,000',
            grossYield: '15.1%',
            netReturn: '11.8%',
            priceSqm: 'USD 5,440',
            landSize: '180 m²',
            bedrooms: 0,
            rentStatus: 'Leased (Corporate)',
            images: ['/static/images/villa1.webp'],
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
        },
    };

    // Default fallback
    const DEFAULT_SLUG = 'bali-villa-canggu-12';

    function getAssetSlug() {
        const params = new URLSearchParams(window.location.search);
        return params.get('asset') || DEFAULT_SLUG;
    }

    function formatUSD(val) {
        return '$' + val.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }

    // ── Populate Hero ──
    function populateHero(asset) {
        document.getElementById('tv2-breadcrumb-name').textContent = asset.name;
        document.getElementById('tv2-title').textContent = asset.name;
        document.getElementById('tv2-location-text').textContent = asset.location;
        document.getElementById('tv2-description').textContent = asset.description;
        document.getElementById('tv2-type-badge').textContent = asset.type;
        document.getElementById('tv2-status-badge').textContent = asset.sellOrders.length > 0 ? 'Active' : 'No Offers';
        document.getElementById('tv2-token-price').textContent = formatUSD(asset.tokenPrice);
        document.getElementById('tv2-yield').textContent = asset.annualYield + '%';
        document.getElementById('tv2-occupancy').textContent = asset.occupancy + '%';
        document.getElementById('tv2-total-supply').textContent = asset.totalSupply.toLocaleString();

        // Gallery
        const mainImg = document.getElementById('tv2-main-img');
        mainImg.src = asset.images[0] || '/static/images/villa1.webp';
        mainImg.alt = asset.name;

        const thumbContainer = document.getElementById('tv2-gallery-thumbs');
        thumbContainer.innerHTML = '';
        asset.images.forEach(function (src, i) {
            const thumb = document.createElement('div');
            thumb.className = 'tv2-thumb' + (i === 0 ? ' active' : '');
            thumb.innerHTML = '<img src="' + src + '" alt="Image ' + (i + 1) + '" />';
            thumb.addEventListener('click', function () {
                mainImg.src = src;
                document.querySelectorAll('.tv2-thumb').forEach(function (t) { t.classList.remove('active'); });
                thumb.classList.add('active');
            });
            thumbContainer.appendChild(thumb);
        });

        // Property Details
        document.getElementById('tv2-prop-value').textContent = asset.propertyValue;
        document.getElementById('tv2-gross-yield').textContent = asset.grossYield;
        document.getElementById('tv2-net-return').textContent = asset.netReturn;
        document.getElementById('tv2-price-sqm').textContent = asset.priceSqm;
        document.getElementById('tv2-prop-type').textContent = asset.type;
        document.getElementById('tv2-land-size').textContent = asset.landSize;
        document.getElementById('tv2-bedrooms').textContent = asset.bedrooms > 0 ? asset.bedrooms : 'N/A';
        document.getElementById('tv2-rent-status').textContent = asset.rentStatus;

        // Set default price
        document.getElementById('tv2-price').value = asset.tokenPrice.toFixed(2);
        updateOrderSummary();
    }

    // ── Populate Order Book ──
    function populateOrderBook(asset) {
        const sellRows = document.getElementById('tv2-sell-rows');
        const buyRows = document.getElementById('tv2-buy-rows');
        const sellEmpty = document.getElementById('tv2-sell-empty');
        const buyEmpty = document.getElementById('tv2-buy-empty');

        document.getElementById('tv2-sell-count').textContent = asset.sellOrders.length;
        document.getElementById('tv2-buy-count').textContent = asset.buyBids.length;

        // Sell orders
        sellRows.innerHTML = '';
        if (asset.sellOrders.length === 0) {
            sellEmpty.style.display = 'block';
        } else {
            sellEmpty.style.display = 'none';
            asset.sellOrders.forEach(function (o) {
                var row = document.createElement('tr');
                row.innerHTML =
                    '<td>' + o.seller + '</td>' +
                    '<td>' + o.tokens + '</td>' +
                    '<td>' + formatUSD(o.price) + '</td>' +
                    '<td>' + formatUSD(o.tokens * o.price) + '</td>' +
                    '<td><button class="tv2-ob-action-btn tv2-ob-action-btn--buy" data-price="' + o.price + '" data-qty="' + o.tokens + '">Buy</button></td>';
                sellRows.appendChild(row);
            });
        }

        // Buy bids
        buyRows.innerHTML = '';
        if (asset.buyBids.length === 0) {
            buyEmpty.style.display = 'block';
        } else {
            buyEmpty.style.display = 'none';
            asset.buyBids.forEach(function (o) {
                var row = document.createElement('tr');
                row.innerHTML =
                    '<td>' + o.buyer + '</td>' +
                    '<td>' + o.tokens + '</td>' +
                    '<td>' + formatUSD(o.price) + '</td>' +
                    '<td>' + formatUSD(o.tokens * o.price) + '</td>' +
                    '<td><button class="tv2-ob-action-btn tv2-ob-action-btn--sell" data-price="' + o.price + '" data-qty="' + o.tokens + '">Sell</button></td>';
                buyRows.appendChild(row);
            });
        }
    }

    // ── Order Summary Calculation ──
    function updateOrderSummary() {
        var qty = parseInt(document.getElementById('tv2-qty').value) || 0;
        var price = parseFloat(document.getElementById('tv2-price').value) || 0;
        var subtotal = qty * price;
        var fee = subtotal * 0.05;
        var total = subtotal + fee;

        document.getElementById('tv2-subtotal').textContent = formatUSD(subtotal);
        document.getElementById('tv2-fee').textContent = formatUSD(fee);
        document.getElementById('tv2-total').textContent = formatUSD(total);
    }

    // ── Buy/Sell Toggle ──
    var currentSide = 'buy';

    function setSide(side) {
        currentSide = side;
        var buyBtn = document.getElementById('tv2-toggle-buy');
        var sellBtn = document.getElementById('tv2-toggle-sell');
        var submitBtn = document.getElementById('tv2-submit-btn');

        buyBtn.classList.toggle('active', side === 'buy');
        sellBtn.classList.toggle('active', side === 'sell');

        if (side === 'buy') {
            submitBtn.textContent = 'Place Buy Order';
            submitBtn.className = 'tv2-submit-btn tv2-submit-btn--buy';
        } else {
            submitBtn.textContent = 'Place Sell Order';
            submitBtn.className = 'tv2-submit-btn tv2-submit-btn--sell';
        }
    }

    // ── Tab filtering ──
    function setTab(tab) {
        var sellSection = document.getElementById('tv2-sell-section');
        var buySection = document.getElementById('tv2-buy-section');

        document.querySelectorAll('.tv2-tab').forEach(function (t) {
            t.classList.toggle('active', t.dataset.tab === tab);
        });

        if (tab === 'sell') {
            sellSection.style.display = 'block';
            buySection.style.display = 'none';
        } else if (tab === 'buy') {
            sellSection.style.display = 'none';
            buySection.style.display = 'block';
        } else {
            sellSection.style.display = 'block';
            buySection.style.display = 'block';
        }
    }

    // ── Init ──
    document.addEventListener('DOMContentLoaded', function () {
        var slug = getAssetSlug();
        var asset = ASSETS[slug] || ASSETS[DEFAULT_SLUG];

        populateHero(asset);
        populateOrderBook(asset);

        // Inputs
        document.getElementById('tv2-qty').addEventListener('input', updateOrderSummary);
        document.getElementById('tv2-price').addEventListener('input', updateOrderSummary);

        // Buy/Sell toggle
        document.getElementById('tv2-toggle-buy').addEventListener('click', function () { setSide('buy'); });
        document.getElementById('tv2-toggle-sell').addEventListener('click', function () { setSide('sell'); });

        // Tabs
        document.querySelectorAll('.tv2-tab').forEach(function (tab) {
            tab.addEventListener('click', function () { setTab(tab.dataset.tab); });
        });

        // Quick fill from order book buttons
        document.addEventListener('click', function (e) {
            var btn = e.target.closest('.tv2-ob-action-btn');
            if (btn) {
                var price = btn.dataset.price;
                var qty = btn.dataset.qty;
                document.getElementById('tv2-price').value = parseFloat(price).toFixed(2);
                document.getElementById('tv2-qty').value = qty;
                updateOrderSummary();

                if (btn.classList.contains('tv2-ob-action-btn--buy')) {
                    setSide('buy');
                } else {
                    setSide('sell');
                }

                // Scroll to order form
                document.querySelector('.tv2-order-panel').scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        });

        // Submit
        document.getElementById('tv2-order-form').addEventListener('submit', function (e) {
            e.preventDefault();
            var submitBtn = document.getElementById('tv2-submit-btn');
            var origText = submitBtn.textContent;
            submitBtn.textContent = '✓ Order Placed Successfully';
            submitBtn.disabled = true;
            submitBtn.style.opacity = '0.7';
            setTimeout(function () {
                submitBtn.textContent = origText;
                submitBtn.disabled = false;
                submitBtn.style.opacity = '1';
            }, 2500);
        });
    });
})();
