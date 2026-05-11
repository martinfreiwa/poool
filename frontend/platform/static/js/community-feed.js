window.initCommunityFeed = function() {
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
        let interval = seconds / 31536000;
        if (interval > 1) return Math.floor(interval) + " years ago";
        interval = seconds / 2592000;
        if (interval > 1) return Math.floor(interval) + " months ago";
        interval = seconds / 86400;
        if (interval > 1) return Math.floor(interval) + " days ago";
        interval = seconds / 3600;
        if (interval > 1) return Math.floor(interval) + " hours ago";
        interval = seconds / 60;
        if (interval > 1) return Math.floor(interval) + " minutes ago";
        return Math.floor(seconds) + " seconds ago";
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
            await loadComments(postId);
        }
    };

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
            comments.forEach(c => {
                const row = document.createElement('div');
                row.className = 'community-comment-row';
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
                body.appendChild(contentDiv);

                // 14.8.5 — own-comment edit affordance.
                if (currentUserId && c.author_id === currentUserId) {
                    const editBtn = document.createElement('button');
                    editBtn.type = 'button';
                    editBtn.className = 'ds-btn ds-btn--ghost ds-btn--sm community-comment-row__edit-btn';
                    editBtn.textContent = 'Edit';
                    editBtn.setAttribute('aria-label', 'Edit comment');
                    editBtn.addEventListener('click', () =>
                        startCommentEdit(row, c.id, c.content, contentDiv, timeSpan, editBtn)
                    );
                    body.appendChild(editBtn);
                }

                row.appendChild(body);
                listContainer.appendChild(row);
            });
        } catch (e) {
            console.error(e);
            listContainer.innerHTML = '<div style="font-size: 13px; color: #D92D20;">Failed to load comments.</div>';
        }
    };

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
            await loadComments(postId); // reload list
        } catch (e) {
            console.error(e);
            alert("Failed to post comment: " + e.message);
            input.disabled = false;
        }
    };

    // ─── USER PROFILE LOGIC (M3) ─────────────────────────────
    
    let currentProfileId = null;

    window.openUserProfile = async function(userId) {
        currentProfileId = userId;
        if (typeof window.openCommunityModal === 'function') {
            window.openCommunityModal('user-profile-modal');
        } else {
            document.getElementById('user-profile-modal').style.display = 'block';
        }
        document.getElementById('profile-loading-state').style.display = 'block';
        document.getElementById('profile-content-state').style.display = 'none';

        try {
            const res = await fetch(`/api/community/profile/${userId}`, { credentials: 'same-origin' });
            if (!res.ok) throw new Error("Profile not found");
            const profile = await res.json();

            // Populate Modal
            document.getElementById('profile-modal-name').innerText = profile.display_name;
            document.getElementById('profile-modal-bio').innerText = profile.bio || "This user hasn't written a bio yet.";
            document.getElementById('profile-modal-followers').innerText = profile.follower_count;
            document.getElementById('profile-modal-following').innerText = profile.following_count;
            document.getElementById('profile-modal-posts').innerText = profile.post_count;

            const badgesContainer = document.getElementById('profile-modal-badges');
            badgesContainer.replaceChildren();
            if (profile.badges && profile.badges.length > 0) {
                profile.badges.forEach((badge) => {
                    const badgeEl = document.createElement('div');
                    badgeEl.title = badge.name || '';
                    badgeEl.className = 'community-profile-badge';

                    const icon = document.createElement('span');
                    icon.textContent = badge.icon || '';
                    const name = document.createElement('span');
                    name.className = 'community-profile-badge__name';
                    name.textContent = badge.name || 'Badge';

                    badgeEl.appendChild(icon);
                    badgeEl.appendChild(name);
                    badgesContainer.appendChild(badgeEl);
                });
            } else {
                appendTextEmptyState(badgesContainer, 'No badges earned yet.', 'font-size:13px; color:#98A2B3;');
            }

            const avatarContainer = document.getElementById('profile-modal-avatar');
            avatarContainer.replaceChildren();
            if (profile.avatar_url) {
                avatarContainer.classList.add('community-profile-modal__avatar--has-image');
                const img = document.createElement('img');
                img.src = profile.avatar_url;
                img.alt = '';
                img.className = 'community-profile-modal__avatar-img';
                avatarContainer.appendChild(img);
            } else {
                avatarContainer.classList.add('community-profile-modal__avatar--has-image');
                const parts = String(profile.display_name || 'User').split(' ');
                const init = parts.length > 1 ? parts[0][0] + parts[1][0] : parts[0].substring(0, 2);
                const initials = document.createElement('span');
                initials.id = 'profile-modal-initials';
                initials.textContent = init.toUpperCase();
                avatarContainer.appendChild(initials);
            }

            const followBtn = document.getElementById('profile-modal-follow-btn');
            // Remove previous listeners
            const newBtn = followBtn.cloneNode(true);
            followBtn.parentNode.replaceChild(newBtn, followBtn);
            
            if (profile.is_following) {
                newBtn.innerText = "Unfollow";
                newBtn.className = "ds-btn ds-btn--secondary";
            } else {
                newBtn.innerText = "Follow User";
                newBtn.className = "ds-btn ds-btn--primary";
            }
            newBtn.style.width = "100%";
            
            newBtn.onclick = () => toggleFollow(userId, profile.is_following, newBtn);

            // 14.8.2: wire mute + block secondary actions.
            const muteBtn = document.getElementById('profile-modal-mute-btn');
            const blockBtn = document.getElementById('profile-modal-block-btn');
            if (muteBtn) {
                const freshMute = muteBtn.cloneNode(false);
                freshMute.textContent = profile.is_muted ? 'Unmute' : 'Mute';
                muteBtn.parentNode.replaceChild(freshMute, muteBtn);
                freshMute.onclick = () => window.toggleMute(userId, profile.is_muted === true, freshMute);
            }
            if (blockBtn) {
                const freshBlock = blockBtn.cloneNode(false);
                freshBlock.textContent = profile.is_blocked ? 'Unblock' : 'Block';
                freshBlock.className =
                    'ds-btn ds-btn--ghost ds-btn--sm community-profile-modal__danger';
                blockBtn.parentNode.replaceChild(freshBlock, blockBtn);
                freshBlock.onclick = () => window.toggleBlock(userId, profile.is_blocked === true, freshBlock);
            }

            document.getElementById('profile-loading-state').style.display = 'none';
            document.getElementById('profile-content-state').style.display = 'block';

        } catch (e) {
            console.error(e);
            document.getElementById('profile-loading-state').innerHTML = `<p style="color: #D92D20;">Failed to load profile.</p>`;
        }
    };

    window.toggleFollow = async function(userId, currentlyFollowing, btnElement) {
        try {
            btnElement.disabled = true;
            btnElement.innerText = "Updating...";

            if (currentlyFollowing) {
                const res = await fetch(`/api/community/follow/${userId}`, { method: 'DELETE', credentials: 'same-origin', headers: csrfHeaders() });
                if (!res.ok) throw new Error("Failed to unfollow");
                
                btnElement.innerText = "Follow User";
                btnElement.className = "ds-btn ds-btn--primary";
                
                // Optimistically update followers count 
                const followersEl = document.getElementById('profile-modal-followers');
                followersEl.innerText = Math.max(0, parseInt(followersEl.innerText) - 1);
            } else {
                const res = await fetch(`/api/community/follow/${userId}`, { method: 'POST', credentials: 'same-origin', headers: csrfHeaders() });
                if (!res.ok) {
                    const err = await res.text();
                    throw new Error(err);
                }
                btnElement.innerText = "Unfollow";
                btnElement.className = "ds-btn ds-btn--secondary";
                
                // Optimistically update followers count
                const followersEl = document.getElementById('profile-modal-followers');
                followersEl.innerText = parseInt(followersEl.innerText) + 1;
            }
            // Bind the new toggle state
            btnElement.onclick = () => toggleFollow(userId, !currentlyFollowing, btnElement);
        } catch (e) {
            alert(e.message || "Failed to toggle follow status");
            btnElement.innerText = currentlyFollowing ? "Unfollow" : "Follow User";
        } finally {
            btnElement.disabled = false;
        }
    };

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
        document.getElementById('post-type-input').value = btn.getAttribute('data-type');
    };

    const contentInput = document.getElementById('post-content-input');
    if (contentInput) {
        contentInput.addEventListener('input', () => {
            const val = contentInput.value.toLowerCase();
            const investmentKeywords = ["invest", "return", "yield", "profit", "dividend", "roi", "price target", "buy now", "sell now"];
            const needsDisclaimer = investmentKeywords.some(k => val.includes(k));
            document.getElementById('post-disclaimer-warning').style.display = needsDisclaimer ? 'block' : 'none';
        });
    }


    window.postImageUrls = [];
    
    window.uploadPostImage = async function(e) {
        if (!e.target.files || e.target.files.length === 0) return;
        
        if (window.postImageUrls.length >= 4) {
            alert("Maximum 4 images allowed per post.");
            return;
        }

        const file = e.target.files[0];
        if (file.size > 5 * 1024 * 1024) {
            alert("Image must be smaller than 5MB");
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
            alert(err.message);
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

    window.submitUserPost = async function() {
        const postType = document.getElementById('post-type-input').value;
        const content = document.getElementById('post-content-input').value.trim();
        
        if (!content) return alert("Content cannot be empty");
        
        const requestBody = {
            post_type: postType,
            content: content,
            asset_id: null,
            image_urls: window.postImageUrls.length > 0 ? window.postImageUrls : null,
            // UX.11: Poll data
            poll_question: null,
            poll_options: null,
            poll_expires_hours: null,
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
                }
            }
        }
        
        const submitBtn = document.getElementById('submit-post-btn');
        const oldText = submitBtn.innerText;
        submitBtn.innerText = "Posting...";
        submitBtn.disabled = true;

        try {
            const res = await fetch('/api/community/posts', {
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
            document.getElementById('post-disclaimer-warning').style.display = 'none';
            window.postImageUrls = [];
            renderPostImagePreviews();
            
            // Refresh feed via HTMX event
            document.body.dispatchEvent(new Event('reload-feed'));
        } catch (e) {
            console.error(e);
            alert("Failed to submit post: " + e.message);
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
    // user-data.js publishes __POOOL_USER asynchronously; retry briefly so
    // posts rendered before /api/me resolves still get the kebab on the next
    // tick.
    let ownPostsRetries = 0;
    const ownPostsInterval = setInterval(() => {
        if (window.__POOOL_USER || ownPostsRetries++ > 20) {
            clearInterval(ownPostsInterval);
            markOwnPosts();
        }
    }, 200);

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

    window.deleteOwnPost = async function (postId) {
        if (!confirm('Delete this post? This cannot be undone.')) return;
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
        } catch (err) {
            console.error('Delete post failed', err);
            if (window.showToast) window.showToast('Failed to delete post', 'error');
            else alert(err.message);
        }
    };

    window.openReportModal = function(postId) {
        document.getElementById('report-post-id').value = postId;
        if (typeof window.openCommunityModal === 'function') {
            window.openCommunityModal('report-post-modal');
        } else {
            document.getElementById('report-post-modal').style.display = 'block';
        }
    };

    window.submitReport = async function() {
        const postId = document.getElementById('report-post-id').value;
        const reason = document.getElementById('report-reason').value;
        
        try {
            const res = await fetch(`/api/community/posts/${postId}/report`, {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify({ reason })
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
            alert('Report submitted successfully. Our team will review it shortly.');
        } catch (e) {
            console.error(e);
            alert("Failed to submit report: " + e.message);
        }
    };

    window.loadTrendingAssets = async function() {
        const container = document.getElementById('trending-assets-container');
        if (!container) return;
        
        try {
            const res = await fetch('/api/community/trending-assets', { credentials: 'same-origin' });
            if (!res.ok) return;

            const assets = await res.json();
            
            if (assets.length === 0) {
                container.innerHTML = '<div style="padding: 20px; text-align: center; color: #667085; font-size: 13px;">No trending assets yet</div>';
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
            console.error("Failed to load trending assets", e);
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
            feedContainer.innerHTML = '<div style="padding: 24px; color: #D92D20; text-align: center;">Failed to load posts for this hashtag.</div>';
        }
    }

    // Expose for external usage
    window.loadHashtagFeed = loadHashtagFeed;

    // Resolve a @handle mention to a user_id and open the profile modal.
    // Until the dedicated /api/community/users/by-handle/:handle endpoint
    // ships (Phase 2), fall back to a community search for an exact handle
    // match.
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
        // Elements from community.html
        const nameEl = document.getElementById('my-profile-name');
        const bioEl = document.getElementById('my-profile-bio');
        const postEl = document.getElementById('my-profile-posts');
        const folEl = document.getElementById('my-profile-followers');
        const fngEl = document.getElementById('my-profile-following');
        const avatarEl = document.getElementById('my-profile-avatar-circle');

        if (!window.__POOOL_USER && retryCount < 10) {
            // Give user-data.js a moment to finish its /api/me fetch
            setTimeout(() => updateMyProfileCard(profile, retryCount + 1), 200);
            return;
        }

        if (nameEl && window.__POOOL_USER) {
            nameEl.textContent = window.__POOOL_USER.name || "User";
        }
        if (avatarEl && window.__POOOL_USER) {
            avatarEl.textContent = (window.__POOOL_USER.name || "U")[0].toUpperCase();
        }
        if (bioEl) {
            bioEl.textContent = profile.bio || "No bio yet • Start your journey 🌱";
        }
        if (postEl) postEl.textContent = profile.post_count || 0;
        if (folEl) folEl.textContent = profile.follower_count || 0;
        if (fngEl) fngEl.textContent = profile.following_count || 0;

        // Also populate inline composer UI elements
        const fbName = document.getElementById('fb-compose-name');
        const fbAvatar = document.getElementById('fb-compose-avatar');
        const contentInput = document.getElementById('post-content-input');

        if (fbName && window.__POOOL_USER) {
            const fullName = window.__POOOL_USER.name || "User";
            fbName.textContent = fullName;
            if (contentInput) {
                const firstName = fullName.split(' ')[0];
                contentInput.placeholder = `What's on your mind, ${firstName}?`;
            }
        }
        if (fbAvatar && window.__POOOL_USER) {
            fbAvatar.textContent = (window.__POOOL_USER.name || "U")[0].toUpperCase();
        }
        const badgesEl = document.getElementById('my-profile-badges');
        if (badgesEl) {
            badgesEl.innerHTML = '';
            if (profile.badges && profile.badges.length > 0) {
                profile.badges.forEach(b => {
                    const span = document.createElement('span');
                    span.className = 'profile-badge profile-badge--gold';
                    let label = b.name || b.badge_type || '';
                    // Try to map some to standard ones
                    if (label.toLowerCase().includes('verified')) span.className = 'profile-badge profile-badge--verified';
                    if (label.toLowerCase().includes('investor')) span.className = 'profile-badge profile-badge--investor';
                    span.textContent = label;
                    if (b.description) span.title = b.description;
                    badgesEl.appendChild(span);
                });
            }
        }
    }

    async function loadMyProfile() {
        try {
            const res = await fetch('/api/community/profile/me', { credentials: 'same-origin' });
            if (!res.ok) return;
            const profile = await res.json();
            updateMyProfileCard(profile);
        } catch (e) {
            console.error("Failed to load community profile", e);
        }
    }

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
        const card = document.createElement('div');
        card.className = 'feed-post';
        card.style.cssText = 'background: var(--card-bg); border: 1px solid var(--card-border-color); border-radius: var(--card-border-radius); padding: 20px; margin-bottom: 16px;';

        // Header
        const header = document.createElement('div');
        header.className = 'feed-post-header';
        header.style.cssText = 'display:flex;align-items:center;gap:12px;margin-bottom:12px;';

        const avatar = document.createElement('div');
        avatar.className = 'feed-post-avatar-circle';
        avatar.style.cssText = 'width:40px;height:40px;border-radius:50%;background:#EEF4FF;color:var(--btn-primary-bg);display:flex;align-items:center;justify-content:center;font-weight:600;font-size:16px;flex-shrink:0;';
        if (p.author_avatar) {
            const img = document.createElement('img');
            img.src = p.author_avatar;
            img.style.cssText = 'width:40px;height:40px;border-radius:50%;object-fit:cover;';
            avatar.innerHTML = '';
            avatar.appendChild(img);
        } else {
            avatar.textContent = (p.author_name || 'U').charAt(0).toUpperCase();
        }
        header.appendChild(avatar);

        const meta = document.createElement('div');
        const nameEl = document.createElement('div');
        nameEl.style.cssText = 'font-size:14px;font-weight:600;color:var(--page-title-color);';
        nameEl.textContent = p.author_name || 'Community Member';
        meta.appendChild(nameEl);
        const timeEl = document.createElement('div');
        timeEl.style.cssText = 'font-size:12px;color:#98A2B3;';
        timeEl.textContent = p.created_at ? timeAgo(p.created_at) : '';
        meta.appendChild(timeEl);
        header.appendChild(meta);
        card.appendChild(header);

        // Body
        const body = document.createElement('div');
        body.className = 'feed-post-body';
        body.style.cssText = 'font-size:14px;color:var(--body-color);line-height:1.6;';
        renderContentWithHashtags(body, p.content || '');
        card.appendChild(body);

        // Footer
        const footer = document.createElement('div');
        footer.style.cssText = 'display:flex;align-items:center;gap:16px;margin-top:12px;padding-top:12px;border-top:1px solid var(--card-border-color);font-size:13px;color:#667085;';
        const likes = document.createElement('span');
        likes.textContent = `\u2764\uFE0F ${p.reaction_count || 0}`;
        footer.appendChild(likes);
        const comments = document.createElement('span');
        comments.textContent = `\uD83D\uDCAC ${p.comment_count || 0}`;
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

    async function checkBookmarkStatus(postId, btn) {
        try {
            const res = await fetch(`/api/community/posts/${postId}/bookmark/status`, { credentials: 'same-origin' });
            if (res.ok) {
                const data = await res.json();
                if (data.bookmarked) {
                    btn.classList.add('bookmarked');
                    btn.title = 'Remove Bookmark';
                    btn.setAttribute('aria-label', 'Remove saved post');
                    btn.setAttribute('aria-pressed', 'true');
                }
            }
        } catch (e) {
            // Silently fail — non-critical
        }
    }

    window.toggleBookmark = async function(postId, btn) {
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

    // Phase 2 task 15: trigger the saved-posts HTMX swap. The container in
    // community.html listens for the `load-saved-posts` body event and fetches
    // /community/partials/feed/list?source=bookmarks, which returns the same
    // server-rendered card as the main feed (reactions, comments, bookmark
    // toggle, report, owner kebab — all wired).
    window.loadSavedPosts = function () {
        document.body.dispatchEvent(new Event('load-saved-posts'));
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
        votesSpan.textContent = `📊 ${poll.total_votes} vote${poll.total_votes !== 1 ? 's' : ''}`;
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
            alert('Failed to vote: ' + e.message);
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // UX.4: TRENDING HASHTAGS SIDEBAR
    // ═══════════════════════════════════════════════════════════════

    async function loadTrendingHashtags() {
        const container = document.getElementById('trending-hashtags-container');
        if (!container) return;

        try {
            const res = await fetch('/api/community/hashtags/trending', { credentials: 'same-origin' });
            if (!res.ok) return;
            const hashtags = await res.json();

            if (!hashtags || hashtags.length === 0) {
                container.innerHTML = '<div style="font-size: 13px; color: #98A2B3; text-align: center; padding: 12px;">No trending hashtags yet.</div>';
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
            console.error('Failed to load sidebar AMA', e);
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

    function paintAvatarPreview(url) {
        const preview = document.getElementById('edit-profile-avatar-preview');
        if (!preview) return;
        if (url) {
            preview.style.backgroundImage = `url("${url}")`;
            preview.dataset.avatarUrl = url;
        } else {
            preview.style.backgroundImage = '';
            delete preview.dataset.avatarUrl;
        }
    }

    window.openProfileEditModal = async function() {
        const modal = document.getElementById('edit-profile-modal');
        if (!modal) return;

        try {
            const res = await fetch('/api/community/profile/me');
            if (res.ok) {
                const profile = await res.json();
                document.getElementById('edit-profile-bio').value = profile.bio || '';
                paintAvatarPreview(profile.avatar_url || (window.__POOOL_USER && window.__POOOL_USER.avatar_url) || null);
            }
        } catch (e) {
            console.error('Failed to load profile for editing', e);
        }

        if (typeof window.openCommunityModal === 'function') {
            window.openCommunityModal('edit-profile-modal');
        } else {
            modal.style.display = 'block';
        }
    };

    // Phase 3 task 19: client side of the avatar upload. Reuses the existing
    // POST /api/upload/avatar endpoint (writes to users.avatar_url) and paints
    // the new image into the preview immediately so the save button just needs
    // to flush the bio change.
    window.uploadProfileAvatar = async function (event) {
        const file = event && event.target && event.target.files && event.target.files[0];
        if (!file) return;
        const statusEl = document.getElementById('edit-profile-avatar-status');
        const showStatus = (text, error) => {
            if (!statusEl) return;
            statusEl.textContent = text;
            statusEl.hidden = false;
            statusEl.style.color = error ? '#B42318' : '#475467';
        };
        if (file.size > 5 * 1024 * 1024) {
            showStatus('Image must be 5 MB or smaller.', true);
            event.target.value = '';
            return;
        }
        showStatus('Uploading...');
        const form = new FormData();
        form.append('file', file);
        try {
            const res = await fetch('/api/upload/avatar', {
                method: 'POST',
                credentials: 'same-origin',
                headers: csrfHeaders(),
                body: form,
            });
            const data = await res.json();
            if (!res.ok) throw new Error(data.error || `Upload failed (${res.status})`);
            paintAvatarPreview(data.avatar_url);
            showStatus('Photo updated. Save changes to keep the new picture.');
            // Reflect immediately in sidebar avatar.
            const sidebarAvatar = document.getElementById('my-profile-avatar-circle');
            if (sidebarAvatar) {
                sidebarAvatar.style.backgroundImage = `url("${data.avatar_url}")`;
                sidebarAvatar.style.backgroundSize = 'cover';
                sidebarAvatar.style.backgroundPosition = 'center';
                sidebarAvatar.textContent = '';
            }
            if (window.__POOOL_USER) window.__POOOL_USER.avatar_url = data.avatar_url;
        } catch (err) {
            console.error('Avatar upload failed', err);
            showStatus(err.message || 'Upload failed.', true);
        } finally {
            event.target.value = '';
        }
    };

    window.saveProfileDetails = async function() {
        const bio = document.getElementById('edit-profile-bio').value.trim();
        const btn = document.getElementById('save-profile-btn');
        const originalText = btn.textContent;
        btn.textContent = 'Saving...';
        btn.disabled = true;
        
        try {
            const res = await fetch('/api/community/profile', {
                method: 'PUT',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify({ bio: bio })
            });
            
            if (res.ok) {
                const updatedProfile = await res.json();
                const bioEl = document.getElementById('my-profile-bio');
                if (bioEl) bioEl.textContent = updatedProfile.bio || "No bio yet • Start your journey 🌱";
                if (typeof window.closeCommunityModal === 'function') {
                    window.closeCommunityModal('edit-profile-modal');
                } else {
                    document.getElementById('edit-profile-modal').style.display = 'none';
                }
                if (window.showToast) window.showToast('Profile updated successfully');
            } else {
                const err = await res.json();
                alert(err.error || 'Failed to update profile');
            }
        } catch (e) {
            console.error('Failed to save profile', e);
            alert('A network error occurred');
        } finally {
            btn.textContent = originalText;
            btn.disabled = false;
        }
    };

    renderSsrPostDetail();

};

document.addEventListener('DOMContentLoaded', window.initCommunityFeed);
document.addEventListener('htmx:afterSwap', (e) => {
    if (e.target.id === 'community-content-area') {
        window.initCommunityFeed();
    }
});
