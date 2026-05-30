/**
 * community-circles.js — Circles & XP Tab Logic
 * Wires the My Circles tab to real /api/community/ endpoints
 */
window.initCommunityCircles = function () {
    // Bail only if the My Circles tab markup hasn't loaded yet — without the
    // modal mount point there's nothing to wire up. (The legacy `xp-level-icon`
    // probe was deleted with the XP card and was killing all bindings.)
    if (!document.getElementById('create-circle-modal')) return;

    // Lightweight toast helper — falls back to window.alert only if toast.js
    // failed to load. Default kind is 'error' so unannotated failure paths
    // get the right styling.
    const toast = (msg, kind) => (typeof window.showToast === 'function')
        ? window.showToast(msg, kind || 'error')
        : window.alert(msg);

    const XP_REASON_LABELS = {
        'post_created': '📝 Post Created',
        'comment_created': '💬 Comment Posted',
        'reaction_given': '🔥 Reaction Given',
        'reaction_received': '❤️ Reaction Received',
        'follow_gained': '👤 New Follower',
        'profile_completed': '✅ Profile Completed',
        'first_post': '🎉 First Post!',
        'first_investment': '💎 First Investment!',
        'investment_milestone_5': '🚀 5 Investments',
        'investment_milestone_10': '🏆 10 Investments',
        'investment_milestone_25': '⭐ 25 Investments',
        'investment_milestone_50': '👑 50 Investments',
        'circle_created': '🟢 Circle Created',
        'circle_joined': '🤝 Joined Circle',
        'circle_invite_accepted': '📩 Invite Accepted',
        'daily_login': '📅 Daily Login',
        'login_streak_7': '🔥 7-Day Streak',
        'login_streak_30': '💪 30-Day Streak',
        'badge_earned': '🏅 Badge Earned',
        'referral_signup': '🤝 Referral Signup',
        'referral_first_investment': '💰 Referral First Investment',
        'onboarding_complete': '🎓 Onboarding Complete',
        'ama_question': '🎙️ AMA Question Answered',
        'admin_grant': '⚡ Admin Grant',
        'admin_revoke': '⚠️ Admin Adjustment',
    };

    function appendEmptyState(container, text, _legacyStyles) {
        if (!container) return;
        container.replaceChildren();
        const empty = document.createElement('div');
        empty.className = 'community-loading-state';
        empty.textContent = text;
        container.appendChild(empty);
    }

    function createButton(label, className, onClick) {
        const button = document.createElement('button');
        button.type = 'button';
        button.className = className;
        button.textContent = label;
        button.addEventListener('click', onClick);
        return button;
    }

    // ─── Load XP Summary ─────────────────────────────────────────

    // Cache the last known level so we can fire the level-up animation
    // when the next /xp poll shows the user has crossed a tier.
    let _lastKnownLevel = null;

    async function loadXpSummary() {
        try {
            const res = await fetch('/api/community/xp');
            if (!res.ok) return;
            const data = await res.json();
            const newLevel = Number(data.level || 1);

            const setText = (id, val) => { const el = document.getElementById(id); if (el) el.textContent = val; };
            const setWidth = (id, val) => { const el = document.getElementById(id); if (el) el.style.width = val; };

            setText('xp-level-icon', data.level_icon || '🌱');
            setText('xp-level-name', data.level_name || 'Seedling');
            setText('xp-level-num', 'Level ' + newLevel);
            setText('xp-total', (data.xp_total || 0).toLocaleString());
            setWidth('xp-progress-bar', (data.progress_pct || 0) + '%');
            setText('xp-to-next', (data.xp_to_next || 0).toLocaleString() + ' XP to next level');
            setText('xp-progress-pct', Math.round(data.progress_pct || 0) + '%');

            // Login streak
            const streakEl = document.getElementById('xp-login-streak');
            if (streakEl && data.login_streak > 0) {
                streakEl.textContent = '🔥 ' + data.login_streak + '-day streak';
                streakEl.hidden = false;
            } else if (streakEl) {
                streakEl.hidden = true;
            }

            // Level-up celebration: only fire when we have a previous
            // baseline AND the level actually increased. First-load won't
            // pop the modal because _lastKnownLevel is null.
            if (_lastKnownLevel !== null && newLevel > _lastKnownLevel) {
                try {
                    showLevelUpAnimation(newLevel, data.level_name || '');
                } catch (e) {
                    console.error('showLevelUpAnimation failed', e);
                }
            }
            _lastKnownLevel = newLevel;
        } catch (e) {
            console.error('Failed to load XP summary', e);
        }
    }

    // ─── Load XP History ─────────────────────────────────────────

    async function loadXpHistory() {
        const container = document.getElementById('xp-history-list');
        if (!container) return;
        try {
            const res = await fetch('/api/community/xp/history?page=1');
            if (!res.ok) {
                appendEmptyState(container, 'No XP activity yet.', 'text-align:center;color:#667085;padding:24px;font-size:14px;');
                return;
            }
            const data = await res.json();
            const entries = data.entries || [];

            if (entries.length === 0) {
                appendEmptyState(container, 'No XP activity yet. Start posting and investing to earn XP!', 'text-align:center;color:#667085;padding:24px;font-size:14px;');
                return;
            }

            container.replaceChildren();
            for (const e of entries) {
                const label = XP_REASON_LABELS[e.reason] || e.reason;
                const isPositive = e.amount > 0;
                const sign = isPositive ? '+' : '';
                const date = new Date(e.created_at);
                const timeStr = date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' }) + ' · ' + date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });

                const row = document.createElement('div');
                row.className = 'community-xp-row';

                const meta = document.createElement('div');
                const reason = document.createElement('div');
                reason.className = 'community-xp-row__reason';
                reason.textContent = label;
                meta.appendChild(reason);

                const time = document.createElement('div');
                time.className = 'community-xp-row__time';
                time.textContent = timeStr;
                meta.appendChild(time);
                row.appendChild(meta);

                const amount = document.createElement('div');
                amount.className = 'community-xp-row__amount ' + (isPositive
                    ? 'community-xp-row__amount--positive'
                    : 'community-xp-row__amount--negative');
                amount.textContent = `${sign}${e.amount} XP`;
                row.appendChild(amount);
                container.appendChild(row);
            }
        } catch (e) {
            console.error('Failed to load XP history', e);
            appendEmptyState(container, 'Failed to load XP history.');
        }
    }

    // ─── Load Circle ─────────────────────────────────────────────

    async function loadMyCircle() {
        try {
            const res = await fetch('/api/community/circles/me');
            if (!res.ok) return;
            const data = await res.json();

            if (!data.circle) {
                document.getElementById('no-circle-state').style.display = 'block';
                document.getElementById('circle-content').style.display = 'none';
                document.getElementById('circle-stats-row').style.display = 'none';
                return;
            }

            document.getElementById('no-circle-state').style.display = 'none';
            document.getElementById('circle-content').style.display = 'block';
            document.getElementById('circle-stats-row').style.display = '';

            const c = data.circle;
            document.getElementById('circle-name-header').textContent = (c.avatar_emoji || '🟢') + ' ' + c.name;
            document.getElementById('circle-member-count').textContent = c.member_count;
            document.getElementById('circle-total-xp').textContent = (c.total_xp || 0).toLocaleString();
            document.getElementById('circle-level').textContent = 'Lv.' + c.level + ' ' + c.level_name;

            // Cache circle id for invite/role-mgmt actions
            window.currentCircleId = c.id;

            // CO.2 — render banner if set
            const banner = document.getElementById('circle-banner');
            if (banner) {
                if (c.banner_url) {
                    banner.style.backgroundImage = `url(${c.banner_url})`;
                    banner.hidden = false;
                } else {
                    banner.style.backgroundImage = '';
                    banner.hidden = true;
                }
            }

            // Set referral link (signup auto-joins this owner's circle)
            document.getElementById('circle-invite-link').value = window.location.origin + '/signup?ref=' + c.owner_id;

            // Hide leave button for owners (find ME, not "any owner")
            const members = data.members || [];
            const myUserId = (window.__POOOL_USER && window.__POOOL_USER.id) || null;
            const me = myUserId ? members.find(m => m.user_id === myUserId) : null;
            const myRole = me ? me.role : null;
            if (myRole === 'owner') {
                document.getElementById('leave-circle-btn').style.display = 'none';
            }

            // Render members with role-mgmt actions
            renderMembers(members, c.id, myUserId, myRole);

        } catch (e) {
            console.error('Failed to load circle', e);
        }
    }

    function renderMembers(members, circleId, myUserId, myRole) {
        const container = document.getElementById('circle-member-list');
        const colors = ['#E3F2FD', '#F3E5F5', '#E8F5E9', '#FFF3E0', '#FCE4EC', '#E0F2F1'];
        const canManage = myRole === 'owner' || myRole === 'admin';

        container.replaceChildren();
        members.forEach((m, i) => {
            const bg = colors[i % colors.length];
            const displayName = m.display_name || ('Investor #' + (m.user_id || '').substring(0, 6));
            const initials = displayName
                .split(/\s+/)
                .filter(Boolean)
                .slice(0, 2)
                .map((s) => s.charAt(0).toUpperCase())
                .join('') || displayName.substring(0, 2).toUpperCase();
            const joined = new Date(m.joined_at).toLocaleDateString('en-US', { month: 'short', year: 'numeric' });

            const row = document.createElement('div');
            row.className = 'circle-member';

            const avatar = document.createElement('div');
            avatar.className = 'circle-member-avatar';
            if (m.avatar_url) {
                avatar.style.backgroundImage = `url(${m.avatar_url})`;
                avatar.style.backgroundSize = 'cover';
                avatar.style.backgroundPosition = 'center';
                avatar.textContent = '';
            } else {
                avatar.style.background = bg;
                avatar.textContent = initials;
            }

            const info = document.createElement('div');
            info.className = 'circle-member-info';

            const name = document.createElement('span');
            name.className = 'circle-member-name';
            name.textContent = displayName;

            if (m.role === 'owner' || m.role === 'admin') {
                const roleLabel = document.createElement('span');
                roleLabel.className = 'circle-member-role circle-member-role--' + m.role;
                roleLabel.textContent = m.role === 'owner' ? 'Owner' : 'Admin';
                name.appendChild(roleLabel);
            }

            const detail = document.createElement('span');
            detail.className = 'circle-member-detail';
            detail.textContent = `Joined ${joined}`;

            const status = document.createElement('span');
            status.className = 'circle-member-status circle-member-status--active';
            status.textContent = m.role || 'member';

            info.appendChild(name);
            info.appendChild(detail);
            row.appendChild(avatar);
            row.appendChild(info);
            row.appendChild(status);

            // Role management actions (owner: full control; admin: kick members only)
            const isSelf = myUserId && m.user_id === myUserId;
            if (canManage && !isSelf && m.role !== 'owner') {
                const actions = document.createElement('div');
                actions.className = 'circle-member-actions';
                actions.style.cssText = 'display:flex; gap:6px; margin-left:8px;';

                if (myRole === 'owner') {
                    if (m.role === 'admin') {
                        actions.appendChild(makeRowBtn('Demote', () => window.changeMemberRole(m.user_id, displayName, 'member')));
                    } else {
                        actions.appendChild(makeRowBtn('Promote', () => window.changeMemberRole(m.user_id, displayName, 'admin')));
                    }
                    actions.appendChild(makeRowBtn('Transfer', () => window.transferCircleOwnership(m.user_id, displayName), 'secondary'));
                }
                actions.appendChild(makeRowBtn('Kick', () => window.kickCircleMember(m.user_id, displayName), 'danger'));

                row.appendChild(actions);
            }

            container.appendChild(row);
        });

        if (members.length === 0) {
            appendEmptyState(container, 'No members yet');
        }
    }

    // ─── Load Circle Leaderboard ─────────────────────────────────────

    let myJoinRequestCircleIds = new Set(); // circle IDs where I have a pending request

    async function loadMyJoinRequests() {
        try {
            const res = await fetch('/api/community/circles/requests/mine');
            if (!res.ok) return;
            const data = await res.json();
            myJoinRequestCircleIds = new Set((data.requests || []).map(r => r.circle_id));
        } catch (e) { /* non-critical */ }
    }

    async function loadCircleLeaderboard() {
        const container = document.getElementById('circle-leaderboard-list');
        if (!container) return;
        try {
            await loadMyJoinRequests();

            const res = await fetch('/api/community/circles/leaderboard');
            if (!res.ok) return;
            const data = await res.json();
            const circles = data.circles || [];

            if (circles.length === 0) {
                appendEmptyState(container, 'No circles yet. Be the first!');
                return;
            }

            const medals = ['🥇', '🥈', '🥉'];
            container.replaceChildren();
            circles.forEach((c, i) => {
                const medal = medals[i] || `#${i + 1}`;
                const isPrivate = !c.is_public;

                const item = document.createElement('div');
                item.className = 'circle-lb-item';

                const medalEl = document.createElement('span');
                medalEl.className = 'circle-lb-item__medal';
                medalEl.textContent = medal;

                const emoji = document.createElement('span');
                emoji.className = 'circle-lb-item__emoji';
                emoji.textContent = c.avatar_emoji || '🟢';

                const info = document.createElement('div');
                info.className = 'circle-lb-item__info';

                const title = document.createElement('div');
                title.className = 'circle-lb-item__title';
                title.textContent = c.name || 'Circle';

                const privacyBadge = document.createElement('span');
                privacyBadge.className = 'circle-lb-item__privacy ' + (isPrivate
                    ? 'circle-lb-item__privacy--private'
                    : 'circle-lb-item__privacy--public');
                privacyBadge.textContent = isPrivate ? '🔒 Private' : '🌐 Public';
                title.appendChild(privacyBadge);

                const meta = document.createElement('div');
                meta.className = 'circle-lb-item__meta';
                meta.textContent = `${Number(c.member_count || 0).toLocaleString()} members · Lv.${c.level || 1}`;

                const actions = document.createElement('div');
                actions.className = 'circle-lb-item__actions';

                const xp = document.createElement('span');
                xp.className = 'circle-lb-item__xp';
                xp.textContent = `${(c.total_xp || 0).toLocaleString()} XP`;

                let actionEl;
                if (isPrivate && myJoinRequestCircleIds.has(c.id)) {
                    actionEl = document.createElement('span');
                    actionEl.className = 'circle-lb-item__pending';
                    actionEl.textContent = '⏳ Pending';
                } else if (isPrivate) {
                    actionEl = createButton('🔒 Request', 'ds-btn ds-btn--secondary ds-btn--sm', () => window.handleRequestJoinCircle(c.id));
                } else {
                    actionEl = createButton('Join', 'ds-btn ds-btn--primary ds-btn--sm', () => window.handleJoinCircle(c.id));
                }

                info.appendChild(title);
                info.appendChild(meta);
                actions.appendChild(xp);
                actions.appendChild(actionEl);
                item.appendChild(medalEl);
                item.appendChild(emoji);
                item.appendChild(info);
                item.appendChild(actions);
                container.appendChild(item);
            });
        } catch (e) {
            console.error('Failed to load circle leaderboard', e);
        }
    }

    // ─── Load pending invites ────────────────────────────────────

    async function loadPendingInvites() {
        try {
            const res = await fetch('/api/community/invites');
            if (!res.ok) return;
            const data = await res.json();
            const invites = data.invites || [];

            if (invites.length === 0) return;

            document.getElementById('pending-invites-section').style.display = 'block';
            const container = document.getElementById('invite-list');
            container.replaceChildren();
            for (const inv of invites) {
                const row = document.createElement('div');
                row.className = 'community-invite-row';

                const info = document.createElement('div');
                const title = document.createElement('div');
                title.className = 'community-invite-row__title';
                title.textContent = `Circle invite from #${(inv.inviter_id || '').substring(0, 6)}`;
                const expires = document.createElement('div');
                expires.className = 'community-invite-row__expires';
                expires.textContent = `Expires ${new Date(inv.expires_at).toLocaleDateString()}`;

                const actions = document.createElement('div');
                actions.className = 'community-invite-row__actions';
                actions.appendChild(createButton('Accept', 'ds-btn ds-btn--primary ds-btn--sm', () => window.handleAcceptInvite(inv.id)));
                actions.appendChild(createButton('Decline', 'ds-btn ds-btn--secondary ds-btn--sm', () => window.handleDeclineInvite(inv.id)));

                info.appendChild(title);
                info.appendChild(expires);
                row.appendChild(info);
                row.appendChild(actions);
                container.appendChild(row);
            }
        } catch (e) {
            console.error('Failed to load invites', e);
        }
    }

    // ─── Actions ─────────────────────────────────────────────────

    window.handleCreateCircle = async function () {
        const btn = document.querySelector('#create-circle-modal .ds-btn--primary');
        const name = document.getElementById('circle-name-input').value.trim();
        if (!name) return toast('Please enter a circle name', 'warning');

        const desc = document.getElementById('circle-desc-input').value.trim();
        // Emoji input was removed from the UI; backend gets a sensible default.
        const emoji = (document.getElementById('circle-emoji-input')?.value.trim()) || '🟢';

        if (btn) { btn.disabled = true; btn.dataset._label = btn.textContent; btn.textContent = 'Creating…'; }
        try {
            const res = await fetch('/api/community/circles', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                credentials: 'same-origin',
                body: JSON.stringify({ name, description: desc || null, emoji })
            });
            let payload = null;
            try { payload = await res.json(); } catch (_) { /* non-JSON body */ }
            if (!res.ok) {
                const msg = (payload && (payload.error || payload.message))
                    || `Request failed (${res.status})`;
                throw new Error(msg);
            }
            toast('Circle created', 'success');
            if (typeof window.closeCommunityModal === 'function') {
                window.closeCommunityModal('create-circle-modal');
            } else {
                document.getElementById('create-circle-modal').style.display = 'none';
            }
            // Reset inputs so the modal opens clean next time.
            const nameEl = document.getElementById('circle-name-input');
            const descEl = document.getElementById('circle-desc-input');
            if (nameEl) nameEl.value = '';
            if (descEl) descEl.value = '';
            loadAll();
        } catch (e) {
            toast(e.message || 'Failed to create circle', 'error');
        } finally {
            if (btn) { btn.disabled = false; if (btn.dataset._label) btn.textContent = btn.dataset._label; }
        }
    };

    window.handleLeaveCircle = async function () {
        if (!confirm('Are you sure you want to leave this circle?')) return;
        try {
            const res = await fetch('/api/community/circles/leave', { method: 'POST' });
            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }
            loadAll();
        } catch (e) {
            toast('Failed to leave circle: ' + e.message);
        }
    };

    window.handleAcceptInvite = async function (inviteId) {
        try {
            const res = await fetch(`/api/community/invites/${inviteId}/accept`, { method: 'POST' });
            if (!res.ok) throw new Error(await res.text());
            loadAll();
        } catch (e) {
            toast('Failed: ' + e.message);
        }
    };

    window.handleDeclineInvite = async function (inviteId) {
        try {
            const res = await fetch(`/api/community/invites/${inviteId}/decline`, { method: 'POST' });
            if (!res.ok) throw new Error(await res.text());
            loadAll();
        } catch (e) {
            toast('Failed: ' + e.message);
        }
    };

    window.openCircleSettings = async function () {
        try {
            const res = await fetch('/api/community/circles/me', { credentials: 'same-origin' });
            if (!res.ok) { toast('Could not load circle data.'); return; }
            const data = await res.json();
            if (!data.circle) { toast('You are not in a circle.'); return; }

            const c = data.circle;
            window._currentCircleId = c.id;
            // Snapshot original gate state so save can detect a no-op vs real change
            window._currentCircleGate = {
                asset_id: c.token_gate_asset_id || null,
                min_value_cents: typeof c.token_gate_min_value_cents === 'number' ? c.token_gate_min_value_cents : null,
            };

            // Pre-fill fields
            document.getElementById('settings-circle-name').value = c.name || '';
            document.getElementById('settings-circle-desc').value = c.description || '';
            document.getElementById('settings-circle-emoji').value = c.avatar_emoji || '🟢';

            // Set toggle state
            const isPublic = !!c.is_public;
            const checkbox = document.getElementById('settings-circle-public');
            checkbox.checked = isPublic;
            const track = document.getElementById('settings-toggle-track');
            if (track && track.parentElement) {
                track.parentElement.classList.toggle('community-modal__switch--on', isPublic);
            }

            // CO.2 — circle banner field. Snapshot the current URL so the
            // PUT only sends a change (or empty string for clear).
            window._currentCircleBanner = c.banner_url || null;
            applyBannerPreview(c.banner_url || null);
            const hidden = document.getElementById('settings-circle-banner-url');
            if (hidden) hidden.value = c.banner_url || '';

            // Token-gate fields (W3.1)
            await loadGateAssetOptions(c.token_gate_asset_id || '');
            const minInput = document.getElementById('settings-circle-gate-min');
            if (minInput) {
                const cents = c.token_gate_min_value_cents;
                minInput.value = (typeof cents === 'number' && cents > 0) ? Math.round(cents / 100) : '';
            }

            // Show modal
            if (typeof window.openCommunityModal === 'function') {
                window.openCommunityModal('circle-settings-modal');
            } else {
                document.getElementById('circle-settings-modal').style.display = 'flex';
            }
        } catch (e) {
            console.error('Failed to open circle settings', e);
            toast('Error loading settings: ' + e.message);
        }
    };

    // CO.2 — banner preview helper. Renders the current image inside the
    // settings preview panel and toggles the "Remove" button visibility.
    function applyBannerPreview(url) {
        const preview = document.getElementById('settings-circle-banner-preview');
        const clearBtn = document.getElementById('settings-circle-banner-clear-btn');
        if (!preview) return;
        if (url) {
            preview.style.backgroundImage = `url(${url})`;
            preview.textContent = '';
            if (clearBtn) clearBtn.hidden = false;
        } else {
            preview.style.backgroundImage = '';
            preview.textContent = 'No banner yet';
            if (clearBtn) clearBtn.hidden = true;
        }
    }

    window.uploadCircleBanner = async function (event) {
        const file = event && event.target && event.target.files && event.target.files[0];
        if (!file) return;
        if (file.size > 5 * 1024 * 1024) {
            toast('Banner must be smaller than 5 MB.');
            event.target.value = '';
            return;
        }
        const status = document.getElementById('settings-circle-banner-status');
        if (status) status.textContent = 'Uploading…';
        try {
            const fd = new FormData();
            fd.append('file', file);
            const res = await fetch('/api/upload/post-image', { method: 'POST', credentials: 'same-origin', body: fd });
            const data = await res.json();
            if (!res.ok) throw new Error(data.error || ('Upload failed (' + res.status + ')'));
            const url = data.image_url;
            const hidden = document.getElementById('settings-circle-banner-url');
            if (hidden) hidden.value = url;
            applyBannerPreview(url);
            if (status) status.textContent = 'Banner ready — Save to apply.';
        } catch (e) {
            console.error('uploadCircleBanner failed', e);
            if (status) status.textContent = e.message || 'Upload failed.';
        } finally {
            event.target.value = '';
        }
    };

    window.clearCircleBanner = function () {
        const hidden = document.getElementById('settings-circle-banner-url');
        if (hidden) hidden.value = '';
        applyBannerPreview(null);
        const status = document.getElementById('settings-circle-banner-status');
        if (status) status.textContent = 'Banner will be removed on Save.';
    };

    // Load published assets into the token-gate <select>. Cached for the session.
    let _gateAssetsCache = null;
    async function loadGateAssetOptions(selectedAssetId) {
        const sel = document.getElementById('settings-circle-gate-asset');
        if (!sel) return;
        if (!_gateAssetsCache) {
            try {
                const res = await fetch('/api/marketplace/secondary/assets', { credentials: 'same-origin' });
                if (!res.ok) throw new Error('asset list failed (' + res.status + ')');
                const data = await res.json();
                _gateAssetsCache = Array.isArray(data) ? data : (data.assets || []);
            } catch (e) {
                console.error('loadGateAssetOptions failed', e);
                _gateAssetsCache = [];
            }
        }
        sel.replaceChildren();
        const ph = document.createElement('option');
        ph.value = '';
        ph.textContent = 'No token gate';
        sel.appendChild(ph);
        for (const a of _gateAssetsCache) {
            const opt = document.createElement('option');
            opt.value = a.id;
            opt.textContent = a.name || a.title || a.slug || ('Asset ' + String(a.id).slice(0, 8));
            sel.appendChild(opt);
        }
        sel.value = selectedAssetId || '';
    }

    window.handleSaveCircleSettings = async function () {
        const circleId = window._currentCircleId;
        if (!circleId) { toast('No circle selected'); return; }

        const name = document.getElementById('settings-circle-name').value.trim();
        if (!name) { toast('Circle name is required.'); return; }

        const description = document.getElementById('settings-circle-desc').value.trim();
        const emoji = document.getElementById('settings-circle-emoji').value.trim() || '🟢';
        const isPublic = document.getElementById('settings-circle-public').checked;

        const saveBtn = document.getElementById('settings-save-btn');
        const originalText = saveBtn.textContent;
        saveBtn.textContent = 'Saving...';
        saveBtn.disabled = true;

        try {
            // CO.2 — only include banner_url in the body if it actually changed.
            // The backend reads "field present" as "set this", so omitting it
            // leaves the existing value alone; sending empty string clears it.
            const bannerHidden = document.getElementById('settings-circle-banner-url');
            const newBanner = bannerHidden ? bannerHidden.value : null;
            const oldBanner = (window._currentCircleBanner == null) ? '' : String(window._currentCircleBanner);
            const updateBody = { name, description: description || null, emoji };
            if ((newBanner || '') !== oldBanner) {
                updateBody.banner_url = newBanner || '';
            }

            // Update name/description/emoji/banner
            const updateRes = await fetch(`/api/community/circles/${circleId}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                credentials: 'same-origin',
                body: JSON.stringify(updateBody)
            });
            if (!updateRes.ok) {
                const err = await updateRes.text();
                throw new Error(err);
            }

            // Update privacy
            const privacyRes = await fetch(`/api/community/circles/${circleId}/privacy`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                credentials: 'same-origin',
                body: JSON.stringify({ is_public: isPublic })
            });
            if (!privacyRes.ok) {
                const err = await privacyRes.text();
                throw new Error(err);
            }

            // Update token-gate (W3.1) — only POST if values changed
            const gateAssetEl = document.getElementById('settings-circle-gate-asset');
            const gateMinEl = document.getElementById('settings-circle-gate-min');
            if (gateAssetEl) {
                const newAssetId = gateAssetEl.value || null;
                const minDollars = parseFloat(gateMinEl ? gateMinEl.value : '');
                const newMinCents = (newAssetId && Number.isFinite(minDollars) && minDollars > 0)
                    ? Math.round(minDollars * 100)
                    : null;
                const original = window._currentCircleGate || { asset_id: null, min_value_cents: null };
                const changed = (original.asset_id || null) !== newAssetId
                    || (original.min_value_cents || null) !== newMinCents;
                if (changed) {
                    const gateRes = await fetch(`/api/community/circles/${circleId}/token-gate`, {
                        method: 'POST',
                        credentials: 'same-origin',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ asset_id: newAssetId, min_value_cents: newMinCents }),
                    });
                    if (!gateRes.ok) {
                        const err = await gateRes.text();
                        throw new Error('Token gate: ' + err);
                    }
                }
            }

            // Close modal and reload data
            if (typeof window.closeCommunityModal === 'function') {
                window.closeCommunityModal('circle-settings-modal');
            } else {
                document.getElementById('circle-settings-modal').style.display = 'none';
            }
            if (typeof window.loadCirclesAndXp === 'function') window.loadCirclesAndXp();
        } catch (e) {
            toast('Failed to save settings: ' + e.message);
        } finally {
            saveBtn.textContent = originalText;
            saveBtn.disabled = false;
        }
    };

    window.handleDeleteCircle = async function () {
        const circleId = window._currentCircleId;
        if (!circleId) return;

        try {
            const res = await fetch(`/api/community/circles/${circleId}`, {
                method: 'DELETE',
                credentials: 'same-origin'
            });
            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }
            if (typeof window.closeCommunityModal === 'function') {
                window.closeCommunityModal('circle-settings-modal');
            } else {
                document.getElementById('circle-settings-modal').style.display = 'none';
            }
            if (typeof window.loadCirclesAndXp === 'function') window.loadCirclesAndXp();
        } catch (e) {
            toast('Failed to delete circle: ' + e.message);
        }
    };

    function makeRowBtn(label, onClick, variant) {
        const cls = variant === 'danger'
            ? 'ds-btn ds-btn--ghost ds-btn--sm'
            : variant === 'secondary'
                ? 'ds-btn ds-btn--secondary ds-btn--sm'
                : 'ds-btn ds-btn--primary ds-btn--sm';
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = cls;
        btn.textContent = label;
        if (variant === 'danger') {
            btn.style.color = '#B42318';
        }
        btn.onclick = onClick;
        return btn;
    }

    async function postJson(url, body) {
        const res = await fetch(url, {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json' },
            body: body == null ? undefined : JSON.stringify(body),
        });
        if (!res.ok) {
            const data = await res.json().catch(() => ({}));
            throw new Error(data.error || `Request failed (${res.status})`);
        }
        return res.json().catch(() => ({}));
    }

    window.changeMemberRole = async function (userId, displayName, role) {
        const circleId = window.currentCircleId;
        if (!circleId) return;
        try {
            await postJson(`/api/community/circles/${circleId}/roles`, { user_id: userId, role });
            if (window.showToast) window.showToast(`${displayName || 'Member'} → ${role}`, 'success');
            if (typeof window.loadCirclesAndXp === 'function') window.loadCirclesAndXp();
        } catch (e) {
            toast('Role change failed: ' + e.message, 'error');
        }
    };

    window.kickCircleMember = async function (userId, displayName) {
        const circleId = window.currentCircleId;
        if (!circleId) return;
        if (!confirm(`Remove ${displayName || 'this member'} from the circle?`)) return;
        try {
            await postJson(`/api/community/circles/${circleId}/kick/${userId}`);
            if (window.showToast) window.showToast(`${displayName || 'Member'} removed`, 'success');
            if (typeof window.loadCirclesAndXp === 'function') window.loadCirclesAndXp();
        } catch (e) {
            toast('Remove failed: ' + e.message, 'error');
        }
    };

    window.transferCircleOwnership = async function (userId, displayName) {
        const circleId = window.currentCircleId;
        if (!circleId) return;
        const name = displayName || 'this member';
        if (!confirm(`Transfer ownership to ${name}? You will become a regular admin and cannot undo this without their consent.`)) return;
        try {
            await postJson(`/api/community/circles/${circleId}/transfer`, { new_owner_id: userId });
            if (window.showToast) window.showToast(`Ownership transferred to ${name}`, 'success');
            if (typeof window.loadCirclesAndXp === 'function') window.loadCirclesAndXp();
        } catch (e) {
            toast('Transfer failed: ' + e.message, 'error');
        }
    };

    window.copyInviteLink = function () {
        const input = document.getElementById('circle-invite-link');
        input.select();
        document.execCommand('copy');
        const btn = input.nextElementSibling;
        btn.textContent = 'Copied!';
        setTimeout(() => { btn.textContent = 'Copy'; }, 2000);
    };

    // ─── Search & Invite Existing Investors ──────────────────────────
    let _inviteSearchTimer = null;

    window.searchUsersToInvite = function (raw) {
        const query = (raw || '').trim();
        const resultsEl = document.getElementById('circle-invite-results');
        const statusEl = document.getElementById('circle-invite-status');
        if (!resultsEl) return;
        clearTimeout(_inviteSearchTimer);
        statusEl.textContent = '';
        if (query.length < 2) {
            resultsEl.style.display = 'none';
            resultsEl.replaceChildren();
            return;
        }
        _inviteSearchTimer = setTimeout(async () => {
            try {
                const res = await fetch('/api/community/mentions/suggest?q=' + encodeURIComponent(query), { credentials: 'same-origin' });
                if (!res.ok) throw new Error('search failed (' + res.status + ')');
                const data = await res.json();
                const users = Array.isArray(data.users) ? data.users : [];
                resultsEl.replaceChildren();
                if (users.length === 0) {
                    const empty = document.createElement('div');
                    empty.className = 'cc-invite-results__empty';
                    empty.textContent = 'No matches.';
                    resultsEl.appendChild(empty);
                } else {
                    for (const u of users) {
                        const row = document.createElement('button');
                        row.type = 'button';
                        row.className = 'cc-invite-results__row';

                        const avatar = document.createElement('div');
                        avatar.className = 'cc-invite-results__avatar';
                        if (u.avatar_url) {
                            avatar.style.background = `url(${u.avatar_url}) center/cover`;
                        } else {
                            avatar.textContent = (u.display_name || '?').charAt(0).toUpperCase();
                        }

                        const name = document.createElement('span');
                        name.className = 'cc-invite-results__name';
                        name.textContent = u.display_name || ('User ' + String(u.user_id).slice(0, 8));

                        const action = document.createElement('span');
                        action.className = 'cc-invite-results__cta';
                        action.textContent = 'Invite';

                        row.appendChild(avatar);
                        row.appendChild(name);
                        row.appendChild(action);
                        row.onclick = () => window.inviteUserToCircle(u.user_id, u.display_name);
                        resultsEl.appendChild(row);
                    }
                }
                resultsEl.style.display = 'block';
            } catch (e) {
                console.error('searchUsersToInvite failed', e);
                statusEl.textContent = 'Search failed. Try again.';
            }
        }, 200);
    };

    window.inviteUserToCircle = async function (userId, displayName) {
        const circleId = window.currentCircleId;
        const statusEl = document.getElementById('circle-invite-status');
        if (!circleId) {
            statusEl.textContent = 'Circle not loaded yet.';
            return;
        }
        statusEl.textContent = 'Sending invite…';
        try {
            const res = await fetch(`/api/community/circles/${circleId}/invite`, {
                method: 'POST',
                credentials: 'same-origin',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ invitee_id: userId }),
            });
            const data = await res.json().catch(() => ({}));
            if (!res.ok) throw new Error(data.error || `Invite failed (${res.status})`);
            statusEl.textContent = `Invite sent to ${displayName || 'user'}.`;
            if (window.showToast) window.showToast(`Invite sent to ${displayName || 'user'}`, 'success');
            const search = document.getElementById('circle-invite-search');
            if (search) search.value = '';
            const resultsEl = document.getElementById('circle-invite-results');
            if (resultsEl) { resultsEl.replaceChildren(); resultsEl.style.display = 'none'; }
        } catch (e) {
            console.error('inviteUserToCircle failed', e);
            statusEl.textContent = e.message || 'Failed to send invite.';
        }
    };

    // ─── Level-Up Animation ──────────────────────────────────────

    function showLevelUpAnimation(level, name) {
        const overlay = document.createElement('div');
        overlay.innerHTML = `
        <div style="position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.7);z-index:9999;display:flex;align-items:center;justify-content:center;animation:fadeIn 0.3s ease;">
            <div style="background:linear-gradient(135deg,#0a0b2e,#1a1b4b);border-radius:24px;padding:48px;text-align:center;max-width:400px;animation:bounceIn 0.5s ease;">
                <div style="font-size:64px;margin-bottom:16px;animation:float 2s ease-in-out infinite;">🎉</div>
                <h2 style="font-size:28px;font-weight:700;color:#fff;margin:0 0 8px;">Level Up!</h2>
                <p style="font-size:18px;color:var(--btn-primary-bg, #0000FF);font-weight:600;margin:0 0 8px;">Level ${level} — ${name}</p>
                <p style="font-size:14px;color:#98a2b3;margin:0 0 24px;">Keep investing and engaging to reach new heights!</p>
                <button onclick="this.closest('div').parentElement.remove()" class="ds-btn ds-btn--primary">Awesome! 🚀</button>
            </div>
        </div>`;
        document.body.appendChild(overlay);
        setTimeout(() => overlay.remove(), 10000);
    }

    // ─── Join Requests (for owners/admins) ──────────────────────────

    async function loadPendingJoinRequests() {
        const myCircleRes = await fetch('/api/community/circles/me');
        if (!myCircleRes.ok) return;
        const myData = await myCircleRes.json();
        if (!myData.circle) return;

        const circleId = myData.circle.id;
        // Check if I'm owner or admin
        const me = myData.members.find(m => m.role === 'owner' || m.role === 'admin');
        if (!me) return;

        try {
            const res = await fetch(`/api/community/circles/${circleId}/requests`);
            if (!res.ok) return;
            const data = await res.json();
            const requests = data.requests || [];

            const section = document.getElementById('pending-requests-section');
            const container = document.getElementById('requests-list');
            const badge = document.getElementById('requests-count-badge');

            if (requests.length === 0) {
                section.style.display = 'none';
                return;
            }

            section.style.display = 'block';
            badge.textContent = requests.length + ' pending';

            container.replaceChildren();
            for (const req of requests) {
                const date = new Date(req.created_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
                const row = document.createElement('div');
                row.className = 'community-request-row';

                const requester = document.createElement('div');
                requester.className = 'community-request-row__requester';

                const avatar = document.createElement('div');
                avatar.className = 'community-request-row__avatar';
                avatar.textContent = (req.user_name || 'U').charAt(0).toUpperCase();

                const info = document.createElement('div');
                const name = document.createElement('div');
                name.className = 'community-request-row__name';
                name.textContent = req.user_name || 'Unknown User';
                const requested = document.createElement('div');
                requested.className = 'community-request-row__date';
                requested.textContent = `Requested ${date}`;

                const actions = document.createElement('div');
                actions.className = 'community-request-row__actions';
                actions.appendChild(createButton('✓ Approve', 'ds-btn ds-btn--primary ds-btn--sm', () => window.handleApproveRequest(req.id)));
                actions.appendChild(createButton('✗ Decline', 'ds-btn ds-btn--secondary ds-btn--sm community-request-row__decline', () => window.handleDeclineRequest(req.id)));

                info.appendChild(name);
                info.appendChild(requested);
                requester.appendChild(avatar);
                requester.appendChild(info);
                row.appendChild(requester);
                row.appendChild(actions);
                container.appendChild(row);
            }
        } catch (e) {
            console.error('Failed to load join requests', e);
        }
    }

    window.handleApproveRequest = async function (requestId) {
        try {
            const res = await fetch(`/api/community/circles/requests/${requestId}/approve`, { method: 'POST' });
            if (!res.ok) throw new Error(await res.text());
            loadAll();
        } catch (e) {
            toast('Failed to approve: ' + e.message);
        }
    };

    window.handleDeclineRequest = async function (requestId) {
        try {
            const res = await fetch(`/api/community/circles/requests/${requestId}/decline`, { method: 'POST' });
            if (!res.ok) throw new Error(await res.text());
            loadAll();
        } catch (e) {
            toast('Failed to decline: ' + e.message);
        }
    };

    // ─── Join / Request Join Handlers ─────────────────────────────

    window.handleJoinCircle = async function (circleId) {
        try {
            const res = await fetch(`/api/community/circles/${circleId}/join`, {
                method: 'POST',
                credentials: 'same-origin'
            });
            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }
            loadAll();
        } catch (e) {
            toast('Failed to join circle: ' + e.message);
        }
    };

    window.handleRequestJoinCircle = async function (circleId) {
        try {
            const res = await fetch(`/api/community/circles/${circleId}/request`, {
                method: 'POST',
                credentials: 'same-origin'
            });
            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }
            myJoinRequestCircleIds.add(circleId);
            loadCircleLeaderboard(); // Re-render to show "Pending"
        } catch (e) {
            toast('Failed to send request: ' + e.message);
        }
    };

    // ─── Init ────────────────────────────────────────────────────

    function loadAll() {
        // 2026-05-16: the MyCircle tab was redesigned as a multi-circle
        // discovery + my-circles list (`community-circles-discover.js`).
        // The legacy XP-summary + single-circle stats + recent-activity
        // sections were removed from the partial. Skip the legacy load
        // path when none of its target nodes are present so we don't
        // burn HTTP requests on dead UI.
        var legacyMode = !!document.getElementById('xp-summary-card')
                      || !!document.getElementById('circle-stats-row')
                      || !!document.getElementById('xp-history-list');
        if (legacyMode) {
            loadXpSummary();
            loadMyCircle();
            loadXpHistory();
            loadCircleLeaderboard();
        }
        // Pending invites + join requests are still rendered in the new
        // partial under the same #pending-* IDs, so always run.
        loadPendingInvites();
        loadPendingJoinRequests();
    }


    // Only load when the tab becomes visible (via HTMX swap or direct click)
    const circleTabBtn = document.querySelector('[data-tab="community-circle-tab"]');
    if (circleTabBtn) {
        circleTabBtn.addEventListener('click', function () {
            loadAll();
        });
    }

    // Also expose for other scripts
    window.loadCirclesAndXp = loadAll;
    window.showLevelUpAnimation = showLevelUpAnimation;
    loadAll();
};

document.addEventListener('DOMContentLoaded', window.initCommunityCircles);
document.addEventListener('htmx:afterSwap', (e) => {
    if (e.target.id === 'community-content-area') window.initCommunityCircles();
});
