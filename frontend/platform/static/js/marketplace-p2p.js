// frontend/platform/static/js/marketplace-p2p.js
// P2P Offers, Cap Table (Holders), Tabs, 2FA Modal Logic

(function () {
    'use strict';

    // ── Mock Data ────────────────────────────────────────────
    const MOCK_ORDERS = [
        { id: 'ORD-7a3f01', side: 'buy', price: 10480, qty: 15, status: 'Open', created: '2026-03-20 14:32' },
        { id: 'ORD-9b2e03', side: 'sell', price: 10620, qty: 8, status: 'Open', created: '2026-03-20 13:15' },
        { id: 'ORD-c4d105', side: 'buy', price: 10350, qty: 25, status: 'Partial', created: '2026-03-19 09:45' },
    ];

    const MOCK_TRADES = [
        { time: '15:42:18', side: 'buy', price: 10500, qty: 10, fee: 105 },
        { time: '15:38:03', side: 'sell', price: 10480, qty: 5, fee: 52 },
        { time: '14:55:21', side: 'buy', price: 10520, qty: 20, fee: 210 },
        { time: '14:12:09', side: 'sell', price: 10450, qty: 8, fee: 84 },
        { time: '13:47:33', side: 'buy', price: 10490, qty: 12, fee: 126 },
    ];

    const MOCK_P2P_OFFERS = [
        {
            id: 'P2P-001',
            from: 'Investor #42',
            shares: 20,
            price: 10350,
            message: 'Quick trade, fair price.',
            type: 'incoming'
        },
    ];

    const MOCK_HOLDERS = [
        { name: 'Investor #7a3f', shares: 120, pct: 24.0, isYou: false },
        { name: 'Investor #b2c4', shares: 95, pct: 19.0, isYou: false },
        { name: 'You', shares: 80, pct: 16.0, isYou: true },
        { name: 'Investor #e5d1', shares: 65, pct: 13.0, isYou: false },
        { name: 'Investor #42', shares: 50, pct: 10.0, isYou: false },
    ];

    // ── Tab Switching ────────────────────────────────────────
    function initTabs() {
        const tabs = document.querySelectorAll('.mkt-tab');
        const contents = document.querySelectorAll('.mkt-tab-content');

        tabs.forEach(tab => {
            tab.addEventListener('click', () => {
                tabs.forEach(t => t.classList.remove('active'));
                contents.forEach(c => c.classList.remove('active'));

                tab.classList.add('active');
                const target = document.getElementById('tab-' + tab.dataset.tab);
                if (target) target.classList.add('active');
            });
        });
    }

    // ── My Open Orders ───────────────────────────────────────
    function renderMyOrders() {
        const body = document.getElementById('my-orders-body');
        if (!body) return;

        body.innerHTML = MOCK_ORDERS.map(o => `
            <tr>
                <td style="font-family:monospace;font-size:12px;">${o.id}</td>
                <td class="side-${o.side}">${o.side.toUpperCase()}</td>
                <td>$${(o.price / 100).toFixed(2)}</td>
                <td>${o.qty}</td>
                <td>${o.status}</td>
                <td style="color:var(--mkt-text-sec);font-size:12px;">${o.created}</td>
                <td><button class="mkt-cancel-btn" data-id="${o.id}">Cancel</button></td>
            </tr>
        `).join('');

        // Cancel buttons
        body.querySelectorAll('.mkt-cancel-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const row = btn.closest('tr');
                row.style.opacity = '0.3';
                btn.disabled = true;
                btn.textContent = 'Cancelling…';
                setTimeout(() => {
                    row.remove();
                    showToast('✅ Order cancelled', 'success');
                }, 800);
            });
        });
    }

    // ── Trade History ────────────────────────────────────────
    function renderTradeHistory() {
        const body = document.getElementById('trade-history-body');
        if (!body) return;

        body.innerHTML = MOCK_TRADES.map(t => {
            const total = (t.price * t.qty) / 100;
            return `
                <tr>
                    <td style="color:var(--mkt-text-sec);font-size:12px;">${t.time}</td>
                    <td class="side-${t.side}">${t.side.toUpperCase()}</td>
                    <td>$${(t.price / 100).toFixed(2)}</td>
                    <td>${t.qty}</td>
                    <td>$${total.toFixed(2)}</td>
                    <td style="color:var(--mkt-text-sec);">$${(t.fee / 100).toFixed(2)}</td>
                </tr>
            `;
        }).join('');
    }

    // ── P2P Offers ───────────────────────────────────────────
    function renderP2POffers() {
        const list = document.getElementById('p2p-offers-list');
        if (!list) return;

        if (MOCK_P2P_OFFERS.length === 0) {
            list.innerHTML = '<p style="color:var(--mkt-text-sec);text-align:center;padding:24px;">No P2P offers yet.</p>';
            return;
        }

        // Show badge
        const badge = document.getElementById('p2p-badge');
        const incomingCount = MOCK_P2P_OFFERS.filter(o => o.type === 'incoming').length;
        if (badge && incomingCount > 0) {
            badge.style.display = 'inline-flex';
            badge.textContent = incomingCount;
        }

        list.innerHTML = MOCK_P2P_OFFERS.map(o => `
            <div class="mkt-p2p-card" data-id="${o.id}">
                <div class="mkt-p2p-card__info">
                    <div class="mkt-p2p-card__user">${o.from}</div>
                    <div class="mkt-p2p-card__details">
                        Offers <strong>${o.shares} Shares</strong> @ <strong>$${(o.price / 100).toFixed(2)}</strong>
                        ${o.message ? `<br><em style="opacity:0.7;">"${o.message}"</em>` : ''}
                    </div>
                </div>
                <div class="mkt-p2p-card__actions">
                    <button class="mkt-p2p-accept" data-id="${o.id}">Accept</button>
                    <button class="mkt-p2p-decline" data-id="${o.id}">Decline</button>
                </div>
            </div>
        `).join('');

        // Accept / Decline handlers
        list.querySelectorAll('.mkt-p2p-accept').forEach(btn => {
            btn.addEventListener('click', () => {
                const card = btn.closest('.mkt-p2p-card');
                card.style.opacity = '0.3';
                btn.disabled = true;
                setTimeout(() => {
                    card.remove();
                    showToast('✅ P2P offer accepted', 'success');
                    if (badge) badge.style.display = 'none';
                }, 600);
            });
        });

        list.querySelectorAll('.mkt-p2p-decline').forEach(btn => {
            btn.addEventListener('click', () => {
                const card = btn.closest('.mkt-p2p-card');
                card.style.opacity = '0.3';
                btn.disabled = true;
                setTimeout(() => {
                    card.remove();
                    showToast('Offer declined', 'info');
                    if (badge) badge.style.display = 'none';
                }, 600);
            });
        });
    }

    // ── Holders / Cap Table ───────────────────────────────────
    function renderHolders() {
        const body = document.getElementById('holders-body');
        if (!body) return;

        body.innerHTML = MOCK_HOLDERS.map(h => `
            <tr class="${h.isYou ? 'mkt-holder-you' : ''}">
                <td>
                    <span style="font-weight:${h.isYou ? '700' : '500'};">${h.name}</span>
                    ${h.isYou ? '<span style="font-size:10px;color:var(--mkt-blue);margin-left:6px;">● You</span>' : ''}
                </td>
                <td>${h.shares}</td>
                <td>
                    ${h.pct.toFixed(1)}%
                    <div class="mkt-holder-bar-wrap">
                        <div class="mkt-holder-bar" style="width:${h.pct}%"></div>
                    </div>
                </td>
                <td>
                    ${!h.isYou ? `<button class="mkt-send-offer-btn" data-holder="${h.name}">Send Offer</button>` : ''}
                </td>
            </tr>
        `).join('');

        // Send Offer buttons → open P2P modal
        body.querySelectorAll('.mkt-send-offer-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                openP2PModal(btn.dataset.holder);
            });
        });
    }

    // ── P2P Modal ────────────────────────────────────────────
    function openP2PModal(holderName) {
        const modal = document.getElementById('p2p-modal');
        const recipient = document.getElementById('p2p-modal-recipient');
        if (!modal) return;

        if (recipient) recipient.textContent = `Offer to: ${holderName || 'Investor'}`;

        modal.style.display = 'flex';

        // Focus first input
        setTimeout(() => {
            const priceInput = document.getElementById('p2p-price');
            if (priceInput) priceInput.focus();
        }, 100);
    }

    function closeP2PModal() {
        const modal = document.getElementById('p2p-modal');
        if (modal) modal.style.display = 'none';

        // Reset fields
        ['p2p-price', 'p2p-qty', 'p2p-msg'].forEach(id => {
            const el = document.getElementById(id);
            if (el) el.value = '';
        });
    }

    function initP2PModal() {
        const closeBtn = document.getElementById('p2p-modal-close');
        const cancelBtn = document.getElementById('p2p-cancel-btn');
        const sendBtn = document.getElementById('p2p-send-btn');
        const overlay = document.getElementById('p2p-modal');

        if (closeBtn) closeBtn.addEventListener('click', closeP2PModal);
        if (cancelBtn) cancelBtn.addEventListener('click', closeP2PModal);

        // Click outside
        if (overlay) {
            overlay.addEventListener('click', (e) => {
                if (e.target === overlay) closeP2PModal();
            });
        }

        // Send offer
        if (sendBtn) {
            sendBtn.addEventListener('click', () => {
                sendBtn.disabled = true;
                sendBtn.innerHTML = '<span class="mkt-spinner"></span> Sending…';
                setTimeout(() => {
                    closeP2PModal();
                    showToast('📩 Offer sent successfully', 'success');
                    sendBtn.disabled = false;
                    sendBtn.textContent = 'Send Offer';
                }, 1200);
            });
        }
    }

    // ── 2FA Modal ────────────────────────────────────────────
    function open2FAModal(onVerified) {
        const modal = document.getElementById('twofa-modal');
        if (!modal) return;

        modal.style.display = 'flex';
        window._2faCallback = onVerified;

        // Clear and focus first digit
        const digits = modal.querySelectorAll('.mkt-2fa-digit');
        digits.forEach(d => { d.value = ''; });
        setTimeout(() => digits[0]?.focus(), 100);

        const verifyBtn = document.getElementById('twofa-verify-btn');
        if (verifyBtn) verifyBtn.disabled = true;
    }

    function close2FAModal() {
        const modal = document.getElementById('twofa-modal');
        if (modal) modal.style.display = 'none';
        window._2faCallback = null;
    }

    function init2FAModal() {
        const modal = document.getElementById('twofa-modal');
        if (!modal) return;

        const digits = modal.querySelectorAll('.mkt-2fa-digit');
        const verifyBtn = document.getElementById('twofa-verify-btn');

        // Auto-advance between digits
        digits.forEach((input, idx) => {
            input.addEventListener('input', (e) => {
                const val = e.target.value.replace(/\D/g, '');
                e.target.value = val.slice(0, 1);

                if (val && idx < digits.length - 1) {
                    digits[idx + 1].focus();
                }

                // Check if all filled
                const code = Array.from(digits).map(d => d.value).join('');
                if (verifyBtn) verifyBtn.disabled = (code.length < 6);
            });

            input.addEventListener('keydown', (e) => {
                if (e.key === 'Backspace' && !input.value && idx > 0) {
                    digits[idx - 1].focus();
                }
            });

            // Paste support
            input.addEventListener('paste', (e) => {
                e.preventDefault();
                const paste = (e.clipboardData || window.clipboardData).getData('text').replace(/\D/g, '');
                paste.split('').slice(0, 6).forEach((char, i) => {
                    if (digits[i]) digits[i].value = char;
                });
                const lastIdx = Math.min(paste.length, 6) - 1;
                if (digits[lastIdx]) digits[lastIdx].focus();
                if (verifyBtn && paste.length >= 6) verifyBtn.disabled = false;
            });
        });

        // Verify button
        if (verifyBtn) {
            verifyBtn.addEventListener('click', () => {
                verifyBtn.disabled = true;
                verifyBtn.innerHTML = '<span class="mkt-spinner"></span> Verifying…';

                setTimeout(() => {
                    close2FAModal();
                    showToast('✅ 2FA Verified', 'success');
                    verifyBtn.disabled = false;
                    verifyBtn.textContent = 'Verify & Submit Order';

                    if (typeof window._2faCallback === 'function') {
                        window._2faCallback();
                    }
                }, 1000);
            });
        }

        // Close on overlay click
        modal.addEventListener('click', (e) => {
            if (e.target === modal) close2FAModal();
        });

        // ESC
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                if (modal.style.display !== 'none') close2FAModal();
                const p2p = document.getElementById('p2p-modal');
                if (p2p && p2p.style.display !== 'none') closeP2PModal();
            }
        });
    }

    // ── Toast Utility ────────────────────────────────────────
    function showToast(message, type) {
        if (typeof window.showMarketToast === 'function') {
            window.showMarketToast(message, type);
            return;
        }
        const area = document.getElementById('market-toast-area');
        if (!area) { console.log(`[Toast] ${type}: ${message}`); return; }

        const div = document.createElement('div');
        div.className = `market-toast market-toast--${type || 'info'}`;
        div.textContent = message;
        area.appendChild(div);
        setTimeout(() => {
            div.style.opacity = '0';
            div.style.transform = 'translateX(100%)';
            div.style.transition = 'all 0.3s ease';
            setTimeout(() => div.remove(), 300);
        }, 4000);
    }

    // Expose globally
    window.showMarketToast = showToast;
    window.open2FAModal = open2FAModal;
    window.openP2PModal = openP2PModal;

    // ── Init ─────────────────────────────────────────────────
    document.addEventListener('DOMContentLoaded', () => {
        initTabs();
        renderMyOrders();
        renderTradeHistory();
        renderP2POffers();
        renderHolders();
        initP2PModal();
        init2FAModal();
    });
})();
