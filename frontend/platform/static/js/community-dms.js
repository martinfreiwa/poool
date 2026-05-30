/**
 * Community direct messages (14.8.20).
 *
 * Backend (community/routes.rs):
 *   GET  /api/community/dms/threads
 *   POST /api/community/dms/threads                 { recipient_user_id, content }
 *   GET  /api/community/dms/threads/:id/messages   (also marks unread as read)
 *   POST /api/community/dms/threads/:id/messages    { content }
 *
 * Block enforcement is server-side. Polls active thread every 8s while the DM
 * tab is open. Stops polling when the user switches tabs.
 */
(function () {
  'use strict';

  const POLL_INTERVAL_MS = 8000;
  let activeThreadId = null;
  let activeOther = null;
  let currentUserId = null;
  let pollTimer = null;
  let searchDebounce = null;

  function csrfHeaders(extra = {}) {
    const m = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]+)/);
    const token = m ? decodeURIComponent(m[1]) : '';
    return token ? { ...extra, 'X-CSRF-Token': token } : extra;
  }

  function timeShort(iso) {
    if (!iso) return '';
    try {
      const d = new Date(iso);
      const now = new Date();
      const sameDay = d.toDateString() === now.toDateString();
      if (sameDay) {
        return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
      }
      return d.toLocaleDateString();
    } catch (_) {
      return '';
    }
  }

  async function fetchCurrentUserId() {
    if (currentUserId) return currentUserId;
    try {
      const res = await fetch('/api/community/profile/me', { credentials: 'same-origin' });
      if (res.ok) {
        const data = await res.json();
        currentUserId = data.user_id || data.id || null;
      }
    } catch (_) {
      currentUserId = null;
    }
    return currentUserId;
  }

  async function loadThreads() {
    const list = document.getElementById('community-dm-thread-list');
    if (!list) return;
    try {
      const res = await fetch('/api/community/dms/threads', { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      renderThreads(data.threads || []);
    } catch (err) {
      list.replaceChildren();
      const div = document.createElement('div');
      div.className = 'community-dms__empty';
      div.style.color = '#B42318';
      div.textContent = `Failed to load conversations: ${err.message}`;
      list.appendChild(div);
    }
  }
  window.loadDmThreads = loadThreads;

  function renderThreads(threads) {
    const list = document.getElementById('community-dm-thread-list');
    list.replaceChildren();
    if (!threads.length) {
      const empty = document.createElement('div');
      empty.className = 'community-dms__empty';
      empty.textContent = 'No conversations yet. Start one with the button above.';
      list.appendChild(empty);
      return;
    }
    threads.forEach((t) => list.appendChild(threadRow(t)));
  }

  function threadRow(t) {
    const row = document.createElement('button');
    row.type = 'button';
    row.className = 'community-dms__thread';
    if (activeThreadId === t.thread_id) row.classList.add('community-dms__thread--active');

    const avatar = document.createElement('div');
    avatar.className = 'community-dms__thread-avatar';
    if (t.other_avatar_url) {
      const img = document.createElement('img');
      img.src = t.other_avatar_url;
      img.alt = '';
      avatar.appendChild(img);
    } else {
      avatar.textContent = (t.other_display_name || 'U').charAt(0).toUpperCase();
    }

    const body = document.createElement('div');
    body.className = 'community-dms__thread-body';
    const name = document.createElement('div');
    name.className = 'community-dms__thread-name';
    name.textContent = t.other_display_name || 'Anonymous';
    const preview = document.createElement('div');
    preview.className = 'community-dms__thread-preview';
    preview.textContent = t.last_message_preview || 'No messages yet';
    body.append(name, preview);

    const meta = document.createElement('div');
    meta.className = 'community-dms__thread-meta';
    const time = document.createElement('div');
    time.className = 'community-dms__thread-time';
    time.textContent = timeShort(t.last_message_at);
    meta.appendChild(time);
    if (t.unread_count && t.unread_count > 0) {
      const badge = document.createElement('span');
      badge.className = 'community-dms__thread-unread';
      badge.textContent = String(t.unread_count);
      meta.appendChild(badge);
    }

    row.append(avatar, body, meta);
    row.addEventListener('click', () => openThread(t));
    return row;
  }

  async function openThread(thread) {
    activeThreadId = thread.thread_id;
    activeOther = thread;
    stopPolling();

    document.getElementById('community-dm-empty-state').hidden = true;
    document.getElementById('community-dm-active').hidden = false;

    const header = document.getElementById('community-dm-active-header');
    header.replaceChildren();
    const name = document.createElement('span');
    name.className = 'community-dms__active-name';
    name.textContent = thread.other_display_name || 'Anonymous';
    const viewProfile = document.createElement('a');
    viewProfile.href = `/community/u/${encodeURIComponent(thread.other_user_id)}`;
    viewProfile.className = 'community-dms__active-link';
    viewProfile.textContent = 'View profile';
    header.append(name, viewProfile);

    document.querySelectorAll('.community-dms__thread').forEach((el) =>
      el.classList.remove('community-dms__thread--active'),
    );

    await loadMessages();
    startPolling();
    const input = document.getElementById('community-dm-input');
    if (input) input.focus();
  }

  async function loadMessages() {
    if (!activeThreadId) return;
    try {
      const res = await fetch(
        `/api/community/dms/threads/${activeThreadId}/messages`,
        { credentials: 'same-origin' },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      renderMessages(data.messages || []);
    } catch (err) {
      const list = document.getElementById('community-dm-message-list');
      if (list) {
        list.replaceChildren();
        const div = document.createElement('div');
        div.className = 'community-dms__empty';
        div.style.color = '#B42318';
        div.textContent = `Failed to load messages: ${err.message}`;
        list.appendChild(div);
      }
    }
  }

  function renderMessages(messages) {
    const list = document.getElementById('community-dm-message-list');
    if (!list) return;
    list.replaceChildren();
    messages.forEach((m) => list.appendChild(messageBubble(m)));
    list.scrollTop = list.scrollHeight;
  }

  function messageBubble(m) {
    const isSelf = currentUserId && m.sender_id === currentUserId;
    const wrap = document.createElement('div');
    wrap.className = `community-dms__bubble-row ${isSelf ? 'community-dms__bubble-row--self' : 'community-dms__bubble-row--other'}`;

    const bubble = document.createElement('div');
    bubble.className = `community-dms__bubble ${isSelf ? 'community-dms__bubble--self' : 'community-dms__bubble--other'}`;
    bubble.textContent = m.content || '';

    const time = document.createElement('div');
    time.className = 'community-dms__bubble-time';
    time.textContent = timeShort(m.created_at);

    wrap.append(bubble, time);
    return wrap;
  }

  window.sendDmMessage = async function (event) {
    event.preventDefault();
    if (!activeThreadId) return;
    const input = document.getElementById('community-dm-input');
    const content = input.value.trim();
    if (!content) return;
    input.disabled = true;
    try {
      const res = await fetch(
        `/api/community/dms/threads/${activeThreadId}/messages`,
        {
          method: 'POST',
          credentials: 'same-origin',
          headers: csrfHeaders({ 'Content-Type': 'application/json' }),
          body: JSON.stringify({ content }),
        },
      );
      if (!res.ok) {
        const errText = await res.text();
        throw new Error(`HTTP ${res.status}: ${errText}`);
      }
      input.value = '';
      await loadMessages();
      await loadThreads();
    } catch (err) {
      alert(`Failed to send: ${err.message}`);
    } finally {
      input.disabled = false;
      input.focus();
    }
  };

  function startPolling() {
    stopPolling();
    pollTimer = setInterval(loadMessages, POLL_INTERVAL_MS);
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  // ─── New-thread modal ──────────────────────────────────────────────────

  function openNewModal() {
    const modal = document.getElementById('community-dm-new-modal');
    if (!modal) return;
    document.getElementById('community-dm-search').value = '';
    document.getElementById('community-dm-search-results').replaceChildren();
    showSearchStage();
    if (typeof window.openCommunityModal === 'function') {
      window.openCommunityModal('community-dm-new-modal');
    } else {
      modal.style.display = 'flex';
    }
    document.getElementById('community-dm-search').focus();
  }

  async function searchUsers(q) {
    const results = document.getElementById('community-dm-search-results');
    if (!q || q.length < 2) {
      results.replaceChildren();
      return;
    }
    try {
      const url = new URL('/api/community/search', window.location.origin);
      url.searchParams.set('q', q);
      url.searchParams.set('type', 'users');
      const res = await fetch(url.toString(), { credentials: 'same-origin' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      renderSearchResults(data.users || []);
    } catch (err) {
      results.replaceChildren();
      const div = document.createElement('div');
      div.className = 'ds-helper-text';
      div.style.color = '#B42318';
      div.textContent = `Search failed: ${err.message}`;
      results.appendChild(div);
    }
  }

  function renderSearchResults(users) {
    const list = document.getElementById('community-dm-search-results');
    list.replaceChildren();
    if (!users.length) {
      const div = document.createElement('div');
      div.className = 'ds-helper-text';
      div.textContent = 'No users found.';
      list.appendChild(div);
      return;
    }
    users.forEach((u) => list.appendChild(searchRow(u)));
  }

  function searchRow(u) {
    if (u.is_self) return document.createComment('skip-self');
    const row = document.createElement('button');
    row.type = 'button';
    row.className = 'community-dms__search-row';

    const avatar = document.createElement('div');
    avatar.className = 'community-dms__search-avatar';
    if (u.avatar_url) {
      const img = document.createElement('img');
      img.src = u.avatar_url;
      img.alt = '';
      avatar.appendChild(img);
    } else {
      avatar.textContent = (u.display_name || 'U').charAt(0).toUpperCase();
    }
    const name = document.createElement('div');
    name.className = 'community-dms__search-name';
    name.textContent = u.display_name || 'Anonymous';

    row.append(avatar, name);
    row.addEventListener('click', () => startConversation(u));
    return row;
  }

  // The modal has 2 stages: search → compose. We stash the picked
  // recipient on this module-local var so the Send handler can read it.
  let _pendingRecipient = null;

  function showSearchStage() {
    document.getElementById('community-dm-new-search-stage').hidden = false;
    document.getElementById('community-dm-new-compose-stage').hidden = true;
    document.getElementById('community-dm-new-footer').hidden = true;
    const status = document.getElementById('community-dm-new-status');
    if (status) {
      status.textContent = '';
      status.style.color = '';
    }
    _pendingRecipient = null;
  }

  function showComposeStage(u) {
    _pendingRecipient = u;
    document.getElementById('community-dm-new-search-stage').hidden = true;
    document.getElementById('community-dm-new-compose-stage').hidden = false;
    document.getElementById('community-dm-new-footer').hidden = false;
    const nameEl = document.getElementById('community-dm-new-recipient-name');
    if (nameEl) nameEl.textContent = u.display_name || 'this user';
    const ta = document.getElementById('community-dm-new-message');
    if (ta) {
      ta.value = '';
      const counter = document.getElementById('community-dm-new-counter');
      if (counter) counter.textContent = '0';
      // Wire counter once
      if (ta.dataset.counterBound !== '1') {
        ta.dataset.counterBound = '1';
        ta.addEventListener('input', () => {
          const c = document.getElementById('community-dm-new-counter');
          if (c) c.textContent = String(ta.value.length);
        });
      }
      // Defer focus so the modal's autofocus on first focusable doesn't
      // steal the cursor back to the search input.
      setTimeout(() => ta.focus(), 0);
    }
  }

  // Triggered when the user picks someone from the search results.
  function startConversation(u) {
    showComposeStage(u);
  }

  async function sendNewConversation() {
    const u = _pendingRecipient;
    if (!u) return;
    const ta = document.getElementById('community-dm-new-message');
    const status = document.getElementById('community-dm-new-status');
    const sendBtn = document.getElementById('community-dm-new-send-btn');
    const content = (ta.value || '').trim();
    if (content.length < 1) {
      status.style.color = '#B42318';
      status.textContent = 'Message cannot be empty.';
      return;
    }
    status.style.color = '';
    status.textContent = 'Creating conversation…';
    sendBtn.disabled = true;
    try {
      const res = await fetch('/api/community/dms/threads', {
        method: 'POST',
        credentials: 'same-origin',
        headers: csrfHeaders({ 'Content-Type': 'application/json' }),
        body: JSON.stringify({ recipient_user_id: u.user_id, content }),
      });
      if (!res.ok) {
        const errText = await res.text();
        throw new Error(`HTTP ${res.status}: ${errText}`);
      }
      const data = await res.json();
      if (typeof window.closeCommunityModal === 'function') {
        window.closeCommunityModal('community-dm-new-modal');
      } else {
        document.getElementById('community-dm-new-modal').style.display = 'none';
      }
      showSearchStage();
      await loadThreads();
      openThread({
        thread_id: data.thread_id,
        other_user_id: u.user_id,
        other_display_name: u.display_name,
        other_avatar_url: u.avatar_url,
      });
    } catch (err) {
      status.style.color = '#B42318';
      status.textContent = `Failed: ${err.message}`;
    } finally {
      sendBtn.disabled = false;
    }
  }

  function init() {
    fetchCurrentUserId();
    const newBtn = document.getElementById('community-dm-new-btn');
    if (newBtn) newBtn.addEventListener('click', openNewModal);

    const search = document.getElementById('community-dm-search');
    if (search) {
      search.addEventListener('input', () => {
        clearTimeout(searchDebounce);
        searchDebounce = setTimeout(() => searchUsers(search.value.trim()), 200);
      });
    }

    // Compose-stage controls
    const backBtn = document.getElementById('community-dm-new-back-btn');
    if (backBtn) backBtn.addEventListener('click', showSearchStage);
    const sendBtn = document.getElementById('community-dm-new-send-btn');
    if (sendBtn) sendBtn.addEventListener('click', sendNewConversation);
    const messageTa = document.getElementById('community-dm-new-message');
    if (messageTa) {
      messageTa.addEventListener('keydown', (event) => {
        // Ctrl/Cmd+Enter sends — easier than reaching for the button.
        if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
          event.preventDefault();
          sendNewConversation();
        }
      });
    }

    // Stop polling when leaving the DM tab.
    document.querySelectorAll('.community-tab-btn').forEach((btn) => {
      btn.addEventListener('click', () => {
        if (btn.dataset.clientTab !== 'dms') stopPolling();
      });
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
