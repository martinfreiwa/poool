// frontend/platform/static/js/marketplace-trading-v3.js
// V3 Trading — Simple Trade Widget (OP Principle, Variant 3: Market Price Toggle)

(function () {
    'use strict';

    // ── Mock Asset Data ──
    const ASSETS = {};

    const DEFAULT_SLUG = 'bali-villa-canggu-12';

    let currentUserEmail = null;

    function getAssetSlug() {
        return new URLSearchParams(window.location.search).get('asset') || DEFAULT_SLUG;
    }

    function fmt(val) {
        if (val == null || !isFinite(val)) return '—';
        return '$' + val.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    }
    function fmtInt(val) {
        if (val == null || !isFinite(val)) return '—';
        return 'USD ' + val.toLocaleString('en-US');
    }

    function readNumber(source, keys) {
        for (const key of keys) {
            const value = source && source[key];
            const number = Number(value);
            if (Number.isFinite(number)) return number;
        }
        return null;
    }

    function readPercent(source, percentKeys, bpsKeys) {
        const percent = readNumber(source, percentKeys);
        if (percent != null) return percent;
        const bps = readNumber(source, bpsKeys);
        return bps != null ? bps / 100 : null;
    }

    function fmtPct(value, opts = {}) {
        const number = Number(value);
        if (!Number.isFinite(number)) return '—';
        const formatted = number.toFixed(1).replace(/\.0$/, '');
        return (opts.sign && number > 0 ? '+' : '') + formatted + '%';
    }

    function setPerformanceValue(element, value) {
        if (!element) return;
        const number = Number(value);
        element.classList.remove('tv3-sp-value--green', 'tv3-sp-value--red');
        if (!Number.isFinite(number)) {
            element.textContent = '—';
            return;
        }
        element.textContent = fmtPct(number, { sign: true });
        element.classList.add(number >= 0 ? 'tv3-sp-value--green' : 'tv3-sp-value--red');
    }

    function getPlatformFeeRate() {
        const feePct = Number(window.POOOL_FEE_PCT);
        return Number.isFinite(feePct) && feePct >= 0 ? feePct / 100 : 0;
    }

    // ── Toast Notification ──
    function showTradeToast(message, type) {
        const existing = document.querySelector('.tv3-toast');
        if (existing) existing.remove();
        const toast = document.createElement('div');
        toast.className = 'tv3-toast tv3-toast--' + type;
        toast.setAttribute('role', 'alert');
        toast.innerHTML = message;
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

    // ── Order Confirmation Modal ──
    // Industry-standard pre-trade confirmation modal (FINRA 15c3-5 / MiFID II
    // RTS 7 disclosures): asset, side, price, qty, est. fill ladder, fee,
    // funds-locked vs net-proceeds figure, resting-order warning when no
    // immediate match, explicit affirmation checkbox for non-marketable
    // orders, and TIF disclosure. Returns true on confirm, false on cancel.
    function showOrderConfirmModal({ side, assetName, priceDisplay, quantity, orderType, totalValue, feeValue, grandTotal, restingOnly, partialFill, fillEstimate, tif }) {
        return new Promise((resolve) => {
            // Remove any existing modal
            const existing = document.getElementById('tv3-confirm-overlay');
            if (existing) existing.remove();

            const sideLabel = side === 'buy' ? 'Buy' : 'Sell';
            const sideColor = side === 'buy' ? '#00c896' : '#ef4444';
            const sideBg = side === 'buy' ? 'rgba(0,200,150,0.08)' : 'rgba(239,68,68,0.08)';
            const fmt = (v) => v == null || !isFinite(v)
                ? '—'
                : '$' + Math.abs(v).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

            // Build fill-ladder HTML when order spans multiple price tiers
            // (taker sweeping across asks/bids). Standard L2 disclosure.
            const tiers = (fillEstimate && fillEstimate.tiers) || [];
            const showLadder = tiers.length > 1;
            const ladderHtml = showLadder ? `
                <div style="
                    background:#fff; border:1px solid #eaecf0; border-radius:10px;
                    padding:10px 12px; margin-bottom:12px;
                ">
                    <div style="font-size:11px; font-weight:700; color:#667085; letter-spacing:0.04em; text-transform:uppercase; margin-bottom:6px;">
                        Estimated fill across ${tiers.length} price tier${tiers.length > 1 ? 's' : ''}
                    </div>
                    ${tiers.map(t => `
                        <div style="display:flex; justify-content:space-between; padding:3px 0; font-size:13px;">
                            <span style="color:#475467;">${t.qty} sh @ ${fmt(t.price)}</span>
                            <span style="color:#101828; font-weight:600;">${fmt(t.cost)}</span>
                        </div>`).join('')}
                    <div style="display:flex; justify-content:space-between; padding:6px 0 0; font-size:12px; color:#667085; border-top:1px solid #eaecf0; margin-top:4px;">
                        <span>VWAP</span>
                        <span style="color:#101828; font-weight:700;">${fmt(fillEstimate.vwap)}/share</span>
                    </div>
                </div>` : '';

            // Resting-order callout: nothing matches at the given limit price,
            // so order will sit in the book until cancel/expire/match.
            const restingHtml = restingOnly ? `
                <div style="
                    display:flex; align-items:flex-start; gap:10px; padding:12px 14px;
                    background:#EFF8FF; border:1px solid #B2DDFF; border-radius:10px;
                    font-size:12px; color:#175CD3; line-height:1.5; margin-bottom:12px;
                ">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#1570EF" stroke-width="2" style="flex-shrink:0; margin-top:1px;">
                        <circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>
                    </svg>
                    <span><strong>This is a resting limit order.</strong> No counterparty currently matches this price. Your order will sit in the order book until matched, cancelled, or expired (90 days). <strong>Fill is not guaranteed.</strong></span>
                </div>` : '';

            // Partial-fill callout: some quantity fills now, the rest rests.
            const partialHtml = partialFill ? `
                <div style="
                    display:flex; align-items:flex-start; gap:10px; padding:12px 14px;
                    background:#FFFAEB; border:1px solid #FEC84B; border-radius:10px;
                    font-size:12px; color:#93370D; line-height:1.5; margin-bottom:12px;
                ">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#B54708" stroke-width="2" style="flex-shrink:0; margin-top:1px;">
                        <path d="M12 9v4M12 17h.01"/><circle cx="12" cy="12" r="10"/>
                    </svg>
                    <span><strong>Partial fill expected.</strong> ${fillEstimate.filledQty} share${fillEstimate.filledQty === 1 ? '' : 's'} match now; remaining ${fillEstimate.unfilledQty} will rest as a limit order at ${fmt(priceDisplay)}.</span>
                </div>` : '';

            const overlay = document.createElement('div');
            overlay.id = 'tv3-confirm-overlay';
            overlay.className = 'ds-modal-overlay active';
            overlay.setAttribute('role', 'dialog');
            overlay.setAttribute('aria-modal', 'true');
            overlay.setAttribute('aria-labelledby', 'tv3-confirm-title');

            overlay.innerHTML = `
                <div class="ds-modal ds-modal--sm">
                    <div class="ds-modal__header">
                        <div style="display:flex; align-items:center; gap:12px;">
                            <div style="
                                width:40px; height:40px; border-radius:10px; flex-shrink:0;
                                background:${sideBg}; display:flex; align-items:center; justify-content:center;
                            ">
                                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="${sideColor}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                                    ${side === 'buy'
                                        ? '<path d="M12 19V5"/><polyline points="5 12 12 5 19 12"/>'
                                        : '<path d="M12 5v14"/><polyline points="19 12 12 19 5 12"/>'
                                    }
                                </svg>
                            </div>
                            <div>
                                <h3 class="ds-modal__title" id="tv3-confirm-title">Confirm ${sideLabel} Order</h3>
                                <p style="font-size:13px; color:#667085; margin:2px 0 0;">Please review your order details before confirming.</p>
                            </div>
                        </div>
                        <button id="tv3-confirm-close" class="ds-modal__close" aria-label="Close">
                            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                        </button>
                    </div>

                    <div class="ds-modal__body">
                        ${restingHtml}
                        ${partialHtml}
                        ${ladderHtml}
                        <!-- Order Details -->
                        <div style="
                            background: #f9fafb; border-radius: 12px; padding: 16px 18px;
                            border: 1px solid #eaecf0; margin-bottom: 16px;
                        ">
                            <div style="display:flex; justify-content:space-between; padding:6px 0; font-size:14px;">
                                <span style="color:#667085;">Asset</span>
                                <span style="font-weight:600; color:#101828; max-width:220px; text-align:right; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">${escapeHtml(assetName)}</span>
                            </div>
                            <div style="display:flex; justify-content:space-between; padding:6px 0; font-size:14px; border-top:1px solid #eaecf0;">
                                <span style="color:#667085;">Side</span>
                                <span style="font-weight:700; color:${sideColor};">${sideLabel.toUpperCase()}</span>
                            </div>
                            <div style="display:flex; justify-content:space-between; padding:6px 0; font-size:14px; border-top:1px solid #eaecf0;">
                                <span style="color:#667085;">Price (${orderType})</span>
                                <span style="font-weight:600; color:#101828;">${fmt(priceDisplay)} / share</span>
                            </div>
                            <div style="display:flex; justify-content:space-between; padding:6px 0; font-size:14px; border-top:1px solid #eaecf0;">
                                <span style="color:#667085;">Quantity</span>
                                <span style="font-weight:600; color:#101828;">${quantity} share${quantity > 1 ? 's' : ''}</span>
                            </div>
                            <div style="display:flex; justify-content:space-between; padding:6px 0; font-size:14px; border-top:1px solid #eaecf0;">
                                <span style="color:#667085;">Subtotal</span>
                                <span style="font-weight:600; color:#101828;">${fmt(totalValue)}</span>
                            </div>
                            <div style="display:flex; justify-content:space-between; padding:6px 0; font-size:14px; border-top:1px solid #eaecf0;">
                                <span style="color:#667085;">Platform Fee (${window.POOOL_FEE_DISPLAY || '5'}%)</span>
                                <span style="font-weight:600; color:#667085;">${side === 'buy' ? '+' : '−'}${fmt(feeValue)}</span>
                            </div>
                            <div style="display:flex; justify-content:space-between; padding:6px 0; font-size:13px; border-top:1px solid #eaecf0; color:#667085;">
                                <span>Time in force</span>
                                <span style="font-weight:600; color:#101828;">${(function() {
                                    switch ((tif || 'gtc').toLowerCase()) {
                                        case 'day': return 'Day (24h)';
                                        case 'ioc': return 'Immediate-or-Cancel';
                                        case 'fok': return 'Fill-or-Kill';
                                        default:    return 'Good-Til-Cancelled (90d)';
                                    }
                                })()}</span>
                            </div>
                            <div style="display:flex; justify-content:space-between; padding:10px 0 4px; font-size:16px; border-top:2px solid #d0d5dd; margin-top:4px;">
                                <span style="font-weight:700; color:#101828;">${side === 'buy' ? 'Funds locked' : 'Net proceeds'}</span>
                                <span style="font-weight:800; color:${sideColor}; font-size:18px;">${fmt(grandTotal)}</span>
                            </div>
                        </div>

                        <!-- Warning -->
                        <div style="
                            display:flex; align-items:flex-start; gap:10px; padding:12px 14px;
                            background:#FFFAEB; border:1px solid #FEC84B; border-radius:10px;
                            font-size:12px; color:#93370D; line-height:1.5;
                        ">
                            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#B54708" stroke-width="2" style="flex-shrink:0; margin-top:1px;">
                                <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
                            </svg>
                            <span>${side === 'buy'
                                ? `${fmt(grandTotal)} will be <strong>reserved (held)</strong> on your balance until the order matches with a seller. You can cancel any time before then to release the hold.`
                                : `Your shares will be listed for sale and held in escrow until matched with a buyer. You can cancel any time to return them.`
                            }</span>
                        </div>

                        <!-- Affirmation checkbox: required by FINRA 15c3-5 for
                             non-marketable orders. Always present for clarity. -->
                        <label id="tv3-affirm-row" style="
                            display:flex; align-items:flex-start; gap:10px; padding:12px 0 0;
                            font-size:12px; color:#475467; line-height:1.5; cursor:pointer;
                        ">
                            <input id="tv3-affirm-cb" type="checkbox" style="
                                margin-top:2px; width:16px; height:16px; flex-shrink:0; cursor:pointer;
                            "/>
                            <span>I understand that ${restingOnly
                                ? '<strong>this order may not fill</strong> and my funds remain held until match, cancellation, or expiry.'
                                : (side === 'buy'
                                    ? 'my funds will be <strong>held (not spent)</strong> until the order matches.'
                                    : 'my shares will be <strong>held in escrow</strong> until the order matches.')}
                            </span>
                        </label>
                    </div>

                    <div class="ds-modal__footer ds-modal__footer--bordered">
                        <button id="tv3-confirm-cancel" class="ds-btn ds-btn--secondary">Cancel</button>
                        <button id="tv3-confirm-ok" class="ds-btn ds-btn--primary" disabled style="opacity:0.5; cursor:not-allowed;">Confirm ${restingOnly ? 'Resting Order' : sideLabel}</button>
                    </div>
                </div>
            `;

            document.body.appendChild(overlay);

            // Focus confirm button
            const confirmBtn = overlay.querySelector('#tv3-confirm-ok');
            const cancelBtn = overlay.querySelector('#tv3-confirm-cancel');
            const closeBtn = overlay.querySelector('#tv3-confirm-close');
            setTimeout(() => confirmBtn.focus(), 30);

            function close(result) {
                document.removeEventListener('keydown', onKey);
                overlay.classList.remove('active');
                setTimeout(() => { if (overlay.parentNode) overlay.remove(); }, 200);
                resolve(result);
            }

            // Gate confirm button on affirmation checkbox.
            const affirmCb = overlay.querySelector('#tv3-affirm-cb');
            if (affirmCb) {
                affirmCb.addEventListener('change', () => {
                    if (affirmCb.checked) {
                        confirmBtn.disabled = false;
                        confirmBtn.style.opacity = '';
                        confirmBtn.style.cursor = '';
                    } else {
                        confirmBtn.disabled = true;
                        confirmBtn.style.opacity = '0.5';
                        confirmBtn.style.cursor = 'not-allowed';
                    }
                });
            }

            confirmBtn.addEventListener('click', () => {
                if (confirmBtn.disabled) return;
                close(true);
            });
            cancelBtn.addEventListener('click', () => close(false));
            closeBtn.addEventListener('click', () => close(false));
            overlay.addEventListener('click', (e) => { if (e.target === overlay) close(false); });

            function onKey(e) {
                if (e.key === 'Escape') { e.preventDefault(); close(false); }
            }
            document.addEventListener('keydown', onKey);
        });
    }

    function escapeHtml(str) {
        return String(str || '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
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

    function normalizeSecondaryLocation(rawLocation, rawCountry) {
        const location = String(rawLocation || '').trim();
        const country = String(rawCountry || '').trim();
        const locationParts = location.split(',').map(part => part.trim()).filter(Boolean);
        const city = locationParts[0] || 'N/A';
        const hasCountry = country
            && country !== 'N/A'
            && locationParts.some(part => part.toLowerCase() === country.toLowerCase());

        return {
            displayLocation: location
                ? (hasCountry || !country || country === 'N/A' ? location : `${location}, ${country}`)
                : (country || 'N/A'),
            country: country || (locationParts.length > 1 ? locationParts[locationParts.length - 1] : 'N/A'),
            city
        };
    }

    function mapOrderbookLevelsToOrders(levels) {
        return (Array.isArray(levels) ? levels : [])
            .map(level => ({
                tokens: Number(level.total_quantity ?? level.totalQuantity ?? 0),
                price: Number(level.price_cents ?? level.priceCents ?? 0) / 100,
                count: Number(level.order_count ?? level.orderCount ?? 0),
                // unique_users: distinct trader count at this level (always
                // <= count). Lets UI label "5 orders from 3 traders" honestly.
                uniqueUsers: Number(level.unique_users ?? level.uniqueUsers ?? 0)
            }))
            .filter(level => level.tokens > 0 && level.price > 0);
    }

    async function fetchLiveOrderbook(slug) {
        const res = await fetch(`/api/marketplace/${encodeURIComponent(slug)}/orderbook`);
        if (!res.ok) throw new Error('Orderbook fetch failed');
        const snapshot = await res.json();
        return {
            sellOrders: mapOrderbookLevelsToOrders(snapshot.asks),
            buyBids: mapOrderbookLevelsToOrders(snapshot.bids),
            lastPrice: snapshot.last_price_cents ?? snapshot.lastPriceCents ?? null
        };
    }

    // ── Populate Hero ──
    function populateHero(asset) {
        document.getElementById('tv3-bc-name').textContent = asset.name;
        document.getElementById('tv3-title').textContent = asset.name;
        document.getElementById('tv3-location-text').textContent = asset.location;

        // Financial hero stats
        document.getElementById('tv3-token-price').textContent = fmt(asset.tokenPrice);
        document.getElementById('tv3-yield').textContent = fmtPct(asset.annualYield, { sign: true });
        document.getElementById('tv3-prop-val').textContent = fmtInt(asset.propertyValue);
        document.getElementById('tv3-net-ret').textContent = fmtPct(asset.netReturn);
        const available = asset.totalSupply - (asset.sellOrders.reduce((s, o) => s + o.tokens, 0) || 0);
        document.getElementById('tv3-available').innerHTML = available.toLocaleString() + ' <small>/ ' + asset.totalSupply.toLocaleString() + '</small>';

        // Gallery — Mosaic layout
        const mainImg = document.getElementById('tv3-main-img');
        mainImg.classList.remove('loaded');
        mainImg.parentElement.classList.remove('img-loading-complete');
        mainImg.src = asset.images[0];
        mainImg.alt = asset.name;

        // Fill 4 mosaic grid thumbnails. Each thumb shows a UNIQUE image
        // (no duplicate-fill), and thumbs without a matching image are
        // hidden so the click → lightbox opens the correct index.
        const mosaicThumbs = document.querySelectorAll('.tv3-mosaic-thumb');
        mosaicThumbs.forEach((thumb, i) => {
            const img = thumb.querySelector('img:not(.tv3-loader-logo)');
            if (!img) return;
            img.classList.remove('loaded');
            img.parentElement.classList.remove('img-loading-complete');
            const imgIdx = i + 1;
            if (asset.images[imgIdx]) {
                thumb.style.display = '';
                img.src = asset.images[imgIdx];
                img.alt = asset.name + ' ' + (imgIdx + 1);
            } else {
                // No image at this slot — hide the entire cell instead of
                // rendering a duplicate that misleads the click-handler
                // into opening the wrong index in the lightbox.
                thumb.style.display = 'none';
                img.removeAttribute('src');
            }
        });

        // Click on ANY gallery image (main or thumb) → open lightbox at that index
        const galleryImages = asset.images.filter(Boolean);
        const mainEl = document.getElementById('tv3-gallery-main');
        if (mainEl) {
            mainEl.style.cursor = 'zoom-in';
            mainEl.onclick = () => openLightbox(galleryImages, 0, asset.name);
        }
        document.querySelectorAll('.tv3-mosaic-thumb').forEach((thumb, i) => {
            thumb.style.cursor = 'zoom-in';
            // Index in gallery: thumb 0 → image 1, thumb 1 → image 2, etc.
            // (main image already takes index 0)
            const idx = Math.min(i + 1, galleryImages.length - 1);
            thumb.onclick = () => openLightbox(galleryImages, idx, asset.name);
        });
    }

    // ── Lightbox / Fullscreen Image Viewer ──
    function openLightbox(images, startIndex, alt) {
        if (!images || images.length === 0) return;

        // Remove any existing lightbox first (idempotent)
        document.getElementById('tv3-lightbox')?.remove();

        let idx = Math.max(0, Math.min(startIndex, images.length - 1));
        let touchStartX = 0;
        let touchStartY = 0;
        let touchDeltaX = 0;
        let isSwiping = false;
        const returnFocusEl = document.activeElement instanceof HTMLElement ? document.activeElement : null;

        const overlay = document.createElement('div');
        overlay.id = 'tv3-lightbox';
        overlay.className = 'lightbox-modal lightbox-opening';
        overlay.style.display = 'flex';
        overlay.setAttribute('role', 'dialog');
        overlay.setAttribute('aria-modal', 'true');
        overlay.setAttribute('aria-label', alt ? alt + ' gallery' : 'Asset gallery');

        const topBar = document.createElement('div');
        topBar.className = 'lightbox-top-bar';

        const counter = document.createElement('span');
        counter.className = 'lightbox-counter';

        const close = document.createElement('button');
        close.className = 'lightbox-close';
        close.type = 'button';
        close.setAttribute('aria-label', 'Close gallery');
        close.innerHTML = `
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                <path d="M18 6L6 18M6 6L18 18" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"></path>
            </svg>
        `;
        topBar.appendChild(counter);
        topBar.appendChild(close);

        const imageWrapper = document.createElement('div');
        imageWrapper.className = 'lightbox-image-wrapper';

        const img = document.createElement('img');
        img.className = 'lightbox-content';
        img.alt = alt || '';
        img.src = images[idx];
        img.onclick = (e) => e.stopPropagation();
        imageWrapper.appendChild(img);

        const thumbnails = document.createElement('div');
        thumbnails.className = 'lightbox-thumbnails';

        const renderCounter = () => {
            counter.textContent = `${idx + 1} / ${images.length}`;
        };

        const updateThumbnails = () => {
            thumbnails.querySelectorAll('.lightbox-thumb').forEach((thumb, thumbIdx) => {
                thumb.classList.toggle('active', thumbIdx === idx);
            });
            const active = thumbnails.querySelector('.lightbox-thumb.active');
            if (active) {
                active.scrollIntoView({ behavior: 'smooth', inline: 'center', block: 'nearest' });
            }
        };

        const showImage = () => {
            img.src = images[idx];
            renderCounter();
            updateThumbnails();
        };

        images.forEach((src, thumbIdx) => {
            const thumb = document.createElement('button');
            thumb.className = 'lightbox-thumb';
            thumb.type = 'button';
            thumb.setAttribute('aria-label', 'Go to image ' + (thumbIdx + 1));
            const thumbImg = document.createElement('img');
            thumbImg.alt = alt ? alt + ' ' + (thumbIdx + 1) : '';
            thumbImg.draggable = false;
            thumbImg.src = src;
            thumb.appendChild(thumbImg);
            thumb.addEventListener('click', (e) => {
                e.stopPropagation();
                if (thumbIdx === idx) return;
                idx = thumbIdx;
                showImage();
            });
            thumbnails.appendChild(thumb);
        });

        const navBtn = (className, label, path, onClick) => {
            const b = document.createElement('button');
            b.className = className;
            b.type = 'button';
            b.setAttribute('aria-label', label);
            b.innerHTML = `
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                    ${path}
                </svg>
            `;
            b.onclick = (e) => { e.stopPropagation(); onClick(); };
            return b;
        };

        const goPrev = () => {
            idx = (idx - 1 + images.length) % images.length;
            showImage();
        };
        const goNext = () => {
            idx = (idx + 1) % images.length;
            showImage();
        };

        const prev = navBtn(
            'lightbox-prev',
            'Previous image',
            '<path d="M15 18L9 12L15 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"></path>',
            goPrev
        );
        const next = navBtn(
            'lightbox-next',
            'Next image',
            '<path d="M9 18L15 12L9 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"></path>',
            goNext
        );

        overlay.appendChild(topBar);
        overlay.appendChild(imageWrapper);
        if (images.length > 1) {
            overlay.appendChild(prev);
            overlay.appendChild(next);
        }
        overlay.appendChild(thumbnails);
        document.body.appendChild(overlay);
        document.body.style.overflow = 'hidden';
        document.body.classList.add('lightbox-open');
        showImage();
        setTimeout(() => overlay.classList.remove('lightbox-opening'), 300);

        const closeLightbox = () => {
            overlay.classList.add('lightbox-closing');
            setTimeout(() => {
                overlay.remove();
                document.body.style.overflow = '';
                document.body.classList.remove('lightbox-open');
                document.removeEventListener('keydown', onKey);
                if (returnFocusEl && document.contains(returnFocusEl)) {
                    returnFocusEl.focus();
                }
            }, 300);
        };
        const onKey = (e) => {
            if (e.key === 'Escape') closeLightbox();
            else if (e.key === 'ArrowLeft' && images.length > 1) goPrev();
            else if (e.key === 'ArrowRight' && images.length > 1) goNext();
        };

        imageWrapper.addEventListener('touchstart', (e) => {
            if (e.touches.length !== 1) return;
            touchStartX = e.touches[0].clientX;
            touchStartY = e.touches[0].clientY;
            touchDeltaX = 0;
            isSwiping = false;
        }, { passive: true });

        imageWrapper.addEventListener('touchmove', (e) => {
            if (e.touches.length !== 1 || images.length <= 1) return;
            const dx = e.touches[0].clientX - touchStartX;
            const dy = e.touches[0].clientY - touchStartY;
            if (!isSwiping && Math.abs(dx) > 10 && Math.abs(dx) > Math.abs(dy)) {
                isSwiping = true;
            }
            if (!isSwiping) return;
            touchDeltaX = dx;
            e.preventDefault();
            img.style.transition = 'none';
            img.style.transform = 'translateX(' + dx + 'px)';
            img.style.opacity = Math.max(0.4, 1 - Math.abs(dx) / 400).toString();
        }, { passive: false });

        imageWrapper.addEventListener('touchend', () => {
            if (!isSwiping) return;
            isSwiping = false;
            img.style.transition = '';
            img.style.transform = '';
            img.style.opacity = '';
            if (touchDeltaX < -50) goNext();
            else if (touchDeltaX > 50) goPrev();
            touchDeltaX = 0;
        }, { passive: true });

        overlay.addEventListener('click', (e) => {
            if (e.target === overlay || e.target === imageWrapper) closeLightbox();
        });
        close.onclick = (e) => {
            e.stopPropagation();
            closeLightbox();
        };
        document.addEventListener('keydown', onKey);
        close.focus();
    }

    // ── Populate Property Details ──
    function populateDetails(asset) {
        document.getElementById('tv3-description').textContent = asset.description;

        // Property info
        document.getElementById('tv3-prop-value').textContent = fmtInt(asset.propertyValue);
        document.getElementById('tv3-gross-yield').textContent = fmtPct(asset.annualYield);
        document.getElementById('tv3-net-return').textContent = fmtPct(asset.netReturn);
        document.getElementById('tv3-price-sqm').textContent = fmtInt(asset.priceSqm);
        document.getElementById('tv3-prop-type').textContent = asset.type;
        document.getElementById('tv3-prop-land').textContent = asset.landSize;
        document.getElementById('tv3-prop-beds').textContent = asset.bedrooms > 0 ? asset.bedrooms : 'N/A';
        document.getElementById('tv3-prop-status').textContent = asset.rentStatus;

        // Info badges
        document.getElementById('tv3-info-country').textContent = asset.country + ', ' + asset.city;
        document.getElementById('tv3-info-status').textContent = asset.rentStatus;
        document.getElementById('tv3-info-yield').textContent = fmtPct(asset.annualYield) + ' annual rental yield';
        document.getElementById('tv3-info-growth').textContent = fmtPct(asset.annualYield) + ' annual gross yield';
        document.getElementById('tv3-info-net').textContent = asset.netReturn == null
            ? 'Net yield unavailable. Price per m²: USD ' + asset.priceSqm.toLocaleString()
            : 'With a net yield of ' + fmtPct(asset.netReturn) + ' and price per m² of USD ' + asset.priceSqm.toLocaleString();

        // Financials
        document.getElementById('tv3-fin-price').textContent = fmtInt(asset.propertyValue);
        document.getElementById('tv3-fin-fee').textContent = '+ ' + fmtInt(asset.platformFee);
        document.getElementById('tv3-fin-total').textContent = '= ' + fmtInt(asset.propertyValue + asset.platformFee);
        document.getElementById('tv3-fin-gross').textContent = fmtPct(asset.annualYield);
        document.getElementById('tv3-fin-proj').textContent = fmtPct(asset.projectedReturn);
        document.getElementById('tv3-fin-net').textContent = fmtPct(asset.netReturn);
        document.getElementById('tv3-fin-note2').textContent = 'Based on ' + fmtPct(asset.annualYield) + ' annual rental yield';

        // Location
        const locSubtitle = document.getElementById('tv3-loc-subtitle');
        if (locSubtitle) locSubtitle.textContent = asset.city + ', ' + asset.country;
        document.getElementById('tv3-loc-desc').textContent = asset.locationDesc || '';
    }

    // ── Trade Widget: Market Data ──
    // CLOB semantics: bestPrice is null when opposing side empty. UI MUST NOT
    // substitute primary-issue tokenPrice — that quote misleads users into
    // posting phantom bids/asks at a price no counterparty agreed to.
    function getMarketData(asset, side) {
        if (side === 'buy') {
            // Buying: show sell offers (cheapest first)
            const sorted = [...asset.sellOrders].sort((a, b) => a.price - b.price);
            const totalShares = sorted.reduce((s, o) => s + o.tokens, 0);
            const orderCount = sorted.reduce((s, o) => s + (o.count || 1), 0);
            // Approximate distinct traders across price levels: max() over
            // levels (a single trader's orders at multiple prices counts once).
            // Frontend can't perfectly dedupe without per-order user IDs.
            const uniqueUsers = sorted.reduce((m, o) => Math.max(m, Number(o.uniqueUsers || 0)), 0);
            const bestPrice = sorted.length > 0 ? sorted[0].price : null;
            return { bestPrice, totalShares, count: orderCount, uniqueUsers, orders: sorted };
        } else {
            // Selling: show buy bids (highest first)
            const sorted = [...asset.buyBids].sort((a, b) => b.price - a.price);
            const totalShares = sorted.reduce((s, o) => s + o.tokens, 0);
            const orderCount = sorted.reduce((s, o) => s + (o.count || 1), 0);
            const uniqueUsers = sorted.reduce((m, o) => Math.max(m, Number(o.uniqueUsers || 0)), 0);
            const bestPrice = sorted.length > 0 ? sorted[0].price : null;
            return { bestPrice, totalShares, count: orderCount, uniqueUsers, orders: sorted };
        }
    }

    // Fetch user's holdings + open sell orders for the current asset, then
    // recompute `userSellable`. Called on asset load and after each
    // orderbook_update WS event (which fires after the user's own orders too,
    // so held figure stays fresh). Falls back to 0 sellable if the user is
    // not authenticated — backend will reject anyway.
    async function refreshUserHoldings() {
        if (!currentAsset || !currentAsset.id) return;
        try {
            const [portfolio, myOrders] = await Promise.all([
                fetch('/api/portfolio', { credentials: 'same-origin' }).then(r =>
                    r.ok ? r.json() : null),
                fetch('/api/marketplace/orders/mine?limit=200', { credentials: 'same-origin' })
                    .then(r => (r.ok ? r.json() : [])),
            ]);
            const inv = portfolio?.investments?.find(i => i.asset_id === currentAsset.id);
            userOwnedTokens = inv ? Number(inv.tokens_owned) : 0;

            // Sum remaining qty on this user's open SELL orders for this asset.
            // Use only open / partially_filled rows (others are terminal).
            const orders = Array.isArray(myOrders) ? myOrders : (myOrders?.orders || []);
            userHeldFromSells = orders
                .filter(o =>
                    o.asset_id === currentAsset.id &&
                    o.side === 'sell' &&
                    (o.status === 'open' || o.status === 'partially_filled'))
                .reduce((s, o) => s + (Number(o.quantity) - Number(o.quantity_filled || 0)), 0);

            userSellable = Math.max(0, userOwnedTokens - userHeldFromSells);
        } catch (e) {
            userOwnedTokens = 0;
            userHeldFromSells = 0;
            userSellable = 0;
        }
        applySellQtyCap();
        updateSummary();
        renderDepthLadder();
    }

    // Apply ownership cap to the qty input + render an inline holdings hint.
    // Only enforces the cap on the SELL side; buyer cap is handled by balance.
    function applySellQtyCap() {
        const qtyInput = document.getElementById('tv3-qty');
        if (!qtyInput) return;

        if (currentSide === 'sell') {
            // Hard cap input attribute so browser validation kicks in too.
            qtyInput.max = String(Math.max(1, userSellable));
            const cur = parseInt(qtyInput.value, 10) || 0;
            if (userSellable === 0) {
                // No sellable shares — clamp to 0 and let updateSummary disable submit.
                qtyInput.value = '0';
            } else if (cur > userSellable) {
                qtyInput.value = String(userSellable);
            }
        } else {
            // On buy side, remove the cap (use a large default).
            qtyInput.max = '';
        }
        renderHoldingsHint();
    }

    // Inline "Owned: 100 · 30 in open sell orders · 70 sellable" line so
    // sellers see at a glance what they actually have to work with. Mirrors
    // FINRA position-disclosure norms — always show the user their position
    // before they enter a quantity.
    function renderHoldingsHint() {
        let hint = document.getElementById('tv3-holdings-hint');
        const sharesField = document.querySelector('.tv3-shares-field');
        if (!sharesField) return;

        if (currentSide !== 'sell') {
            if (hint) hint.hidden = true;
            return;
        }

        if (!hint) {
            hint = document.createElement('div');
            hint.id = 'tv3-holdings-hint';
            hint.style.cssText =
                'font-size:12px; color:#475467; margin-top:4px; line-height:1.4;';
            sharesField.appendChild(hint);
        }
        hint.hidden = false;

        if (userOwnedTokens <= 0) {
            hint.innerHTML =
                '<span style="color:#b91c1c; font-weight:600;">⚠ You don\'t own any shares of this asset.</span>';
            return;
        }
        const heldTxt = userHeldFromSells > 0
            ? ` · <span style="color:#b54708;">${userHeldFromSells.toLocaleString()} held in open sells</span>`
            : '';
        hint.innerHTML =
            `<strong>${userOwnedTokens.toLocaleString()}</strong> owned${heldTxt} · ` +
            `<strong>${userSellable.toLocaleString()}</strong> sellable now`;
    }

    // Render the live depth ladder above the cost summary. Shows the opposing
    // side the user will actually fill against (buyer sees asks; seller sees
    // bids). Rows that the chosen quantity consumes are highlighted, with a
    // running cumulative total + VWAP. This is the FINRA Rule 5310 / MiFID II
    // best-execution disclosure done in-line, not buried in a confirmation
    // modal — sellers see "9 @ $800, then 10 @ $700, then 10 @ $600..."
    // before they ever click sell.
    function renderDepthLadder() {
        const wrap = document.getElementById('tv3-depth-ladder');
        const title = document.getElementById('tv3-depth-ladder-title');
        const vwapEl = document.getElementById('tv3-depth-ladder-vwap');
        const rowsEl = document.getElementById('tv3-depth-ladder-rows');
        const emptyEl = document.getElementById('tv3-depth-ladder-empty');
        if (!wrap || !rowsEl || !currentAsset) return;

        const data = getMarketData(currentAsset, currentSide);
        const orders = data.orders || [];
        const isBuy = currentSide === 'buy';

        // Title reflects the side the user is acting against.
        if (title) {
            title.textContent = isBuy
                ? `Asks (sellers) · ${orders.length} ${orders.length === 1 ? 'level' : 'levels'}`
                : `Bids (buyers) · ${orders.length} ${orders.length === 1 ? 'level' : 'levels'}`;
        }

        if (!orders.length) {
            rowsEl.innerHTML = '';
            if (emptyEl) {
                emptyEl.hidden = false;
                emptyEl.textContent = isBuy
                    ? 'No asks — sellers will see your bid; rest until matched.'
                    : 'No bids — buyers will see your ask; rest until matched.';
            }
            if (vwapEl) vwapEl.textContent = '';
            return;
        }
        if (emptyEl) emptyEl.hidden = true;

        const requestedQty = parseInt(document.getElementById('tv3-qty')?.value, 10) || 0;
        const fill = simulateFill(orders, requestedQty);
        const fillByPrice = new Map();
        fill.tiers.forEach(t => fillByPrice.set(t.price, (fillByPrice.get(t.price) || 0) + t.qty));

        // Show top 8 levels max — keeps the widget compact, deeper book is
        // visible in the admin orderbook page.
        const visible = orders.slice(0, 8);
        let cumQty = 0;
        const html = visible.map((o, i) => {
            cumQty += o.tokens;
            const fillsAtThis = fillByPrice.get(o.price) || 0;
            const isHit = fillsAtThis > 0;
            const isExhausted = fillsAtThis >= o.tokens;
            const partialFill = isHit && !isExhausted;
            const priceColor = isBuy ? '#ef4444' : '#15803d'; // red ask / green bid
            const rowBg = isHit
                ? (isBuy ? 'rgba(239,68,68,0.10)' : 'rgba(34,197,94,0.10)')
                : 'transparent';
            const fillTag = isHit
                ? `<span style="font-size:10px; padding:1px 6px; border-radius:8px; background:${priceColor}; color:#fff; font-weight:700; margin-left:6px;">${
                    isExhausted ? 'FILL' : `${fillsAtThis}/${o.tokens}`
                }</span>`
                : '';
            return `
                <div style="
                    display:grid; grid-template-columns: 1fr auto auto; gap:8px;
                    padding:4px 6px; border-radius:6px; background:${rowBg};
                    align-items:center;
                ">
                    <span style="color:${priceColor}; font-weight:700;">${fmt(o.price)}${fillTag}</span>
                    <span style="color:#475467;">${o.tokens.toLocaleString()} sh</span>
                    <span style="color:#98a2b3; font-size:11px;">cum ${cumQty.toLocaleString()}</span>
                </div>
            `;
        }).join('');
        rowsEl.innerHTML = html;

        // Show VWAP across the consumed tiers + unfilled note when applicable.
        if (vwapEl) {
            if (fill.filledQty > 0) {
                const vwap = fmt(fill.vwap);
                const unfilled = fill.unfilledQty > 0
                    ? ` · ${fill.unfilledQty} unfilled (would rest)`
                    : '';
                vwapEl.textContent = `Avg ${vwap} for ${fill.filledQty}${unfilled}`;
            } else {
                vwapEl.textContent = requestedQty > 0
                    ? 'No matching depth at your price'
                    : '';
            }
        }
    }

    // Walk the opposing book greedily for `qty` shares. Returns tier breakdown,
    // VWAP, filled/unfilled split. Used for multi-level fill preview + worst-case
    // (limit) cost. Pre-trade cost transparency required by FINRA 15c3-5.
    function simulateFill(orders, qty) {
        const tiers = [];
        let remaining = Math.max(0, Math.floor(qty));
        let cumCost = 0;
        let cumQty = 0;
        for (const o of orders) {
            if (remaining <= 0) break;
            const take = Math.min(o.tokens, remaining);
            tiers.push({ qty: take, price: o.price, cost: take * o.price });
            cumCost += take * o.price;
            cumQty += take;
            remaining -= take;
        }
        return {
            tiers,
            filledQty: cumQty,
            unfilledQty: remaining,
            vwap: cumQty > 0 ? cumCost / cumQty : null,
            subtotal: cumCost,
        };
    }

    // Whether opposing side has any liquidity. When false, Market mode must be
    // disabled and order forced to limit ("Place bid"/"Place ask" UX).
    function hasOpposingLiquidity(asset, side) {
        return getMarketData(asset, side).totalShares > 0;
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
            // The number is total SHARES wanted across all open buy bids,
            // not the count of orders. Earlier label said "buy offers" which
            // misled users into thinking it was order count.
            depthBuy.textContent = buyData.totalShares > 0
                ? buyData.totalShares + ' shares wanted from ' + fmt(buyData.bestPrice)
                : 'No buy offers placed';
        }
    }

    function populateCalculator(asset) {
        const slider = document.getElementById('tv3-calc-slider-1');
        const limitLabel = document.getElementById('tv3-calc-slider-limit-1');
        if (slider && asset.propertyValue) {
            slider.max = Math.round(asset.propertyValue);
            // If the current value is still the default 100k, we set it to the max if max < 100k, 
            // or keep it at 100k if max > 100k.
            if (slider.value === "100000") {
                slider.value = Math.min(100000, asset.propertyValue).toString();
            }
            if (limitLabel) {
                limitLabel.textContent = '$' + new Intl.NumberFormat('en-US').format(Math.round(asset.propertyValue));
            }
            // Trigger track fill update
            const event = new Event('input');
            slider.dispatchEvent(event);
        }
    }


    // ── Trade Widget State ──
    let currentSide = 'buy';
    let priceMode = 'market'; // 'market' or 'custom'
    let currentAsset = null;
    // User's holdings for the current asset, used to cap sell quantity at the
    // shares the user actually owns minus shares already locked in open sell
    // orders. Backend re-validates inside the create_order tx (FOR UPDATE on
    // the investments row) so this is UX guidance, not a security boundary.
    let userOwnedTokens = 0;       // total tokens user holds for this asset
    let userHeldFromSells = 0;     // sum of remaining qty across user's open sells
    let userSellable = 0;          // = userOwnedTokens - userHeldFromSells

    function getActivePrice() {
        if (priceMode === 'market') {
            const data = getMarketData(currentAsset, currentSide);
            return data.bestPrice; // may be null when book empty
        } else {
            const v = parseFloat(document.getElementById('tv3-price').value);
            return isFinite(v) && v > 0 ? v : null;
        }
    }

    // ── Update Summary ──
    // CLOB cost model:
    //   Buyer:  locked = subtotal + fee_reserve (taker fee — refunded if maker)
    //   Seller: net   = subtotal − fee
    // The widget displays "Total" as funds LOCKED for buyer, NET PROCEEDS for seller.
    // When opposing book empty, order rests as limit — label as "Place bid/ask" not
    // "Buy/Sell" to set expectation that fill is not guaranteed.
    function updateSummary() {
        if (!currentAsset) return;

        const qty = parseInt(document.getElementById('tv3-qty').value) || 0;
        const price = getActivePrice();
        const feePct = getPlatformFeeRate();
        const hasPrice = price != null && price > 0 && qty > 0;

        // VWAP-aware subtotal:
        //   - Walk the opposing book at acceptable prices, sum the actual
        //     fill cost across consumed tiers (VWAP × filledQty).
        //   - Any unfilled remainder is valued at the user's limit price
        //     (= what they'll lock as buyer / receive at-best as seller if
        //     the remainder eventually fills at limit).
        // This replaces the previous (price × qty) calc which silently
        // assumed every share fills at best-bid/best-ask. With a thin book,
        // that misled sellers by tens of thousands of $.
        let subtotal = 0;
        if (hasPrice) {
            const data = getMarketData(currentAsset, currentSide);
            const matchable = (data.orders || []).filter(o =>
                currentSide === 'buy' ? o.price <= price : o.price >= price
            );
            const fill = simulateFill(matchable, qty);
            const unfilledAtLimit = fill.unfilledQty * price;
            subtotal = fill.subtotal + unfilledAtLimit;
        }

        const fee = subtotal * feePct;
        // Buyer pays subtotal + fee (locked). Seller receives subtotal − fee (net).
        const total = currentSide === 'buy' ? subtotal + fee : subtotal - fee;

        document.getElementById('tv3-subtotal').textContent = hasPrice ? fmt(subtotal) : '—';
        document.getElementById('tv3-fee').textContent = hasPrice ? fmt(fee) : '—';
        document.getElementById('tv3-total').textContent = hasPrice ? fmt(total) : '—';

        // Relabel total row by side ("Locked" for buyer, "Net proceeds" for seller).
        const totalLabel = document.querySelector('#tv3-order-summary-total-label')
            || document.querySelector('.tv3-sum-row--total span:first-child');
        if (totalLabel) {
            totalLabel.textContent = currentSide === 'buy' ? 'Locked' : 'Net proceeds';
        }

        // Submit button state machine
        const btn = document.getElementById('tv3-submit-btn');
        const opposingHasLiquidity = hasOpposingLiquidity(currentAsset, currentSide);
        const isMarketWithoutLiquidity = priceMode === 'market' && !opposingHasLiquidity;

        const setDisabled = (label) => {
            btn.textContent = label;
            btn.disabled = true;
            btn.style.opacity = '0.5';
            btn.style.cursor = 'not-allowed';
            btn.style.fontSize = '12px';
        };
        const setEnabled = (label) => {
            btn.textContent = label;
            btn.disabled = false;
            btn.style.opacity = '1';
            btn.style.cursor = 'pointer';
            btn.style.fontSize = '';
        };

        if (isMarketWithoutLiquidity) {
            setDisabled(currentSide === 'buy'
                ? 'No asks available — switch to Custom Price to place a bid'
                : 'No bids available — switch to Custom Price to place an ask');
            return;
        }
        if (!hasPrice) {
            setDisabled(currentSide === 'buy' ? 'Enter price and quantity' : 'Enter price and quantity');
            return;
        }

        // Sell-side ownership gate: can't sell more than you actually hold
        // (minus shares already locked in your open sell orders). Backend
        // enforces this transactionally; UI gate prevents rejection round-trip.
        if (currentSide === 'sell') {
            if (userOwnedTokens <= 0) {
                setDisabled('You don\'t own shares of this asset');
                return;
            }
            if (qty > userSellable) {
                setDisabled(`Max ${userSellable.toLocaleString()} sellable (${userHeldFromSells.toLocaleString()} held in open sells)`);
                return;
            }
        }

        const shareText = qty === 1 ? 'Share' : 'Shares';
        // "Place bid/ask" framing when this order will rest (no opposing liquidity).
        // "Buy/Sell" framing when order will (likely) match instantly.
        const action = !opposingHasLiquidity
            ? (currentSide === 'buy' ? 'Place Bid for' : 'Place Ask for')
            : (currentSide === 'buy' ? 'Buy' : 'Sell');
        setEnabled(action + ' ' + qty + ' ' + shareText + ' · ' + fmt(total));

        // Re-render depth ladder so highlighted rows reflect current qty.
        try { renderDepthLadder(); } catch (_) { /* widget may not be present */ }
    }

    // ── Set Buy/Sell Side ──
    function setSide(side) {
        currentSide = side;
        const btnBuyToggle = document.getElementById('tv3-toggle-buy');
        const btnSellToggle = document.getElementById('tv3-toggle-sell');
        
        if (btnBuyToggle) btnBuyToggle.classList.toggle('active', side === 'buy');
        if (btnSellToggle) btnSellToggle.classList.toggle('active', side === 'sell');

        const btn = document.getElementById('tv3-submit-btn');
        btn.className = side === 'buy'
            ? 'tv3-submit-btn tv3-submit-btn--buy'
            : 'tv3-submit-btn tv3-submit-btn--sell';

        // Update market info for the active side
        updateMarketInfo();
        applySellQtyCap();
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
    // Labels reflect actual book state. Counts are ORDERS (not unique users —
    // backend doesn't dedupe by user_id, so "1 buyer" was a lie). When no
    // opposing liquidity, force Custom Price mode and hint user to place a
    // resting order. Display reference info (best opposing price, depth) only
    // when it exists; never substitute primary-issue tokenPrice.
    function updateMarketInfo() {
        if (!currentAsset) return;
        const data = getMarketData(currentAsset, currentSide);
        const hasLiquidity = data.totalShares > 0;

        const bestPriceEl = document.getElementById('tv3-best-price');
        const availSharesEl = document.getElementById('tv3-avail-shares');
        const availSellersEl = document.getElementById('tv3-avail-sellers');
        const marketInfoEl = document.getElementById('tv3-market-info');
        const customHint = document.getElementById('tv3-custom-hint');
        const marketModeBtn = document.getElementById('tv3-mode-market');

        // Best price: show real value or em-dash. NEVER fall back to tokenPrice.
        if (bestPriceEl) bestPriceEl.textContent = data.bestPrice != null ? fmt(data.bestPrice) : '—';

        if (availSharesEl) {
            availSharesEl.textContent = hasLiquidity
                ? data.totalShares + (data.totalShares === 1 ? ' share' : ' shares')
                : '0 shares';
        }
        // count = aggregated order count. uniqueUsers = distinct traders.
        // Label both when they differ ("5 orders from 3 traders") so users see
        // depth concentration. Falls back to orders-only when uniqueUsers
        // unavailable (older payload).
        if (availSellersEl) {
            const orders = data.count;
            const traders = Number(data.uniqueUsers || 0);
            const ordersTxt = orders + (orders === 1 ? ' order' : ' orders');
            if (traders > 0 && traders < orders) {
                availSellersEl.textContent =
                    ordersTxt + ' from ' + traders + (traders === 1 ? ' trader' : ' traders');
            } else {
                availSellersEl.textContent = ordersTxt;
            }
        }

        if (marketInfoEl) {
            marketInfoEl.style.background = 'transparent';
            marketInfoEl.style.borderColor = 'var(--tv3-border)';
            if (bestPriceEl) bestPriceEl.style.color = 'var(--tv3-text)';
        }

        // Label: real-state aware
        const infoLabel = document.querySelector('.tv3-market-info-label');
        if (infoLabel) {
            if (!hasLiquidity) {
                infoLabel.textContent = currentSide === 'buy'
                    ? 'No asks — place a resting bid'
                    : 'No bids — place a resting ask';
            } else {
                infoLabel.textContent = currentSide === 'buy' ? 'Best ask' : 'Best bid';
            }
        }

        // Custom price hint: only show real reference when book non-empty.
        if (customHint) {
            customHint.textContent = data.bestPrice != null
                ? (currentSide === 'buy' ? 'Best ask: ' : 'Best bid: ') + fmt(data.bestPrice) + '/share'
                : (currentSide === 'buy'
                    ? 'No asks in book — your bid will rest until a seller matches.'
                    : 'No bids in book — your ask will rest until a buyer matches.');
        }

        // Disable Market mode when no opposing liquidity; force Custom mode.
        if (marketModeBtn) {
            if (!hasLiquidity) {
                marketModeBtn.disabled = true;
                marketModeBtn.style.opacity = '0.45';
                marketModeBtn.style.cursor = 'not-allowed';
                marketModeBtn.title = currentSide === 'buy'
                    ? 'Market buy disabled — no asks. Use Custom Price to place a bid.'
                    : 'Market sell disabled — no bids. Use Custom Price to place an ask.';
                if (priceMode === 'market') {
                    // Auto-flip to custom so user lands in correct flow.
                    setPriceMode('custom');
                }
            } else {
                marketModeBtn.disabled = false;
                marketModeBtn.style.opacity = '';
                marketModeBtn.style.cursor = '';
                marketModeBtn.title = '';
            }
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
            // Only prefill when a real opposing-side price exists. When book is
            // empty, leave input blank — forcing the user to consciously enter
            // a price. Avoids silently posting a bid at primary-issue tokenPrice.
            if (priceInput && !priceInput.value && data.bestPrice != null) {
                priceInput.value = data.bestPrice.toFixed(2);
            }
            // Make custom price input recompute summary live.
            if (priceInput && !priceInput._tv3Wired) {
                priceInput.addEventListener('input', updateSummary);
                priceInput._tv3Wired = true;
            }
        }

        updateSummary();
    }

    // ── Init ──
    document.addEventListener('DOMContentLoaded', async () => {
        const slug = getAssetSlug();

        // Check user identity
        try {
            const res = await fetch('/api/me');
            if (res.ok) {
                const data = await res.json();
                currentUserEmail = data.email || data.user?.email;
            }
        } catch (e) {
            console.warn('Could not fetch user profile:', e);
        }

        let asset;
        try {
            const res = await fetch('/api/marketplace/secondary/assets');
            if (!res.ok) throw new Error('API fetch failed');
            const secondaryAssets = await res.json();
            const rawAsset = secondaryAssets.find(a => a.slug === slug);
            
            if (!rawAsset) {
                document.getElementById('tv3-title').textContent = 'Asset Not Found';
                return;
            }

            let liveOrderbook = null;
            try {
                liveOrderbook = await fetchLiveOrderbook(rawAsset.slug);
            } catch (orderbookError) {
                console.warn('Could not fetch live orderbook, using secondary aggregate fallback:', orderbookError);
            }

            const buyInterest = rawAsset.buyInterest ?? rawAsset.buy_interest ?? 0;
            const locationParts = normalizeSecondaryLocation(rawAsset.location, rawAsset.country);
            const fallbackSellOrders = rawAsset.sellOrders > 0
                ? [{ tokens: rawAsset.sellOrders, price: rawAsset.price / 100, count: 1 }]
                : [];
            const fallbackBuyBids = buyInterest > 0
                ? [{ tokens: buyInterest, price: rawAsset.price / 100, count: 1 }]
                : [];
            const sellOrders = liveOrderbook ? liveOrderbook.sellOrders : fallbackSellOrders;
            const buyBids = liveOrderbook ? liveOrderbook.buyBids : fallbackBuyBids;
            const displayPriceCents = liveOrderbook?.lastPrice || rawAsset.price;
            const annualYield = readPercent(rawAsset, ['roi', 'annualYield', 'annual_yield'], ['annual_yield_bps']);
            const platformFeeRate = getPlatformFeeRate();

            // Map backend structure to UI standard
            asset = {
                slug: rawAsset.slug,
                name: rawAsset.name,
                type: rawAsset.type,
                location: locationParts.displayLocation,
                country: locationParts.country,
                city: locationParts.city,
                description: rawAsset.description || 'No description available for this property.',
                tokenPrice: displayPriceCents / 100,
                annualYield,
                projectedReturn: readPercent(
                    rawAsset,
                    ['projectedReturn', 'projected_return', 'projected_return_pct'],
                    ['projected_return_bps']
                ),
                netReturn: readPercent(
                    rawAsset,
                    ['netReturn', 'net_return', 'netYield', 'net_yield'],
                    ['net_return_bps', 'net_yield_bps']
                ),
                occupancy: rawAsset.occupancy || 100,
                totalSupply: rawAsset.totalSupply,
                propertyValue: rawAsset.propertyValue ? rawAsset.propertyValue / 100 : 0,
                priceSqm: rawAsset.propertyValue && rawAsset.landSize ? Math.round((rawAsset.propertyValue / 100) / parseFloat(rawAsset.landSize || '100')) : 0,
                landSize: rawAsset.landSize || 'N/A',
                bedrooms: rawAsset.bedrooms || 0,
                rentStatus: rawAsset.rentStatus || 'N/A',
                platformFee: rawAsset.propertyValue ? (rawAsset.propertyValue / 100) * platformFeeRate : 0,
                images: rawAsset.images && rawAsset.images.length > 0 ? rawAsset.images : ['/static/images/seed/villa1.webp'],
                sellOrders,
                buyBids,
                performance: {
                    threeMonth: readPercent(rawAsset, ['performance3m', 'performance_3m', 'performance_3m_pct', 'return3m', 'return_3m'], ['performance_3m_bps', 'return_3m_bps']),
                    sixMonth: readPercent(rawAsset, ['performance6m', 'performance_6m', 'performance_6m_pct', 'return6m', 'return_6m'], ['performance_6m_bps', 'return_6m_bps']),
                    twelveMonth: readPercent(rawAsset, ['performance12m', 'performance_12m', 'performance_12m_pct', 'return12m', 'return_12m'], ['performance_12m_bps', 'return_12m_bps'])
                },
                locationDesc: rawAsset.locationDesc || '',
                fundingStatus: rawAsset.fundingStatus,
                // Carry through the asset UUID — needed for WebSocket subscribe.
                id: rawAsset.id,
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
        populateCalculator(asset);

        // Initialize trade widget
        updateMarketInfo();
        updateSummary();
        renderDepthLadder();

        // Fetch user holdings + open orders so we can cap sell qty at what
        // the user actually owns (minus shares already locked in open sells).
        // Backend re-validates inside the create_order tx — this is UX-only.
        try {
            await refreshUserHoldings();
        } catch (e) {
            console.warn('[tv3] Could not fetch user holdings:', e);
        }

        // ── Live orderbook via WebSocket ──
        // Server-pushed orderbook_update events replace the prior 5-second
        // poll. Falls back gracefully if WS infra missing (e.g. older bundle).
        // Updates `currentAsset.sellOrders / buyBids` in place, then re-renders
        // the trade widget. Hero "available" count updates too so users see
        // depth changes immediately without a page refresh.
        if (window.MarketWS && window.MarketBus && asset?.id) {
            // Bus listener BEFORE connect so we don't miss the first message
            // delivered while the socket settles.
            window.MarketBus.on('orderbook:update', (msg) => {
                if (!currentAsset) return;
                // Server may key by asset slug or UUID — accept both.
                if (
                    msg.asset_id &&
                    msg.asset_id !== currentAsset.id &&
                    msg.asset_id !== currentAsset.slug
                ) {
                    return;
                }
                currentAsset.sellOrders = mapOrderbookLevelsToOrders(msg.asks);
                currentAsset.buyBids = mapOrderbookLevelsToOrders(msg.bids);

                // Re-render every panel that reads orderbook data.
                populateTradeWidget(currentAsset);
                updateMarketInfo();
                updateSummary();

                // Refresh holdings — fills against this user's resting orders
                // change `held_tokens`, so sellable cap can drift between
                // matches. Don't await; UI updates async when it lands.
                refreshUserHoldings().catch(() => { /* silent */ });

                // Hero "shares available" derives from sellOrders depth.
                const available =
                    currentAsset.totalSupply -
                    (currentAsset.sellOrders.reduce((s, o) => s + o.tokens, 0) || 0);
                const availEl = document.getElementById('tv3-available');
                if (availEl) {
                    availEl.innerHTML =
                        available.toLocaleString() +
                        ' <small>/ ' +
                        currentAsset.totalSupply.toLocaleString() +
                        '</small>';
                }
            });
            try {
                window.MarketWS.connect(asset.id);
            } catch (e) {
                console.warn('[tv3] MarketWS.connect failed; falling back to no-WS mode:', e);
            }
        }

        // ── Performance Strip ──
        document.getElementById('tv3-ticker-price').textContent = fmt(asset.tokenPrice);
        document.getElementById('tv3-ticker-yield').textContent = fmtPct(asset.annualYield);

        const el3m = document.getElementById('tv3-ticker-3m');
        const el6m = document.getElementById('tv3-ticker-6m');
        const el12m = document.getElementById('tv3-ticker-12m');
        setPerformanceValue(el3m, asset.performance && asset.performance.threeMonth);
        setPerformanceValue(el6m, asset.performance && asset.performance.sixMonth);
        setPerformanceValue(el12m, asset.performance && asset.performance.twelveMonth);

        // ── Mobile footer ──
        const mobilePrice = document.getElementById('tv3-mobile-price');
        const mobileYield = document.getElementById('tv3-mobile-yield');
        if (mobilePrice) mobilePrice.textContent = fmt(asset.tokenPrice);
        if (mobileYield) mobileYield.textContent = fmtPct(asset.annualYield, { sign: true }) + ' yield';

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

        // Submit — real API call with confirmation modal + double-click guard
        let isSubmitting = false;
        // Idempotency key: one per page load, prevents duplicate orders on back-button resubmit
        const _idemKey = crypto.randomUUID();
        sessionStorage.setItem('tv3_idem_key', _idemKey);
        document.getElementById('tv3-order-form').addEventListener('submit', async (e) => {
            e.preventDefault();
            if (isSubmitting) return; // Prevent double-click
            // Prevent back-button resubmit: key is cleared on success
            const idemKey = sessionStorage.getItem('tv3_idem_key');
            if (!idemKey) {
                showTradeToast('Order already submitted. Go to My Trading to track it.', 'error');
                return;
            }

            const btn = document.getElementById('tv3-submit-btn');
            const qty = parseInt(document.getElementById('tv3-qty').value) || 0;
            if (qty <= 0) {
                showTradeToast('Please enter a valid quantity', 'error');
                return;
            }

            const orderType = priceMode === 'market' ? 'market' : 'limit';
            const data = getMarketData(currentAsset, currentSide);
            let priceCents = null;
            let priceDisplay = 0;
            if (orderType === 'limit') {
                const priceVal = parseFloat(document.getElementById('tv3-price').value);
                if (!priceVal || priceVal <= 0) {
                    showTradeToast('Please enter a valid price', 'error');
                    return;
                }
                priceCents = Math.round(priceVal * 100);
                priceDisplay = priceVal;
            } else {
                // Hard guard: market order with no opposing liquidity must NEVER
                // submit. Backend rejects with NO_LIQUIDITY but UI must catch
                // first to prevent a malformed (NaN price) request hitting API.
                if (data.bestPrice == null) {
                    showTradeToast(currentSide === 'buy'
                        ? 'No asks in book — switch to Custom Price to place a bid.'
                        : 'No bids in book — switch to Custom Price to place an ask.', 'error');
                    return;
                }
                priceCents = Math.round(data.bestPrice * 100);
                priceDisplay = data.bestPrice;
            }

            const totalValue = priceDisplay * qty;
            const feeRate = getPlatformFeeRate();
            const feeValue = totalValue * feeRate;
            const grandTotal = currentSide === 'buy' ? totalValue + feeValue : totalValue - feeValue;

            // Pre-trade fill simulation against the opposing book at this price.
            // Used to flag resting-only orders + show tier breakdown in modal.
            const opposingOrders = currentSide === 'buy' ? data.orders : data.orders;
            const matchableOrders = opposingOrders.filter(o =>
                currentSide === 'buy' ? o.price <= priceDisplay : o.price >= priceDisplay
            );
            const fill = simulateFill(matchableOrders, qty);
            const restingOnly = fill.filledQty === 0; // nothing matches at this limit
            const partialFill = fill.filledQty > 0 && fill.unfilledQty > 0;

            const tifValue = document.getElementById('tv3-tif')?.value || 'gtc';

            // ── Show Confirmation Modal ──
            const confirmed = await showOrderConfirmModal({
                side: currentSide,
                assetName: currentAsset?.title || currentAsset?.name || 'Asset',
                priceDisplay: priceDisplay,
                quantity: qty,
                orderType: orderType,
                totalValue: totalValue,
                feeValue: feeValue,
                grandTotal: grandTotal,
                restingOnly: restingOnly,
                partialFill: partialFill,
                fillEstimate: fill,
                tif: tifValue,
            });
            if (!confirmed) return;

            // ── Submit Order ──
            isSubmitting = true;
            const orig = btn.textContent;
            btn.textContent = 'Placing Order…';
            btn.disabled = true;
            btn.style.opacity = '0.7';

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
                        idempotency_key: idemKey,
                        time_in_force: (document.getElementById('tv3-tif')?.value || 'gtc')
                    })
                });
                const result = await res.json();
                if (res.ok) {
                    // Clear idempotency key so back-button resubmit is blocked
                    sessionStorage.removeItem('tv3_idem_key');
                    sessionStorage.setItem('trade_success', JSON.stringify({
                        side: currentSide,
                        asset: currentAsset?.title || currentAsset?.name || 'Asset',
                        qty: String(qty),
                        price: priceDisplay.toFixed(2),
                        subtotal: totalValue.toFixed(2),
                        fee: feeValue.toFixed(2),
                        total: grandTotal.toFixed(2),
                        order_id: result.order_id || result.id || '',
                        slug: asset.slug || ''
                    }));
                    window.location.href = '/trade-success';
                } else if (res.status === 428) {
                    // Step-up 2FA required (trades >= $500). Send to the
                    // step-up verify page; if user isn't enrolled yet it
                    // server-redirects to /auth/2fa/setup. After verify
                    // (or enrollment), user is sent back here via return_to.
                    const returnTo = encodeURIComponent(window.location.pathname + window.location.search);
                    showTradeToast('Two-factor authentication required…', 'info');
                    setTimeout(() => {
                        window.location.href = `/auth/2fa/step-up?return_to=${returnTo}&action=trade`;
                    }, 600);
                } else {
                    // Dispatch on stable error_code (FINRA/MiFID structured
                    // error envelope). Falls back to the human message.
                    const code = result.error_code;
                    const msg = result.error || 'Order failed';
                    if (code === 'NO_LIQUIDITY') {
                        // Backend confirms zero opposing depth — force user
                        // into Custom Price flow so they can place a resting
                        // bid/ask. UI was supposed to catch this client-side
                        // but the book may have changed between render + submit.
                        if (priceMode === 'market') setPriceMode('custom');
                        showTradeToast(msg, 'error');
                    } else if (code === 'PRICE_COLLAR_BREACH') {
                        // Highlight the price input and bring user's attention.
                        const priceInput = document.getElementById('tv3-price');
                        if (priceInput) {
                            priceInput.style.borderColor = '#ef4444';
                            priceInput.style.boxShadow = '0 0 0 3px rgba(239,68,68,0.15)';
                            priceInput.focus();
                            priceInput.select();
                            setTimeout(() => {
                                priceInput.style.borderColor = '';
                                priceInput.style.boxShadow = '';
                            }, 4000);
                        }
                        showTradeToast(msg, 'error');
                    } else if (code === 'CONCENTRATION_LIMIT') {
                        showTradeToast(msg, 'error');
                    } else if (code === 'INSUFFICIENT_BALANCE' || code === 'INSUFFICIENT_TOKENS') {
                        showTradeToast(msg, 'error');
                    } else if (code === 'TOO_MANY_OPEN_ORDERS') {
                        showTradeToast(msg, 'error');
                    } else if (code === 'DUPLICATE_IDEMPOTENCY_KEY') {
                        // Order already accepted — clear key so user can place new one.
                        sessionStorage.removeItem('tv3_idem_key');
                        showTradeToast(msg, 'error');
                    } else {
                        showTradeToast(msg, 'error');
                    }
                    btn.textContent = orig; btn.disabled = false; btn.style.opacity = '1';
                    isSubmitting = false;
                }
            } catch (err) {
                console.error('Order submission failed:', err);
                showTradeToast('Network error — please try again', 'error');
                btn.textContent = orig; btn.disabled = false; btn.style.opacity = '1';
                isSubmitting = false;
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
            // Default sheet to current widget side (not always 'buy') so the
            // price prefill matches what the user is doing on the desktop card.
            const side = currentSide || 'buy';
            const data = getMarketData(asset, side);
            // Only prefill when opposing book has liquidity; never substitute
            // primary-issue tokenPrice. Empty input forces user to pick a price.
            if (sheetPrice) {
                sheetPrice.value = data.bestPrice != null ? data.bestPrice.toFixed(2) : '';
            }
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
            const subtotal = q * p;
            const fee = subtotal * getPlatformFeeRate();
            // Side-aware total: buyer locks subtotal+fee, seller nets subtotal−fee.
            const sheetSide = sheetBuy?.classList.contains('active') ? 'buy' : 'sell';
            const total = sheetSide === 'buy' ? subtotal + fee : subtotal - fee;
            if (sheetTotal) sheetTotal.textContent = fmt(total);
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
        let isSheetSubmitting = false;
        if (sheetSubmit) sheetSubmit.addEventListener('click', async () => {
            if (isSheetSubmitting) return;

            const sheetSide = sheetBuy?.classList.contains('active') ? 'buy' : 'sell';
            const qty = parseInt(sheetQty?.value) || 0;
            const priceVal = parseFloat(sheetPrice?.value) || 0;
            if (qty <= 0 || priceVal <= 0) {
                showTradeToast('Enter valid price and quantity', 'error');
                return;
            }

            const totalValue = priceVal * qty;
            const feeValue = totalValue * getPlatformFeeRate();
            const grandTotal = sheetSide === 'buy' ? totalValue + feeValue : totalValue - feeValue;

            const confirmed = await showOrderConfirmModal({
                side: sheetSide,
                assetName: currentAsset?.title || currentAsset?.name || asset?.slug || 'Asset',
                priceDisplay: priceVal,
                quantity: qty,
                orderType: 'limit',
                totalValue: totalValue,
                feeValue: feeValue,
                grandTotal: grandTotal,
            });
            if (!confirmed) return;

            isSheetSubmitting = true;
            const orig = sheetSubmit.textContent;
            sheetSubmit.textContent = 'Placing…';
            sheetSubmit.disabled = true;

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
                    sessionStorage.setItem('trade_success', JSON.stringify({
                        side: sheetSide,
                        asset: currentAsset?.title || currentAsset?.name || 'Asset',
                        qty: String(qty),
                        price: priceVal.toFixed(2),
                        total: grandTotal.toFixed(2),
                        order_id: result.order_id || result.id || '',
                        slug: asset.slug || ''
                    }));
                    window.location.href = '/trade-success';
                } else {
                    showTradeToast(result.error || 'Order failed', 'error');
                    sheetSubmit.textContent = orig; sheetSubmit.disabled = false;
                    isSheetSubmitting = false;
                }
            } catch (err) {
                showTradeToast('Network error', 'error');
                sheetSubmit.textContent = orig; sheetSubmit.disabled = false;
                isSheetSubmitting = false;
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
            slider.style.background = 'transparent'; // Clear inline background that causes thick track
            slider.style.setProperty('--slider-progress', pct + '%');
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

// ═══════════════════════════════════════
// FAQ SECTION
// ═══════════════════════════════════════
(function() {
    document.addEventListener('DOMContentLoaded', function() {
        var faqItems = document.querySelectorAll('.faq-item');
        if (faqItems.length === 0) return;

        faqItems.forEach(function(item) {
            var itemContent = item.querySelector('.faq-item-content');
            if (itemContent) {
                itemContent.addEventListener('click', function() {
                    var isActive = item.classList.contains('active');
                    
                    // Close all
                    faqItems.forEach(function(t) { t.classList.remove('active'); });
                    
                    // If it wasn't active, open it
                    if (!isActive) {
                        item.classList.add('active');
                    }
                });
            }
        });
    });
})();
