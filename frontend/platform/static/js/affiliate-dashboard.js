/**
 * Affiliate Dashboard Controller
 * Fetches dashboard data and renders UI elements.
 */

(function () {
    'use strict';

    let dashboardData = null;

    async function loadDashboard() {
        try {
            const res = await fetch('/api/affiliate/dashboard');
            if (!res.ok) {
                if (res.status === 403 || res.status === 401) {
                    window.location.href = '/affiliate/onboarding';
                    return;
                }
                throw new Error('Failed to load dashboard data');
            }

            dashboardData = await res.json();
            
            // Pending applicants stay in the affiliate flow until admin approval.
            if (dashboardData.status === 'pending_approval') {
                renderPendingReviewState();
                return;
            }

            renderDashboard();
        } catch (err) {
            console.error('Affiliate Dashboard Load Error:', err);
            // Show error state gracefully
        }
    }

    function renderDashboard() {
        if (!dashboardData) return;

        // 1. Profile & Tier
        document.getElementById('tier-name').textContent = dashboardData.current_tier || 'Partner';
        const rawBps = dashboardData.commission_rate_bps || 0;
        document.getElementById('tier-rate').textContent = `${rawBps} bps (${(rawBps / 100).toFixed(2)}%)`;

        // 2. Earnings KPI
        const earnings = dashboardData.earnings || {};
        const total = (earnings.provisional_cents + earnings.on_hold_cents + earnings.payable_cents + earnings.paid_cents) / 100;
        const paid = (earnings.paid_cents || 0) / 100;
        const payable = (earnings.payable_cents || 0) / 100;
        const provisional = (earnings.provisional_cents || 0) / 100;

        document.getElementById('kpi-total-earnings').textContent = formatCurrency(total);
        document.getElementById('kpi-paid-earnings').textContent = formatCurrency(paid);
        document.getElementById('kpi-payable-earnings').textContent = formatCurrency(payable);
        document.getElementById('kpi-provisional-earnings').textContent = formatCurrency(provisional);

        const btn = document.getElementById('request-payout-btn');
        if (payable >= 50) {
            btn.disabled = false;
            btn.innerHTML = 'Request Payout';
            btn.onclick = async () => {
                const prev = btn.innerHTML;
                btn.innerHTML = 'Requesting...';
                btn.disabled = true;
                try {
                    const res = await fetch('/api/affiliate/payout/request', { method: 'POST' });
                    if (res.ok) {
                        btn.innerHTML = 'Admin Notified!';
                        alert('Your payout request has been sent to our admin team. They will review and execute the batch payout shortly.');
                    } else {
                        const err = await res.json();
                        alert('Error: ' + (err.error || 'Could not request payout'));
                        btn.disabled = false;
                        btn.innerHTML = prev;
                    }
                } catch (e) {
                    alert('Network error');
                    btn.disabled = false;
                    btn.innerHTML = prev;
                }
            };
        } else {
            btn.disabled = true;
            btn.innerHTML = `Need $${(50 - payable).toFixed(2)} to Payout`;
            btn.onclick = null;
        }

        // 3. Funnel Metrics
        const refs = dashboardData.referrals || {};
        const clicks = dashboardData.referral_clicks || 0;
        const signups = refs.registered + refs.under_holdback + refs.qualified; 
        
        document.getElementById('f-clicks').textContent = clicks.toLocaleString();
        document.getElementById('f-registered').textContent = signups.toLocaleString();
        document.getElementById('f-holdback').textContent = refs.under_holdback.toLocaleString();
        document.getElementById('f-qualified').textContent = refs.qualified.toLocaleString();

        // 4. Referral Link & QR Code
        const refUrl = dashboardData.referral_url || '';
        document.getElementById('referral-url-input').value = refUrl;
        updateDynamicLink();
        
        // Postback Setup
        const pbInput = document.getElementById('postback-url-input');
        if (pbInput && dashboardData.postback_url) {
            pbInput.value = dashboardData.postback_url;
        }

        // 5. Recent Commissions
        const list = document.getElementById('recent-commissions-list');
        const recent = dashboardData.recent_commissions || [];

        if (recent.length === 0) {
            list.innerHTML = '<li class="empty-activity">No recent commissions</li>';
        } else {
            list.innerHTML = recent.map(c => `
                <li class="activity-item">
                    <div class="activity-left">
                        <span class="activity-amount">${formatCurrency((c.amount_cents || 0) / 100)}</span>
                        <span class="activity-status status-${c.status || 'unknown'}">${c.status}</span>
                    </div>
                    <div class="activity-right">
                        <span class="activity-date">${new Date(c.created_at).toLocaleDateString()}</span>
                    </div>
                </li>
            `).join('');
        }

        // 6. GAP-12: Tier Progression Visual
        renderTierProgression();

        // 7. GAP-08: Policy re-acceptance banner
        if (dashboardData.policy_reacceptance_required) {
            showPolicyReacceptanceBanner(dashboardData.current_policy_version);
        }
    }

    function renderPendingReviewState() {
        const container = document.querySelector('.affiliate-dashboard-container');
        if (!container) return;

        container.innerHTML = `
            <section class="affiliate-review-state ds-card" aria-labelledby="affiliate-review-title">
                <div class="affiliate-review-icon" aria-hidden="true">
                    <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <circle cx="12" cy="12" r="10"></circle>
                        <path d="M12 6v6l4 2"></path>
                    </svg>
                </div>
                <p class="affiliate-review-eyebrow">Application submitted</p>
                <h1 id="affiliate-review-title" class="affiliate-review-title">Your affiliate application is under review</h1>
                <p class="affiliate-review-copy">
                    Thanks for applying to the POOOL Partner Syndicate. An admin needs to review and approve your application before your referral link, materials, payout settings, and reporting tools unlock.
                </p>
                <div class="affiliate-review-steps" aria-label="Affiliate approval progress">
                    <div class="affiliate-review-step affiliate-review-step--done">
                        <span class="affiliate-review-dot"></span>
                        <div>
                            <strong>Application received</strong>
                            <span>Your profile, tax details, agreements, and exam answers were submitted.</span>
                        </div>
                    </div>
                    <div class="affiliate-review-step affiliate-review-step--active">
                        <span class="affiliate-review-dot"></span>
                        <div>
                            <strong>Admin review</strong>
                            <span>We verify fit, compliance posture, and KYC before activation.</span>
                        </div>
                    </div>
                    <div class="affiliate-review-step">
                        <span class="affiliate-review-dot"></span>
                        <div>
                            <strong>Full affiliate access</strong>
                            <span>After approval, this page becomes your affiliate dashboard.</span>
                        </div>
                    </div>
                </div>
                <div class="affiliate-review-actions">
                    <a href="/affiliate/onboarding" class="ds-btn ds-btn--secondary">Review Application</a>
                    <a href="/support" class="ds-btn ds-btn--ghost">Contact Support</a>
                </div>
            </section>
        `;
    }

    /**
     * GAP-12: Renders the tier progression bar showing the affiliate's position
     * on the tier ladder and how many more qualified referrals are needed for the next tier.
     */
    function renderTierProgression() {
        const tiers = dashboardData.tier_thresholds || [];
        const qualifiedCount = (dashboardData.referrals || {}).qualified || 0;
        const currentTierName = dashboardData.current_tier || 'Access';

        const fill = document.getElementById('tier-progress-fill');
        const markers = document.getElementById('tier-markers');
        const hintEl = document.querySelector('.tier-hint');

        if (!fill || !markers || tiers.length === 0) return;

        // Find current tier index and next tier
        const currentIdx = tiers.findIndex(t => t.tier === currentTierName);
        const nextTier = tiers[currentIdx + 1] || null;

        // Calculate progress within current tier band
        const currentThreshold = tiers[currentIdx]?.min_qualified_referrals ?? 0;
        const nextThreshold = nextTier?.min_qualified_referrals ?? currentThreshold;

        let progressPct = 100; // Default: top tier
        if (nextTier) {
            const band = nextThreshold - currentThreshold;
            const progress = qualifiedCount - currentThreshold;
            progressPct = band > 0 ? Math.min(100, Math.round((progress / band) * 100)) : 100;
        }

        fill.style.width = progressPct + '%';

        // Build tier markers along the progress bar
        const maxThreshold = tiers[tiers.length - 1].min_qualified_referrals || 1;
        markers.innerHTML = tiers.map(t => {
            const pct = Math.min(100, Math.round((t.min_qualified_referrals / maxThreshold) * 100));
            const isCurrent = t.tier === currentTierName;
            const isPast = t.min_qualified_referrals <= qualifiedCount;
            return `
                <div class="tier-marker ${isCurrent ? 'tier-marker--active' : ''} ${isPast ? 'tier-marker--past' : ''}"
                     style="left: ${pct}%"
                     title="${t.tier}: ${t.min_qualified_referrals} qualified referrals (${t.commission_rate_bps} bps)">
                    <span class="tier-marker-dot"></span>
                    <span class="tier-marker-label">${t.tier}</span>
                </div>
            `;
        }).join('');

        // Update hint text
        if (hintEl) {
            if (nextTier) {
                const needed = nextThreshold - qualifiedCount;
                hintEl.textContent = `${qualifiedCount} qualified referral${qualifiedCount !== 1 ? 's' : ''} · ${needed} more to reach ${nextTier.tier} (${nextTier.commission_rate_bps} bps)`;
            } else {
                hintEl.textContent = `🏆 You've reached the highest tier: ${currentTierName} (${dashboardData.commission_rate_bps} bps). Congratulations!`;
            }
        }
    }

    /**
     * GAP-08: Shows a blocking banner when the affiliate must re-accept updated policies.
     */
    function showPolicyReacceptanceBanner(version) {
        const existing = document.getElementById('policy-reaccept-banner');
        if (existing) return; // Already shown

        const banner = document.createElement('div');
        banner.id = 'policy-reaccept-banner';
        banner.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:9999;background:#ff6b35;color:#fff;padding:16px 24px;display:flex;align-items:center;justify-content:space-between;box-shadow:0 2px 8px rgba(0,0,0,0.2);';
        banner.innerHTML = `
            <div>
                <strong>Action Required:</strong> Our affiliate policies have been updated (v${version}).
                Please review and re-accept to continue accessing your dashboard.
            </div>
            <a href="/affiliate/settings?reaccept=1" style="background:#fff;color:#ff6b35;padding:8px 16px;border-radius:4px;font-weight:600;text-decoration:none;white-space:nowrap;margin-left:16px;">
                Review &amp; Accept
            </a>
        `;
        document.body.prepend(banner);
    }

    function formatCurrency(num) {
        return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(num);
    }

    // Dynamic Link Generator
    function updateDynamicLink() {
        if (!dashboardData || !dashboardData.referral_url) return;
        
        let urlObj;
        try {
            urlObj = new URL(dashboardData.referral_url);
        } catch (e) {
            return;
        }

        const subid = document.getElementById('link-gen-subid')?.value.trim();
        const utm = document.getElementById('link-gen-utm')?.value.trim();

        if (subid) urlObj.searchParams.set('subid', subid);
        if (utm) urlObj.searchParams.set('utm_source', utm);

        const newUrl = urlObj.toString();
        const inputEl = document.getElementById('referral-url-input');
        if (inputEl) inputEl.value = newUrl;

        // Update QR Code
        const qrcodeContainer = document.getElementById('qrcode');
        if (qrcodeContainer) {
            qrcodeContainer.innerHTML = '';
            if (typeof qrcode !== 'undefined') {
                const qr = qrcode(0, 'L');
                qr.addData(newUrl);
                qr.make();
                qrcodeContainer.innerHTML = qr.createSvgTag({ scalable: true, margin: 0 });
            }
        }
    }

    // Copy to clipboard
    document.getElementById('copy-ref-btn')?.addEventListener('click', () => {
        const input = document.getElementById('referral-url-input');
        input.select();
        input.setSelectionRange(0, 99999);
        navigator.clipboard.writeText(input.value).then(() => {
            const btn = document.getElementById('copy-ref-btn');
            const originalHtml = btn.innerHTML;
            btn.innerHTML = 'Copied!';
            btn.classList.add('btn-success');
            setTimeout(() => {
                btn.innerHTML = originalHtml;
                btn.classList.remove('btn-success');
            }, 2000);
        });
    });

    // Set postback url
    document.getElementById('save-postback-btn')?.addEventListener('click', async () => {
        const btn = document.getElementById('save-postback-btn');
        const input = document.getElementById('postback-url-input');
        if (!input) return;
        
        btn.disabled = true;
        btn.innerHTML = 'Saving...';
        
        try {
            const res = await fetch('/api/affiliate/postback', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ postback_url: input.value })
            });

            if (res.ok) {
                btn.innerHTML = 'Saved!';
                setTimeout(() => { btn.innerHTML = 'Save'; btn.disabled = false; }, 2000);
            } else {
                const err = await res.json();
                alert('Error processing Postback URL: ' + (err.error || 'Unknown Error'));
                btn.innerHTML = 'Save';
                btn.disabled = false;
            }
        } catch (e) {
            alert('Network Error');
            btn.innerHTML = 'Save';
            btn.disabled = false;
        }
    });

    async function loadSubIDStats() {
        try {
            const res = await fetch('/api/affiliate/subid-stats');
            if (res.ok) {
                const data = await res.json();
                const tbody = document.getElementById('subid-stats-body');
                if (!tbody) return;
                
                if (!data.stats || data.stats.length === 0) {
                    tbody.innerHTML = '<tr><td colspan="3" style="padding: 24px; text-align: center; color: var(--admin-text-muted);">No tracked SubIDs yet.</td></tr>';
                    return;
                }

                data.stats.sort((a, b) => (b.earned_cents || 0) - (a.earned_cents || 0)); // Sort by highest earnings

                tbody.innerHTML = data.stats.map(s => `
                    <tr style="border-bottom: 1px solid var(--admin-border);">
                        <td style="padding: 12px; font-weight: 500; color: var(--admin-text-primary);"><code style="background: var(--admin-bg); padding: 2px 6px; border-radius: 4px;">${s.sub_id}</code></td>
                        <td style="padding: 12px; color: var(--admin-text-secondary);">${(s.clicks || 0).toLocaleString()}</td>
                        <td style="padding: 12px; color: var(--admin-success); font-weight: 600;">${(s.registrations || 0).toLocaleString()}</td>
                        <td style="padding: 12px; color: var(--admin-success); font-weight: 700;">${formatCurrency((s.earned_cents || 0) / 100)}</td>
                        <td style="padding: 12px; color: var(--admin-text-muted);">${formatCurrency((s.pending_cents || 0) / 100)}</td>
                    </tr>
                `).join('');
            }
        } catch (e) {
            console.error('Failed to load subID stats:', e);
        }
    }

    // Initialize
    document.addEventListener('DOMContentLoaded', () => {
        document.getElementById('link-gen-subid')?.addEventListener('input', updateDynamicLink);
        document.getElementById('link-gen-utm')?.addEventListener('input', updateDynamicLink);
        loadDashboard();
        loadSubIDStats();
    });
})();
