/**
 * Payment Success Page JS
 * Loads order details from the API and populates the confirmation page.
 * Handles both wallet (immediate success) and bank transfer (pending) flows.
 */
(function () {
  'use strict';

  document.addEventListener('DOMContentLoaded', async function () {
    const params = new URLSearchParams(window.location.search);
    const orderId = params.get('order_id') || params.get('id');

    try {
      let orderData;

      if (orderId) {
        const resp = await fetch('/api/orders/' + orderId);
        if (!resp.ok) throw new Error('Failed to load order');
        orderData = await resp.json();
      } else {
        const resp = await fetch('/api/orders/latest');
        if (!resp.ok) throw new Error('No recent order found');
        orderData = await resp.json();
      }

      renderOrderDetails(orderData);
    } catch (e) {
      showFallback();
    }
  });

  function renderOrderDetails(order) {
    const isBank = order.payment_method === 'bank';
    const isPending = order.status === 'pending' || isBank;

    // Update icon
    var iconEl = document.getElementById('status-icon');
    if (iconEl) {
      if (isPending) {
        iconEl.className = 'payment-success-icon payment-success-icon--pending';
        iconEl.innerHTML = [
          '<svg width="36" height="36" viewBox="0 0 24 24" fill="none">',
          '  <path d="M12 6V12L16 14" stroke="#B54708" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>',
          '  <circle cx="12" cy="12" r="10" stroke="#B54708" stroke-width="2.5"/>',
          '</svg>'
        ].join('');
      }
    }

    // Update title
    var titleEl = document.getElementById('status-title');
    if (titleEl) {
      titleEl.textContent = isPending ? 'Payment In Progress' : 'Payment Successful';
    }

    // Update description
    var descEl = document.getElementById('status-description');
    if (descEl) {
      if (isPending) {
        descEl.textContent = 'Your bank transfer order has been received. We will verify your payment and update your portfolio within 1–3 business days.';
      } else {
        descEl.textContent = 'Your tokens have been added to your portfolio. You can view your transactions and download the invoice from your settings.';
      }
    }

    // Show bank pending notice
    if (isPending) {
      var noticeEl = document.getElementById('bank-pending-notice');
      if (noticeEl) noticeEl.style.display = 'flex';
    }

    // Populate order fields
    var numEl = document.getElementById('display-order-number');
    if (numEl) numEl.textContent = '#' + (order.order_number || order.id || '—');

    var dateEl = document.getElementById('display-order-date');
    if (dateEl && order.created_at) {
      dateEl.textContent = new Date(order.created_at).toLocaleDateString('en-US', {
        year: 'numeric', month: 'long', day: 'numeric'
      });
    }

    var currency = order.payment_currency || order.currency || 'USD';

    var methodEl = document.getElementById('display-payment-method');
    if (methodEl) {
      var method = isBank ? 'Bank Transfer' : 'Wallet';
      methodEl.textContent = currency + ' ' + method;
    }

    var totalEl = document.getElementById('display-total-amount');
    if (totalEl && typeof order.total_cents === 'number') {
      totalEl.textContent = formatMoney(order.total_cents, currency);
    }

    // Render order items
    var itemsContainer = document.getElementById('order-items-list');
    if (itemsContainer && order.items && order.items.length > 0) {
      var html = '<div class="payment-success-order__items-title">Items Purchased</div>';
      order.items.forEach(function (item) {
        var subtotal = item.total_cents || (item.token_price_cents * item.tokens_quantity);
        html += [
          '<div class="payment-success-order__item">',
          '  <span class="payment-success-order__item-name">',
          '    ' + esc(item.tokens_quantity) + '× ' + esc(item.asset_title || 'Asset'),
          '  </span>',
          '  <span class="payment-success-order__item-price">',
          '    ' + esc(formatMoney(subtotal, currency)),
          '  </span>',
          '</div>'
        ].join('');
      });
      itemsContainer.innerHTML = html;
    }

    // Show the order details block
    var loadingEl = document.getElementById('order-loading');
    var detailsEl = document.getElementById('order-details');
    if (loadingEl) loadingEl.style.display = 'none';
    if (detailsEl) detailsEl.style.display = 'block';
  }

  function showFallback() {
    var loadingEl = document.getElementById('order-loading');
    var detailsEl = document.getElementById('order-details');
    if (loadingEl) loadingEl.style.display = 'none';
    if (detailsEl) {
      detailsEl.style.display = 'block';
      detailsEl.innerHTML = '<div style="text-align:center;color:#717680;padding:16px 0;">Order completed. <a href="/portfolio" style="color:var(--primary-color);font-weight:600;">View your portfolio</a> to see your assets.</div>';
    }
  }

  function esc(s) {
    var d = document.createElement('div');
    d.textContent = String(s);
    return d.innerHTML;
  }

  function formatMoney(cents, currency) {
    try {
      return new Intl.NumberFormat('en-US', {
        style: 'currency',
        currency: currency || 'USD',
        minimumFractionDigits: 2,
        maximumFractionDigits: 2
      }).format(cents / 100);
    } catch (_) {
      return (currency || 'USD') + ' ' + (cents / 100).toLocaleString('en-US', {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2
      });
    }
  }
})();
