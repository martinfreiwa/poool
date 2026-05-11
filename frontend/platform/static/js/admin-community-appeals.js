/**
 * Admin Community Ban Appeals (Phase 2 task 18).
 *
 * Lists ban appeals from GET /api/admin/community/appeals?status=<status>
 * and lets the moderator approve or reject each one via
 * POST /api/admin/community/appeals/:id/review with body
 * { action: "approve" | "reject", admin_notes?: string }.
 */
(function () {
  'use strict';

  const STATUS_BADGES = {
    pending:  { text: 'Pending',  bg: '#FFFAEB', color: '#B54708' },
    approved: { text: 'Approved', bg: '#ECFDF3', color: '#067647' },
    rejected: { text: 'Rejected', bg: '#FEF3F2', color: '#B42318' },
  };

  function getCsrfToken() {
    const match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]+)/);
    return match ? decodeURIComponent(match[1]) : '';
  }

  function csrfHeaders(extra = {}) {
    const token = getCsrfToken();
    return token ? { ...extra, 'X-CSRF-Token': token } : extra;
  }

  function formatDate(iso) {
    if (!iso) return '—';
    try {
      const d = new Date(iso);
      return d.toLocaleString();
    } catch (_e) {
      return iso;
    }
  }

  async function loadAppeals() {
    const tbody = document.getElementById('appeals-table');
    const status = document.getElementById('appeals-status-filter').value || 'pending';
    tbody.replaceChildren(messageRow('Loading appeals...'));
    try {
      const res = await fetch(`/api/admin/community/appeals?status=${encodeURIComponent(status)}`, {
        credentials: 'same-origin',
      });
      if (!res.ok) throw new Error(`Request failed (${res.status})`);
      const data = await res.json();
      const appeals = Array.isArray(data.appeals) ? data.appeals : [];
      renderAppeals(appeals);
    } catch (err) {
      console.error('Failed to load ban appeals', err);
      tbody.replaceChildren(messageRow('Failed to load appeals.', true));
    }
  }

  function renderAppeals(appeals) {
    const tbody = document.getElementById('appeals-table');
    tbody.replaceChildren();
    if (appeals.length === 0) {
      tbody.appendChild(messageRow('No appeals for this filter.'));
      return;
    }
    for (const appeal of appeals) {
      tbody.appendChild(appealRow(appeal));
    }
  }

  function appealRow(appeal) {
    const row = document.createElement('tr');

    const userCell = document.createElement('td');
    const userName = document.createElement('div');
    userName.style.fontWeight = '500';
    userName.textContent = appeal.display_name || 'Unknown';
    const userIdLine = document.createElement('div');
    userIdLine.style.fontSize = '11px';
    userIdLine.style.color = '#98A2B3';
    userIdLine.style.fontFamily = 'monospace';
    userIdLine.textContent = appeal.user_id ? String(appeal.user_id).substring(0, 8) + '…' : '';
    userCell.append(userName, userIdLine);
    row.appendChild(userCell);

    const textCell = document.createElement('td');
    textCell.style.maxWidth = '480px';
    textCell.style.whiteSpace = 'normal';
    textCell.style.fontSize = '13px';
    textCell.style.color = '#344054';
    const snippet = (appeal.appeal_text || '').slice(0, 240);
    textCell.textContent = snippet + ((appeal.appeal_text || '').length > 240 ? '…' : '');
    row.appendChild(textCell);

    const statusCell = document.createElement('td');
    const status = STATUS_BADGES[appeal.status] || STATUS_BADGES.pending;
    const badge = document.createElement('span');
    badge.className = 'admin-badge';
    badge.style.background = status.bg;
    badge.style.color = status.color;
    badge.textContent = status.text;
    statusCell.appendChild(badge);
    row.appendChild(statusCell);

    const dateCell = document.createElement('td');
    dateCell.style.fontSize = '12px';
    dateCell.style.color = '#667085';
    dateCell.textContent = formatDate(appeal.created_at);
    row.appendChild(dateCell);

    const actionCell = document.createElement('td');
    actionCell.style.textAlign = 'right';
    if (appeal.status === 'pending') {
      const reviewBtn = document.createElement('button');
      reviewBtn.type = 'button';
      reviewBtn.className = 'admin-btn admin-btn--secondary admin-btn--sm';
      reviewBtn.textContent = 'Review';
      reviewBtn.addEventListener('click', () => openReviewModal(appeal));
      actionCell.appendChild(reviewBtn);
    } else {
      const resolvedLabel = document.createElement('div');
      resolvedLabel.style.fontSize = '12px';
      resolvedLabel.style.color = '#667085';
      resolvedLabel.textContent = appeal.resolved_at ? `Resolved ${formatDate(appeal.resolved_at)}` : '—';
      actionCell.appendChild(resolvedLabel);
      if (appeal.admin_notes) {
        const notes = document.createElement('div');
        notes.style.fontSize = '11px';
        notes.style.color = '#98A2B3';
        notes.style.marginTop = '4px';
        notes.textContent = appeal.admin_notes;
        actionCell.appendChild(notes);
      }
    }
    row.appendChild(actionCell);
    return row;
  }

  function messageRow(text, isError) {
    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 5;
    cell.style.textAlign = 'center';
    cell.style.padding = '40px';
    cell.style.color = isError ? '#B42318' : 'var(--admin-text-muted)';
    cell.textContent = text;
    row.appendChild(cell);
    return row;
  }

  function openReviewModal(appeal) {
    const modal = document.getElementById('appeal-review-modal');
    if (!modal) return;
    document.getElementById('appeal-modal-id').value = appeal.id;
    document.getElementById('appeal-modal-user').textContent =
      `${appeal.display_name || 'Unknown'} (${appeal.user_id || ''})`;
    document.getElementById('appeal-modal-text').textContent = appeal.appeal_text || '';
    document.getElementById('appeal-modal-notes').value = '';
    document.getElementById('appeal-modal-error').style.display = 'none';
    modal.style.display = 'flex';
    modal.setAttribute('aria-hidden', 'false');
  }

  function closeReviewModal() {
    const modal = document.getElementById('appeal-review-modal');
    if (!modal) return;
    modal.style.display = 'none';
    modal.setAttribute('aria-hidden', 'true');
  }

  async function submitReview(action) {
    const appealId = document.getElementById('appeal-modal-id').value;
    const adminNotes = document.getElementById('appeal-modal-notes').value.trim();
    const errEl = document.getElementById('appeal-modal-error');
    errEl.style.display = 'none';

    if (!appealId) return;

    try {
      const res = await fetch(`/api/admin/community/appeals/${encodeURIComponent(appealId)}/review`, {
        method: 'POST',
        credentials: 'same-origin',
        headers: csrfHeaders({ 'Content-Type': 'application/json' }),
        body: JSON.stringify({ action, admin_notes: adminNotes || null }),
      });
      if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `Request failed (${res.status})`);
      }
      closeReviewModal();
      loadAppeals();
    } catch (err) {
      console.error('Failed to review appeal', err);
      errEl.textContent = err.message || 'Failed to submit review.';
      errEl.style.display = 'block';
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    document.getElementById('appeals-refresh-btn')?.addEventListener('click', loadAppeals);
    document.getElementById('appeals-status-filter')?.addEventListener('change', loadAppeals);
    document.querySelectorAll('[data-appeals-close-modal]').forEach((el) =>
      el.addEventListener('click', closeReviewModal)
    );
    document.getElementById('appeal-approve-btn')?.addEventListener('click', () => submitReview('approve'));
    document.getElementById('appeal-reject-btn')?.addEventListener('click', () => submitReview('reject'));
    loadAppeals();
  });
})();
