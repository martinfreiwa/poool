/**
 * community-circles.js — Circles & XP Tab Logic
 * Wires the My Circle tab to real /api/community/ endpoints
 */
document.addEventListener('DOMContentLoaded', function () {

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

    // ─── Load XP Summary ─────────────────────────────────────────

    async function loadXpSummary() {
        try {
            const res = await fetch('/api/community/xp');
            if (!res.ok) return;
            const data = await res.json();

            document.getElementById('xp-level-icon').textContent = data.level_icon || '🌱';
            document.getElementById('xp-level-name').textContent = data.level_name || 'Seedling';
            document.getElementById('xp-level-num').textContent = 'Level ' + (data.level || 1);
            document.getElementById('xp-total').textContent = (data.xp_total || 0).toLocaleString();
            document.getElementById('xp-progress-bar').style.width = (data.progress_pct || 0) + '%';
            document.getElementById('xp-to-next').textContent = (data.xp_to_next || 0).toLocaleString() + ' XP to next level';
            document.getElementById('xp-progress-pct').textContent = Math.round(data.progress_pct || 0) + '%';

            // Login streak
            const streakEl = document.getElementById('xp-login-streak');
            if (streakEl && data.login_streak > 0) {
                streakEl.textContent = '🔥 ' + data.login_streak + '-day streak';
                streakEl.style.display = '';
            } else if (streakEl) {
                streakEl.style.display = 'none';
            }
        } catch (e) {
            console.error('Failed to load XP summary', e);
        }
    }

    // ─── Load XP History ─────────────────────────────────────────

    async function loadXpHistory() {
        const container = document.getElementById('xp-history-list');
        try {
            const res = await fetch('/api/community/xp/history?page=1');
            if (!res.ok) {
                container.innerHTML = '<div style="text-align:center;color:#667085;padding:24px;font-size:14px;">No XP activity yet.</div>';
                return;
            }
            const data = await res.json();
            const entries = data.entries || [];

            if (entries.length === 0) {
                container.innerHTML = '<div style="text-align:center;color:#667085;padding:24px;font-size:14px;">No XP activity yet. Start posting and investing to earn XP!</div>';
                return;
            }

            let html = '';
            for (const e of entries) {
                const label = XP_REASON_LABELS[e.reason] || e.reason;
                const isPositive = e.amount > 0;
                const color = isPositive ? '#027A48' : '#F04438';
                const bg = isPositive ? '#ECFDF3' : '#FEF3F2';
                const sign = isPositive ? '+' : '';
                const date = new Date(e.created_at);
                const timeStr = date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' }) + ' · ' + date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });

                html += `
                <div style="display:flex;align-items:center;justify-content:space-between;padding:12px 24px;border-bottom:1px solid var(--card-border-color);">
                    <div>
                        <div style="font-size:14px;font-weight:500;color:#101828;">${label}</div>
                        <div style="font-size:12px;color:#667085;margin-top:2px;">${timeStr}</div>
                    </div>
                    <div style="font-size:14px;font-weight:700;color:${color};background:${bg};padding:4px 12px;border-radius:20px;">
                        ${sign}${e.amount} XP
                    </div>
                </div>`;
            }
            container.innerHTML = html;
        } catch (e) {
            console.error('Failed to load XP history', e);
            container.innerHTML = '<div style="text-align:center;color:#667085;padding:24px;">Failed to load XP history.</div>';
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

            // Set invite link
            document.getElementById('circle-invite-link').value = window.location.origin + '/signup?ref=' + c.owner_id;

            // Hide leave button for owners
            const members = data.members || [];
            const currentMember = members.find(m => m.role === 'owner');
            if (currentMember) {
                document.getElementById('leave-circle-btn').style.display = 'none';
            }

            // Render members
            renderMembers(members, c.id);

        } catch (e) {
            console.error('Failed to load circle', e);
        }
    }

    function renderMembers(members, circleId) {
        const container = document.getElementById('circle-member-list');
        const colors = ['#E3F2FD', '#F3E5F5', '#E8F5E9', '#FFF3E0', '#FCE4EC', '#E0F2F1'];

        let html = '';
        members.forEach((m, i) => {
            const bg = colors[i % colors.length];
            const initials = (m.user_id || '').substring(0, 2).toUpperCase();
            const roleLabel = m.role === 'owner' ? '<span style="font-size:10px;background:#0000FF;color:#fff;padding:1px 6px;border-radius:4px;margin-left:4px;">Owner</span>'
                : m.role === 'admin' ? '<span style="font-size:10px;background:#7A5AF8;color:#fff;padding:1px 6px;border-radius:4px;margin-left:4px;">Admin</span>'
                : '';
            const joined = new Date(m.joined_at).toLocaleDateString('en-US', { month: 'short', year: 'numeric' });

            html += `
            <div class="circle-member">
                <div class="circle-member-avatar" style="background:${bg};">${initials}</div>
                <div class="circle-member-info">
                    <span class="circle-member-name">Investor #${(m.user_id || '').substring(0, 6)} ${roleLabel}</span>
                    <span class="circle-member-detail">Joined ${joined}</span>
                </div>
                <span class="circle-member-status circle-member-status--active">${m.role}</span>
            </div>`;
        });

        container.innerHTML = html || '<div style="text-align:center;padding:16px;color:#667085;">No members yet</div>';
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
        try {
            await loadMyJoinRequests();

            const res = await fetch('/api/community/circles/leaderboard');
            if (!res.ok) return;
            const data = await res.json();
            const circles = data.circles || [];

            if (circles.length === 0) {
                container.innerHTML = '<div style="text-align:center;padding:16px;color:#667085;font-size:13px;">No circles yet. Be the first!</div>';
                return;
            }

            const medals = ['🥇', '🥈', '🥉'];
            let html = '';
            circles.forEach((c, i) => {
                const medal = medals[i] || `#${i + 1}`;
                const isPrivate = !c.is_public;
                const privacyBadge = isPrivate
                    ? '<span style="font-size:10px;background:#F2F4F7;color:#667085;padding:1px 6px;border-radius:4px;margin-left:4px;">🔒 Private</span>'
                    : '<span style="font-size:10px;background:#ECFDF3;color:#027A48;padding:1px 6px;border-radius:4px;margin-left:4px;">🌐 Public</span>';

                let actionBtn = '';
                if (isPrivate) {
                    if (myJoinRequestCircleIds.has(c.id)) {
                        actionBtn = `<span style="font-size:12px;color:#667085;background:#F2F4F7;padding:4px 10px;border-radius:6px;">⏳ Pending</span>`;
                    } else {
                        actionBtn = `<button class="ds-btn ds-btn--secondary ds-btn--sm" onclick="handleRequestJoinCircle('${c.id}')" style="font-size:12px;">🔒 Request</button>`;
                    }
                } else {
                    actionBtn = `<button class="ds-btn ds-btn--primary ds-btn--sm" onclick="handleJoinCircle('${c.id}')" style="font-size:12px;">Join</button>`;
                }

                html += `
                <div class="circle-lb-item" style="display:flex;align-items:center;gap:8px;padding:10px 0;border-bottom:1px solid var(--card-border-color);">
                    <span style="font-size:18px;min-width:28px;text-align:center;">${medal}</span>
                    <span style="font-size:18px;">${c.avatar_emoji || '🟢'}</span>
                    <div style="flex:1;">
                        <div style="font-size:14px;font-weight:600;color:var(--text-primary);">${c.name} ${privacyBadge}</div>
                        <div style="font-size:11px;color:#667085;">${c.member_count} members · Lv.${c.level}</div>
                    </div>
                    <div style="display:flex;flex-direction:column;align-items:flex-end;gap:4px;">
                        <span style="font-size:14px;font-weight:700;color:var(--primary-color);">${(c.total_xp || 0).toLocaleString()} XP</span>
                        ${actionBtn}
                    </div>
                </div>`;
            });
            container.innerHTML = html;
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
            let html = '';
            for (const inv of invites) {
                html += `
                <div style="display:flex;align-items:center;justify-content:space-between;padding:12px 0;border-bottom:1px solid var(--card-border-color);">
                    <div>
                        <div style="font-size:14px;font-weight:500;color:#101828;">Circle invite from #${(inv.inviter_id || '').substring(0, 6)}</div>
                        <div style="font-size:12px;color:#667085;">Expires ${new Date(inv.expires_at).toLocaleDateString()}</div>
                    </div>
                    <div style="display:flex;gap:8px;">
                        <button class="ds-btn ds-btn--primary ds-btn--sm" onclick="handleAcceptInvite('${inv.id}')">Accept</button>
                        <button class="ds-btn ds-btn--secondary ds-btn--sm" onclick="handleDeclineInvite('${inv.id}')">Decline</button>
                    </div>
                </div>`;
            }
            container.innerHTML = html;
        } catch (e) {
            console.error('Failed to load invites', e);
        }
    }

    // ─── Actions ─────────────────────────────────────────────────

    window.handleCreateCircle = async function () {
        const name = document.getElementById('circle-name-input').value.trim();
        if (!name) return alert('Please enter a circle name');

        const desc = document.getElementById('circle-desc-input').value.trim();
        const emoji = document.getElementById('circle-emoji-input').value.trim() || '🟢';

        try {
            const res = await fetch('/api/community/circles', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name, description: desc || null, emoji })
            });
            if (!res.ok) {
                const err = await res.text();
                throw new Error(err);
            }
            document.getElementById('create-circle-modal').style.display = 'none';
            loadAll();
        } catch (e) {
            alert('Failed to create circle: ' + e.message);
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
            alert('Failed to leave circle: ' + e.message);
        }
    };

    window.handleAcceptInvite = async function (inviteId) {
        try {
            const res = await fetch(`/api/community/invites/${inviteId}/accept`, { method: 'POST' });
            if (!res.ok) throw new Error(await res.text());
            loadAll();
        } catch (e) {
            alert('Failed: ' + e.message);
        }
    };

    window.handleDeclineInvite = async function (inviteId) {
        try {
            const res = await fetch(`/api/community/invites/${inviteId}/decline`, { method: 'POST' });
            if (!res.ok) throw new Error(await res.text());
            loadAll();
        } catch (e) {
            alert('Failed: ' + e.message);
        }
    };

    window.openCircleSettings = async function () {
        try {
            const res = await fetch('/api/community/circles/me', { credentials: 'same-origin' });
            if (!res.ok) { alert('Could not load circle data.'); return; }
            const data = await res.json();
            if (!data.circle) { alert('You are not in a circle.'); return; }

            const c = data.circle;
            window._currentCircleId = c.id;

            // Pre-fill fields
            document.getElementById('settings-circle-name').value = c.name || '';
            document.getElementById('settings-circle-desc').value = c.description || '';
            document.getElementById('settings-circle-emoji').value = c.avatar_emoji || '🟢';
            
            // Set toggle state
            const isPublic = !!c.is_public;
            const checkbox = document.getElementById('settings-circle-public');
            checkbox.checked = isPublic;
            const track = document.getElementById('settings-toggle-track');
            track.style.backgroundColor = isPublic ? '#0000FF' : '#D0D5DD';
            const knob = track.querySelector('span');
            if (knob) knob.style.transform = isPublic ? 'translateX(20px)' : 'translateX(0)';

            // Show modal
            document.getElementById('circle-settings-modal').style.display = 'flex';
        } catch (e) {
            console.error('Failed to open circle settings', e);
            alert('Error loading settings: ' + e.message);
        }
    };

    window.handleSaveCircleSettings = async function () {
        const circleId = window._currentCircleId;
        if (!circleId) { alert('No circle selected'); return; }

        const name = document.getElementById('settings-circle-name').value.trim();
        if (!name) { alert('Circle name is required.'); return; }

        const description = document.getElementById('settings-circle-desc').value.trim();
        const emoji = document.getElementById('settings-circle-emoji').value.trim() || '🟢';
        const isPublic = document.getElementById('settings-circle-public').checked;

        const saveBtn = document.getElementById('settings-save-btn');
        const originalText = saveBtn.textContent;
        saveBtn.textContent = 'Saving...';
        saveBtn.disabled = true;

        try {
            // Update name/description/emoji
            const updateRes = await fetch(`/api/community/circles/${circleId}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                credentials: 'same-origin',
                body: JSON.stringify({ name, description: description || null, emoji })
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

            // Close modal and reload data
            document.getElementById('circle-settings-modal').style.display = 'none';
            if (typeof window.loadCirclesAndXp === 'function') window.loadCirclesAndXp();
        } catch (e) {
            alert('Failed to save settings: ' + e.message);
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
            document.getElementById('circle-settings-modal').style.display = 'none';
            if (typeof window.loadCirclesAndXp === 'function') window.loadCirclesAndXp();
        } catch (e) {
            alert('Failed to delete circle: ' + e.message);
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

    // ─── Level-Up Animation ──────────────────────────────────────

    function showLevelUpAnimation(level, name) {
        const overlay = document.createElement('div');
        overlay.innerHTML = `
        <div style="position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.7);z-index:9999;display:flex;align-items:center;justify-content:center;animation:fadeIn 0.3s ease;">
            <div style="background:linear-gradient(135deg,#0a0b2e,#1a1b4b);border-radius:24px;padding:48px;text-align:center;max-width:400px;animation:bounceIn 0.5s ease;">
                <div style="font-size:64px;margin-bottom:16px;animation:float 2s ease-in-out infinite;">🎉</div>
                <h2 style="font-size:28px;font-weight:700;color:#fff;margin:0 0 8px;">Level Up!</h2>
                <p style="font-size:18px;color:#03FF88;font-weight:600;margin:0 0 8px;">Level ${level} — ${name}</p>
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

            let html = '';
            for (const req of requests) {
                const date = new Date(req.created_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
                html += `
                <div style="display:flex;align-items:center;justify-content:space-between;padding:12px 0;border-bottom:1px solid var(--card-border-color);">
                    <div style="display:flex;align-items:center;gap:10px;">
                        <div style="width:36px;height:36px;border-radius:50%;background:#EEF4FF;display:flex;align-items:center;justify-content:center;font-size:14px;font-weight:600;color:#2E90FA;">
                            ${(req.user_name || 'U').charAt(0).toUpperCase()}
                        </div>
                        <div>
                            <div style="font-size:14px;font-weight:500;color:#101828;">${req.user_name || 'Unknown User'}</div>
                            <div style="font-size:12px;color:#667085;">Requested ${date}</div>
                        </div>
                    </div>
                    <div style="display:flex;gap:8px;">
                        <button class="ds-btn ds-btn--primary ds-btn--sm" onclick="handleApproveRequest('${req.id}')">✓ Approve</button>
                        <button class="ds-btn ds-btn--secondary ds-btn--sm" onclick="handleDeclineRequest('${req.id}')" style="color:#F04438;">✗ Decline</button>
                    </div>
                </div>`;
            }
            container.innerHTML = html;
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
            alert('Failed to approve: ' + e.message);
        }
    };

    window.handleDeclineRequest = async function (requestId) {
        try {
            const res = await fetch(`/api/community/circles/requests/${requestId}/decline`, { method: 'POST' });
            if (!res.ok) throw new Error(await res.text());
            loadAll();
        } catch (e) {
            alert('Failed to decline: ' + e.message);
        }
    };

    // ─── Init ────────────────────────────────────────────────────

    function loadAll() {
        loadXpSummary();
        loadMyCircle();
        loadXpHistory();
        loadCircleLeaderboard();
        loadPendingInvites();
        loadPendingJoinRequests();
    }


    // Only load when the tab becomes visible
    const circleTabBtn = document.querySelector('[data-tab="community-circle-tab"]');
    if (circleTabBtn) {
        circleTabBtn.addEventListener('click', function () {
            loadAll();
        });
    }

    // Also expose for other scripts
    window.loadCirclesAndXp = loadAll;
    window.showLevelUpAnimation = showLevelUpAnimation;
});
