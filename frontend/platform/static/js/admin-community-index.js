(function () {
  const KPI_FIELDS = [
    ['kpi-profiles', 'active_profiles'],
    ['kpi-posts', 'total_posts'],
    ['kpi-comments', 'total_comments'],
    ['kpi-reactions', 'total_reactions'],
    ['kpi-circles', 'total_circles'],
    ['kpi-xp', 'total_xp'],
    ['kpi-reports', 'pending_reports_count'],
  ];

  function clearElement(element) {
    while (element && element.firstChild) {
      element.removeChild(element.firstChild);
    }
  }

  function setStatus(message, isError) {
    const status = document.getElementById('community-overview-status');
    if (!status) return;

    clearElement(status);
    if (!message) return;

    const wrapper = document.createElement('div');
    wrapper.style.display = 'flex';
    wrapper.style.alignItems = 'center';
    wrapper.style.justifyContent = 'space-between';
    wrapper.style.gap = '12px';
    wrapper.style.padding = '12px 14px';
    wrapper.style.borderRadius = '8px';
    wrapper.style.border = isError ? '1px solid #FDA29B' : '1px solid #ABEFC6';
    wrapper.style.background = isError ? '#FEF3F2' : '#ECFDF3';
    wrapper.style.color = isError ? '#B42318' : '#027A48';

    const text = document.createElement('span');
    text.textContent = message;
    wrapper.appendChild(text);

    if (isError) {
      const retry = document.createElement('button');
      retry.type = 'button';
      retry.className = 'admin-btn admin-btn--secondary admin-btn--sm';
      retry.textContent = 'Retry';
      retry.addEventListener('click', loadDashboard);
      wrapper.appendChild(retry);
    }

    status.appendChild(wrapper);
  }

  function appendTableMessage(message, isError) {
    const tbody = document.getElementById('recent-announcements-table');
    if (!tbody) return;

    clearElement(tbody);
    const row = document.createElement('tr');
    const cell = document.createElement('td');
    cell.colSpan = 4;
    cell.style.textAlign = 'center';
    cell.style.padding = '20px';
    cell.style.color = isError ? '#d92d20' : 'var(--admin-text-muted)';

    const messageText = document.createElement('span');
    messageText.textContent = message;
    cell.appendChild(messageText);

    if (isError) {
      const retry = document.createElement('button');
      retry.type = 'button';
      retry.className = 'admin-btn admin-btn--secondary admin-btn--sm';
      retry.style.marginLeft = '12px';
      retry.textContent = 'Retry';
      retry.addEventListener('click', reloadAnnouncementsOnly);
      cell.appendChild(retry);
    }

    row.appendChild(cell);
    tbody.appendChild(row);
  }

  function asInteger(value) {
    const number = Number(value || 0);
    return Number.isFinite(number) ? number : 0;
  }

  function formatInteger(value) {
    return asInteger(value).toLocaleString();
  }

  function safeAvatarUrl(value) {
    if (!value || typeof value !== 'string') return null;
    try {
      const parsed = new URL(value, window.location.origin);
      if (parsed.protocol === 'http:' || parsed.protocol === 'https:') {
        return parsed.href;
      }
    } catch (_error) {
      return null;
    }
    return null;
  }

  function renderAuthorCell(cell, announcement) {
    const wrapper = document.createElement('div');
    wrapper.style.display = 'flex';
    wrapper.style.alignItems = 'center';
    wrapper.style.gap = '8px';

    const avatarUrl = safeAvatarUrl(announcement.author_avatar);
    if (avatarUrl) {
      const img = document.createElement('img');
      img.src = avatarUrl;
      img.alt = '';
      img.style.width = '24px';
      img.style.height = '24px';
      img.style.borderRadius = '50%';
      img.style.objectFit = 'cover';
      wrapper.appendChild(img);
    } else {
      const placeholder = document.createElement('div');
      placeholder.style.width = '24px';
      placeholder.style.height = '24px';
      placeholder.style.borderRadius = '50%';
      placeholder.style.background = '#f2f4f7';
      wrapper.appendChild(placeholder);
    }

    const name = document.createElement('span');
    name.style.fontWeight = '500';
    name.textContent = announcement.author_name || 'POOOL Official';
    wrapper.appendChild(name);
    cell.appendChild(wrapper);
  }

  function renderAnnouncements(announcements) {
    const tbody = document.getElementById('recent-announcements-table');
    if (!tbody) return;

    clearElement(tbody);
    if (!Array.isArray(announcements) || announcements.length === 0) {
      appendTableMessage('No announcements yet.', false);
      return;
    }

    announcements.slice(0, 5).forEach((announcement) => {
      const row = document.createElement('tr');

      const authorCell = document.createElement('td');
      renderAuthorCell(authorCell, announcement);

      const categoryCell = document.createElement('td');
      const categoryBadge = document.createElement('span');
      categoryBadge.className = 'admin-badge';
      categoryBadge.style.background = '#eef4ff';
      categoryBadge.style.color = '#3538cd';
      categoryBadge.textContent = announcement.category || '-';
      categoryCell.appendChild(categoryBadge);

      const dateCell = document.createElement('td');
      const createdAt = announcement.created_at ? new Date(announcement.created_at) : null;
      dateCell.textContent = createdAt && !Number.isNaN(createdAt.getTime())
        ? createdAt.toLocaleDateString()
        : '-';

      const engagementCell = document.createElement('td');
      const engagement = document.createElement('span');
      engagement.style.fontSize = '12px';
      engagement.style.color = '#667085';
      engagement.textContent = `Reactions ${formatInteger(announcement.reaction_count)} · Comments ${formatInteger(announcement.comment_count)}`;
      engagementCell.appendChild(engagement);

      row.append(authorCell, categoryCell, dateCell, engagementCell);
      tbody.appendChild(row);
    });
  }

  async function loadStats() {
    KPI_FIELDS.forEach(([elementId]) => {
      const element = document.getElementById(elementId);
      if (element) element.textContent = '...';
    });

    const response = await fetch('/api/admin/community/stats', {
      credentials: 'same-origin',
      headers: { Accept: 'application/json' },
    });
    if (!response.ok) {
      throw new Error(await response.text());
    }

    const stats = await response.json();
    KPI_FIELDS.forEach(([elementId, field]) => {
      const element = document.getElementById(elementId);
      if (element) element.textContent = formatInteger(stats[field]);
    });
  }

  async function loadAnnouncements() {
    appendTableMessage('Loading announcements...', false);

    const response = await fetch('/api/admin/community/announcements', {
      credentials: 'same-origin',
      headers: { Accept: 'application/json' },
    });
    if (!response.ok) {
      throw new Error(await response.text());
    }

    renderAnnouncements(await response.json());
  }

  async function loadDashboard() {
    setStatus('');
    await Promise.all([
      loadStats().catch((error) => {
        console.error('Failed to load community stats', error);
        KPI_FIELDS.forEach(([elementId]) => {
          const element = document.getElementById(elementId);
          if (element) element.textContent = '--';
        });
        setStatus('Unable to load community overview data. Please retry.', true);
      }),
      loadAnnouncements().catch((error) => {
        console.error('Failed to load recent announcements', error);
        appendTableMessage('Unable to load recent announcements.', true);
      }),
    ]);
  }

  async function reloadAnnouncementsOnly() {
    try {
      await loadAnnouncements();
    } catch (error) {
      console.error('Failed to load community overview', error);
      appendTableMessage('Unable to load recent announcements.', true);
    }
  }

  window.reloadCommunityOverview = loadDashboard;
  window.reloadCommunityAnnouncements = reloadAnnouncementsOnly;
  document.addEventListener('DOMContentLoaded', loadDashboard);
})();
