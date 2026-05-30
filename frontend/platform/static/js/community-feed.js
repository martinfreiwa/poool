window.initCommunityFeed = function() {
    // Lightweight toast helper — falls back to window.alert only if toast.js
    // failed to load. Default kind is 'error' so unannotated failure paths
    // get the right styling.
    const toast = (msg, kind) => (typeof window.showToast === 'function')
        ? window.showToast(msg, kind || 'error')
        : window.alert(msg);

    // MOB.6: tiny haptic helper. Vibration API only works on Android-style
    // touch devices that opt in; iOS Safari and desktop are no-ops. Call
    // sites should keep durations short (≤10ms) so the device feels snappy
    // rather than buzzy.
    function haptic(ms) {
        try {
            if (navigator.vibrate) navigator.vibrate(ms || 10);
        } catch (_) { /* permission denied / unsupported */ }
    }
    window.pooolHaptic = haptic;

    function getCsrfToken() {
        if (typeof window.getCsrfToken === 'function') return window.getCsrfToken();
        if (typeof window.csrfToken === 'function') return window.csrfToken();
        const value = `; ${document.cookie}`;
        const parts = value.split('; csrf_token=');
        return parts.length === 2 ? decodeURIComponent(parts.pop().split(';').shift()) : '';
    }

    function csrfHeaders(headers = {}) {
        const token = getCsrfToken();
        return token ? { ...headers, 'X-CSRF-Token': token } : headers;
    }

    function renderSkeleton() {
        const feedContainer = document.getElementById('community-feed-container');
        if (!feedContainer) return;
        feedContainer.innerHTML = `
            <div class="feed-post" style="opacity: 0.6; pointer-events: none;">
                <div class="feed-post-header">
                    <div class="feed-post-author">
                        <div class="feed-post-avatar-circle" style="background:#eaecf0;"></div>
                        <div class="feed-post-meta" style="width: 150px;">
                            <div style="height: 14px; background: #eaecf0; border-radius: 4px; margin-bottom: 6px;"></div>
                            <div style="height: 10px; width: 80px; background: #f2f4f7; border-radius: 4px;"></div>
                        </div>
                    </div>
                </div>
                <div class="feed-post-body">
                    <div style="height: 14px; background: #eaecf0; border-radius: 4px; margin-bottom: 8px; width: 90%;"></div>
                    <div style="height: 14px; background: #eaecf0; border-radius: 4px; margin-bottom: 8px; width: 80%;"></div>
                    <div style="height: 14px; background: #eaecf0; border-radius: 4px; width: 60%;"></div>
                </div>
            </div>
            <div class="feed-post" style="opacity: 0.4; pointer-events: none;">
                <div class="feed-post-header">
                    <div class="feed-post-author">
                        <div class="feed-post-avatar-circle" style="background:#eaecf0;"></div>
                        <div class="feed-post-meta" style="width: 150px;">
                            <div style="height: 14px; background: #eaecf0; border-radius: 4px; margin-bottom: 6px;"></div>
                        </div>
                    </div>
                </div>
                <div class="feed-post-body">
                    <div style="height: 14px; background: #eaecf0; border-radius: 4px; margin-bottom: 8px; width: 50%;"></div>
                </div>
            </div>
        `;
    }

    function renderEmptyState() {
        const feedContainer = document.getElementById('community-feed-container');
        if (!feedContainer) return;
        feedContainer.innerHTML = `
            <div style="text-align: center; padding: 40px 20px; color: #667085;">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#D0D5DD" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="margin-bottom:16px;">
                    <circle cx="12" cy="12" r="10"></circle>
                    <path d="M8 12h8"></path>
                </svg>
                <div style="font-size: 16px; font-weight: 500; color: #101828; margin-bottom: 4px;">No announcements yet</div>
                <div style="font-size: 14px;">Check back later for updates from the platform and community.</div>
            </div>
        `;
    }

    async function toggleReaction(postId, btn, type) {
        const isCurrentlyActive = btn.classList.contains('active');
        const countSpan = btn.querySelector('span');
        const currentCount = Number.parseInt(countSpan.textContent, 10) || 0;
        btn.disabled = true;
        haptic(8);

        try {
            const res = await fetch(`/api/community/posts/${postId}/reactions`, {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify({ reaction_type: type })
            });
            if (!res.ok) throw new Error(await res.text());
            const data = await res.json();
            btn.classList.toggle('active', Boolean(data.added));
            btn.setAttribute('aria-pressed', data.added ? 'true' : 'false');
            countSpan.textContent = Number.isInteger(data.reaction_count)
                ? data.reaction_count
                : Math.max(0, currentCount + (data.added ? 1 : -1));
        } catch (e) {
            console.error('Failed to toggle reaction', e);
            btn.classList.toggle('active', isCurrentlyActive);
            btn.setAttribute('aria-pressed', isCurrentlyActive ? 'true' : 'false');
            countSpan.textContent = currentCount;
        } finally {
            btn.disabled = false;
        }
    }

    window.toggleReaction = toggleReaction; // Global binding for inline handlers

    function timeAgo(dateString) {
        const date = new Date(dateString);
        const seconds = Math.floor((new Date() - date) / 1000);
        const fmt = (n, unit) => `${n} ${unit}${n === 1 ? '' : 's'} ago`;
        let n = Math.floor(seconds / 31536000);
        if (n >= 1) return fmt(n, 'year');
        n = Math.floor(seconds / 2592000);
        if (n >= 1) return fmt(n, 'month');
        n = Math.floor(seconds / 86400);
        if (n >= 1) return fmt(n, 'day');
        n = Math.floor(seconds / 3600);
        if (n >= 1) return fmt(n, 'hour');
        n = Math.floor(seconds / 60);
        if (n >= 1) return fmt(n, 'minute');
        return fmt(Math.max(1, Math.floor(seconds)), 'second');
    }

    let currentFeedMode = 'all';
    let currentSortMode = 'fresh';

    function setToggleActive(activeBtn, inactiveBtn) {
        if (!activeBtn || !inactiveBtn) return;
        activeBtn.classList.add('active');
        activeBtn.setAttribute('aria-pressed', 'true');
        inactiveBtn.classList.remove('active');
        inactiveBtn.setAttribute('aria-pressed', 'false');
    }

    window.setFeedMode = function(mode) {
        currentFeedMode = mode;
        const btnAll = document.getElementById('feed-btn-all');
        const btnFollowing = document.getElementById('feed-btn-following');
        if (mode === 'all') {
            setToggleActive(btnAll, btnFollowing);
        } else {
            setToggleActive(btnFollowing, btnAll);
        }

        const formInput = document.getElementById('form-feed-mode');
        if (formInput) formInput.value = mode;
        document.body.dispatchEvent(new Event('reload-feed'));
    };

    window.setSortMode = function(mode) {
        currentSortMode = mode;
        const btnFresh = document.getElementById('sort-btn-fresh');
        const btnHot = document.getElementById('sort-btn-hot');
        if (mode === 'fresh') {
            setToggleActive(btnFresh, btnHot);
        } else {
            setToggleActive(btnHot, btnFresh);
        }

        const formInput = document.getElementById('form-sort-by');
        if (formInput) formInput.value = mode;
        document.body.dispatchEvent(new Event('reload-feed'));
    };

    window.setPostTypeFilter = function(postType) {
        const value = postType || 'all';
        const formInput = document.getElementById('form-post-type');
        if (formInput) formInput.value = value;
        const select = document.getElementById('feed-post-type-filter');
        if (select && select.value !== value) select.value = value;
        document.body.dispatchEvent(new Event('reload-feed'));
    };

    window.setPostTagFilter = function(tag) {
        const value = tag || 'all';
        const formInput = document.getElementById('form-post-tag');
        if (formInput) formInput.value = value;
        const select = document.getElementById('feed-post-tag-filter');
        if (select && select.value !== value) select.value = value;
        document.body.dispatchEvent(new Event('reload-feed'));
    };

    // ─── XSS-safe helpers ───────────────────────────────────────
    function getInitials(name) {
        if (!name) return '?';
        const parts = name.split(' ');
        return (parts.length > 1 ? parts[0][0] + parts[1][0] : parts[0].substring(0, 2)).toUpperCase();
    }

    function escapeAttr(str) {
        return escapeHtml(str).replace(/"/g, '&quot;').replace(/'/g, '&#39;');
    }

    function escapeHtml(str) {
        if (!str) return '';
        return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }

    function appendTextEmptyState(container, text, _legacyStyles) {
        if (!container) return;
        container.replaceChildren();
        const empty = document.createElement('div');
        empty.className = 'community-loading-state';
        empty.textContent = text;
        container.appendChild(empty);
    }

    // Deprecated Client-Side Functions have been removed in favor of HTMX server-side rendering.

    // ─── COMMENTS LOGIC ───────────────────────────────────────
    
    // UX.12 extension: comment-textarea drafts. Per-post key so multiple
    // half-written comments survive a tab close. Cleared on successful submit.
    function commentDraftKey(postId) { return 'poool:community:comment:' + postId + ':v1'; }
    function restoreCommentDraft(postId) {
        const ta = document.getElementById('comment-input-' + postId);
        if (!ta) return;
        try {
            const draft = localStorage.getItem(commentDraftKey(postId));
            if (draft && !ta.value) ta.value = draft;
        } catch (_) { /* localStorage disabled */ }
    }

    window.toggleComments = async function(postId, trigger) {
        const section = document.getElementById(`comments-section-${postId}`);
        const isOpening = section.hidden
            || section.style.display === 'none'
            || section.style.display === '';
        section.hidden = false;
        section.style.display = isOpening ? 'block' : 'none';
        document
            .querySelectorAll(`[aria-controls="comments-section-${postId}"]`)
            .forEach((control) => control.setAttribute('aria-expanded', isOpening ? 'true' : 'false'));
        if (trigger && trigger.setAttribute) {
            trigger.setAttribute('aria-expanded', isOpening ? 'true' : 'false');
        }
        if (isOpening) {
            section.style.display = 'block';
            restoreCommentDraft(postId);
            await loadComments(postId);
        }
    };

    // Delegated, debounced auto-save for any comment textarea in the document.
    // Survives HTMX swaps and feed reloads without a per-card binding.
    let _commentDraftTimer = null;
    document.addEventListener('input', (event) => {
        const target = event.target;
        if (!(target instanceof HTMLTextAreaElement)) return;
        if (!target.id || !target.id.startsWith('comment-input-')) return;
        const postId = target.id.replace('comment-input-', '');
        clearTimeout(_commentDraftTimer);
        _commentDraftTimer = setTimeout(() => {
            try {
                if (target.value && target.value.trim()) {
                    localStorage.setItem(commentDraftKey(postId), target.value);
                } else {
                    localStorage.removeItem(commentDraftKey(postId));
                }
            } catch (_) { /* quota / disabled */ }
        }, 500);
    });

    window.loadComments = async function(postId) {
        const listContainer = document.getElementById(`comments-list-${postId}`);
        try {
            const res = await fetch(`/api/community/posts/${postId}/comments`, { credentials: 'same-origin' });
            if (!res.ok) throw new Error("Failed to load comments");
            const comments = await res.json();
            
            if (comments.length === 0) {
                const emptyEl = document.createElement('div');
                emptyEl.className = 'community-comments-empty';
                emptyEl.textContent = 'No comments yet. Be the first to start the discussion!';
                listContainer.replaceChildren(emptyEl);
                return;
            }

            listContainer.replaceChildren();
            const currentUserId =
                (window.__POOOL_USER && (window.__POOOL_USER.id || window.__POOOL_USER.user_id)) || null;

            // WS1.1 — group comments into top-level + replies. Depth cap of 1
            // is enforced server-side; client just renders top-level rows
            // and nests reply children under each parent.
            const repliesByParent = new Map();
            const topLevel = [];
            comments.forEach((c) => {
                if (c.parent_comment_id) {
                    if (!repliesByParent.has(c.parent_comment_id)) {
                        repliesByParent.set(c.parent_comment_id, []);
                    }
                    repliesByParent.get(c.parent_comment_id).push(c);
                } else {
                    topLevel.push(c);
                }
            });

            const buildCommentRow = (c, isReply) => {
                const row = document.createElement('div');
                row.className = 'community-comment-row' + (isReply ? ' community-comment-row--reply' : '');
                row.dataset.commentId = c.id;

                // Avatar
                if (c.author_avatar) {
                    const img = document.createElement('img');
                    img.src = c.author_avatar;
                    img.alt = '';
                    img.className = 'community-comment-row__avatar';
                    row.appendChild(img);
                } else {
                    const avatarDiv = document.createElement('div');
                    avatarDiv.className = 'community-comment-row__avatar community-comment-row__avatar--initials';
                    avatarDiv.textContent = getInitials(c.author_name);
                    row.appendChild(avatarDiv);
                }

                // Comment body
                const body = document.createElement('div');
                body.className = 'community-comment-row__body';

                const header = document.createElement('div');
                header.className = 'community-comment-row__header';
                const nameSpan = document.createElement('span');
                nameSpan.className = 'community-comment-row__name';
                nameSpan.textContent = c.author_name; // SAFE: textContent escapes HTML
                const timeSpan = document.createElement('span');
                timeSpan.className = 'community-comment-row__time';
                let timeText = timeAgo(c.created_at);
                if (c.edited_at) timeText += ' · Edited';
                timeSpan.textContent = timeText;
                header.appendChild(nameSpan);
                header.appendChild(timeSpan);

                const contentDiv = document.createElement('div');
                contentDiv.className = 'community-comment-row__content';
                contentDiv.textContent = c.content; // SAFE: textContent escapes HTML

                body.appendChild(header);
                if (c.is_official_answer || c.is_verified_answer) {
                    const answerBadges = document.createElement('div');
                    answerBadges.className = 'community-comment-row__answer-badges';
                    if (c.is_official_answer) {
                        const officialBadge = document.createElement('span');
                        officialBadge.className = 'community-comment-row__answer-badge community-comment-row__answer-badge--official';
                        officialBadge.textContent = 'Official Answer';
                        answerBadges.appendChild(officialBadge);
                    }
                    if (c.is_verified_answer) {
                        const verifiedBadge = document.createElement('span');
                        verifiedBadge.className = 'community-comment-row__answer-badge community-comment-row__answer-badge--verified';
                        verifiedBadge.textContent = 'Verified Answer';
                        answerBadges.appendChild(verifiedBadge);
                    }
                    body.appendChild(answerBadges);
                }
                body.appendChild(contentDiv);

                // 14.8.6 — reaction button on every comment row.
                const reactionRow = document.createElement('div');
                reactionRow.className = 'community-comment-row__reactions';
                const reactBtn = document.createElement('button');
                reactBtn.type = 'button';
                reactBtn.className = 'community-comment-row__reaction-btn';
                const initialCount = Number.isInteger(c.reaction_count) ? c.reaction_count : 0;
                reactBtn.setAttribute('aria-pressed', 'false');
                reactBtn.setAttribute('aria-label', 'React to comment');
                const heartIcon = document.createElement('span');
                heartIcon.setAttribute('aria-hidden', 'true');
                heartIcon.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/></svg>';
                heartIcon.style.display = 'inline-flex';
                heartIcon.style.alignItems = 'center';
                const countSpan = document.createElement('span');
                countSpan.className = 'community-comment-row__reaction-count';
                countSpan.textContent = String(initialCount);
                reactBtn.appendChild(heartIcon);
                reactBtn.appendChild(countSpan);
                reactBtn.addEventListener('click', () => toggleCommentReaction(c.id, reactBtn));
                reactionRow.appendChild(reactBtn);
                body.appendChild(reactionRow);

                // 14.8.5 — own-comment edit affordance.
                if (currentUserId && c.author_id === currentUserId) {
                    const ownActions = document.createElement('div');
                    ownActions.className = 'community-comment-row__own-actions';

                    const editBtn = document.createElement('button');
                    editBtn.type = 'button';
                    editBtn.className = 'ds-btn ds-btn--ghost ds-btn--sm community-comment-row__edit-btn';
                    editBtn.textContent = 'Edit';
                    editBtn.setAttribute('aria-label', 'Edit comment');
                    editBtn.addEventListener('click', () =>
                        startCommentEdit(row, c.id, c.content, contentDiv, timeSpan, editBtn)
                    );
                    ownActions.appendChild(editBtn);

                    // Phase 3 task 26 — own-comment delete.
                    const deleteBtn = document.createElement('button');
                    deleteBtn.type = 'button';
                    deleteBtn.className = 'ds-btn ds-btn--ghost ds-btn--sm community-comment-row__delete-btn';
                    deleteBtn.textContent = 'Delete';
                    deleteBtn.setAttribute('aria-label', 'Delete comment');
                    deleteBtn.addEventListener('click', async () => {
                        if (!confirm('Delete this comment? This cannot be undone.')) return;
                        deleteBtn.disabled = true;
                        try {
                            const res = await fetch(`/api/community/comments/${encodeURIComponent(c.id)}`, {
                                method: 'DELETE',
                                credentials: 'same-origin',
                                headers: csrfHeaders(),
                            });
                            if (!res.ok) throw new Error(await res.text());
                            row.style.transition = 'opacity 0.2s ease';
                            row.style.opacity = '0';
                            setTimeout(() => row.remove(), 200);
                            if (window.showToast) window.showToast('Comment deleted', 'success');
                        } catch (err) {
                            console.error('Delete comment failed', err);
                            if (window.showToast) window.showToast('Failed to delete comment', 'error');
                            deleteBtn.disabled = false;
                        }
                    });
                    ownActions.appendChild(deleteBtn);
                    body.appendChild(ownActions);
                }

                if (c.can_mark_official_answer && !isReply) {
                    const answerAction = document.createElement('button');
                    answerAction.type = 'button';
                    answerAction.className = 'ds-btn ds-btn--ghost ds-btn--sm community-comment-row__answer-btn';
                    answerAction.textContent = c.is_official_answer ? 'Remove official answer' : 'Mark official answer';
                    answerAction.setAttribute('aria-label', answerAction.textContent);
                    answerAction.addEventListener('click', () =>
                        window.markOfficialAnswer(c.id, !c.is_official_answer)
                    );
                    body.appendChild(answerAction);
                }

                // WS1.1 — Reply button on top-level comments. Opens an inline
                // textarea that submits with parent_comment_id set.
                if (!isReply) {
                    const replyBtn = document.createElement('button');
                    replyBtn.type = 'button';
                    replyBtn.className = 'ds-btn ds-btn--ghost ds-btn--sm community-comment-row__reply-btn';
                    replyBtn.textContent = 'Reply';
                    replyBtn.setAttribute('aria-label', 'Reply to comment');
                    replyBtn.addEventListener('click', () => openInlineReply(row, c.id, postId));
                    body.appendChild(replyBtn);
                }

                row.appendChild(body);
                return row;
            };

            // UX.7 — collapsible reply threads. If a parent has more than
            // two replies we hide them behind a "Show N replies" toggle so
            // the comment column stays scannable.
            topLevel.forEach((c) => {
                const parentRow = buildCommentRow(c, false);
                listContainer.appendChild(parentRow);

                const replies = repliesByParent.get(c.id) || [];
                if (replies.length === 0) return;

                const repliesWrap = document.createElement('div');
                repliesWrap.className = 'community-comment-replies';
                replies.forEach((r) => repliesWrap.appendChild(buildCommentRow(r, true)));

                if (replies.length > 2) {
                    const toggleBtn = document.createElement('button');
                    toggleBtn.type = 'button';
                    toggleBtn.className = 'ds-btn ds-btn--ghost ds-btn--sm community-comment-replies-toggle';
                    let collapsed = true;
                    const labelCollapsed = `▾ Show ${replies.length} replies`;
                    const labelExpanded = '▴ Hide replies';
                    toggleBtn.textContent = labelCollapsed;
                    toggleBtn.setAttribute('aria-expanded', 'false');
                    repliesWrap.hidden = true;
                    toggleBtn.addEventListener('click', () => {
                        collapsed = !collapsed;
                        repliesWrap.hidden = collapsed;
                        toggleBtn.textContent = collapsed ? labelCollapsed : labelExpanded;
                        toggleBtn.setAttribute('aria-expanded', collapsed ? 'false' : 'true');
                    });
                    listContainer.appendChild(toggleBtn);
                }
                listContainer.appendChild(repliesWrap);
            });
        } catch (e) {
            console.error(e);
            listContainer.innerHTML = '<div class="community-comments-error">Failed to load comments.</div>';
        }
    };

    // WS1.1 — inline reply composer rendered below a top-level comment.
    function openInlineReply(parentRow, parentCommentId, postId) {
        // Avoid duplicate inline composer on rapid clicks.
        if (parentRow.nextElementSibling && parentRow.nextElementSibling.classList.contains('community-comment-row__reply-form')) {
            parentRow.nextElementSibling.querySelector('textarea')?.focus();
            return;
        }
        const wrap = document.createElement('div');
        wrap.className = 'community-comment-row__reply-form';
        const ta = document.createElement('textarea');
        ta.className = 'ds-input';
        ta.rows = 1;
        ta.placeholder = 'Write a reply...';
        const actions = document.createElement('div');
        actions.className = 'community-comment-row__reply-form-actions';
        const cancel = document.createElement('button');
        cancel.type = 'button';
        cancel.className = 'ds-btn ds-btn--ghost ds-btn--sm';
        cancel.textContent = 'Cancel';
        cancel.addEventListener('click', () => wrap.remove());
        const send = document.createElement('button');
        send.type = 'button';
        send.className = 'ds-btn ds-btn--primary ds-btn--sm';
        send.textContent = 'Reply';
        send.addEventListener('click', async () => {
            const content = ta.value.trim();
            if (!content) return;
            send.disabled = true;
            send.textContent = 'Posting...';
            try {
                const res = await fetch(`/api/community/posts/${postId}/comments`, {
                    method: 'POST',
                    credentials: 'same-origin',
                    headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                    body: JSON.stringify({ content, parent_comment_id: parentCommentId }),
                });
                if (!res.ok) throw new Error(await res.text());
                wrap.remove();
                await window.loadComments(postId);
                loadCircleEngagement();
            } catch (err) {
                console.error('Reply failed', err);
                send.disabled = false;
                send.textContent = 'Reply';
                if (window.showToast) window.showToast('Failed to post reply', 'error');
            }
        });
        actions.append(cancel, send);
        wrap.append(ta, actions);
        parentRow.after(wrap);
        ta.focus();
    }

    window.submitComment = async function(postId) {
        const input = document.getElementById(`comment-input-${postId}`);
        const content = input.value.trim();
        if (!content) return;

        try {
            input.disabled = true;
            const res = await fetch(`/api/community/posts/${postId}/comments`, {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify({ content })
            });
            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }
            input.value = '';
            input.disabled = false;
            // UX.12: drop the per-post comment draft after successful post.
            try { localStorage.removeItem(commentDraftKey(postId)); } catch (_) {}
            await loadComments(postId); // reload list
            loadCircleEngagement();
        } catch (e) {
            console.error(e);
            toast("Failed to post comment: " + e.message);
            input.disabled = false;
        }
    };

    // ─── USER PROFILE LOGIC (M3) ─────────────────────────────
    
    let currentProfileId = null;
    // Exposed so the followers/following stat buttons (rendered in
    // community.html) can target the right user without an extra plumbing
    // round-trip.
    Object.defineProperty(window, 'currentProfileId', {
        get() { return currentProfileId; },
        configurable: true,
    });

    // openUserProfile now navigates to the dedicated /community/u/:id sub-page
    // instead of opening the in-page modal. The full sub-page renders the
    // same data plus the user's posts and a real follow/mute/block toolbar
    // with no z-index quirks. The legacy modal markup in community.html is
    // unused and can be removed in a follow-up commit.
    window.openUserProfile = function (userId) {
        if (!userId) return;
        window.location.href = '/community/u/' + encodeURIComponent(userId);
    };

    window.toggleFollow = async function(userId, currentlyFollowing, btnElement) {
        try {
            btnElement.disabled = true;
            btnElement.innerText = "Updating...";

            // Optional optimistic-counter element — only exists on the
            // legacy profile-modal layout. On the standalone
            // /community/u/:id page it's absent, so guard with null check.
            const followersEl = document.getElementById('profile-modal-followers');
            if (currentlyFollowing) {
                const res = await fetch(`/api/community/follow/${userId}`, { method: 'DELETE', credentials: 'same-origin', headers: csrfHeaders() });
                if (!res.ok) throw new Error("Failed to unfollow");

                btnElement.innerText = "Follow User";
                btnElement.className = "ds-btn ds-btn--primary";

                if (followersEl) {
                    followersEl.innerText = Math.max(0, parseInt(followersEl.innerText) - 1);
                }
            } else {
                const res = await fetch(`/api/community/follow/${userId}`, { method: 'POST', credentials: 'same-origin', headers: csrfHeaders() });
                if (!res.ok) {
                    const err = await res.text();
                    throw new Error(err);
                }
                btnElement.innerText = "Unfollow";
                btnElement.className = "ds-btn ds-btn--secondary";

                if (followersEl) {
                    followersEl.innerText = parseInt(followersEl.innerText) + 1;
                }
            }
            // Bind the new toggle state
            btnElement.onclick = () => toggleFollow(userId, !currentlyFollowing, btnElement);
        } catch (e) {
            toast(e.message || "Failed to toggle follow status");
            btnElement.innerText = currentlyFollowing ? "Unfollow" : "Follow User";
        } finally {
            btnElement.disabled = false;
        }
    };

    // ─── 14.8.6: comment reactions ─────────────────────────────────
    async function toggleCommentReaction(commentId, btn) {
        const countEl = btn.querySelector('.community-comment-row__reaction-count');
        const wasPressed = btn.getAttribute('aria-pressed') === 'true';
        btn.disabled = true;
        haptic(8);
        try {
            const res = await fetch(`/api/community/comments/${commentId}/reactions`, {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify({ reaction_type: 'fire' }),
            });
            if (!res.ok) throw new Error(await res.text());
            const data = await res.json();
            btn.setAttribute('aria-pressed', data.added ? 'true' : 'false');
            btn.classList.toggle('community-comment-row__reaction-btn--active', Boolean(data.added));
            if (countEl && Number.isInteger(data.reaction_count)) {
                countEl.textContent = String(data.reaction_count);
            }
        } catch (e) {
            console.error('Failed to toggle comment reaction', e);
            btn.setAttribute('aria-pressed', wasPressed ? 'true' : 'false');
        } finally {
            btn.disabled = false;
        }
    }
    window.toggleCommentReaction = toggleCommentReaction;

    // ─── 14.8.5: comment edit (own) ────────────────────────────────
    function startCommentEdit(row, commentId, currentText, contentDiv, timeSpan, editBtn) {
        // Build inline editor.
        const editor = document.createElement('div');
        editor.className = 'community-comment-row__editor';
        const textarea = document.createElement('textarea');
        textarea.className = 'ds-input community-comment-row__editor-textarea';
        textarea.rows = 3;
        textarea.value = currentText;
        editor.appendChild(textarea);
        const actions = document.createElement('div');
        actions.className = 'community-comment-row__editor-actions';
        const cancel = document.createElement('button');
        cancel.type = 'button';
        cancel.className = 'ds-btn ds-btn--secondary ds-btn--sm';
        cancel.textContent = 'Cancel';
        const save = document.createElement('button');
        save.type = 'button';
        save.className = 'ds-btn ds-btn--primary ds-btn--sm';
        save.textContent = 'Save';
        actions.appendChild(cancel);
        actions.appendChild(save);
        editor.appendChild(actions);

        contentDiv.replaceWith(editor);
        editBtn.hidden = true;
        textarea.focus();
        textarea.setSelectionRange(textarea.value.length, textarea.value.length);

        cancel.addEventListener('click', () => {
            editor.replaceWith(contentDiv);
            editBtn.hidden = false;
        });

        save.addEventListener('click', async () => {
            const next = textarea.value.trim();
            if (next.length < 1) return;
            if (next === currentText) {
                editor.replaceWith(contentDiv);
                editBtn.hidden = false;
                return;
            }
            save.disabled = true;
            save.textContent = 'Saving…';
            try {
                const res = await fetch(`/api/community/comments/${commentId}`, {
                    method: 'PUT',
                    credentials: 'same-origin',
                    headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                    body: JSON.stringify({ content: next }),
                });
                if (!res.ok) {
                    const err = await res.text();
                    throw new Error(err || `HTTP ${res.status}`);
                }
                contentDiv.textContent = next;
                if (timeSpan && !timeSpan.textContent.includes('Edited')) {
                    timeSpan.textContent = `${timeSpan.textContent} · Edited`;
                }
                editor.replaceWith(contentDiv);
                editBtn.hidden = false;
                if (typeof window.showToast === 'function') {
                    window.showToast('Comment updated.');
                }
            } catch (e) {
                console.error('Failed to update comment', e);
                save.disabled = false;
                save.textContent = 'Save';
                if (typeof window.showToast === 'function') {
                    window.showToast('Could not update comment. Please try again.');
                }
            }
        });
    }

    // ─── 14.8.2: block / mute toggles ──────────────────────────────
    async function toggleRelation({ kind, userId, currentlyOn, btnElement, onLabel, offLabel }) {
        try {
            btnElement.disabled = true;
            const prevText = btnElement.textContent;
            btnElement.textContent = currentlyOn ? `Un${kind}ing…` : `${kind.charAt(0).toUpperCase() + kind.slice(1)}ing…`;
            const method = currentlyOn ? 'DELETE' : 'POST';
            const res = await fetch(`/api/community/users/${userId}/${kind}`, {
                method,
                credentials: 'same-origin',
                headers: csrfHeaders(),
            });
            if (!res.ok) throw new Error(prevText);
            const newState = !currentlyOn;
            btnElement.textContent = newState ? offLabel : onLabel;
            btnElement.onclick = () => toggleRelation({ kind, userId, currentlyOn: newState, btnElement, onLabel, offLabel });
            if (typeof window.showToast === 'function') {
                window.showToast(
                    newState
                        ? `${kind.charAt(0).toUpperCase() + kind.slice(1)}ed.`
                        : `Un${kind}d.`
                );
            }
            if (kind === 'block') {
                // Reciprocal block hides the target's posts; refresh feed if visible.
                document.body.dispatchEvent(new Event('reload-feed'));
            }
        } catch (e) {
            console.error(`Failed to toggle ${kind}`, e);
            if (typeof window.showToast === 'function') {
                window.showToast(`Could not update ${kind} state. Please try again.`);
            }
        } finally {
            btnElement.disabled = false;
        }
    }

    window.toggleMute = function(userId, currentlyMuted, btnElement) {
        toggleRelation({
            kind: 'mute',
            userId,
            currentlyOn: currentlyMuted,
            btnElement,
            onLabel: 'Mute',
            offLabel: 'Unmute',
        });
    };

    window.toggleBlock = function(userId, currentlyBlocked, btnElement) {
        toggleRelation({
            kind: 'block',
            userId,
            currentlyOn: currentlyBlocked,
            btnElement,
            onLabel: 'Block',
            offLabel: 'Unblock',
        });
    };

    // ─── M2 CREATE POST & REPORT LOGIC ─────────────────────────────
    
    window.selectPostType = function(btn) {
        document.querySelectorAll('.post-type-btn').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        const typeInput = document.getElementById('post-type-input');
        if (typeInput) typeInput.value = btn.getAttribute('data-type');
        updateDisclaimerWarning();
    };

    const contentInput = document.getElementById('post-content-input');
    const postTypeInput = document.getElementById('post-type-input');
    const postTagsInput = document.getElementById('post-tags-input');
    const circleContext = window.POOOL_CIRCLE_CONTEXT && window.POOOL_CIRCLE_CONTEXT.id
        ? window.POOOL_CIRCLE_CONTEXT
        : null;
    function getPostCreateEndpoint() {
        if (!circleContext) return '/api/community/posts';
        return `/api/community/circles/${encodeURIComponent(circleContext.id)}/posts`;
    }

    async function fetchCircleJson(path) {
        const res = await fetch(path, { credentials: 'same-origin' });
        if (!res.ok) throw new Error(await res.text());
        return res.json();
    }

    function formatCircleDate(value) {
        if (!value) return '';
        const date = new Date(value);
        if (Number.isNaN(date.getTime())) return '';
        return new Intl.DateTimeFormat(undefined, {
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
        }).format(date);
    }

    function setCircleEngagementEmpty(containerId, message) {
        const container = document.getElementById(containerId);
        if (!container) return;
        const empty = document.createElement('p');
        empty.className = 'circle-engagement-empty';
        empty.textContent = message;
        container.replaceChildren(empty);
    }

    function renderCircleAnnouncements(payload) {
        const container = document.getElementById('circle-announcements-list');
        if (!container) return;
        const announcements = Array.isArray(payload && payload.announcements) ? payload.announcements : [];
        if (!announcements.length) {
            setCircleEngagementEmpty('circle-announcements-list', 'No Circle announcements yet.');
            return;
        }
        container.replaceChildren(...announcements.slice(0, 3).map((announcement) => {
            const item = document.createElement('article');
            item.className = 'circle-engagement-item';

            const title = document.createElement('div');
            title.className = 'circle-engagement-item__title';
            title.textContent = announcement.post_type === 'official_update'
                ? 'Official Update'
                : 'Announcement';

            const body = document.createElement('p');
            body.className = 'circle-engagement-item__body';
            body.textContent = announcement.content || '';

            const meta = document.createElement('div');
            meta.className = 'circle-engagement-item__meta';
            const createdAt = formatCircleDate(announcement.created_at);
            meta.textContent = [announcement.author_name, createdAt].filter(Boolean).join(' · ');

            item.append(title, body, meta);
            return item;
        }));
    }

    function renderCircleEvents(payload) {
        const container = document.getElementById('circle-events-list');
        if (!container) return;
        const events = Array.isArray(payload && payload.events) ? payload.events : [];
        if (!events.length) {
            setCircleEngagementEmpty('circle-events-list', 'No Circle AMAs scheduled.');
            return;
        }
        container.replaceChildren(...events.slice(0, 3).map((event) => {
            const item = document.createElement('article');
            item.className = 'circle-engagement-item';

            const title = document.createElement('div');
            title.className = 'circle-engagement-item__title';
            title.textContent = event.title || 'Circle AMA';

            const body = document.createElement('p');
            body.className = 'circle-engagement-item__body';
            body.textContent = event.description || event.expert_name || '';

            const meta = document.createElement('div');
            meta.className = 'circle-engagement-item__meta';
            const scheduledAt = formatCircleDate(event.scheduled_at);
            meta.textContent = [event.status, scheduledAt].filter(Boolean).join(' · ');

            item.append(title, body, meta);
            return item;
        }));
    }

    function renderCircleResources(payload) {
        const container = document.getElementById('circle-resources-list');
        if (!container) return;
        const resources = Array.isArray(payload && payload.resources) ? payload.resources : [];
        if (!resources.length) {
            setCircleEngagementEmpty('circle-resources-list', 'No resources available yet.');
            return;
        }
        container.replaceChildren(...resources.slice(0, 5).map((resource) => {
            const item = document.createElement('article');
            item.className = 'circle-engagement-item circle-resource-item';

            const title = document.createElement(resource.delivery_url ? 'a' : 'div');
            title.className = 'circle-engagement-item__title circle-resource-item__title';
            title.textContent = resource.title || 'Circle resource';
            if (resource.delivery_url) {
                title.href = resource.delivery_url;
                title.rel = 'noopener noreferrer';
                title.target = '_blank';
            }

            const body = document.createElement('p');
            body.className = 'circle-engagement-item__body';
            body.textContent = resource.description || '';

            const meta = document.createElement('div');
            meta.className = 'circle-engagement-item__meta';
            const typeLabel = String(resource.resource_type || 'resource').replace(/_/g, ' ');
            const scopeLabel = String(resource.access_scope || 'member').replace(/_/g, ' ');
            const versionLabel = resource.version_label ? `Version ${resource.version_label}` : null;
            meta.textContent = [
                resource.is_official ? 'Official' : null,
                typeLabel,
                scopeLabel,
                versionLabel,
            ].filter(Boolean).join(' · ');

            item.append(title, body, meta);
            return item;
        }));
    }

    function renderCircleChallenges(payload) {
        const container = document.getElementById('circle-challenges-list');
        if (!container) return;
        const challenges = Array.isArray(payload && payload.challenges) ? payload.challenges : [];
        if (!challenges.length) {
            setCircleEngagementEmpty('circle-challenges-list', 'No Circle challenges available.');
            return;
        }
        container.replaceChildren(...challenges.slice(0, 6).map((challenge) => {
            const item = document.createElement('div');
            item.className = 'circle-engagement-challenge';
            if (challenge.is_completed) item.classList.add('circle-engagement-challenge--complete');

            const label = document.createElement('div');
            label.className = 'circle-engagement-challenge__label';
            label.textContent = challenge.title || 'Circle challenge';

            const value = document.createElement('div');
            value.className = 'circle-engagement-challenge__value';
            const current = Number(challenge.current_value || 0);
            const target = Number(challenge.requirement_value || 1);
            value.textContent = challenge.is_completed ? 'Complete' : `${Math.min(current, target)} / ${target}`;

            const bar = document.createElement('div');
            bar.className = 'circle-engagement-challenge__bar';
            const fill = document.createElement('span');
            fill.style.width = `${Math.min(100, Math.round((current / Math.max(target, 1)) * 100))}%`;
            bar.appendChild(fill);

            item.append(label, value, bar);
            return item;
        }));
    }

    function renderCircleOnboarding(payload) {
        const panel = document.getElementById('circle-onboarding-panel');
        const list = document.getElementById('circle-onboarding-steps');
        const progress = document.getElementById('circle-onboarding-progress');
        if (!panel || !list) return;
        const steps = Array.isArray(payload && payload.steps) ? payload.steps : [];
        if (!payload || !payload.enabled || payload.is_completed || !steps.length) {
            panel.hidden = true;
            return;
        }

        const completed = steps.filter((step) => step.completed).length;
        if (progress) progress.textContent = `${completed} / ${steps.length}`;

        list.replaceChildren(...steps.map((step) => {
            const row = document.createElement('div');
            row.className = 'circle-onboarding-step';
            if (step.completed) row.classList.add('circle-onboarding-step--complete');

            const status = document.createElement('span');
            status.className = 'circle-onboarding-step__status';
            status.setAttribute('aria-hidden', 'true');
            status.textContent = step.completed ? '✓' : '';

            const label = document.createElement('span');
            label.className = 'circle-onboarding-step__label';
            label.textContent = step.label || step.code;

            row.append(status, label);
            if (!step.completed && step.action === 'confirm') {
                const button = document.createElement('button');
                button.type = 'button';
                button.className = 'circle-onboarding-step__button';
                button.textContent = 'Done';
                button.addEventListener('click', () => window.markCircleOnboardingStep(step.code));
                row.appendChild(button);
            } else if (!step.completed && step.action === 'post_question') {
                const button = document.createElement('button');
                button.type = 'button';
                button.className = 'circle-onboarding-step__button';
                button.textContent = 'Ask';
                button.addEventListener('click', () => {
                    const select = document.getElementById('post-type-input');
                    const input = document.getElementById('post-content-input');
                    if (select) select.value = 'question';
                    if (input) input.focus();
                });
                row.appendChild(button);
            }
            return row;
        }));
        panel.hidden = false;
    }

    async function loadCircleEngagement() {
        if (!circleContext || !circleContext.id) return;
        const id = encodeURIComponent(circleContext.id);
        const requests = [
            fetchCircleJson(`/api/community/circles/${id}/announcements`).then(renderCircleAnnouncements).catch(() => setCircleEngagementEmpty('circle-announcements-list', 'Announcements could not be loaded.')),
            fetchCircleJson(`/api/community/circles/${id}/events`).then(renderCircleEvents).catch(() => setCircleEngagementEmpty('circle-events-list', 'Events could not be loaded.')),
            fetchCircleJson(`/api/community/circles/${id}/resources`).then(renderCircleResources).catch(() => setCircleEngagementEmpty('circle-resources-list', 'Resources could not be loaded.')),
            fetchCircleJson(`/api/community/circles/${id}/challenges`).then(renderCircleChallenges).catch(() => setCircleEngagementEmpty('circle-challenges-list', 'Challenges could not be loaded.')),
            fetchCircleJson(`/api/community/circles/${id}/onboarding`).then(renderCircleOnboarding).catch(() => {
                const panel = document.getElementById('circle-onboarding-panel');
                if (panel) panel.hidden = true;
            }),
        ];
        await Promise.allSettled(requests);
    }

    window.loadCircleEngagement = loadCircleEngagement;
    window.markCircleOnboardingStep = async function(stepCode) {
        if (!circleContext || !circleContext.id || !stepCode) return;
        const res = await fetch(`/api/community/circles/${encodeURIComponent(circleContext.id)}/onboarding/${encodeURIComponent(stepCode)}`, {
            method: 'POST',
            credentials: 'same-origin',
            headers: csrfHeaders({ 'Content-Type': 'application/json' }),
        });
        if (!res.ok) {
            toast('Could not update Circle onboarding.');
            return;
        }
        loadCircleEngagement();
    };
    // UX.12: composer drafts auto-saved to localStorage so an accidental
    // navigation doesn't lose work. Cleared on successful submit. Per-user
    // namespacing isn't necessary because the draft only lives in the
    // current browser profile.
    const DRAFT_KEY = circleContext
        ? `poool:community:circle:${circleContext.id}:draft:v1`
        : 'poool:community:draft:v1';
    let _draftSaveTimer = null;
    const COMPLIANCE_POST_TYPES = new Set([
        'market_insight',
        'property_update',
        'due_diligence',
        'risk_discussion',
        'official_update',
    ]);
    const COMPLIANCE_TAGS = new Set([
        'risk',
        'yield',
        'real_estate',
        'commodity',
        'tokenization',
        'property_update',
        'due_diligence',
        'legal',
        'tax',
        'liquidity',
    ]);
    const INVESTMENT_KEYWORDS = [
        'invest',
        'return',
        'yield',
        'profit',
        'dividend',
        'roi',
        'price target',
        'buy now',
        'sell now',
    ];

    function canonicalCommunityCode(value) {
        return String(value || '')
            .trim()
            .toLowerCase()
            .replace(/[^a-z0-9]+/g, '_')
            .replace(/^_+|_+$/g, '')
            .replace(/_{2,}/g, '_');
    }

    function getSelectedPostType() {
        return canonicalCommunityCode(postTypeInput ? postTypeInput.value : 'discussion') || 'discussion';
    }

    function parsePostTags() {
        if (!postTagsInput || !postTagsInput.value.trim()) return [];
        const tags = [];
        postTagsInput.value.split(',').forEach((raw) => {
            const tag = canonicalCommunityCode(raw);
            if (tag && !tags.includes(tag)) tags.push(tag);
        });
        return tags.slice(0, 8);
    }

    function updateDisclaimerWarning() {
        const warning = document.getElementById('post-disclaimer-warning');
        if (!warning) return;
        const postType = getSelectedPostType();
        const tags = parsePostTags();
        const content = contentInput ? contentInput.value.toLowerCase() : '';
        const needsDisclaimer = COMPLIANCE_POST_TYPES.has(postType)
            || tags.some((tag) => COMPLIANCE_TAGS.has(tag))
            || INVESTMENT_KEYWORDS.some((keyword) => content.includes(keyword));
        warning.hidden = !needsDisclaimer;
        warning.style.display = needsDisclaimer ? 'block' : 'none';
    }

    function saveDraft(value) {
        clearTimeout(_draftSaveTimer);
        _draftSaveTimer = setTimeout(() => {
            try {
                if (value && value.trim()) {
                    localStorage.setItem(DRAFT_KEY, value);
                } else {
                    localStorage.removeItem(DRAFT_KEY);
                }
            } catch (_) { /* quota / disabled */ }
        }, 500);
    }
    window._clearCommunityDraft = function () {
        try { localStorage.removeItem(DRAFT_KEY); } catch (_) {}
    };
    if (contentInput) {
        contentInput.addEventListener('input', () => {
            updateDisclaimerWarning();
            saveDraft(contentInput.value);
        });
        // Restore on init only when the textarea is currently empty (don't
        // overwrite a freshly composed message).
        try {
            const draft = localStorage.getItem(DRAFT_KEY);
            if (draft && !contentInput.value) {
                contentInput.value = draft;
                contentInput.dispatchEvent(new Event('input'));
            }
        } catch (_) { /* localStorage disabled */ }
    }
    if (postTypeInput) postTypeInput.addEventListener('change', updateDisclaimerWarning);
    if (postTagsInput) postTagsInput.addEventListener('input', updateDisclaimerWarning);


    window.postImageUrls = [];

    window.openCircleQaTab = function(event) {
        if (event) event.preventDefault();
        window.location.hash = 'qa';
        const tabItems = document.querySelectorAll('.circle-space-tabs__item');
        tabItems.forEach((item) => {
            const isQa = item.getAttribute('href') && item.getAttribute('href').endsWith('#qa');
            item.classList.toggle('circle-space-tabs__item--active', Boolean(isQa));
            item.setAttribute('aria-selected', isQa ? 'true' : 'false');
            item.setAttribute('tabindex', isQa ? '0' : '-1');
        });
        const panel = document.getElementById('circle-feed-panel');
        if (panel) panel.setAttribute('aria-labelledby', 'circle-tab-qa');
        window.setPostTypeFilter('question');
    };

    window.markOfficialAnswer = async function(commentId, shouldMark) {
        if (!commentId) return;
        try {
            const response = await fetch(`/api/community/comments/${encodeURIComponent(commentId)}/official-answer`, {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify({
                    is_official_answer: Boolean(shouldMark),
                    is_verified_answer: Boolean(shouldMark),
                }),
            });
            const data = await response.json().catch(() => ({}));
            if (!response.ok) throw new Error(data.error || 'Could not update answer status.');
            if (data.post_id) {
                await window.loadComments(data.post_id);
            }
            document.body.dispatchEvent(new Event('reload-feed'));
            if (window.showToast) {
                window.showToast(shouldMark ? 'Official answer marked.' : 'Official answer removed.', 'success');
            }
        } catch (error) {
            console.error('markOfficialAnswer failed', error);
            if (window.showToast) window.showToast(error.message || 'Could not update answer status.', 'error');
        }
    };
    
    // ─── Phase 4 task 33: post image lightbox ─────────────────
    function ensureLightbox() {
        let lb = document.getElementById('community-lightbox');
        if (lb) return lb;
        lb = document.createElement('div');
        lb.id = 'community-lightbox';
        lb.className = 'community-lightbox';
        lb.hidden = true;
        lb.setAttribute('role', 'dialog');
        lb.setAttribute('aria-modal', 'true');
        lb.innerHTML = '<button type="button" class="community-lightbox__close" aria-label="Close">✕</button><img alt="">';
        document.body.appendChild(lb);
        lb.addEventListener('click', (event) => {
            if (event.target === lb || event.target.classList.contains('community-lightbox__close')) {
                lb.hidden = true;
            }
        });
        document.addEventListener('keydown', (event) => {
            if (event.key === 'Escape' && !lb.hidden) lb.hidden = true;
        });
        return lb;
    }
    document.addEventListener('click', (event) => {
        const img = event.target.closest('.feed-post-image-grid__item img');
        if (!img) return;
        event.preventDefault();
        const lb = ensureLightbox();
        const target = lb.querySelector('img');
        target.src = img.src;
        target.alt = img.alt || '';
        lb.hidden = false;
    });

    // ─── Phase 4 task 35: Ctrl+Enter on composer submits ──────
    document.addEventListener('keydown', (event) => {
        if (event.key !== 'Enter' || !(event.ctrlKey || event.metaKey)) return;
        const target = event.target;
        if (!(target instanceof HTMLElement)) return;
        if (target.id === 'post-content-input') {
            event.preventDefault();
            if (typeof window.submitUserPost === 'function') window.submitUserPost();
        } else if (target.matches('textarea[id^="comment-input-"]')) {
            event.preventDefault();
            const postId = target.id.replace('comment-input-', '');
            if (typeof window.submitComment === 'function') window.submitComment(postId);
        }
    });

    // ─── Phase 4 task 36: drag-drop image upload onto composer ──
    window.handleComposerDrop = function (event) {
        event.preventDefault();
        const composer = event.currentTarget;
        if (composer && composer.classList) composer.classList.remove('community-composer--dragover');
        const files = event.dataTransfer && event.dataTransfer.files;
        if (!files || files.length === 0) return;
        // Reuse the existing single-file upload path for each dropped image.
        for (const f of files) {
            const pseudo = { target: { files: [f], value: '' } };
            if (typeof window.uploadPostImage === 'function') {
                window.uploadPostImage(pseudo);
            }
        }
    };

    window.uploadPostImage = async function(e) {
        if (!e.target.files || e.target.files.length === 0) return;
        
        if (window.postImageUrls.length >= 4) {
            toast("Maximum 4 images allowed per post.");
            return;
        }

        const file = e.target.files[0];
        if (file.size > 5 * 1024 * 1024) {
            toast("Image must be smaller than 5MB");
            return;
        }

        document.getElementById('post-image-uploading').style.display = 'block';
        
        const fd = new FormData();
        fd.append('file', file);
        
        try {
            const res = await fetch('/api/upload/post-image', {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders(),
                body: fd
            });
            const data = await res.json();
            
            if (!res.ok) {
                throw new Error(data.error || 'Upload failed');
            }
            
            window.postImageUrls.push(data.image_url);
            renderPostImagePreviews();
            
        } catch (err) {
            console.error(err);
            toast(err.message);
        } finally {
            document.getElementById('post-image-uploading').style.display = 'none';
            e.target.value = ''; // reset file input
        }
    };

    function renderPostImagePreviews() {
        const container = document.getElementById('post-image-previews');
        if (!container) return;

        container.replaceChildren();
        window.postImageUrls.forEach((url, index) => {
            const preview = document.createElement('div');
            preview.className = 'community-composer__preview';

            const image = document.createElement('img');
            image.src = url;
            image.alt = 'Selected post image preview';
            image.className = 'community-composer__preview-img';

            const remove = document.createElement('button');
            remove.type = 'button';
            remove.setAttribute('aria-label', 'Remove selected post image');
            remove.className = 'community-composer__preview-remove';
            remove.textContent = '✕';
            remove.addEventListener('click', () => window.removePostImage(index));

            preview.appendChild(image);
            preview.appendChild(remove);
            container.appendChild(preview);
        });
        
        // Hide upload button if 4 images
        const btn = document.querySelector(`button[onclick="document.getElementById('post-image-file-input').click()"]`);
        if (btn) {
            btn.style.display = window.postImageUrls.length >= 4 ? 'none' : 'inline-flex';
        }
    }

    window.removePostImage = function(index) {
        window.postImageUrls.splice(index, 1);
        renderPostImagePreviews();
    };

    // CO.7 — schedule picker helpers. Show/hide the datetime input and let
    // the user clear a previously chosen time. submitUserPost picks the
    // value up at send time.
    window.toggleSchedulePicker = function () {
        const input = document.getElementById('post-schedule-input');
        const clear = document.getElementById('post-schedule-clear');
        const toggle = document.getElementById('post-schedule-toggle');
        if (!input) return;
        const opening = input.hidden;
        input.hidden = !opening;
        if (clear) clear.hidden = !opening;
        if (toggle) toggle.setAttribute('aria-pressed', opening ? 'true' : 'false');
        if (opening && !input.value) {
            // Default to one hour ahead so the user has a sensible starting point.
            const d = new Date(Date.now() + 60 * 60 * 1000);
            const pad = (n) => String(n).padStart(2, '0');
            input.value = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
        }
        if (opening) input.focus();
    };
    window.clearScheduledPost = function () {
        const input = document.getElementById('post-schedule-input');
        const clear = document.getElementById('post-schedule-clear');
        const toggle = document.getElementById('post-schedule-toggle');
        if (input) { input.value = ''; input.hidden = true; }
        if (clear) clear.hidden = true;
        if (toggle) toggle.setAttribute('aria-pressed', 'false');
    };

    window.submitUserPost = async function() {
        const postType = getSelectedPostType();
        const content = document.getElementById('post-content-input').value.trim();
        const contentTags = parsePostTags();

        if (!content) return toast("Content cannot be empty");
        
        // CO.7 — pick up scheduled timestamp if the picker is open + filled.
        let scheduledFor = null;
        const scheduleInput = document.getElementById('post-schedule-input');
        if (scheduleInput && !scheduleInput.hidden && scheduleInput.value) {
            const ts = new Date(scheduleInput.value);
            if (!Number.isNaN(ts.getTime()) && ts.getTime() > Date.now()) {
                scheduledFor = ts.toISOString();
            } else if (!Number.isNaN(ts.getTime())) {
                return toast("Scheduled time must be in the future.");
            }
        }

        const requestBody = {
            post_type: postType,
            content: content,
            content_tags: contentTags,
            asset_id: null,
            circle_id: circleContext ? circleContext.id : null,
            image_urls: window.postImageUrls.length > 0 ? window.postImageUrls : null,
            // UX.11: Poll data
            poll_question: null,
            poll_options: null,
            poll_expires_hours: null,
            // CO.7: ISO8601 future timestamp; backend hides post until then.
            scheduled_for: scheduledFor,
        };

        // Add poll if enabled
        if (window.pollEnabled) {
            const pollQ = document.getElementById('poll-question-input');
            const pollExpiry = document.getElementById('poll-expiry-select');
            if (pollQ && pollQ.value.trim()) {
                const validOptions = window.pollOptions.filter(o => o.trim() !== '');
                if (validOptions.length >= 2) {
                    requestBody.poll_question = pollQ.value.trim();
                    requestBody.poll_options = validOptions;
                    requestBody.poll_expires_hours = pollExpiry ? parseInt(pollExpiry.value) || null : null;
                    requestBody.post_type = 'poll';
                }
            }
        }
        
        const submitBtn = document.getElementById('submit-post-btn');
        const oldText = submitBtn.innerText;
        submitBtn.innerText = "Posting...";
        submitBtn.disabled = true;

        try {
            const res = await fetch(getPostCreateEndpoint(), {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify(requestBody)
            });
            
            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }
            
            document.getElementById('post-content-input').value = '';
            const tagInput = document.getElementById('post-tags-input');
            if (tagInput) tagInput.value = '';
            const warning = document.getElementById('post-disclaimer-warning');
            if (warning) {
                warning.hidden = true;
                warning.style.display = 'none';
            }
            window.postImageUrls = [];
            renderPostImagePreviews();
            // UX.12: drop the draft so it doesn't reappear on the next visit
            if (typeof window._clearCommunityDraft === 'function') window._clearCommunityDraft();
            // CO.7: reset the schedule picker so the next compose starts clean
            if (typeof window.clearScheduledPost === 'function') window.clearScheduledPost();
            if (scheduledFor && window.showToast) {
                window.showToast('Scheduled — your post will appear at the chosen time.', 'success');
            }

            // Refresh feed via HTMX event
            document.body.dispatchEvent(new Event('reload-feed'));
            loadCircleEngagement();
        } catch (e) {
            console.error(e);
            toast("Failed to submit post: " + e.message);
        } finally {
            submitBtn.innerText = oldText;
            submitBtn.disabled = false;
        }
    };

    // ─── OWNER POST ACTIONS (Phase 2 tasks 12, 13) ──────────────
    //
    // After every feed render we mark posts the viewer authored with
    // data-is-own="true" so the CSS kebab menu becomes visible. We do this in
    // a delegated way so it survives HTMX swaps + the saved/hashtag client
    // builders.
    function markOwnPosts() {
        const me = window.__POOOL_USER && window.__POOOL_USER.id;
        if (!me) return;
        document.querySelectorAll('.feed-post[data-author-id]').forEach((card) => {
            if (card.dataset.authorId === me) {
                card.dataset.isOwn = 'true';
            }
        });
    }

    // Run after HTMX feed list swaps and after initial page paint.
    document.body.addEventListener('htmx:afterSwap', markOwnPosts);
    document.body.addEventListener('reload-feed', () => setTimeout(markOwnPosts, 0));
    window.addEventListener('load', markOwnPosts);
    // user-data.js publishes __POOOL_USER asynchronously and fires
    // `poool:user-ready` once the /api/me payload lands. Listening once
    // beats the old 200ms × 20 polling loop and removes the 4 s window
    // where the kebab menu was missing on slow connections.
    if (window.__POOOL_USER) {
        markOwnPosts();
    } else {
        window.addEventListener('poool:user-ready', markOwnPosts, { once: true });
    }

    window.togglePostOwnerMenu = function (postId, triggerBtn) {
        const dropdown = document.getElementById(`owner-menu-${postId}`);
        if (!dropdown) return;
        const open = dropdown.hasAttribute('hidden');
        // Close any other open menus first.
        document.querySelectorAll('.feed-post-owner-menu__dropdown').forEach((d) => {
            if (d !== dropdown) d.setAttribute('hidden', '');
        });
        if (open) {
            dropdown.removeAttribute('hidden');
            triggerBtn?.setAttribute('aria-expanded', 'true');
        } else {
            dropdown.setAttribute('hidden', '');
            triggerBtn?.setAttribute('aria-expanded', 'false');
        }
    };

    document.addEventListener('click', (event) => {
        if (event.target.closest('.feed-post-owner-menu')) return;
        document.querySelectorAll('.feed-post-owner-menu__dropdown').forEach((d) => d.setAttribute('hidden', ''));
        document.querySelectorAll('.feed-post-owner-menu__toggle[aria-expanded="true"]').forEach((t) => t.setAttribute('aria-expanded', 'false'));
    });

    // ─── MOB.8: Pull-to-refresh on touch devices ────────────────
    // Lightweight, no library. Watches touchstart/move on the feed
    // container, only kicks in when scrollY is 0 and the user drags down
    // past the threshold. Reuses the existing `reload-feed` event so
    // HTMX swaps the partial without a hard reload.
    (function setupPullToRefresh() {
        const TRIGGER_PX = 70;
        const MAX_PULL = 120;
        let startY = null;
        let pulling = false;
        let indicator = null;

        function ensureIndicator() {
            if (indicator) return indicator;
            indicator = document.createElement('div');
            indicator.id = 'community-ptr-indicator';
            indicator.setAttribute('aria-hidden', 'true');
            indicator.style.cssText = 'position:fixed; top:8px; left:50%; transform:translateX(-50%) translateY(-100%); '
                + 'background:var(--ds-bg-elevated, #fff); color:var(--ds-text-primary, #101828); '
                + 'padding:6px 14px; border-radius:999px; font-size:13px; z-index:1500; '
                + 'box-shadow:0 4px 12px rgba(0,0,0,0.1); transition:transform 0.15s ease;';
            indicator.textContent = '↓ Pull to refresh';
            document.body.appendChild(indicator);
            return indicator;
        }

        function shouldArm(event) {
            // Only on touch primary screens scrolled to the top, on the feed page.
            if (!('ontouchstart' in window)) return false;
            if (window.scrollY > 0) return false;
            if (!location.pathname.startsWith('/community')) return false;
            // Don't hijack pull when target is a textarea / input (could be a real swipe).
            const t = event.target;
            if (t && (t.tagName === 'TEXTAREA' || t.tagName === 'INPUT' || t.tagName === 'SELECT')) return false;
            return true;
        }

        document.addEventListener('touchstart', (event) => {
            if (!shouldArm(event)) { startY = null; return; }
            startY = event.touches[0].clientY;
            pulling = false;
        }, { passive: true });

        document.addEventListener('touchmove', (event) => {
            if (startY === null) return;
            const deltaY = event.touches[0].clientY - startY;
            if (deltaY <= 0) return;
            pulling = true;
            const ind = ensureIndicator();
            const pull = Math.min(deltaY, MAX_PULL);
            const offset = -100 + (pull / MAX_PULL) * 110;
            ind.style.transform = `translateX(-50%) translateY(${offset}%)`;
            ind.textContent = pull >= TRIGGER_PX ? '↻ Release to refresh' : '↓ Pull to refresh';
        }, { passive: true });

        document.addEventListener('touchend', () => {
            if (!pulling || startY === null) { startY = null; pulling = false; return; }
            const ind = ensureIndicator();
            const final = parseFloat(ind.style.transform.match(/translateY\(([-0-9.]+)%\)/)?.[1] || '-100');
            const pulled = (final + 100) / 110 * MAX_PULL;
            if (pulled >= TRIGGER_PX) {
                ind.textContent = '↻ Refreshing…';
                ind.style.transform = 'translateX(-50%) translateY(0%)';
                document.body.dispatchEvent(new Event('reload-feed'));
                setTimeout(() => {
                    ind.style.transform = 'translateX(-50%) translateY(-100%)';
                }, 700);
            } else {
                ind.style.transform = 'translateX(-50%) translateY(-100%)';
            }
            startY = null;
            pulling = false;
        }, { passive: true });
    })();

    // ─── MOB.11: Web Share API with clipboard fallback ──────────
    window.sharePostLink = async function (url, authorName) {
        if (!url) return;
        const title = authorName ? `Post by ${authorName} on POOOL` : 'Post on POOOL';
        // Modern phones expose the OS share sheet via navigator.share.
        // Desktop browsers (and any context where share isn't allowed)
        // fall through to clipboard copy + toast.
        if (typeof navigator.share === 'function') {
            try {
                await navigator.share({ title, url });
                return;
            } catch (e) {
                // AbortError = user cancelled; treat silently.
                if (e && e.name === 'AbortError') return;
                // Any other error → fall through to clipboard path.
                console.warn('navigator.share failed; falling back to clipboard', e);
            }
        }
        try {
            await navigator.clipboard.writeText(url);
            if (window.showToast) window.showToast('Link copied to clipboard', 'success');
        } catch (e) {
            console.error('Failed to copy link', e);
            toast('Failed to copy link.');
        }
    };

    // ─── UX.16: Quote Repost ────────────────────────────────────
    window.openQuoteComposer = function (postId, authorName, content) {
        const idEl = document.getElementById('quote-post-id');
        const ta = document.getElementById('quote-post-content');
        const authorEl = document.getElementById('quote-post-author');
        const snipEl = document.getElementById('quote-post-snippet');
        const counterEl = document.getElementById('quote-post-counter');
        const errEl = document.getElementById('quote-post-error');
        if (!idEl || !ta) return;
        idEl.value = postId;
        ta.value = '';
        if (counterEl) counterEl.textContent = '0';
        if (errEl) { errEl.hidden = true; errEl.textContent = ''; }
        if (authorEl) authorEl.textContent = authorName || 'Anonymous';
        if (snipEl) {
            const text = String(content || '').trim();
            snipEl.textContent = text.length > 280 ? text.slice(0, 277) + '…' : text;
        }
        // Bind counter once
        if (ta.dataset.counterBound !== '1') {
            ta.dataset.counterBound = '1';
            ta.addEventListener('input', () => {
                if (counterEl) counterEl.textContent = String(ta.value.length);
            });
        }
        if (typeof window.openCommunityModal === 'function') {
            window.openCommunityModal('quote-post-modal');
        } else {
            document.getElementById('quote-post-modal').style.display = 'flex';
        }
    };

    window.submitQuotePost = async function () {
        const idEl = document.getElementById('quote-post-id');
        const ta = document.getElementById('quote-post-content');
        const errEl = document.getElementById('quote-post-error');
        const btn = document.getElementById('submit-quote-post-btn');
        const quotedId = idEl ? idEl.value : '';
        const content = (ta ? ta.value : '').trim();
        if (errEl) { errEl.hidden = true; errEl.textContent = ''; }
        if (!quotedId) return;
        if (!content) {
            if (errEl) { errEl.hidden = false; errEl.textContent = 'Add a comment to share this post.'; }
            return;
        }
        const oldText = btn ? btn.textContent : '';
        if (btn) { btn.textContent = 'Sharing…'; btn.disabled = true; }
        try {
            const res = await fetch(getPostCreateEndpoint(), {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify({
                    post_type: 'discussion',
                    content,
                    content_tags: [],
                    asset_id: null,
                    circle_id: circleContext ? circleContext.id : null,
                    image_urls: null,
                    poll_question: null,
                    poll_options: null,
                    poll_expires_hours: null,
                    quoted_post_id: quotedId,
                }),
            });
            const body = await res.json().catch(() => ({}));
            if (!res.ok) throw new Error(body.error || ('Failed (' + res.status + ')'));
            if (typeof window.closeCommunityModal === 'function') {
                window.closeCommunityModal('quote-post-modal');
            } else {
                document.getElementById('quote-post-modal').style.display = 'none';
            }
            if (window.showToast) window.showToast('Shared with your followers', 'success');
            document.body.dispatchEvent(new Event('reload-feed'));
        } catch (e) {
            console.error('submitQuotePost failed', e);
            if (errEl) { errEl.hidden = false; errEl.textContent = e.message || 'Failed to share.'; }
        } finally {
            if (btn) { btn.textContent = oldText || 'Share'; btn.disabled = false; }
        }
    };

    window.openEditPostModal = function (postId) {
        const card = document.getElementById(`post-${postId}`);
        if (!card) return;
        // Pull the current post body straight from the rendered DOM so we
        // don't need an extra fetch round-trip. Reactions/comments aren't
        // editable so we only need the textual content of feed-post-body > p.
        const bodyEl = card.querySelector('.feed-post-body > p');
        const currentContent = bodyEl ? bodyEl.textContent.trim() : '';
        document.getElementById('edit-post-id').value = postId;
        const ta = document.getElementById('edit-post-content');
        ta.value = currentContent;
        const counter = document.getElementById('edit-post-counter');
        if (counter) counter.textContent = String(ta.value.length);
        ta.oninput = () => { if (counter) counter.textContent = String(ta.value.length); };
        const errEl = document.getElementById('edit-post-error');
        if (errEl) errEl.hidden = true;
        if (typeof window.openCommunityModal === 'function') {
            window.openCommunityModal('edit-post-modal');
        } else {
            document.getElementById('edit-post-modal').style.display = 'block';
        }
        // Close the kebab menu now that the modal owns the focus.
        document.getElementById(`owner-menu-${postId}`)?.setAttribute('hidden', '');
    };

    window.submitEditPost = async function () {
        const postId = document.getElementById('edit-post-id').value;
        const content = document.getElementById('edit-post-content').value.trim();
        const errEl = document.getElementById('edit-post-error');
        if (errEl) errEl.hidden = true;
        if (!content) {
            if (errEl) { errEl.textContent = 'Post content cannot be empty.'; errEl.hidden = false; }
            return;
        }
        const btn = document.getElementById('submit-edit-post-btn');
        const oldText = btn.textContent;
        btn.disabled = true;
        btn.textContent = 'Saving...';
        try {
            const res = await fetch(`/api/community/posts/${encodeURIComponent(postId)}`, {
                method: 'PUT',
                credentials: 'same-origin',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify({ content })
            });
            if (!res.ok) {
                const text = await res.text();
                throw new Error(text || `Request failed (${res.status})`);
            }
            if (typeof window.closeCommunityModal === 'function') {
                window.closeCommunityModal('edit-post-modal');
            } else {
                document.getElementById('edit-post-modal').style.display = 'none';
            }
            if (window.showToast) window.showToast('Post updated', 'success');
            document.body.dispatchEvent(new Event('reload-feed'));
        } catch (err) {
            console.error('Edit post failed', err);
            if (errEl) { errEl.textContent = err.message || 'Failed to save changes.'; errEl.hidden = false; }
        } finally {
            btn.disabled = false;
            btn.textContent = oldText;
        }
    };

    window.deleteOwnPost = function (postId) {
        const idInput = document.getElementById('delete-post-id');
        const errEl = document.getElementById('delete-post-error');
        if (idInput) idInput.value = postId;
        if (errEl) {
            errEl.hidden = true;
            errEl.textContent = '';
        }
        if (typeof window.openCommunityModal === 'function') {
            window.openCommunityModal('delete-post-modal');
        } else {
            document.getElementById('delete-post-modal').style.display = 'block';
        }
        document.getElementById(`owner-menu-${postId}`)?.setAttribute('hidden', '');
    };

    window.submitDeletePost = async function () {
        const postId = document.getElementById('delete-post-id')?.value;
        const btn = document.getElementById('delete-post-confirm-btn');
        const errEl = document.getElementById('delete-post-error');
        if (!postId) return;
        if (errEl) {
            errEl.hidden = true;
            errEl.textContent = '';
        }
        const oldText = btn ? btn.textContent : '';
        if (btn) {
            btn.disabled = true;
            btn.textContent = 'Deleting...';
        }
        try {
            const res = await fetch(`/api/community/posts/${encodeURIComponent(postId)}`, {
                method: 'DELETE',
                credentials: 'same-origin',
                headers: csrfHeaders(),
            });
            if (!res.ok) {
                const text = await res.text();
                throw new Error(text || `Request failed (${res.status})`);
            }
            // Optimistically remove the card; HTMX reload below will reconcile.
            const card = document.getElementById(`post-${postId}`);
            if (card) {
                card.style.transition = 'opacity 0.2s ease';
                card.style.opacity = '0';
                setTimeout(() => card.remove(), 200);
            }
            if (window.showToast) window.showToast('Post deleted', 'success');
            document.body.dispatchEvent(new Event('reload-feed'));
            if (typeof window.closeCommunityModal === 'function') {
                window.closeCommunityModal('delete-post-modal');
            } else {
                document.getElementById('delete-post-modal').style.display = 'none';
            }
        } catch (err) {
            console.error('Delete post failed', err);
            if (errEl) {
                errEl.textContent = err.message || 'Failed to delete post.';
                errEl.hidden = false;
            } else if (window.showToast) window.showToast('Failed to delete post', 'error');
            else toast(err.message);
        } finally {
            if (btn) {
                btn.disabled = false;
                btn.textContent = oldText || 'Delete Post';
            }
        }
    };

    window.openReportModal = function(postId) {
        document.getElementById('report-post-id').value = postId;
        const noteEl = document.getElementById('report-note');
        const counterEl = document.getElementById('report-note-counter');
        if (noteEl) {
            noteEl.value = '';
            if (counterEl) counterEl.textContent = '0 / 500';
            if (!noteEl.dataset.counterBound) {
                noteEl.dataset.counterBound = '1';
                noteEl.addEventListener('input', () => {
                    if (counterEl) counterEl.textContent = `${noteEl.value.length} / 500`;
                });
            }
        }
        if (typeof window.openCommunityModal === 'function') {
            window.openCommunityModal('report-post-modal');
        } else {
            document.getElementById('report-post-modal').style.display = 'block';
        }
    };

    window.submitReport = async function() {
        const postId = document.getElementById('report-post-id').value;
        const reason = document.getElementById('report-reason').value;
        const note = (document.getElementById('report-note')?.value || '').trim();

        try {
            const res = await fetch(`/api/community/posts/${postId}/report`, {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify({ reason, note: note || null })
            });

            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }

            if (typeof window.closeCommunityModal === 'function') {
                window.closeCommunityModal('report-post-modal');
            } else {
                document.getElementById('report-post-modal').style.display = 'none';
            }
            toast('Report submitted. Our team will review it shortly.', 'success');
        } catch (e) {
            console.error(e);
            toast("Failed to submit report: " + e.message);
        }
    };

    window.loadTrendingAssets = async function() {
        const container = document.getElementById('trending-assets-container');
        const widget = document.getElementById('trending-assets-widget');
        if (!container) return;

        try {
            const res = await fetch('/api/community/trending-assets', { credentials: 'same-origin' });
            if (!res.ok) {
                if (widget) widget.hidden = true;
                return;
            }

            const assets = await res.json();

            if (assets.length === 0) {
                // Hide widget entirely rather than rendering yet-another
                // "no data" tile next to its sibling empty states. Getting
                // Started card already covers first-visit onboarding.
                if (widget) widget.hidden = true;
                return;
            }

            const buildAssetUrl = (asset) => {
                if (asset.detail_url) return asset.detail_url;
                const slug = asset.slug || asset.id;
                return asset.asset_type === 'commodity' ? `/commodity/${slug}` : `/property/${slug}`;
            };

            let html = '';
            // Define some emojis or standard icons based on symbol 
            const getIcon = (sym) => {
                sym = String(sym || '').toUpperCase();
                if (sym.includes('CACAO') || sym.includes('COCOA')) return '🍫';
                if (sym.includes('TIMBER') || sym.includes('ALBAC')) return '🌲';
                if (sym.includes('VANIL')) return '🌿';
                if (sym.includes('COFFEE')) return '☕';
                return '💎';
            };

            for (const asset of assets) {
                const detailUrl = escapeAttr(buildAssetUrl(asset));
                const name = escapeHtml(asset.name || 'Asset');
                const symbol = escapeHtml(getIcon(asset.symbol));
                const postCount = Number(asset.post_count || 0).toLocaleString();
                html += `
                <div class="trending-item" style="cursor:pointer;" onclick="window.location.href='${detailUrl}'">
                  <div class="trending-item-icon" style="background:#F2F4F7; color:#344054;">${symbol}</div>
                  <div class="trending-item-info">
                    <div class="trending-item-name">${name}</div>
                    <div class="trending-item-investors">${postCount} discussions</div>
                  </div><span class="trending-item-change" style="color:#027A48;">🔥</span>
                </div>
                `;
            }

            container.innerHTML = html;
        } catch (e) {
            console.warn("Failed to load trending assets", e);
        }
    };

    // Load trending assets on initialization
    loadTrendingAssets();

    // ─── UX.4: HASHTAG FEED FILTER ──────────────────────────────
    let currentHashtagFilter = null;

    async function loadHashtagFeed(tag) {
        currentHashtagFilter = tag;
        const feedContainer = document.getElementById('community-feed-container');
        if (!feedContainer) return;

        // Show a "viewing hashtag" banner at the top
        renderSkeleton();

        try {
            const res = await fetch(`/api/community/hashtags/${encodeURIComponent(tag)}`, { credentials: 'same-origin' });
            if (!res.ok) throw new Error('Failed to load hashtag feed');
            const data = await res.json();

            feedContainer.replaceChildren();

            // Hashtag header banner
            const banner = document.createElement('div');
            banner.className = 'community-hashtag-banner';

            const bannerLeft = document.createElement('div');
            bannerLeft.className = 'community-hashtag-banner__left';
            const hashIcon = document.createElement('div');
            hashIcon.className = 'community-hashtag-banner__icon';
            hashIcon.textContent = '#';
            bannerLeft.appendChild(hashIcon);

            const bannerText = document.createElement('div');
            const tagTitle = document.createElement('div');
            tagTitle.className = 'community-hashtag-banner__title';
            tagTitle.textContent = '#' + tag;
            bannerText.appendChild(tagTitle);
            const tagCount = document.createElement('div');
            tagCount.className = 'community-hashtag-banner__count';
            tagCount.textContent = `${data.posts ? data.posts.length : 0} posts`;
            bannerText.appendChild(tagCount);
            bannerLeft.appendChild(bannerText);
            banner.appendChild(bannerLeft);

            const clearBtn = document.createElement('button');
            clearBtn.className = 'ds-btn ds-btn--secondary ds-btn--sm';
            clearBtn.textContent = '✕ Clear Filter';
            clearBtn.addEventListener('click', () => {
                currentHashtagFilter = null;
                document.body.dispatchEvent(new Event('reload-feed'));
            });
            banner.appendChild(clearBtn);
            feedContainer.appendChild(banner);

            if (!data.posts || data.posts.length === 0) {
                const empty = document.createElement('div');
                empty.className = 'community-loading-state';
                empty.textContent = 'No posts found with this hashtag yet.';
                feedContainer.appendChild(empty);
                return;
            }

            for (const p of data.posts) {
                feedContainer.appendChild(buildPostCard(p));
            }
        } catch (e) {
            console.error(e);
            feedContainer.innerHTML = '<div class="community-state-inline community-state-inline--error">Failed to load posts for this hashtag.</div>';
        }
    }

    // Expose for external usage
    window.loadHashtagFeed = loadHashtagFeed;

    // Resolve a @handle mention to a user_id and open the profile modal.
    // Until the dedicated /api/community/users/by-handle/:handle endpoint
    // ships (Phase 2), fall back to a community search for an exact handle
    // match.
    // ─── Phase 3 task 20 + WS1.2: paginated followers / following list ──
    window.openRelationshipList = async function (profileId, direction) {
        if (!profileId) return;
        const dir = direction === 'following' ? 'following' : 'followers';
        const titleEl = document.getElementById('relationship-list-title');
        const statusEl = document.getElementById('relationship-list-status');
        const rowsEl = document.getElementById('relationship-list-rows');
        if (titleEl) titleEl.textContent = dir === 'following' ? 'Following' : 'Followers';
        if (rowsEl) rowsEl.replaceChildren();
        if (statusEl) { statusEl.textContent = 'Loading…'; statusEl.hidden = false; }

        if (typeof window.openCommunityModal === 'function') {
            window.openCommunityModal('relationship-list-modal');
        } else {
            document.getElementById('relationship-list-modal').style.display = 'block';
        }

        // Track pagination state locally so the Load-more button can fetch
        // the next page without reopening the modal.
        let nextPage = 1;
        let hasMore = false;

        async function loadPage(page) {
            try {
                const res = await fetch(
                    `/api/community/profile/${encodeURIComponent(profileId)}/${dir}?page=${page}`,
                    { credentials: 'same-origin' }
                );
                if (!res.ok) throw new Error(`Request failed (${res.status})`);
                const data = await res.json();
                const users = Array.isArray(data.users) ? data.users : [];
                hasMore = Boolean(data.has_more);
                nextPage = (data.page || page) + 1;
                if (page === 1 && statusEl) statusEl.hidden = true;
                if (page === 1 && users.length === 0) {
                    const empty = document.createElement('div');
                    empty.className = 'community-relationship-list__empty';
                    empty.textContent = dir === 'following' ? 'No one followed yet.' : 'No followers yet.';
                    rowsEl.appendChild(empty);
                    return;
                }
                users.forEach((u) => rowsEl.appendChild(buildRelationshipRow(u)));
                renderLoadMore();
            } catch (err) {
                console.error('Failed to load relationship list', err);
                if (statusEl) {
                    statusEl.textContent = 'Failed to load.';
                    statusEl.hidden = false;
                }
            }
        }

        function renderLoadMore() {
            const existing = rowsEl.querySelector('.community-relationship-list__load-more');
            if (existing) existing.remove();
            if (!hasMore) return;
            const btn = document.createElement('button');
            btn.type = 'button';
            btn.className = 'ds-btn ds-btn--secondary ds-btn--sm community-relationship-list__load-more';
            btn.textContent = 'Load more';
            btn.addEventListener('click', () => {
                btn.disabled = true;
                btn.textContent = 'Loading…';
                loadPage(nextPage);
            });
            rowsEl.appendChild(btn);
        }

        await loadPage(1);
    };

    function buildRelationshipRow(u) {
        const row = document.createElement('div');
        row.className = 'community-relationship-row';

        const left = document.createElement('button');
        left.type = 'button';
        left.className = 'community-relationship-row__user';
        left.addEventListener('click', () => {
            if (typeof window.closeCommunityModal === 'function') {
                window.closeCommunityModal('relationship-list-modal');
            }
            window.openUserProfile(u.user_id);
        });

        const avatar = document.createElement('div');
        avatar.className = 'community-relationship-row__avatar';
        if (u.avatar_url) {
            const img = document.createElement('img');
            img.src = u.avatar_url;
            img.alt = '';
            avatar.appendChild(img);
        } else {
            avatar.textContent = getInitials(u.display_name);
            avatar.classList.add('community-relationship-row__avatar--initials');
        }
        left.appendChild(avatar);

        const name = document.createElement('div');
        name.className = 'community-relationship-row__name';
        name.textContent = u.display_name || 'Anonymous';
        left.appendChild(name);
        row.appendChild(left);

        if (!u.is_self) {
            const btn = document.createElement('button');
            btn.type = 'button';
            btn.className = `ds-btn ds-btn--sm ${u.is_following ? 'ds-btn--secondary' : 'ds-btn--primary'}`;
            btn.textContent = u.is_following ? 'Unfollow' : 'Follow';
            btn.addEventListener('click', (event) => {
                event.stopPropagation();
                window.toggleFollow(u.user_id, u.is_following, btn);
                u.is_following = !u.is_following;
                btn.textContent = u.is_following ? 'Unfollow' : 'Follow';
                btn.className = `ds-btn ds-btn--sm ${u.is_following ? 'ds-btn--secondary' : 'ds-btn--primary'}`;
            });
            row.appendChild(btn);
        }
        return row;
    }

    window.openProfileByHandle = async function (handle) {
        if (!handle) return;
        try {
            const url = new URL('/api/community/search', window.location.origin);
            url.searchParams.set('q', handle);
            url.searchParams.set('type', 'users');
            const res = await fetch(url.toString(), { credentials: 'same-origin' });
            if (!res.ok) throw new Error('Search failed');
            const data = await res.json();
            const exact = (data.users || []).find(
                (u) => (u.display_name || '').toLowerCase() === handle.toLowerCase()
            );
            const fallback = (data.users || [])[0];
            const user = exact || fallback;
            if (user && user.user_id) {
                window.openUserProfile(user.user_id);
            } else if (window.showToast) {
                window.showToast(`User @${handle} not found`, 'warning');
            }
        } catch (err) {
            console.error('openProfileByHandle failed', err);
            if (window.showToast) {
                window.showToast('Could not open profile', 'error');
            }
        }
    };

    // Delegate clicks on server-rendered .mention-tag spans (which carry a
    // data-handle attribute) so they route through openProfileByHandle.
    document.body.addEventListener('click', (event) => {
        const tag = event.target.closest('.mention-tag[data-handle]');
        if (!tag) return;
        event.preventDefault();
        event.stopPropagation();
        window.openProfileByHandle(tag.dataset.handle);
    });

    async function updateMyProfileCard(profile, retryCount = 0) {
        const nameEl = document.getElementById('my-profile-name');
        const bioEl = document.getElementById('my-profile-bio');

        if (!window.__POOOL_USER && retryCount < 10) {
            setTimeout(() => updateMyProfileCard(profile, retryCount + 1), 200);
            return;
        }

        if (nameEl && window.__POOOL_USER) {
            nameEl.textContent = window.__POOOL_USER.name || "User";
        }
        if (bioEl) {
            bioEl.textContent = profile.bio || "No bio yet • Start your journey 🌱";
        }

        // First-visit nudge: 3-step Getting Started card. Each step is
        // ticked off independently based on real profile signals; once
        // all three are complete, the card hides itself entirely.
        const gsEl = document.getElementById('community-getting-started');
        if (gsEl) {
          const steps = {
            bio:    Boolean((profile.bio && profile.bio.trim()) || profile.flair),
            follow: Number(profile.following_count) >= 5,
            post:   Number(profile.post_count) >= 1,
          };
          let done = 0;
          const checkSvg = '<svg width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M3 8l3.5 3.5L13 4" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/></svg>';
          gsEl.querySelectorAll('.community-getting-started__item').forEach((li) => {
            const stepKey = li.dataset.step;
            const isDone = !!steps[stepKey];
            li.classList.toggle('community-getting-started__item--done', isDone);
            const stepBadge = li.querySelector('.community-getting-started__step');
            if (stepBadge) {
              if (isDone) {
                stepBadge.innerHTML = checkSvg;
              } else {
                stepBadge.textContent = ({ bio: '1', follow: '2', post: '3' })[stepKey];
              }
            }
            if (isDone) done++;
          });
          const progressEl = document.getElementById('community-getting-started-progress');
          if (progressEl) progressEl.textContent = `${done} of 3`;
          gsEl.hidden = done >= 3;
        }

        const contentInput = document.getElementById('post-content-input');
        if (contentInput && window.__POOOL_USER) {
            const firstName = (window.__POOOL_USER.name || "User").split(' ')[0];
            contentInput.placeholder = `What's on your mind, ${firstName}?`;
        }
    }

    async function loadMyProfile() {
        try {
            const res = await fetch('/api/community/profile/me', { credentials: 'same-origin' });
            if (!res.ok) return;
            const profile = await res.json();
            updateMyProfileCard(profile);
            // Phase 3 task 30: surface the shadowban banner + warning_count hint.
            const sbBanner = document.getElementById('community-shadowban-banner');
            if (sbBanner) {
                sbBanner.hidden = !profile.is_shadowbanned;
            }
            const modSummary = document.getElementById('edit-profile-mod-log');
            if (modSummary && (profile.is_shadowbanned || (profile.warning_count || 0) > 0)) {
                modSummary.hidden = false;
            }
        } catch (e) {
            console.warn("Failed to load community profile", e);
        }
    }

    // Lazy-fetch the moderation log entries when the summary opens.
    document.addEventListener('toggle', async (event) => {
        const summary = event.target;
        if (!(summary instanceof HTMLDetailsElement)) return;
        if (summary.id !== 'edit-profile-mod-log') return;
        if (!summary.open) return;
        if (summary.dataset.loaded === '1') return;
        const body = document.getElementById('edit-profile-mod-log-body');
        try {
            const res = await fetch('/api/community/profile/me/moderation-log', { credentials: 'same-origin' });
            if (!res.ok) throw new Error('Failed to load history');
            const data = await res.json();
            const entries = Array.isArray(data.entries) ? data.entries : [];
            if (entries.length === 0) {
                body.textContent = 'No moderation actions on record.';
            } else {
                body.replaceChildren();
                entries.forEach((entry) => {
                    const row = document.createElement('div');
                    row.className = 'community-mod-log__row';
                    const label = entry.action.replace('user.', '').replace(/_/g, ' ');
                    row.textContent = `${label} · ${new Date(entry.created_at).toLocaleString()}`;
                    body.appendChild(row);
                });
            }
            summary.dataset.loaded = '1';
        } catch (err) {
            console.error('Failed to load moderation log', err);
            body.textContent = 'Failed to load moderation history.';
        }
    }, true);

    loadMyProfile();

    // ═══════════════════════════════════════════════════════════════
    // UX.4: HASHTAG CONTENT RENDERING
    // ═══════════════════════════════════════════════════════════════

    /**
     * UX.3 + UX.4: Render text content with clickable #hashtags and @mentions.
     * Uses safe DOM construction — textContent for plain text, createElement for links.
     */
    function renderContentWithHashtags(container, text) {
        if (!text) return;
        // Split by hashtag AND mention patterns, preserving delimiters
        const parts = text.split(/(#[\w\u00C0-\u024F]+|@[\w\u00C0-\u024F_-]+)/g);
        parts.forEach(part => {
            if (part.match(/^#[\w\u00C0-\u024F]+$/)) {
                const link = document.createElement('span');
                link.className = 'hashtag-tag';
                link.textContent = part;
                link.addEventListener('click', (e) => {
                    e.stopPropagation();
                    const tag = part.substring(1).toLowerCase();
                    filterByHashtag(tag);
                });
                container.appendChild(link);
            } else if (part.match(/^@[\w\u00C0-\u024F_-]+$/)) {
                const link = document.createElement('span');
                link.className = 'mention-tag';
                link.dataset.handle = part.substring(1);
                link.textContent = part;
                link.addEventListener('click', (e) => {
                    e.stopPropagation();
                    window.openProfileByHandle(link.dataset.handle);
                });
                container.appendChild(link);
            } else {
                const textNode = document.createTextNode(part);
                container.appendChild(textNode);
            }
        });
    }

    window.filterByHashtag = async function(tag) {
        const feedContainer = document.getElementById('community-feed-container');
        if (!feedContainer) return;

        // Show loading
        feedContainer.innerHTML = `<div style="text-align: center; padding: 24px; color: #667085;">
            Loading posts for <strong>#${escapeAttr(tag)}</strong>...
        </div>`;

        try {
            const res = await fetch(`/api/community/hashtags/${encodeURIComponent(tag)}`, { credentials: 'same-origin' });
            if (!res.ok) throw new Error('Failed to fetch');
            const data = await res.json();

            if (!data.posts || data.posts.length === 0) {
                feedContainer.innerHTML = `<div style="text-align: center; padding: 40px 20px;">
                    <div style="font-size: 24px; margin-bottom: 12px;">#️⃣</div>
                    <div style="font-size: 16px; font-weight: 600; color: #101828; margin-bottom: 4px;">No posts with <span style="color: var(--btn-primary-bg, #0000FF);">#${escapeAttr(tag)}</span></div>
                    <div style="font-size: 14px; color: #667085; margin-bottom: 16px;">Be the first to use this hashtag!</div>
                    <button class="ds-btn ds-btn--secondary" onclick="loadFeedFromGlobal()">← Back to Feed</button>
                </div>`;
                return;
            }

            // Add header with back button
            feedContainer.innerHTML = '';
            const headerDiv = document.createElement('div');
            headerDiv.style.cssText = 'display: flex; align-items: center; gap: 12px; margin-bottom: 20px;';
            const backBtn = document.createElement('button');
            backBtn.className = 'ds-btn ds-btn--secondary ds-btn--sm';
            backBtn.textContent = '← Back';
            backBtn.addEventListener('click', () => document.body.dispatchEvent(new Event('reload-feed')));
            headerDiv.appendChild(backBtn);

            const tagLabel = document.createElement('h3');
            tagLabel.style.cssText = 'font-size: 18px; font-weight: 700; color: var(--btn-primary-bg, #0000FF); margin: 0;';
            tagLabel.textContent = `#${data.tag}`;
            headerDiv.appendChild(tagLabel);

            const countLabel = document.createElement('span');
            countLabel.style.cssText = 'font-size: 13px; color: #667085;';
            countLabel.textContent = `${data.posts.length} posts`;
            headerDiv.appendChild(countLabel);

            feedContainer.appendChild(headerDiv);

            for (const p of data.posts) {
                const postEl = buildPostCard(p);
                feedContainer.appendChild(postEl);
            }
        } catch (e) {
            console.error(e);
            feedContainer.innerHTML = `<div style="padding: 24px; color: #D92D20; text-align: center;">Failed to load hashtag posts. <button class="ds-btn ds-btn--secondary" onclick="loadFeedFromGlobal()">Back to Feed</button></div>`;
        }
    };

    window.loadFeedFromGlobal = function() {
        document.body.dispatchEvent(new Event('reload-feed'));
    };

    /**
     * buildPostCard — Minimal XSS-safe client-side post card builder.
     * Used only for hashtag filter and saved posts views that don't have HTMX partials.
     */
    function buildPostCard(p) {
        // WS2.1: pure-class styling; visual rules live in community.css.
        const card = document.createElement('div');
        card.className = 'feed-post feed-post--client';

        const header = document.createElement('div');
        header.className = 'feed-post-header';

        const avatar = document.createElement('div');
        avatar.className = 'feed-post-avatar-circle';
        if (p.author_avatar) {
            const img = document.createElement('img');
            img.src = p.author_avatar;
            img.alt = '';
            img.className = 'feed-post-avatar-circle__img';
            avatar.replaceChildren(img);
            avatar.classList.add('feed-post-avatar-circle--photo');
        } else {
            avatar.textContent = (p.author_name || 'U').charAt(0).toUpperCase();
        }
        header.appendChild(avatar);

        const meta = document.createElement('div');
        meta.className = 'feed-post-meta';
        const nameEl = document.createElement('div');
        nameEl.className = 'feed-post-name';
        nameEl.textContent = p.author_name || 'Community Member';
        meta.appendChild(nameEl);
        const timeEl = document.createElement('div');
        timeEl.className = 'feed-post-time';
        timeEl.textContent = p.created_at ? timeAgo(p.created_at) : '';
        meta.appendChild(timeEl);
        header.appendChild(meta);
        if (p.post_type && p.post_type !== 'general') {
            const typeBadge = document.createElement('span');
            typeBadge.className = 'feed-post-badge feed-post-badge--neutral';
            typeBadge.textContent = String(p.post_type).replace(/_/g, ' ');
            header.appendChild(typeBadge);
        }
        card.appendChild(header);

        const body = document.createElement('div');
        body.className = 'feed-post-body';
        // Content is trusted HTML from backend (sanitised at write time, same
        // path used by {{ p.content | safe }} in the SSR partial).
        const raw = String(p.content || '');
        if (/<\/?[a-z][\s\S]*>/i.test(raw)) {
            body.innerHTML = raw;
        } else {
            renderContentWithHashtags(body, raw);
        }
        if (Array.isArray(p.content_tags) && p.content_tags.length > 0) {
            const tags = document.createElement('div');
            tags.className = 'feed-post-tags';
            p.content_tags.forEach((tag) => {
                const tagButton = document.createElement('button');
                tagButton.type = 'button';
                tagButton.className = 'feed-post-tag';
                tagButton.textContent = String(tag);
                tagButton.addEventListener('click', () => window.setPostTagFilter(String(tag)));
                tags.appendChild(tagButton);
            });
            body.appendChild(tags);
        }
        card.appendChild(body);

        const heartSvg = '<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/></svg>';
        const chatSvg = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/></svg>';

        const footer = document.createElement('div');
        footer.className = 'feed-post-engagement feed-post-engagement--client';
        const likes = document.createElement('span');
        likes.className = 'feed-post-engagement__stat feed-post-engagement__stat--heart';
        likes.innerHTML = `${heartSvg}<span>${Number(p.reaction_count) || 0}</span>`;
        footer.appendChild(likes);
        const comments = document.createElement('span');
        comments.className = 'feed-post-engagement__stat';
        comments.innerHTML = `${chatSvg}<span>${Number(p.comment_count) || 0}</span>`;
        footer.appendChild(comments);
        card.appendChild(footer);

        return card;
    }

    async function renderSsrPostDetail() {
        const postId = typeof window.SSR_POST_ID === 'string' ? window.SSR_POST_ID.trim() : '';
        const contentArea = document.getElementById('community-content-area');
        if (!postId || !contentArea || contentArea.dataset.ssrPostLoaded === 'true') return;

        contentArea.dataset.ssrPostLoaded = 'true';
        contentArea.removeAttribute('hx-get');
        contentArea.removeAttribute('hx-trigger');
        contentArea.innerHTML = '<div style="padding: 40px; text-align: center; color: #667085;">Loading post...</div>';

        const renderNotFound = () => {
            contentArea.innerHTML = `
                <div class="ds-card" style="max-width: 720px; margin: 0 auto; text-align: center;">
                    <h2 class="ds-text-xl" style="margin-bottom: 8px;">Post not found</h2>
                    <p class="ds-text-subtitle" style="margin-bottom: 16px;">This community post is unavailable or has been removed.</p>
                    <a class="ds-btn ds-btn--secondary" href="/community">Back to Community</a>
                </div>`;
        };

        if (window.SSR_POST_FOUND === false) {
            renderNotFound();
            return;
        }

        try {
            const res = await fetch(`/api/community/posts/${encodeURIComponent(postId)}`, { credentials: 'same-origin' });
            if (!res.ok) throw new Error('Post not found');
            const post = await res.json();

            contentArea.innerHTML = '';
            const wrap = document.createElement('div');
            wrap.className = 'community-content-layout';
            wrap.style.cssText = 'display:block;max-width:760px;margin:0 auto;padding-bottom:60px;';

            const back = document.createElement('a');
            back.className = 'ds-btn ds-btn--secondary ds-btn--sm';
            back.href = '/community';
            back.textContent = 'Back to Community';
            back.style.marginBottom = '16px';
            wrap.appendChild(back);

            const card = buildPostCard(post);
            card.id = `post-${post.id}`;
            wrap.appendChild(card);
            contentArea.appendChild(wrap);
        } catch (e) {
            console.error('Failed to load direct community post', e);
            renderNotFound();
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // UX.6: BOOKMARK FUNCTIONS
    // ═══════════════════════════════════════════════════════════════

    // checkBookmarkStatus removed 2026-05-15: server now renders the
    // initial bookmarked state directly on the button (see PostDisplay.is_bookmarked
    // and partials/community_post_card.html), eliminating the per-post
    // /bookmark/status N+1 fetch storm.

    window.toggleBookmark = async function(postId, btn) {
        haptic(8);
        // Optimistic toggle
        const wasBookmarked = btn.classList.contains('bookmarked');
        btn.classList.toggle('bookmarked');
        btn.title = wasBookmarked ? 'Save Post' : 'Remove Bookmark';
        btn.setAttribute('aria-label', wasBookmarked ? 'Save post' : 'Remove saved post');
        btn.setAttribute('aria-pressed', wasBookmarked ? 'false' : 'true');

        try {
            const res = await fetch(`/api/community/posts/${postId}/bookmark`, {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders(),
            });
            if (!res.ok) throw new Error('Failed');
            const data = await res.json();
            // Sync with server state
            if (data.bookmarked) {
                btn.classList.add('bookmarked');
                btn.title = 'Remove Bookmark';
                btn.setAttribute('aria-label', 'Remove saved post');
                btn.setAttribute('aria-pressed', 'true');
            } else {
                btn.classList.remove('bookmarked');
                btn.title = 'Save Post';
                btn.setAttribute('aria-label', 'Save post');
                btn.setAttribute('aria-pressed', 'false');
            }
        } catch (e) {
            // Revert on failure
            if (wasBookmarked) {
                btn.classList.add('bookmarked');
                btn.title = 'Remove Bookmark';
                btn.setAttribute('aria-label', 'Remove saved post');
                btn.setAttribute('aria-pressed', 'true');
            } else {
                btn.classList.remove('bookmarked');
                btn.title = 'Save Post';
                btn.setAttribute('aria-label', 'Save post');
                btn.setAttribute('aria-pressed', 'false');
            }
        }
    };

    // ═══════════════════════════════════════════════════════════════
    // UX.11: POLL FUNCTIONS
    // ═══════════════════════════════════════════════════════════════

    async function loadPollForPost(postId, container) {
        try {
            const res = await fetch(`/api/community/posts/${postId}/poll`, { credentials: 'same-origin' });
            if (!res.ok) return;
            const poll = await res.json();
            if (!poll || !poll.options) return;
            renderPoll(postId, poll, container);
        } catch (e) {
            // No poll for this post — that's fine
        }
    }
    // Expose for the inline <script> in partials/community_post_card.html.
    // Without this, the function lived only inside the IIFE and the partial's
    // `typeof loadPollForPost === 'function'` check silently failed, so polls
    // never rendered after a page reload.
    window.loadPollForPost = loadPollForPost;

    function renderPoll(postId, poll, container) {
        container.innerHTML = '';

        const card = document.createElement('div');
        card.className = 'poll-card';

        const question = document.createElement('div');
        question.className = 'poll-question';
        question.textContent = poll.question;
        card.appendChild(question);

        const optionsWrap = document.createElement('div');

        poll.options.forEach(opt => {
            const optEl = document.createElement('button');
            optEl.type = 'button';
            optEl.className = 'poll-option' + (opt.user_voted ? ' voted' : '');
            optEl.setAttribute('aria-pressed', opt.user_voted ? 'true' : 'false');
            optEl.setAttribute('aria-label', `Vote for ${opt.label}`);
            optEl.style.border = '0';
            optEl.style.width = '100%';
            optEl.style.textAlign = 'left';

            // Percentage bar
            const bar = document.createElement('div');
            bar.className = 'poll-option-bar';
            bar.style.width = (poll.has_voted || poll.is_expired) ? `${opt.percentage}%` : '0%';

            // Content
            const content = document.createElement('div');
            content.className = 'poll-option-content';

            const labelDiv = document.createElement('div');
            labelDiv.style.cssText = 'display: flex; align-items: center; gap: 8px;';

            const check = document.createElement('div');
            check.className = 'poll-option-check';

            const label = document.createElement('span');
            label.className = 'poll-option-label';
            label.textContent = opt.label;

            labelDiv.appendChild(check);
            labelDiv.appendChild(label);

            const statsDiv = document.createElement('div');
            statsDiv.className = 'poll-option-stats';
            if (poll.has_voted || poll.is_expired) {
                statsDiv.textContent = `${opt.percentage}%`;
            }

            content.appendChild(labelDiv);
            content.appendChild(statsDiv);
            optEl.appendChild(bar);
            optEl.appendChild(content);

            if (!poll.is_expired) {
                optEl.addEventListener('click', () => voteOnPoll(postId, opt.id, container));
            } else {
                optEl.disabled = true;
            }

            optionsWrap.appendChild(optEl);
        });

        card.appendChild(optionsWrap);

        // Meta: vote count + expiry
        const meta = document.createElement('div');
        meta.className = 'poll-meta';

        const votesSpan = document.createElement('span');
        votesSpan.innerHTML = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 20V10"/><path d="M12 20V4"/><path d="M6 20v-6"/></svg><span>${poll.total_votes} vote${poll.total_votes !== 1 ? 's' : ''}</span>`;
        meta.appendChild(votesSpan);

        if (poll.is_expired) {
            const expiredSpan = document.createElement('span');
            expiredSpan.style.color = '#D92D20';
            expiredSpan.textContent = '⏰ Poll ended';
            meta.appendChild(expiredSpan);
        } else if (poll.expires_at) {
            const expiresSpan = document.createElement('span');
            const expiresDate = new Date(poll.expires_at);
            const hoursLeft = Math.max(0, Math.ceil((expiresDate - new Date()) / 3600000));
            expiresSpan.textContent = hoursLeft > 24 ? `${Math.ceil(hoursLeft / 24)}d left` : `${hoursLeft}h left`;
            meta.appendChild(expiresSpan);
        }

        card.appendChild(meta);
        container.appendChild(card);
    }

    async function voteOnPoll(postId, optionId, container) {
        try {
            const res = await fetch(`/api/community/posts/${postId}/poll/vote`, {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify({ option_id: optionId })
            });
            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }

            // Reload poll to show results
            const pollRes = await fetch(`/api/community/posts/${postId}/poll`, { credentials: 'same-origin' });
            if (pollRes.ok) {
                const poll = await pollRes.json();
                if (poll) {
                    renderPoll(postId, poll, container);
                }
            }
        } catch (e) {
            console.error('Vote failed:', e);
            toast('Failed to vote: ' + e.message);
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // UX.4: TRENDING HASHTAGS SIDEBAR
    // ═══════════════════════════════════════════════════════════════

    async function loadTrendingHashtags() {
        const container = document.getElementById('trending-hashtags-container');
        const widget = document.getElementById('trending-hashtags-widget');
        if (!container) return;

        try {
            const res = await fetch('/api/community/hashtags/trending', { credentials: 'same-origin' });
            if (!res.ok) {
                if (widget) widget.hidden = true;
                return;
            }
            const hashtags = await res.json();

            if (!hashtags || hashtags.length === 0) {
                if (widget) widget.hidden = true;
                return;
            }

            container.innerHTML = '';
            hashtags.slice(0, 8).forEach(h => {
                const item = document.createElement('div');
                item.className = 'hashtag-trending-item';
                item.addEventListener('click', () => {
                    // Switch to feed tab and filter
                    const feedTab = document.querySelector('.community-tab-btn[data-tab="community-feed-tab"]');
                    if (feedTab) switchCommunityTab(feedTab);
                    filterByHashtag(h.tag);
                });

                const tagSpan = document.createElement('span');
                tagSpan.className = 'hashtag-trending-tag';
                tagSpan.textContent = `#${h.tag}`;

                const countSpan = document.createElement('span');
                countSpan.className = 'hashtag-trending-count';
                countSpan.textContent = `${h.post_count} post${h.post_count !== 1 ? 's' : ''}`;

                item.appendChild(tagSpan);
                item.appendChild(countSpan);
                container.appendChild(item);
            });
        } catch (e) {
            console.error('Failed to load trending hashtags:', e);
        }
    }

    loadTrendingHashtags();

    async function loadSidebarAMA() {
        try {
            const res = await fetch('/api/community/amas');
            if (!res.ok) return;
            const data = await res.json();
            const activeOrUpcoming = data.amas?.find(a => a.status === 'active' || a.status === 'upcoming');
            
            if (activeOrUpcoming) {
                const titleEl = document.getElementById('sidebar-ama-title');
                const expertEl = document.getElementById('sidebar-ama-expert');
                const timeEl = document.getElementById('sidebar-ama-time');
                if (titleEl) titleEl.textContent = activeOrUpcoming.title;
                if (expertEl) expertEl.textContent = `with ${activeOrUpcoming.expert_name || 'an Expert'}`;
                if (timeEl) {
                    const dt = new Date(activeOrUpcoming.scheduled_for);
                    timeEl.textContent = dt.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' }) + ' — ' + dt.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
                }
                const widget = document.getElementById('sidebar-ama-widget');
                if (widget) widget.style.display = 'block';
            }
        } catch (e) {
            console.warn('Failed to load sidebar AMA', e);
        }
    }
    loadSidebarAMA();

    // ═══════════════════════════════════════════════════════════════
    // UX.11: POLL CREATOR IN POST MODAL
    // ═══════════════════════════════════════════════════════════════

    window.pollOptions = ['', ''];
    window.pollEnabled = false;

    window.togglePollCreator = function() {
        window.pollEnabled = !window.pollEnabled;
        const creator = document.getElementById('poll-creator');
        if (creator) {
            creator.style.display = window.pollEnabled ? 'block' : 'none';
        }
        const toggleBtn = document.getElementById('poll-toggle-btn');
        if (toggleBtn) {
            toggleBtn.classList.toggle('active', window.pollEnabled);
        }
    };

    window.addPollOption = function() {
        if (window.pollOptions.length >= 10) return;
        window.pollOptions.push('');
        renderPollInputs();
    };

    window.removePollOption = function(index) {
        if (window.pollOptions.length <= 2) return;
        window.pollOptions.splice(index, 1);
        renderPollInputs();
    };

    window.updatePollOption = function(index, value) {
        window.pollOptions[index] = value;
    };

    function renderPollInputs() {
        const container = document.getElementById('poll-options-inputs');
        if (!container) return;

        container.innerHTML = '';
        window.pollOptions.forEach((opt, i) => {
            const row = document.createElement('div');
            row.className = 'poll-option-input-row';

            const input = document.createElement('input');
            input.type = 'text';
            input.placeholder = `Option ${i + 1}`;
            input.maxLength = 200;
            input.value = opt;
            input.addEventListener('input', (e) => updatePollOption(i, e.target.value));

            row.appendChild(input);

            if (window.pollOptions.length > 2) {
                const removeBtn = document.createElement('button');
                removeBtn.type = 'button';
                removeBtn.textContent = '✕';
                removeBtn.addEventListener('click', () => removePollOption(i));
                row.appendChild(removeBtn);
            }

            container.appendChild(row);
        });
    }

    // ═══════════════════════════════════════════════════════════════
    // UX.15: EDIT COMMUNITY PROFILE
    // ═══════════════════════════════════════════════════════════════
    //
    // The legacy in-page modal was retired in favour of /community/me/edit.
    // openProfileEditModal is the only surface remaining — every call site
    // funnels through it for a consistent navigation contract. The page
    // itself owns its own avatar/bio/flair/verify-owner handlers.
    window.openProfileEditModal = function () {
        window.location.href = '/community/me/edit';
    };

    loadCircleEngagement();
    renderSsrPostDetail();

};

document.addEventListener('DOMContentLoaded', window.initCommunityFeed);
document.addEventListener('htmx:afterSwap', (e) => {
    if (e.target.id === 'community-content-area') {
        window.initCommunityFeed();
    }
});
