/**
 * Admin verified-owner request review (14.8.16).
 *
 * Backend:
 *   GET   /api/admin/community/verified-owner-requests?status=pending|approved|rejected
 *   PATCH /api/admin/community/verified-owner-requests/:id  body { status, admin_notes? }
 *
 * Both require `community.manage`. The PATCH endpoint flips the request and,
 * on approval, awards the Verified Owner badge + sets profile/posts flags
 * server-side (see admin_review_verified_owner_request in routes.rs).
 */
(function () {
  'use strict';

  const STATUS_BADGES = {
    pending:  { text: 'Pending',  bg: '#FFFAEB', color: '#B54708' },
    approved: { text: 'Approved', bg: '#ECFDF3', color: '#067647' },
    rejected: { text: 'Rejected', bg: '#FEF3F2', color: '#B42318' },
  };

  let activeRequestId = null;

  function csrfHeaders(extra = {}) {
    const m = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]+)/);
    const token = m ? decodeURIComponent(m[1]) : '';
    return token ? { ...extra, 'X-CSRF-Token': token } : extra;
  }

  function formatDate(iso) {
    if (!iso) return '—';
    try { return new Date(iso).toLocaleString(); } catch (_) { return iso; }
  }

  function shortId(id) {
    return id ? String(id).substring(0, 8) + '…' : '—';
  }

  function messageRow(text, isError) {
    const tr = document.createElement('tr');
    const td = document.createElement('td');
    td.colSpan = 7;
    td.style.cssText =
      'text-align:center; padding:40px;' +
      (isError ? ' color:#B42318;' : ' color: var(--admin-text-muted);');
    td.textContent = text;
    tr.appendChild(td);
    return tr;
  }

  function badge(status) {
    const cfg = STATUS_BADGES[status] || STATUS_BADGES.pending;
    const span = document.createElement('span');
    span.style.cssText = `
      display: inline-block; padding: 2px 8px; border-radius: 12px;
      font-size: 11px; font-weight: 600;
      background: ${cfg.bg}; color: ${cfg.color};
    `;
    span.textContent = cfg.text;
    return span;
  }

  async function load() {
    const tbody = document.getElementById('vor-table');
    const status = document.getElementById('vor-status-filter').value || 'pending';
    tbody.replaceChildren(messageRow('Loading requests...'));
    try {
      const res = await fetch(
        `/api/admin/community/verified-owner-requests?status=${encodeURIComponent(status)}`,
        { credentials: 'same-origin' },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      const requests = Array.isArray(data.requests) ? data.requests : [];
      render(requests, status);
    } catch (err) {
      console.error('Failed to load verified-owner requests', err);
      tbody.replaceChildren(messageRow('Failed to load requests.', true));
    }
  }

  function render(requests, status) {
    const tbody = document.getElementById('vor-table');
    tbody.replaceChildren();
    if (requests.length === 0) {
      tbody.appendChild(messageRow(`No ${status} requests.`));
      return;
    }
    for (const r of requests) tbody.appendChild(row(r));
  }

  function row(r) {
    const tr = document.createElement('tr');

    const userCell = document.createElement('td');
    const userTop = document.createElement('div');
    userTop.style.fontFamily = 'monospace';
    userTop.style.fontSize = '12px';
    userTop.textContent = shortId(r.user_id);
    userCell.appendChild(userTop);

    const assetCell = document.createElement('td');
    assetCell.style.fontFamily = 'monospace';
    assetCell.style.fontSize = '12px';
    assetCell.textContent = r.asset_id ? shortId(r.asset_id) : '—';

    const noteCell = document.createElement('td');
    noteCell.style.maxWidth = '320px';
    noteCell.style.whiteSpace = 'normal';
    noteCell.textContent = r.note || '—';

    const evCell = document.createElement('td');
    if (r.evidence_url) {
      const a = document.createElement('a');
      a.href = r.evidence_url;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      a.textContent = 'View';
      evCell.appendChild(a);
    } else {
      evCell.textContent = '—';
    }

    const statusCell = document.createElement('td');
    statusCell.appendChild(badge(r.status));

    const dateCell = document.createElement('td');
    dateCell.textContent = formatDate(r.created_at);

    const actionsCell = document.createElement('td');
    actionsCell.style.textAlign = 'right';
    if (r.status === 'pending') {
      const reviewBtn = document.createElement('button');
      reviewBtn.className = 'admin-btn admin-btn--primary admin-btn--sm';
      reviewBtn.type = 'button';
      reviewBtn.textContent = 'Review';
      reviewBtn.onclick = () => openReview(r.id);
      actionsCell.appendChild(reviewBtn);
    } else {
      const notes = document.createElement('div');
      notes.style.fontSize = '11px';
      notes.style.color = '#98A2B3';
      notes.textContent = r.admin_notes ? `Notes: ${r.admin_notes}` : 'Reviewed';
      actionsCell.appendChild(notes);
    }

    tr.append(userCell, assetCell, noteCell, evCell, statusCell, dateCell, actionsCell);
    return tr;
  }

  function openReview(id) {
    activeRequestId = id;
    document.getElementById('vor-review-notes').value = '';
    document.getElementById('vor-review-status').textContent = '';
    document.getElementById('vor-review-modal').style.display = 'flex';
  }

  window.__vorCloseReview = function () {
    activeRequestId = null;
    document.getElementById('vor-review-modal').style.display = 'none';
  };

  async function submitReview(status) {
    if (!activeRequestId) return;
    const notes = document.getElementById('vor-review-notes').value.trim();
    const statusEl = document.getElementById('vor-review-status');
    statusEl.textContent = 'Submitting...';
    statusEl.style.color = '';
    try {
      const res = await fetch(
        `/api/admin/community/verified-owner-requests/${activeRequestId}`,
        {
          method: 'PATCH',
          credentials: 'same-origin',
          headers: csrfHeaders({ 'Content-Type': 'application/json' }),
          body: JSON.stringify({ status, admin_notes: notes || null }),
        },
      );
      if (!res.ok) {
        const errText = await res.text();
        throw new Error(`HTTP ${res.status}: ${errText}`);
      }
      window.__vorCloseReview();
      load();
    } catch (err) {
      statusEl.style.color = '#B42318';
      statusEl.textContent = `Failed: ${err.message}`;
    }
  }

  function init() {
    document.getElementById('vor-status-filter').addEventListener('change', load);
    document.getElementById('vor-refresh-btn').addEventListener('click', load);
    document.getElementById('vor-review-approve-btn').addEventListener('click', () => submitReview('approved'));
    document.getElementById('vor-review-reject-btn').addEventListener('click', () => submitReview('rejected'));
    load();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
