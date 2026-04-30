/**
 * Affiliate Dashboard Controller
 * Fetches dashboard data and renders UI elements.
 */

(function () {
    'use strict';

    let dashboardData = null;

    function getCsrfToken() {
        if (typeof window.getCsrfToken === 'function') {
            return window.getCsrfToken() || '';
        }
        const value = `; ${document.cookie}`;
        const parts = value.split('; csrf_token=');
        return parts.length === 2 ? decodeURIComponent(parts.pop().split(';').shift()) : '';
    }

    function csrfHeaders(headers = {}) {
        const token = getCsrfToken();
        return token ? { ...headers, 'X-CSRF-Token': token } : headers;
    }

    function setStatus(message, type = 'info') {
        const el = document.getElementById('affiliate-action-status');
        if (!el) return;
        el.textContent = message || '';
        el.dataset.status = type;
    }

    function clearChildren(el) {
        while (el.firstChild) {
            el.removeChild(el.firstChild);
        }
    }

    function centsToCurrency(cents) {
        return formatCurrency((Number(cents) || 0) / 100);
    }

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
            setStatus('Could not load affiliate dashboard data. Please refresh and try again.', 'error');
        }
    }

    function renderDashboard() {
        if (!dashboardData) return;

        // 1. Profile & Tier
        const tierNameEl = document.getElementById('tier-name');
        const rawBps = dashboardData.commission_rate_bps || 0;
        const tierRateEl = document.getElementById('tier-rate');
        if (tierNameEl) tierNameEl.textContent = dashboardData.current_tier || 'Partner';
        if (tierRateEl) tierRateEl.textContent = `${rawBps} bps (${(rawBps / 100).toFixed(2)}%)`;

        // 2. Earnings KPI
        const earnings = dashboardData.earnings || {};
        const total = ((earnings.provisional_cents || 0) + (earnings.on_hold_cents || 0) + (earnings.payable_cents || 0) + (earnings.paid_cents || 0)) / 100;
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
            btn.textContent = 'Request Payout';
            btn.onclick = async () => {
                const prev = btn.textContent;
                btn.textContent = 'Requesting...';
                btn.disabled = true;
                setStatus('Submitting payout request...', 'info');
                try {
                    const res = await fetch('/api/affiliate/payout/request', { method: 'POST', headers: csrfHeaders() });
                    if (res.ok) {
                        const data = await res.json().catch(() => ({}));
                        btn.textContent = 'Request Logged';
                        setStatus(data.message || 'Your payout request has been logged for admin review.', 'success');
                    } else {
                        const err = await res.json().catch(() => ({}));
                        setStatus(err.error || 'Could not request payout.', 'error');
                        btn.disabled = false;
                        btn.textContent = prev;
                    }
                } catch (e) {
                    setStatus('Network error while requesting payout.', 'error');
                    btn.disabled = false;
                    btn.textContent = prev;
                }
            };
        } else {
            btn.disabled = true;
            btn.textContent = `Need $${(50 - payable).toFixed(2)} to Payout`;
            btn.onclick = null;
        }

        // 3. Funnel Metrics
        const refs = dashboardData.referrals || {};
        const clicks = dashboardData.clicks ?? dashboardData.referral_clicks ?? 0;
        const signups = (refs.registered || 0) + (refs.under_holdback || 0) + (refs.qualified || 0);
        
        document.getElementById('f-clicks').textContent = clicks.toLocaleString();
        document.getElementById('f-registered').textContent = signups.toLocaleString();
        document.getElementById('f-holdback').textContent = (refs.under_holdback || 0).toLocaleString();
        document.getElementById('f-qualified').textContent = (refs.qualified || 0).toLocaleString();

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
            clearChildren(list);
            const empty = document.createElement('li');
            empty.className = 'empty-activity';
            empty.textContent = 'No recent commissions';
            list.appendChild(empty);
        } else {
            renderRecentCommissions(list, recent);
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

    function renderRecentCommissions(list, commissions) {
        clearChildren(list);
        commissions.forEach((commission) => {
            const item = document.createElement('li');
            item.className = 'activity-item';

            const left = document.createElement('div');
            left.className = 'activity-left';

            const amount = document.createElement('span');
            amount.className = 'activity-amount';
            amount.textContent = centsToCurrency(commission.amount_cents);

            const status = String(commission.status || 'unknown').toLowerCase();
            const statusEl = document.createElement('span');
            statusEl.className = `activity-status status-${status.replace(/[^a-z0-9_-]/g, '') || 'unknown'}`;
            statusEl.textContent = status;

            const right = document.createElement('div');
            right.className = 'activity-right';

            const date = document.createElement('span');
            date.className = 'activity-date';
            const parsedDate = new Date(commission.created_at);
            date.textContent = Number.isNaN(parsedDate.getTime()) ? 'Unknown date' : parsedDate.toLocaleDateString();

            left.append(amount, statusEl);
            right.appendChild(date);
            item.append(left, right);
            list.appendChild(item);
        });
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
        clearChildren(markers);
        tiers.forEach((t) => {
            const pct = Math.min(100, Math.round((t.min_qualified_referrals / maxThreshold) * 100));
            const isCurrent = t.tier === currentTierName;
            const isPast = t.min_qualified_referrals <= qualifiedCount;
            const marker = document.createElement('div');
            marker.className = `tier-marker ${isCurrent ? 'tier-marker--active' : ''} ${isPast ? 'tier-marker--past' : ''}`.trim();
            marker.style.left = `${pct}%`;
            marker.title = `${t.tier}: ${t.min_qualified_referrals} qualified referrals (${t.commission_rate_bps} bps)`;

            const dot = document.createElement('span');
            dot.className = 'tier-marker-dot';
            const label = document.createElement('span');
            label.className = 'tier-marker-label';
            label.textContent = t.tier;

            marker.append(dot, label);
            markers.appendChild(marker);
        });

        // Update hint text
        if (hintEl) {
            if (nextTier) {
                const needed = nextThreshold - qualifiedCount;
                hintEl.textContent = `${qualifiedCount} qualified referral${qualifiedCount !== 1 ? 's' : ''} · ${needed} more to reach ${nextTier.tier} (${nextTier.commission_rate_bps} bps)`;
            } else {
                hintEl.textContent = `Highest tier reached: ${currentTierName} (${dashboardData.commission_rate_bps} bps).`;
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
        const copy = document.createElement('div');
        const strong = document.createElement('strong');
        strong.textContent = 'Action Required:';
        copy.append(strong, ` Our affiliate policies have been updated (v${version || 'current'}). Please review and re-accept to continue accessing your dashboard.`);
        const link = document.createElement('a');
        link.href = '/affiliate/settings?reaccept=1';
        link.style.cssText = 'background:#fff;color:#ff6b35;padding:8px 16px;border-radius:4px;font-weight:600;text-decoration:none;white-space:nowrap;margin-left:16px;';
        link.textContent = 'Review & Accept';
        banner.append(copy, link);
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
            setStatus('Referral link copied.', 'success');
            setTimeout(() => {
                btn.innerHTML = originalHtml;
                btn.classList.remove('btn-success');
            }, 2000);
        }).catch(() => {
            setStatus('Clipboard permission denied. Select the referral link and copy it manually.', 'error');
        });
    });

    // Set postback url
    document.getElementById('save-postback-btn')?.addEventListener('click', async () => {
        const btn = document.getElementById('save-postback-btn');
        const input = document.getElementById('postback-url-input');
        if (!input) return;
        
        btn.disabled = true;
        btn.textContent = 'Saving...';
        setStatus('Saving postback URL...', 'info');
        
        try {
            const res = await fetch('/api/affiliate/postback', {
                method: 'POST',
                headers: csrfHeaders({ 'Content-Type': 'application/json' }),
                body: JSON.stringify({ postback_url: input.value })
            });

            if (res.ok) {
                btn.textContent = 'Saved!';
                setStatus('Postback URL saved.', 'success');
                setTimeout(() => { btn.textContent = 'Save'; btn.disabled = false; }, 2000);
            } else {
                const err = await res.json().catch(() => ({}));
                setStatus(err.error || 'Could not save postback URL.', 'error');
                btn.textContent = 'Save';
                btn.disabled = false;
            }
        } catch (e) {
            setStatus('Network error while saving postback URL.', 'error');
            btn.textContent = 'Save';
            btn.disabled = false;
        }
    });

    function renderSubIDEmpty(tbody, message) {
        clearChildren(tbody);
        const row = document.createElement('tr');
        const cell = document.createElement('td');
        cell.colSpan = 5;
        cell.style.cssText = 'padding: 24px; text-align: center; color: var(--admin-text-muted);';
        cell.textContent = message;
        row.appendChild(cell);
        tbody.appendChild(row);
    }

    function appendTextCell(row, text, cssText) {
        const cell = document.createElement('td');
        cell.style.cssText = cssText;
        cell.textContent = text;
        row.appendChild(cell);
        return cell;
    }

    function renderSubIDStats(tbody, stats) {
        clearChildren(tbody);
        stats.forEach((stat) => {
            const row = document.createElement('tr');
            row.style.borderBottom = '1px solid var(--admin-border)';

            const subIdCell = document.createElement('td');
            subIdCell.style.cssText = 'padding: 12px; font-weight: 500; color: var(--admin-text-primary);';
            const code = document.createElement('code');
            code.style.cssText = 'background: var(--admin-bg); padding: 2px 6px; border-radius: 4px;';
            code.textContent = stat.sub_id || 'unknown';
            subIdCell.appendChild(code);
            row.appendChild(subIdCell);

            appendTextCell(row, (stat.clicks || 0).toLocaleString(), 'padding: 12px; color: var(--admin-text-secondary);');
            appendTextCell(row, (stat.registrations || 0).toLocaleString(), 'padding: 12px; color: var(--admin-success); font-weight: 600;');
            appendTextCell(row, centsToCurrency(stat.earned_cents), 'padding: 12px; color: var(--admin-success); font-weight: 700;');
            appendTextCell(row, centsToCurrency(stat.pending_cents), 'padding: 12px; color: var(--admin-text-muted);');
            tbody.appendChild(row);
        });
    }

    async function loadSubIDStats() {
        try {
            const res = await fetch('/api/affiliate/subid-stats');
            const tbody = document.getElementById('subid-stats-body');
            if (!tbody) return;
            if (res.ok) {
                const data = await res.json();
                if (!data.stats || data.stats.length === 0) {
                    renderSubIDEmpty(tbody, 'No tracked SubIDs yet.');
                    return;
                }

                data.stats.sort((a, b) => (b.earned_cents || 0) - (a.earned_cents || 0));
                renderSubIDStats(tbody, data.stats);
            } else {
                renderSubIDEmpty(tbody, 'Could not load SubID stats.');
            }
        } catch (e) {
            console.error('Failed to load subID stats:', e);
            const tbody = document.getElementById('subid-stats-body');
            if (tbody) renderSubIDEmpty(tbody, 'Could not load SubID stats.');
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
