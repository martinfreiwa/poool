/**
 * P2P Offers — mp-p2p.js
 * Fetches real P2P offers from the backend API and supports audited admin cancellation.
 */
(function () {
  'use strict';

  const API = '/api/admin/marketplace/p2p';
  const tbody = () => document.getElementById('p2p-body');
  let offers = [];

  function csrfToken() {
    return document.cookie
      .split(';')
      .map((part) => part.trim())
      .find((part) => part.startsWith('csrf_token='))
      ?.split('=')
      .slice(1)
      .join('=') || '';
  }

  function formatMoney(cents) {
    if (typeof cents !== 'number') return '--';
    return `$${(cents / 100).toFixed(2)}`;
  }

  function formatEmail(email) {
    if (!email) return '--';
    return email.split('@')[0] || email;
  }

  function shortId(id) {
    return typeof id === 'string' && id.length > 8 ? id.substring(0, 8) : (id || '--');
  }

  function timeAgo(dateStr) {
    const timestamp = new Date(dateStr).getTime();
    if (!Number.isFinite(timestamp)) return '--';
    const diff = Math.max(0, (Date.now() - timestamp) / 1000);
    if (diff < 60) return `${Math.floor(diff)}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  function formatDateTime(dateStr) {
    const date = new Date(dateStr);
    if (Number.isNaN(date.getTime())) return '--';
    return date.toLocaleString();
  }

  function setKpis(warningCount) {
    const total = document.getElementById('kpi-p2p-total');
    const warnings = document.getElementById('kpi-p2p-warnings');
    if (total) total.textContent = offers.length;
    if (warnings) warnings.textContent = warningCount;
  }

  function clearTable() {
    const body = tbody();
    if (!body) return null;
    body.replaceChildren();
    return body;
  }

  function renderState(message, detail, retryable) {
    const body = clearTable();
    if (!body) return;

    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 13;
    cell.className = 'admin-table-empty';

    const title = document.createElement('div');
    title.style.fontWeight = '600';
    title.textContent = message;
    cell.appendChild(title);

    if (detail) {
      const sub = document.createElement('div');
      sub.style.marginTop = '4px';
      sub.style.color = 'var(--admin-text-muted)';
      sub.textContent = detail;
      cell.appendChild(sub);
    }

    if (retryable) {
      const retry = document.createElement('button');
      retry.type = 'button';
      retry.className = 'admin-btn admin-btn--secondary admin-btn--sm';
      retry.style.marginTop = '12px';
      retry.textContent = 'Retry';
      retry.addEventListener('click', loadP2P);
      cell.appendChild(retry);
    }

    row.appendChild(cell);
    body.appendChild(row);
    setKpis(0);
  }

  function appendTextCell(row, text, options = {}) {
    const cell = document.createElement('td');
    if (options.alignRight) cell.style.textAlign = 'right';
    if (options.center) cell.style.textAlign = 'center';
    if (options.muted) cell.style.color = 'var(--admin-text-muted)';
    if (options.bold) cell.style.fontWeight = '600';
    if (options.mono) cell.style.fontVariantNumeric = 'tabular-nums';
    cell.textContent = text;
    row.appendChild(cell);
    return cell;
  }

  function appendCodeCell(row, text) {
    const cell = document.createElement('td');
    const code = document.createElement('code');
    code.style.fontSize = '11px';
    code.style.padding = '2px 6px';
    code.style.background = 'var(--admin-code-bg)';
    code.style.borderRadius = '4px';
    code.textContent = text;
    cell.appendChild(code);
    row.appendChild(cell);
    return cell;
  }

  function badge(text, variant) {
    const span = document.createElement('span');
    const allowed = new Set(['neutral', 'warning', 'success', 'danger']);
    span.className = `admin-badge admin-badge--${allowed.has(variant) ? variant : 'neutral'}`;
    span.textContent = text;
    return span;
  }

  function statusVariant(status) {
    if (status === 'pending') return 'warning';
    if (status === 'accepted') return 'success';
    if (status === 'admin_cancelled' || status === 'cancelled' || status === 'declined') return 'danger';
    return 'neutral';
  }

  function appendDeviationCell(row, offer) {
    const cell = document.createElement('td');
    const marketPrice = typeof offer.market_price_cents === 'number' ? offer.market_price_cents : null;
    const devPct = typeof offer.price_deviation_pct === 'number' ? offer.price_deviation_pct : null;
    if (devPct === null || marketPrice === null) {
      cell.appendChild(badge('N/A', 'neutral'));
    } else {
      const devAbs = Math.abs(devPct);
      const sign = devPct >= 0 ? '+' : '';
      if (devAbs > 20) {
        const warning = document.createElement('span');
        warning.className = 'mp-price-warning';
        warning.textContent = `${sign}${devPct.toFixed(2)}%`;
        cell.appendChild(warning);
      } else if (devAbs > 5) {
        cell.appendChild(badge(`${sign}${devPct.toFixed(2)}%`, 'warning'));
      } else {
        cell.appendChild(badge(`${sign}${devPct.toFixed(2)}%`, 'success'));
      }
    }
    row.appendChild(cell);
  }

  function render() {
    const body = clearTable();
    if (!body) return;

    if (!offers.length) {
      renderState('No P2P offers found', 'Pending, accepted, and cancelled P2P offers will appear here.', false);
      return;
    }

    let warningCount = 0;
    offers.forEach((offer) => {
      const devPct = typeof offer.price_deviation_pct === 'number' ? offer.price_deviation_pct : null;
      if (devPct !== null && Math.abs(devPct) > 20) warningCount += 1;

      const row = document.createElement('tr');
      row.dataset.offerId = offer.id;

      appendCodeCell(row, shortId(offer.id));
      appendTextCell(row, (offer.side || '--').toUpperCase(), { bold: true });
      appendCodeCell(row, formatEmail(offer.maker_email));
      appendCodeCell(row, formatEmail(offer.taker_email));
      appendTextCell(row, offer.asset_name || shortId(offer.asset_id), { bold: true });
      appendTextCell(row, Number(offer.quantity || 0).toLocaleString(), { alignRight: true, mono: true });
      appendTextCell(row, formatMoney(offer.price_cents), { alignRight: true, mono: true, bold: true });
      appendTextCell(row, formatMoney(offer.market_price_cents), { alignRight: true, mono: true, muted: true });
      appendDeviationCell(row, offer);

      const statusCell = document.createElement('td');
      statusCell.appendChild(badge(offer.status || '--', statusVariant(offer.status)));
      row.appendChild(statusCell);

      const expiresCell = appendTextCell(row, formatDateTime(offer.expires_at), { muted: true });
      expiresCell.style.fontSize = '12px';
      const createdCell = appendTextCell(row, timeAgo(offer.created_at), { muted: true });
      createdCell.style.fontSize = '12px';

      const actionCell = document.createElement('td');
      actionCell.style.textAlign = 'center';
      if (offer.status === 'pending') {
        const button = document.createElement('button');
        button.type = 'button';
        button.className = 'admin-btn admin-btn--danger admin-btn--sm btn-cancel-p2p';
        button.textContent = 'Admin Cancel';
        button.addEventListener('click', () => openCancelModal(offer, button));
        actionCell.appendChild(button);
      } else {
        actionCell.textContent = '--';
        actionCell.style.color = 'var(--admin-text-muted)';
      }
      row.appendChild(actionCell);

      body.appendChild(row);
    });

    setKpis(warningCount);
  }

  function buildCancelBody() {
    const wrapper = document.createElement('div');
    wrapper.className = 'admin-form-group';

    const label = document.createElement('label');
    label.className = 'admin-form-label';
    label.setAttribute('for', 'p2p-cancel-reason');
    label.textContent = 'Cancellation Reason *';

    const textarea = document.createElement('textarea');
    textarea.className = 'admin-textarea';
    textarea.id = 'p2p-cancel-reason';
    textarea.maxLength = 500;
    textarea.placeholder = 'e.g. Price significantly deviates from market';
    textarea.setAttribute('aria-describedby', 'p2p-cancel-error');

    const error = document.createElement('div');
    error.id = 'p2p-cancel-error';
    error.style.marginTop = '6px';
    error.style.color = 'var(--admin-danger)';
    error.style.fontSize = '12px';
    error.setAttribute('role', 'alert');

    wrapper.append(label, textarea, error);
    return wrapper;
  }

  function openCancelModal(offer, triggerButton) {
    if (typeof mpModal !== 'function') return;

    mpModal({
      title: 'Cancel P2P Offer',
      subtitle: `${shortId(offer.id)} - ${offer.asset_name || shortId(offer.asset_id)} @ ${formatMoney(offer.price_cents)}`,
      bodyNode: buildCancelBody(),
      confirmLabel: 'Cancel Offer',
      onConfirm: async (overlay) => {
        const textarea = overlay.querySelector('#p2p-cancel-reason');
        const error = overlay.querySelector('#p2p-cancel-error');
        const reason = textarea?.value?.trim() || '';
        if (!reason) {
          if (error) error.textContent = 'Please provide a cancellation reason.';
          textarea?.focus();
          return false;
        }

        if (triggerButton) {
          triggerButton.disabled = true;
          triggerButton.setAttribute('aria-busy', 'true');
        }

        try {
          const res = await fetch(`${API}/${encodeURIComponent(offer.id)}/cancel`, {
            method: 'POST',
            credentials: 'same-origin',
            headers: {
              'Content-Type': 'application/json',
              'X-CSRF-Token': csrfToken()
            },
            body: JSON.stringify({ reason })
          });
          const payload = await res.json().catch(() => ({}));
          if (!res.ok) throw new Error(payload.error || `HTTP ${res.status}`);
          mpToast(`P2P offer ${shortId(offer.id)} cancelled`, 'success');
          await loadP2P();
          return true;
        } catch (err) {
          if (triggerButton) {
            triggerButton.disabled = false;
            triggerButton.removeAttribute('aria-busy');
          }
          if (error) error.textContent = err.message || 'Cancellation failed.';
          mpToast(err.message || 'Cancellation failed', 'error');
          return false;
        }
      }
    });
  }

  async function loadP2P() {
    renderState('Loading P2P offers...', '', false);
    try {
      const res = await fetch(API, { credentials: 'same-origin' });
      const payload = await res.json().catch(() => null);
      if (!res.ok) {
        throw new Error(payload?.error || `HTTP ${res.status}`);
      }
      if (!Array.isArray(payload)) {
        throw new Error('Unexpected P2P response format.');
      }
      offers = payload;
      render();
    } catch (err) {
      offers = [];
      renderState('Unable to load P2P offers', err.message || 'Please try again.', true);
    }
  }

  document.addEventListener('DOMContentLoaded', loadP2P);
})();
