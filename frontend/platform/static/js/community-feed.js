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

    let currentFeedMode = 'all';

    window.setFeedMode = function(mode) {
        currentFeedMode = mode;
        const btnAll = document.getElementById('feed-btn-all');
        const btnFollowing = document.getElementById('feed-btn-following');
        
        if (mode === 'all') {
            btnAll.className = 'ds-btn ds-btn--primary';
            btnFollowing.className = 'ds-btn ds-btn--secondary';
        } else {
            btnAll.className = 'ds-btn ds-btn--secondary';
            btnFollowing.className = 'ds-btn ds-btn--primary';
        }
        
        loadFeed();
    };

    // ─── XSS-safe helpers ───────────────────────────────────────
    function getInitials(name) {
        if (!name) return '?';
        const parts = name.split(' ');
        return (parts.length > 1 ? parts[0][0] + parts[1][0] : parts[0].substring(0, 2)).toUpperCase();
    }

    function escapeAttr(str) {
        if (!str) return '';
        return str.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/'/g, '&#39;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }

    /**
     * Build a post DOM element using safe DOM construction.
     * User-generated content uses textContent (XSS-safe).
     * Only static/developer-controlled strings use innerHTML.
     */
    function buildPostElement(p) {
        const postEl = document.createElement('div');
        postEl.className = 'feed-post';

        // ─── Header ───
        const header = document.createElement('div');
        header.className = 'feed-post-header';

        const authorDiv = document.createElement('div');
        authorDiv.className = 'feed-post-author';
        authorDiv.style.cursor = 'pointer';
        authorDiv.addEventListener('click', () => openUserProfile(p.author_id));

        // Avatar
        if (p.author_avatar) {
            const avatarImg = document.createElement('img');
            avatarImg.src = p.author_avatar;
            avatarImg.className = 'feed-post-avatar-circle';
            avatarImg.style.cssText = 'border:none; object-fit:cover;';
            authorDiv.appendChild(avatarImg);
        } else {
            const avatarDiv = document.createElement('div');
            avatarDiv.className = 'feed-post-avatar feed-post-avatar--announcement';
            avatarDiv.style.cssText = 'font-size:12px; font-weight:bold;';
            avatarDiv.textContent = getInitials(p.author_name);
            authorDiv.appendChild(avatarDiv);
        }

        // Meta (name + time)
        const metaDiv = document.createElement('div');
        metaDiv.className = 'feed-post-meta';

        const nameSpan = document.createElement('span');
        nameSpan.className = 'feed-post-name';
        nameSpan.textContent = p.author_name; // SAFE: textContent

        // Official badge (only for system-controlled POOOL accounts)
        if (p.author_name && p.author_name.includes('POOOL')) {
            const officialBadge = document.createElement('span');
            officialBadge.className = 'feed-post-verified-badge';
            officialBadge.textContent = 'Official';
            nameSpan.appendChild(officialBadge);
        }

        // Verified Owner badge (FIX-F4: boolean flag, not HTML injection)
        if (p.verified_owner) {
            const ownerBadge = document.createElement('span');
            ownerBadge.className = 'feed-post-badge';
            ownerBadge.style.cssText = 'background:#F0FDF4;color:#027A48;border:1px solid #D1FADF;margin-left:6px;font-size:11px;';
            ownerBadge.textContent = 'Verified Owner';
            nameSpan.appendChild(ownerBadge);
        }

        // Author badges (emojis from system, safe)
        if (p.author_badges && p.author_badges.length > 0) {
            p.author_badges.slice(0, 3).forEach(icon => {
                const badgeSpan = document.createElement('span');
                badgeSpan.style.cssText = 'margin-left:4px; font-size:14px;';
                badgeSpan.textContent = icon; // emoji from DB, safe as textContent
                nameSpan.appendChild(badgeSpan);
            });
        }

        const timeSpan = document.createElement('span');
        timeSpan.className = 'feed-post-time';
        timeSpan.textContent = timeAgo(p.created_at);

        metaDiv.appendChild(nameSpan);
        metaDiv.appendChild(timeSpan);
        authorDiv.appendChild(metaDiv);
        header.appendChild(authorDiv);

        // Pinned badge
        if (p.is_pinned) {
            const pinnedBadge = document.createElement('span');
            pinnedBadge.className = 'feed-post-badge';
            pinnedBadge.style.cssText = 'background:#FFF0ED;color:#DC6803;border:1px solid #FFD8CF;';
            pinnedBadge.textContent = 'Pinned';
            header.appendChild(pinnedBadge);
        }

        // Type badge
        if (p.post_type === 'announcement') {
            const typeBadge = document.createElement('span');
            typeBadge.className = 'feed-post-badge feed-post-badge--announcement';
            typeBadge.style.marginLeft = '8px';
            typeBadge.textContent = 'Announcement';
            header.appendChild(typeBadge);
        } else if (p.post_type === 'market_insight') {
            const typeBadge = document.createElement('span');
            typeBadge.className = 'feed-post-badge';
            typeBadge.style.cssText = 'background:#F0FDF4;color:#027A48;border:1px solid #D1FADF;margin-left:8px;';
            typeBadge.textContent = 'Market Insight';
            header.appendChild(typeBadge);
        } else if (p.post_type === 'review') {
            const typeBadge = document.createElement('span');
            typeBadge.className = 'feed-post-badge';
            typeBadge.style.cssText = 'background:#FFF9C4;color:#F57F17;border:1px solid #FFF59D;margin-left:8px;';
            typeBadge.textContent = 'Review';
            header.appendChild(typeBadge);
        }

        postEl.appendChild(header);

        // ─── Body ───
        const body = document.createElement('div');
        body.className = 'feed-post-body';

        const contentP = document.createElement('p');
        contentP.textContent = p.content; // SAFE: textContent — the core XSS fix
        body.appendChild(contentP);

        // Images (URLs are server-controlled GCS paths)
        if (p.image_urls && p.image_urls.length > 0) {
            const imgWrap = document.createElement('div');
            imgWrap.style.marginTop = '16px';
            const img = document.createElement('img');
            img.src = p.image_urls[0];
            img.style.cssText = 'max-width: 100%; border-radius: 12px; border: 1px solid #EAECF0;';
            imgWrap.appendChild(img);
            body.appendChild(imgWrap);
        }

        // Disclaimer (static text, safe)
        if (p.disclaimer_shown) {
            const disclaimer = document.createElement('div');
            disclaimer.className = 'feed-post-disclaimer';
            disclaimer.style.cssText = 'font-size:12px; color:#667085; background:#F9FAFB; padding:8px 12px; border-radius:6px; margin-top:12px; border:1px solid #EAECF0;';
            disclaimer.textContent = '⚠️ Disclaimer: This post contains community generated investment discussion. Do your own research, past performance does not guarantee future results.';
            body.appendChild(disclaimer);
        }

        postEl.appendChild(body);

        // ─── Engagement ───
        const engagement = document.createElement('div');
        engagement.className = 'feed-post-engagement';
        engagement.style.cssText = 'margin-top: 20px; border-top: 1px solid #EAECF0; padding-top: 16px;';

        const reactions = document.createElement('div');
        reactions.className = 'feed-post-reactions';

        const reactionTypes = [
            { emoji: '🔥', type: 'fire', count: p.reaction_count || 0 },
            { emoji: '💡', type: 'idea', count: 0 },
            { emoji: '👏', type: 'clap', count: 0 },
        ];
        reactionTypes.forEach(r => {
            const btn = document.createElement('button');
            btn.className = 'feed-reaction-btn';
            btn.textContent = r.emoji + ' ';
            const countSpan = document.createElement('span');
            countSpan.textContent = r.count;
            btn.appendChild(countSpan);
            btn.addEventListener('click', () => toggleReaction(p.id, btn, r.type));
            reactions.appendChild(btn);
        });

        const statsRow = document.createElement('div');
        statsRow.style.cssText = 'display: flex; gap: 16px; align-items: center;';

        const stats = document.createElement('div');
        stats.className = 'feed-post-stats';
        stats.style.cursor = 'pointer';
        stats.textContent = `${p.reaction_count || 0} reactions · ${p.comment_count || 0} comments`;
        stats.addEventListener('click', () => toggleComments(p.id));

        const reportBtn = document.createElement('button');
        reportBtn.className = 'ds-btn ds-btn--ghost ds-btn--sm';
        reportBtn.title = 'Report Post';
        reportBtn.style.cssText = 'padding:4px; height:auto; border:none;';
        reportBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#98A2B3" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z"></path><line x1="4" y1="22" x2="4" y2="15"></line></svg>';
        reportBtn.addEventListener('click', () => openReportModal(p.id));

        statsRow.appendChild(stats);
        statsRow.appendChild(reportBtn);
        engagement.appendChild(reactions);
        engagement.appendChild(statsRow);
        postEl.appendChild(engagement);

        // ─── Comments Section ───
        const commentsSection = document.createElement('div');
        commentsSection.id = `comments-section-${p.id}`;
        commentsSection.style.cssText = 'display: none; padding-top: 16px;';

        const commentsList = document.createElement('div');
        commentsList.id = `comments-list-${p.id}`;
        commentsList.innerHTML = '<div style="font-size: 13px; color: #667085; text-align: center;">Loading comments...</div>';

        const commentInputRow = document.createElement('div');
        commentInputRow.style.cssText = 'display: flex; gap: 8px; margin-top: 12px; align-items: flex-start;';

        const textarea = document.createElement('textarea');
        textarea.id = `comment-input-${p.id}`;
        textarea.className = 'ds-input';
        textarea.placeholder = 'Write a comment...';
        textarea.rows = 1;
        textarea.style.cssText = 'flex:1; resize:none; overflow-wrap:normal; min-height: 40px; padding: 10px;';

        const submitBtn = document.createElement('button');
        submitBtn.className = 'ds-btn ds-btn--primary';
        submitBtn.style.cssText = 'height: 40px; padding: 0 16px;';
        submitBtn.textContent = 'Post';
        submitBtn.addEventListener('click', () => submitComment(p.id));

        commentInputRow.appendChild(textarea);
        commentInputRow.appendChild(submitBtn);
        commentsSection.appendChild(commentsList);
        commentsSection.appendChild(commentInputRow);
        postEl.appendChild(commentsSection);

        return postEl;
    }

    async function loadFeed() {
        renderSkeleton();
        try {
            let url = '/api/community/feed';
            if (currentFeedMode === 'following') {
                url += '?feed_mode=following';
            }
            const res = await fetch(url);
            if (!res.ok) {
                if(res.status === 401) {
                    throw new Error("unauthorized");
                }
                throw new Error("Failed to fetch feed");
            }
            const posts = await res.json();
            
            if (posts.length === 0) {
                if (currentFeedMode === 'following') {
                     feedContainer.innerHTML = `<div style="padding: 40px 24px; color: #667085; text-align: center; background: white; border-radius: 12px; border: 1px solid #EAECF0;">
                        <div style="font-size: 24px; margin-bottom: 12px;">🔭</div>
                        <h3 style="margin-bottom: 8px; font-weight: 600; color: #101828;">Nothing to see here yet</h3>
                        <p style="font-size: 14px;">You aren't following anyone yet, or the people you follow haven't posted.</p>
                        <button class="ds-btn ds-btn--secondary" style="margin-top: 16px;" onclick="setFeedMode('all')">View All Posts</button>
                    </div>`;
                } else {
                    renderEmptyState();
                }
                return;
            }

            feedContainer.innerHTML = '';
            for (const p of posts) {
                const postEl = buildPostElement(p);
                feedContainer.appendChild(postEl);
            }
        } catch (e) {
            console.error(e);
            if (e.message === "unauthorized") {
                feedContainer.innerHTML = `<div style="padding: 40px 24px; color: #667085; text-align: center; background: white; border-radius: 12px; border: 1px solid #EAECF0;">
                    <div style="font-size: 24px; margin-bottom: 12px;">🔒</div>
                    <h3 style="margin-bottom: 8px; font-weight: 600; color: #101828;">Log in to view this</h3>
                    <p style="font-size: 14px;">You must be logged in to view your personalized feed.</p>
                </div>`;
            } else {
                feedContainer.innerHTML = `<div style="padding: 24px; color: #D92D20; text-align: center;">Failed to load feed. Please try again.</div>`;
            }
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
                listContainer.innerHTML = '<div style="font-size: 13px; color: #667085; padding-bottom: 8px;">No comments yet. Be the first to start the discussion!</div>';
                return;
            }

            listContainer.innerHTML = '';
            comments.forEach(c => {
                const row = document.createElement('div');
                row.style.cssText = 'display:flex; gap: 12px; margin-bottom: 12px; align-items: flex-start;';

                // Avatar
                if (c.author_avatar) {
                    const img = document.createElement('img');
                    img.src = c.author_avatar;
                    img.style.cssText = 'width: 28px; height: 28px; border-radius: 50%; object-fit:cover;';
                    row.appendChild(img);
                } else {
                    const avatarDiv = document.createElement('div');
                    avatarDiv.style.cssText = 'width: 28px; height: 28px; background: #eaecf0; border-radius: 50%; display: flex; align-items:center; justify-content:center; font-size: 10px; font-weight:600; color:#344054;';
                    avatarDiv.textContent = getInitials(c.author_name);
                    row.appendChild(avatarDiv);
                }

                // Comment body
                const body = document.createElement('div');
                body.style.cssText = 'flex:1; background: #F9FAFB; padding: 10px 12px; border-radius: 8px; border: 1px solid #EAECF0;';

                const header = document.createElement('div');
                header.style.cssText = 'display:flex; justify-content: space-between; margin-bottom: 4px;';
                const nameSpan = document.createElement('span');
                nameSpan.style.cssText = 'font-weight: 600; font-size: 13px; color: #344054;';
                nameSpan.textContent = c.author_name; // SAFE: textContent escapes HTML
                const timeSpan = document.createElement('span');
                timeSpan.style.cssText = 'font-size: 12px; color: #667085;';
                timeSpan.textContent = timeAgo(c.created_at);
                header.appendChild(nameSpan);
                header.appendChild(timeSpan);

                const contentDiv = document.createElement('div');
                contentDiv.style.cssText = 'font-size: 14px; color: #475467; word-break: break-word;';
                contentDiv.textContent = c.content; // SAFE: textContent escapes HTML

                body.appendChild(header);
                body.appendChild(contentDiv);
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

    // ─── USER PROFILE LOGIC (M3) ─────────────────────────────
    
    let currentProfileId = null;

    window.openUserProfile = async function(userId) {
        currentProfileId = userId;
        document.getElementById('user-profile-modal').style.display = 'block';
        document.getElementById('profile-loading-state').style.display = 'block';
        document.getElementById('profile-content-state').style.display = 'none';

        try {
            const res = await fetch(`/api/community/profile/${userId}`);
            if (!res.ok) throw new Error("Profile not found");
            const profile = await res.json();

            // Populate Modal
            document.getElementById('profile-modal-name').innerText = profile.display_name;
            document.getElementById('profile-modal-bio').innerText = profile.bio || "This user hasn't written a bio yet.";
            document.getElementById('profile-modal-followers').innerText = profile.follower_count;
            document.getElementById('profile-modal-following').innerText = profile.following_count;
            document.getElementById('profile-modal-posts').innerText = profile.post_count;

            const badgesContainer = document.getElementById('profile-modal-badges');
            if (profile.badges && profile.badges.length > 0) {
                badgesContainer.innerHTML = profile.badges.map(b => 
                    `<div title="${b.name}" style="background:#F2F4F7; border: 1px solid #EAECF0; border-radius:16px; padding: 4px 8px; font-size:14px; cursor:help; display:flex; align-items:center;">
                        ${b.icon} <span style="font-size:12px; font-weight:500; margin-left:6px; color:#344054;">${b.name}</span>
                    </div>`
                ).join('');
            } else {
                badgesContainer.innerHTML = `<div style="font-size:13px; color:#98A2B3;">No badges earned yet.</div>`;
            }

            const avatarContainer = document.getElementById('profile-modal-avatar');
            if (profile.avatar_url) {
                avatarContainer.style.background = `url(${profile.avatar_url}) center/cover`;
                avatarContainer.innerHTML = '';
            } else {
                avatarContainer.style.background = '#F2F4F7';
                const parts = profile.display_name.split(' ');
                const init = parts.length > 1 ? parts[0][0] + parts[1][0] : parts[0].substring(0, 2);
                avatarContainer.innerHTML = `<span id="profile-modal-initials">${init.toUpperCase()}</span>`;
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
                const res = await fetch(`/api/community/follow/${userId}`, { method: 'DELETE' });
                if (!res.ok) throw new Error("Failed to unfollow");
                
                btnElement.innerText = "Follow User";
                btnElement.className = "ds-btn ds-btn--primary";
                
                // Optimistically update followers count 
                const followersEl = document.getElementById('profile-modal-followers');
                followersEl.innerText = Math.max(0, parseInt(followersEl.innerText) - 1);
            } else {
                const res = await fetch(`/api/community/follow/${userId}`, { method: 'POST' });
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

    const createPostBox = document.querySelector('.community-create-post');
    if (createPostBox) {
        createPostBox.addEventListener('click', () => {
            document.getElementById('create-post-modal').style.display = 'block';
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
        
        let html = '';
        window.postImageUrls.forEach((url, index) => {
            html += `
            <div style="position: relative; flex-shrink: 0;">
                <img src="${url}" style="width: 80px; height: 80px; object-fit: cover; border-radius: 8px; border: 1px solid #EAECF0;">
                <button type="button" onclick="removePostImage(${index})" style="position: absolute; top: -6px; right: -6px; background: white; border: 1px solid #EAECF0; border-radius: 50%; width: 20px; height: 20px; cursor: pointer; display: flex; align-items: center; justify-content: center; font-size: 10px; color: #D92D20; font-weight: bold; padding: 0;">✕</button>
            </div>`;
        });
        container.innerHTML = html;
        
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
            image_urls: window.postImageUrls.length > 0 ? window.postImageUrls : null
        };
        
        const submitBtn = document.getElementById('submit-post-btn');
        const oldText = submitBtn.innerText;
        submitBtn.innerText = "Posting...";
        submitBtn.disabled = true;

        try {
            const res = await fetch('/api/community/posts', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(requestBody)
            });
            
            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }
            
            document.getElementById('create-post-modal').style.display = 'none';
            document.getElementById('post-content-input').value = '';
            document.getElementById('post-disclaimer-warning').style.display = 'none';
            window.postImageUrls = [];
            renderPostImagePreviews();
            
            // Refresh feed
            loadFeed();
        } catch (e) {
            console.error(e);
            alert("Failed to submit post: " + e.message);
        } finally {
            submitBtn.innerText = oldText;
            submitBtn.disabled = false;
        }
    };

    window.openReportModal = function(postId) {
        document.getElementById('report-post-id').value = postId;
        document.getElementById('report-post-modal').style.display = 'block';
    };

    window.submitReport = async function() {
        const postId = document.getElementById('report-post-id').value;
        const reason = document.getElementById('report-reason').value;
        
        try {
            const res = await fetch(`/api/community/posts/${postId}/report`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ reason })
            });
            
            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }
            
            document.getElementById('report-post-modal').style.display = 'none';
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
            const res = await fetch('/api/community/trending-assets');
            if (!res.ok) return;

            const assets = await res.json();
            
            if (assets.length === 0) {
                container.innerHTML = '<div style="padding: 20px; text-align: center; color: #667085; font-size: 13px;">No trending assets yet</div>';
                return;
            }

            let html = '';
            // Define some emojis or standard icons based on symbol 
            const getIcon = (sym) => {
                if (sym.includes('CACAO') || sym.includes('COCOA')) return '🍫';
                if (sym.includes('TIMBER') || sym.includes('ALBAC')) return '🌲';
                if (sym.includes('VANIL')) return '🌿';
                if (sym.includes('COFFEE')) return '☕';
                return '💎';
            };

            for (const asset of assets) {
                html += `
                <div class="trending-item" style="cursor:pointer;" onclick="window.location.href='/assets/${asset.id}'">
                  <div class="trending-item-icon" style="background:#F2F4F7; color:#344054;">${getIcon(asset.symbol)}</div>
                  <div class="trending-item-info">
                    <div class="trending-item-name">${asset.name}</div>
                    <div class="trending-item-investors">${asset.post_count} discussions</div>
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

    // Onboarding logic
    window.closeOnboardingModal = function() {
        document.getElementById('onboarding-modal').style.display = 'none';
        localStorage.setItem('poool_community_onboarding_dismissed', 'true');
    };

    async function checkOnboarding() {
        if (localStorage.getItem('poool_community_onboarding_dismissed') === 'true') {
            return;
        }

        try {
            const res = await fetch('/api/community/profile/me');
            if (!res.ok) return;

            const profile = await res.json();
            
            // Checking if they need onboarding (XP concept)
            const hasBio = !!profile.bio;
            const hasPosts = profile.post_count > 0;

            if (!hasBio || !hasPosts) {
                document.getElementById('ob-bio').checked = hasBio;
                document.getElementById('ob-post').checked = hasPosts;
                
                document.getElementById('onboarding-modal').style.display = 'flex';
            } else {
                // If they completed it but never dismissed modal, we can silently dismiss
                localStorage.setItem('poool_community_onboarding_dismissed', 'true');
            }
        } catch (e) {
            console.error("Failed to check onboarding status", e);
        }
    }

    checkOnboarding();

});
