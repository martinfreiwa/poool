window.initCommunityFeed = function() {

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
                credentials: 'same-origin',
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
    let currentSortMode = 'fresh';

    window.setFeedMode = function(mode) {
        currentFeedMode = mode;
        const btnAll = document.getElementById('feed-btn-all');
        const btnFollowing = document.getElementById('feed-btn-following');
        
        if (btnAll && btnFollowing) {
            if (mode === 'all') {
                btnAll.className = 'feed-toggle-btn active';
                btnFollowing.className = 'feed-toggle-btn';
            } else {
                btnAll.className = 'feed-toggle-btn';
                btnFollowing.className = 'feed-toggle-btn active';
            }
        }
        
        const formInput = document.getElementById('form-feed-mode');
        if (formInput) formInput.value = mode;
        document.body.dispatchEvent(new Event('reload-feed'));
    };

    window.setSortMode = function(mode) {
        currentSortMode = mode;
        const btnFresh = document.getElementById('sort-btn-fresh');
        const btnHot = document.getElementById('sort-btn-hot');
        
        if (btnFresh && btnHot) {
            if (mode === 'fresh') {
                btnFresh.className = 'feed-toggle-btn active';
                btnHot.className = 'feed-toggle-btn';
            } else {
                btnFresh.className = 'feed-toggle-btn';
                btnHot.className = 'feed-toggle-btn active';
            }
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

    // Deprecated Client-Side Functions have been removed in favor of HTMX server-side rendering.

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
            const res = await fetch(`/api/community/posts/${postId}/comments`, { credentials: 'same-origin' });
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
                credentials: 'same-origin',
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
                const res = await fetch(`/api/community/follow/${userId}`, { method: 'DELETE', credentials: 'same-origin' });
                if (!res.ok) throw new Error("Failed to unfollow");
                
                btnElement.innerText = "Follow User";
                btnElement.className = "ds-btn ds-btn--primary";
                
                // Optimistically update followers count 
                const followersEl = document.getElementById('profile-modal-followers');
                followersEl.innerText = Math.max(0, parseInt(followersEl.innerText) - 1);
            } else {
                const res = await fetch(`/api/community/follow/${userId}`, { method: 'POST', credentials: 'same-origin' });
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
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(requestBody)
            });
            
            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }
            
            const modal = document.getElementById('create-post-modal');
            if (modal) modal.style.display = 'none';
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
                credentials: 'same-origin',
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

            feedContainer.innerHTML = '';

            // Hashtag header banner
            const banner = document.createElement('div');
            banner.style.cssText = 'display: flex; align-items: center; justify-content: space-between; padding: 16px 20px; background: #EEF4FF; border: 1px solid #D1E0FF; border-radius: 12px; margin-bottom: 20px;';

            const bannerLeft = document.createElement('div');
            bannerLeft.style.cssText = 'display: flex; align-items: center; gap: 12px;';
            const hashIcon = document.createElement('div');
            hashIcon.style.cssText = 'width: 40px; height: 40px; border-radius: 10px; background: #D1E0FF; display: flex; align-items: center; justify-content: center; font-size: 20px; font-weight: 700; color: var(--btn-primary-bg, #0000FF);';
            hashIcon.textContent = '#';
            bannerLeft.appendChild(hashIcon);

            const bannerText = document.createElement('div');
            const tagTitle = document.createElement('div');
            tagTitle.style.cssText = 'font-size: 18px; font-weight: 700; color: #101828;';
            tagTitle.textContent = '#' + tag;
            bannerText.appendChild(tagTitle);
            const tagCount = document.createElement('div');
            tagCount.style.cssText = 'font-size: 13px; color: #667085;';
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
                empty.style.cssText = 'text-align: center; padding: 40px; color: #667085;';
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

    // Onboarding logic
    window.closeOnboardingModal = function() {
        document.getElementById('onboarding-modal').style.display = 'none';
        localStorage.setItem('poool_community_onboarding_dismissed', 'true');
    };

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

    async function checkOnboarding() {
        try {
            const res = await fetch('/api/community/profile/me', { credentials: 'same-origin' });
            if (!res.ok) return;

            const profile = await res.json();

            // Snyc the profile card on the right
            updateMyProfileCard(profile);
            
            if (localStorage.getItem('poool_community_onboarding_dismissed') === 'true') {
                return;
            }

            // Checking if they need onboarding (XP concept)
            const hasBio = !!profile.bio;
            const hasPosts = profile.post_count > 0;

            if (!hasBio || !hasPosts) {
                const bioCB = document.getElementById('ob-bio');
                const postCB = document.getElementById('ob-post');
                if (bioCB) bioCB.checked = hasBio;
                if (postCB) postCB.checked = hasPosts;
                
                const modal = document.getElementById('onboarding-modal');
                if (modal) modal.style.display = 'flex';
            } else {
                localStorage.setItem('poool_community_onboarding_dismissed', 'true');
            }
        } catch (e) {
            console.error("Failed to check onboarding/profile status", e);
        }
    }

    checkOnboarding();

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
                link.style.cssText = 'color: var(--btn-primary-bg, #0000FF); font-weight: 600; cursor: pointer; transition: opacity 0.2s;';
                link.addEventListener('mouseover', () => link.style.opacity = '0.7');
                link.addEventListener('mouseout', () => link.style.opacity = '1');
                link.addEventListener('click', (e) => {
                    e.stopPropagation();
                    const tag = part.substring(1).toLowerCase();
                    filterByHashtag(tag);
                });
                container.appendChild(link);
            } else if (part.match(/^@[\w\u00C0-\u024F_-]+$/)) {
                const link = document.createElement('span');
                link.className = 'mention-tag';
                link.textContent = part;
                link.style.cssText = 'color: #7F56D9; font-weight: 600; cursor: pointer; transition: opacity 0.2s;';
                link.addEventListener('mouseover', () => link.style.opacity = '0.7');
                link.addEventListener('mouseout', () => link.style.opacity = '1');
                link.addEventListener('click', (e) => {
                    e.stopPropagation();
                    const mention = part.substring(1);
                    window.location.href = `/community?search=${encodeURIComponent(mention)}`;
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

        try {
            const res = await fetch(`/api/community/posts/${postId}/bookmark`, {
                method: 'POST',
                credentials: 'same-origin',
            });
            if (!res.ok) throw new Error('Failed');
            const data = await res.json();
            // Sync with server state
            if (data.bookmarked) {
                btn.classList.add('bookmarked');
                btn.title = 'Remove Bookmark';
            } else {
                btn.classList.remove('bookmarked');
                btn.title = 'Save Post';
            }
        } catch (e) {
            // Revert on failure
            if (wasBookmarked) {
                btn.classList.add('bookmarked');
                btn.title = 'Remove Bookmark';
            } else {
                btn.classList.remove('bookmarked');
                btn.title = 'Save Post';
            }
        }
    };

    // Load Saved Posts (for Saved tab)
    window.loadSavedPosts = async function() {
        const container = document.getElementById('saved-posts-container');
        if (!container) return;

        container.innerHTML = '<div style="text-align: center; padding: 24px; color: #667085;">Loading saved posts...</div>';

        try {
            const res = await fetch('/api/community/bookmarks', { credentials: 'same-origin' });
            if (!res.ok) throw new Error('Failed to load');
            const posts = await res.json();

            if (posts.length === 0) {
                container.innerHTML = `<div style="text-align: center; padding: 40px 20px;">
                    <div style="font-size: 32px; margin-bottom: 12px;">🔖</div>
                    <div style="font-size: 16px; font-weight: 600; color: #101828; margin-bottom: 4px;">No saved posts yet</div>
                    <div style="font-size: 14px; color: #667085;">Click the bookmark icon on any post to save it for later.</div>
                </div>`;
                return;
            }

            container.innerHTML = '';
            for (const p of posts) {
                const postEl = buildPostCard(p);
                container.appendChild(postEl);
            }
        } catch (e) {
            console.error(e);
            container.innerHTML = '<div style="padding: 24px; color: #D92D20; text-align: center;">Failed to load saved posts.</div>';
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
            const optEl = document.createElement('div');
            optEl.className = 'poll-option' + (opt.user_voted ? ' voted' : '');

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
                headers: { 'Content-Type': 'application/json' },
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

    window.openProfileEditModal = async function() {
        const modal = document.getElementById('edit-profile-modal');
        if (!modal) return;
        
        try {
            const res = await fetch('/api/community/profile/me');
            if (res.ok) {
                const profile = await res.json();
                document.getElementById('edit-profile-bio').value = profile.bio || '';
            }
        } catch (e) {
            console.error('Failed to load profile for editing', e);
        }
        
        modal.style.display = 'block';
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
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ bio: bio })
            });
            
            if (res.ok) {
                const updatedProfile = await res.json();
                const bioEl = document.getElementById('my-profile-bio');
                if (bioEl) bioEl.textContent = updatedProfile.bio || "No bio yet • Start your journey 🌱";
                document.getElementById('edit-profile-modal').style.display = 'none';
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

};

document.addEventListener('DOMContentLoaded', window.initCommunityFeed);
document.addEventListener('htmx:afterSwap', (e) => {
    if (e.target.id === 'community-content-area') {
        window.initCommunityFeed();
    }
});
