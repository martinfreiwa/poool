document.addEventListener('DOMContentLoaded', () => {
    const feedContainer = document.getElementById('community-feed-container');
    if (!feedContainer) return;

    function renderSkeleton() {
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
        // Optimistic toggle
        const isCurrentlyActive = btn.classList.contains('active');
        const countSpan = btn.querySelector('span');
        let currentCount = parseInt(countSpan.textContent, 10);
        
        if (isCurrentlyActive) {
            btn.classList.remove('active');
            countSpan.textContent = currentCount - 1;
        } else {
            btn.classList.add('active');
            countSpan.textContent = currentCount + 1;
        }

        try {
            await fetch(`/api/community/posts/${postId}/reactions`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ reaction_type: type })
            });
        } catch (e) {
            console.error('Failed to toggle reaction', e);
            // Revert on failure
            if (isCurrentlyActive) {
                btn.classList.add('active');
                countSpan.textContent = currentCount;
            } else {
                btn.classList.remove('active');
                countSpan.textContent = currentCount;
            }
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

    async function loadFeed() {
        renderSkeleton();
        try {
            const res = await fetch('/api/community/feed');
            if (!res.ok) throw new Error("Failed to fetch feed");
            const posts = await res.json();
            
            if (posts.length === 0) {
                renderEmptyState();
                return;
            }

            let html = '';
            for (const p of posts) {
                // Determine initials
                let initials = "?";
                if (p.author_name) {
                    const parts = p.author_name.split(' ');
                    initials = parts.length > 1 ? parts[0][0] + parts[1][0] : parts[0].substring(0, 2);
                }

                html += `
                <div class="feed-post">
                    <div class="feed-post-header">
                        <div class="feed-post-author">
                            ${p.author_avatar ? 
                                `<img src="${p.author_avatar}" class="feed-post-avatar-circle" style="border:none; object-fit:cover;">` : 
                                `<div class="feed-post-avatar feed-post-avatar--announcement" style="font-size:12px; font-weight:bold;">${initials.toUpperCase()}</div>`
                            }
                            <div class="feed-post-meta">
                                <span class="feed-post-name">${p.author_name} ${p.author_name.includes('POOOL') ? '<span class="feed-post-verified-badge">Official</span>' : ''}</span>
                                <span class="feed-post-time">${timeAgo(p.created_at)}</span>
                            </div>
                        </div>
                        ${p.is_pinned ? '<span class="feed-post-badge" style="background:#FFF0ED;color:#DC6803;border:1px solid #FFD8CF;">Pinned</span>' : ''}
                        <span class="feed-post-badge feed-post-badge--announcement" style="margin-left: 8px;">Announcement</span>
                    </div>
                    <div class="feed-post-body">
                        ${p.content}
                        ${p.image_urls && p.image_urls.length > 0 ? 
                            `<div style="margin-top: 16px;"><img src="${p.image_urls[0]}" style="max-width: 100%; border-radius: 12px; border: 1px solid #EAECF0;"></div>` : ''
                        }
                    </div>
                    <div class="feed-post-engagement" style="margin-top: 20px; border-top: 1px solid #EAECF0; padding-top: 16px;">
                        <div class="feed-post-reactions">
                            <button class="feed-reaction-btn" onclick="toggleReaction('${p.id}', this, 'fire')">🔥 <span>${p.reaction_count || 0}</span></button>
                            <button class="feed-reaction-btn" onclick="toggleReaction('${p.id}', this, 'idea')">💡 <span>0</span></button>
                            <button class="feed-reaction-btn" onclick="toggleReaction('${p.id}', this, 'clap')">👏 <span>0</span></button>
                        </div>
                        <div class="feed-post-stats" style="cursor:pointer;" onclick="toggleComments('${p.id}')"><span>${p.reaction_count || 0} reactions</span> · <span>${p.comment_count || 0} comments</span></div>
                    </div>
                    
                    <div id="comments-section-${p.id}" style="display: none; padding-top: 16px;">
                        <div id="comments-list-${p.id}">
                            <div style="font-size: 13px; color: #667085; text-align: center;">Loading comments...</div>
                        </div>
                        <div style="display: flex; gap: 8px; margin-top: 12px; align-items: flex-start;">
                            <textarea id="comment-input-${p.id}" class="ds-input" placeholder="Write a comment..." rows="1" style="flex:1; resize:none; overflow-wrap:normal; min-height: 40px; padding: 10px;"></textarea>
                            <button class="ds-btn ds-btn--primary" onclick="submitComment('${p.id}')" style="height: 40px; padding: 0 16px;">Post</button>
                        </div>
                    </div>

                </div>
                `;
            }
            feedContainer.innerHTML = html;
        } catch (e) {
            console.error(e);
            feedContainer.innerHTML = `<div style="padding: 24px; color: #D92D20; text-align: center;">Failed to load feed. Please try again.</div>`;
        }
    }

    loadFeed();

    // ─── COMMENTS LOGIC ───────────────────────────────────────
    
    window.toggleComments = async function(postId) {
        const section = document.getElementById(`comments-section-${postId}`);
        if (section.style.display === 'none') {
            section.style.display = 'block';
            await loadComments(postId);
        } else {
            section.style.display = 'none';
        }
    };

    window.loadComments = async function(postId) {
        const listContainer = document.getElementById(`comments-list-${postId}`);
        try {
            const res = await fetch(`/api/community/posts/${postId}/comments`);
            if (!res.ok) throw new Error("Failed to load comments");
            const comments = await res.json();
            
            if (comments.length === 0) {
                listContainer.innerHTML = `<div style="font-size: 13px; color: #667085; padding-bottom: 8px;">No comments yet. Be the first to start the discussion!</div>`;
                return;
            }

            let html = '';
            comments.forEach(c => {
                let initials = "?";
                if (c.author_name) {
                    const parts = c.author_name.split(' ');
                    initials = parts.length > 1 ? parts[0][0] + parts[1][0] : parts[0].substring(0, 2);
                }

                html += `
                <div style="display:flex; gap: 12px; margin-bottom: 12px; align-items: flex-start;">
                    ${c.author_avatar ? 
                        `<img src="${c.author_avatar}" style="width: 28px; height: 28px; border-radius: 50%; object-fit:cover;">` : 
                        `<div style="width: 28px; height: 28px; background: #eaecf0; border-radius: 50%; display: flex; align-items:center; justify-content:center; font-size: 10px; font-weight:600; color:#344054;">${initials.toUpperCase()}</div>`
                    }
                    <div style="flex:1; background: #F9FAFB; padding: 10px 12px; border-radius: 8px; border: 1px solid #EAECF0;">
                        <div style="display:flex; justify-content: space-between; margin-bottom: 4px;">
                            <span style="font-weight: 600; font-size: 13px; color: #344054;">${c.author_name}</span>
                            <span style="font-size: 12px; color: #667085;">${timeAgo(c.created_at)}</span>
                        </div>
                        <div style="font-size: 14px; color: #475467; word-break: break-word;">
                            ${c.content}
                        </div>
                    </div>
                </div>
                `;
            });
            listContainer.innerHTML = html;
        } catch (e) {
            console.error(e);
            listContainer.innerHTML = `<div style="font-size: 13px; color: #D92D20;">Failed to load comments.</div>`;
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
                headers: { 'Content-Type': 'application/json' },
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

});
