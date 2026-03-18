/**
 * Settings V2 Controller
 * Vanilla JS logic for the Continuous Scroll & Inline Morphing architecture
 */

/** CSRF token helper for non-SettingsDataService fetch calls */
function getSettingsCsrfToken() {
    const value = `; ${document.cookie}`;
    const parts = value.split(`; csrf_token=`);
    if (parts.length === 2) return parts.pop().split(';').shift();
    return '';
}

/** Global utility: Escape HTML for search results and display */
function escapeSearchHtml(str) {
    if (!str) return "";
    const div = document.createElement("div");
    div.textContent = str;
    return div.innerHTML;
}

/** Live-preview avatar after file selection */
function previewAvatarEdit(input) {
    if (!input.files || !input.files[0]) return;
    const file = input.files[0];
    if (!file.type.startsWith("image/")) {
        alert("Please select a valid image file (JPG, PNG, WebP, GIF).");
        return;
    }
    const reader = new FileReader();
    reader.onload = function (e) {
        const preview = document.getElementById("edit-avatar-preview");
        if (preview) preview.src = e.target.result;
        // Also update READ view avatar
        const readAvatar = document.querySelector(".settings-read-avatar img");
        if (readAvatar) readAvatar.src = e.target.result;
    };
    reader.readAsDataURL(file);
}
window.previewAvatarEdit = previewAvatarEdit;

document.addEventListener("DOMContentLoaded", () => {
    initTabs();
    initMorphForms();
    initLoadData();
    initSearchOverlay();
});

/**
 * Loads data from the existing Settings Service and populates the Read/Edit views
 */
async function initLoadData() {
    try {
        if (typeof SettingsDataService === "undefined" || !SettingsDataService.getSettings) {
            throw new Error("SettingsDataService not loaded properly");
        }

        document.getElementById('settings-loading-skeleton').classList.remove('hidden');

        const data = await SettingsDataService.getSettings();

        if (!data || data.error) {
            throw new Error(data?.error || "Failed to fetch settings");
        }

        populateProfileData(data);
        populateKycData(data);
        populatePreferenceData(data);
        
        // Security data mapping
        populateSessions(data.active_sessions || []);
        populateOAuth(data.oauth_accounts || []);
        populateConsent({
            terms_version: data.latest_terms_version,
            agreed_at: data.latest_terms_accepted_at
        });
        populateEmailBadge(data.email_verified);
        populate2FABadge(data.totp_enabled);

        // Load payment methods for Financial Profile
        loadPaymentMethods();

        // Load leaderboard preferences
        loadLeaderboardPreferences(data);

        document.getElementById('settings-loading-skeleton').classList.add('hidden');
        document.getElementById('settings-content').classList.remove('hidden');

    } catch (error) {
        console.error("Failed to load settings data", error);
        document.getElementById('settings-loading-skeleton').classList.add('hidden');
        document.getElementById('settings-empty-state').classList.remove('hidden');
        document.getElementById('settings-content').classList.add('hidden');
    }
}

/**
 * Populates the UI for Profile (Identity & Contact)
 */
function populateProfileData(profile) {
    // Determine completeness
    const fields = [
        { key: 'first_name', label: 'First Name' },
        { key: 'last_name', label: 'Last Name' },
        { key: 'phone_number', label: 'Phone' },
        { key: 'avatar_url', label: 'Avatar' },
        { key: 'address_line1', label: 'Address' },
        { key: 'city', label: 'City' },
        { key: 'country_code', label: 'Country' },
        { key: 'date_of_birth', label: 'Birthday' },
        { key: 'nationality', label: 'Nationality' },
        { key: 'email_verified', label: 'Email Verified', check: v => v === true },
        { key: 'status', label: 'KYC Verified', check: v => v === 'verified' || v === 'approved' },
    ];
    let filled = 0;
    const missing = [];
    fields.forEach(f => {
        const val = profile[f.key];
        const isFilled = f.check ? f.check(val) : !!val;
        if (isFilled) { filled++; }
        else { missing.push(f.label); }
    });
    
    const pct = Math.round((filled / fields.length) * 100);
    
    // Update the banner
    const banner = document.getElementById('completeness-banner');
    if (banner) {
        if (pct >= 100) {
            banner.classList.add('hidden');
        } else {
            banner.classList.remove('hidden');
            document.getElementById('completeness-pct').innerText = `${pct}%`;
            document.getElementById('completeness-bar-fill').style.width = `${pct}%`;
            
            const subtitle = document.getElementById('completeness-subtitle');
            if (missing.length > 0) {
                subtitle.innerText = `${missing.length} field${missing.length > 1 ? 's' : ''} remaining to complete your profile.`;
            }
            
            const missingEl = document.getElementById('completeness-missing');
            if (missingEl) {
                missingEl.innerHTML = missing.map(m => 
                    `<span class="settings-completeness-banner__tag"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>${m}</span>`
                ).join('');
            }
        }
    }

    // Core Profile (Read)
    const fullName = `${profile.first_name || ''} ${profile.last_name || ''}`.trim();
    document.getElementById('read-name').innerText = fullName || 'Not provided';
    document.getElementById('read-email').innerText = profile.email || 'Not provided'; // Actually from auth, but using placeholder
    document.getElementById('read-phone').innerText = profile.phone_number || 'Not provided';
    
    const readEmailSecurity = document.getElementById('settings-security-email-text');
    if(readEmailSecurity) readEmailSecurity.innerText = profile.email || 'user@example.com';

    // Core Profile (Edit)
    document.getElementById('edit-first-name').value = profile.first_name || '';
    document.getElementById('edit-last-name').value = profile.last_name || '';

    // Address (Read)
    let addrParts = [profile.address_line1, profile.address_line2, profile.city, profile.state_province, profile.postal_code, profile.country_code].filter(Boolean);
    document.getElementById('read-full-address').innerText = addrParts.length > 0 ? addrParts.join(', ') : 'Not provided';

    // Address (Edit)
    document.getElementById('edit-address-1').value = profile.address_line1 || '';
    document.getElementById('edit-address-2').value = profile.address_line2 || '';
    document.getElementById('edit-city').value = profile.city || '';
    document.getElementById('edit-state').value = profile.state_province || '';
    document.getElementById('edit-postal').value = profile.postal_code || '';

    // Identity Vault (Read)
    document.getElementById('read-dob').innerText = profile.date_of_birth || 'Not provided';
    document.getElementById('read-nationality').innerText = profile.nationality || 'Not provided';
    document.getElementById('read-tax-id').innerText = profile.tax_id ? '•••• ' + profile.tax_id.slice(-4) : 'Not provided'; // Masking

    // Identity Vault (Edit)
    document.getElementById('edit-dob').value = profile.date_of_birth || '';
    document.getElementById('edit-nationality').value = profile.nationality || '';
    document.getElementById('edit-tax-id').value = profile.tax_id || '';
}

function populateKycData(kyc) {
    const badge = document.getElementById('settings-kyc-detail-badge');
    if (!badge) return;
    
    badge.classList.remove('settings-badge--loading');
    
    if (kyc.status === 'verified' || kyc.status === 'approved') {
        badge.innerText = '✓ Verified';
        badge.classList.add('settings-badge--verified');
    } else if (kyc.status === 'pending') {
        badge.innerText = '⧖ Pending Review';
        badge.classList.add('settings-badge--pending');
    } else {
        badge.innerText = 'Action Required';
        badge.classList.add('settings-badge--missing');
    }

    const fmtMoney = (cents) => {
        return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format((cents || 0) / 100);
    };

    // Financial limits logic
    document.getElementById('settings-investment-limit').innerText = fmtMoney(kyc.investment_limit_cents);
    document.getElementById('settings-invested-12m').innerText = fmtMoney(kyc.invested_12m_cents);
    document.getElementById('settings-limit-available').innerText = fmtMoney(kyc.limit_available_cents);
    document.getElementById('settings-tier-name').innerText = kyc.tier_name || 'Basic Tier';

    // Referral Code
    const refEl = document.getElementById('settings-referral-code');
    if (refEl) {
        refEl.textContent = kyc.referral_code || '—';
    }
}

/**
 * Copy the referral code to clipboard with visual feedback
 */
function copyReferralCode() {
    const codeEl = document.getElementById('settings-referral-code');
    const btn = document.getElementById('settings-referral-copy');
    if (!codeEl || !btn) return;

    const code = codeEl.textContent.trim();
    if (!code || code === '—' || code === '...') {
        if (window.showToast) window.showToast('No referral code available', 'error');
        return;
    }

    navigator.clipboard.writeText(code).then(() => {
        btn.classList.add('copied');
        btn.innerHTML = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg> Copied!`;
        if (window.showToast) window.showToast('Referral code copied!', 'success');

        setTimeout(() => {
            btn.classList.remove('copied');
            btn.innerHTML = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg> Copy`;
        }, 2000);
    }).catch(() => {
        // Fallback
        const textarea = document.createElement('textarea');
        textarea.value = code;
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand('copy');
        document.body.removeChild(textarea);
        if (window.showToast) window.showToast('Referral code copied!', 'success');
    });
}
window.copyReferralCode = copyReferralCode;

function populatePreferenceData(prefs) {
    document.getElementById('edit-timezone').value = prefs.timezone || 'America/Los_Angeles';
    document.getElementById('edit-currency').value = prefs.currency || 'USD';
    
    document.getElementById('settings-notify-email').checked = prefs.email_notifications ?? true;
    document.getElementById('settings-notify-push').checked = prefs.push_notifications ?? true;
}

function populateSessions(sessions) {
    const list = document.getElementById('settings-sessions-list');
    if(!sessions || sessions.length === 0) {
        list.innerHTML = `<div style="font-size:13px; color:#535862; padding: 12px 16px; background: #f9fafb; border-radius: 8px;">No active sessions found.</div>`;
        return;
    }
    list.innerHTML = sessions.map((s, i) => {
        const title = escapeHtml(s.device_model || 'Unknown Device');
        const loc = escapeHtml(s.location || 'Unknown Location');
        const ip = s.ip_address ? escapeHtml(s.ip_address) : '';
        const meta = [loc, ip].filter(Boolean).join(' &bull; ');
        const badge = s.is_current ? `<span style="font-size:11px; background:#ecfdf3; color:#067647; padding:2px 8px; border-radius:12px; font-weight:600;">Current</span>` : '';
        const btn = s.is_current ? '' : `<button class="settings-btn settings-btn--secondary" style="font-size:12px; padding:6px 12px; color:#D92D20; border-color:#FDA29B; border-radius:6px; background:#fff;" onclick="revokeSession('${escapeHtml(s.session_id || '')}', ${i})">Revoke</button>`;
        
        return `<div id="session-card-${i}" style="display:flex; justify-content:space-between; align-items:center; padding:16px; background:#f9fafb; border:1px solid #eaecf0; border-radius:8px;">
            <div style="display:flex; gap:12px; align-items:center;">
                <div style="width:36px; height:36px; background:#fff; border:1px solid #eaecf0; border-radius:8px; display:flex; align-items:center; justify-content:center;">
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#475467" stroke-width="2"><rect x="4" y="4" width="16" height="16" rx="2" ry="2"/><rect x="9" y="9" width="6" height="6"/></svg>
                </div>
                <div>
                    <div style="font-weight:600; font-size:14px; color:#101828; display:flex; align-items:center; gap:8px;">
                        ${title}
                        ${badge}
                    </div>
                    <div style="font-size:13px; color:#475467; margin-top:2px;">${meta}</div>
                </div>
            </div>
            ${btn}
        </div>`;
    }).join('');
}

function populateEmailBadge(verified) {
    const badge = document.getElementById('settings-email-verified');
    if (!badge) return;
    badge.classList.remove('settings-badge--loading');
    if (verified) {
        badge.innerText = '✓ Verified';
        badge.classList.add('settings-badge--verified');
    } else {
        badge.innerText = 'Unverified';
        badge.classList.add('settings-badge--missing');
    }
}

function populate2FABadge(enabled) {
    const actions = document.getElementById('settings-2fa-actions');
    if (!actions) return;
    
    if (enabled) {
        actions.innerHTML = `
            <div style="display:flex; align-items:center; gap:12px;">
                <span class="settings-badge settings-badge--success" style="font-size:12px;">✓ Enabled</span>
                <button class="settings-btn settings-btn--secondary" style="color:#D92D20; border-color:#FDA29B;" onclick="disable2FA()">Disable 2FA</button>
            </div>
        `;
    } else {
        actions.innerHTML = `<a href="/auth/2fa/setup" class="settings-btn settings-btn--secondary" style="color:#667085;">Not enabled</a>`;
    }
}

function populateOAuth(oauth) {
    const list = document.getElementById('settings-oauth-list');
    const googleAccount = (oauth || []).find(o => (o.provider || '').toLowerCase() === 'google');
    
    let html = '';
    
    if (googleAccount) {
        // Google is linked — show connected state
        html = `<div style="display:flex; justify-content:space-between; align-items:center; padding:16px; background:#f0fdf4; border:1px solid #bbf7d0; border-radius:8px;">
            <div style="display:flex; gap:12px; align-items:center;">
                <div style="width:36px; height:36px; background:#fff; border:1px solid #eaecf0; border-radius:8px; display:flex; align-items:center; justify-content:center;">
                    <img src="/static/images/social-google.svg" alt="Google" style="width:20px; height:20px;">
                </div>
                <div>
                    <div style="font-weight:600; font-size:14px; color:#101828;">Google</div>
                    <div style="font-size:13px; color:#475467; margin-top:2px;">${googleAccount.provider_email ? escapeHtml(googleAccount.provider_email) : 'Connected'}</div>
                </div>
            </div>
            <span style="display:inline-flex; align-items:center; gap:4px; font-size:12px; font-weight:500; color:#067647; background:#ecfdf3; padding:4px 10px; border-radius:16px; border:1px solid #abefc6;">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20 6L9 17l-5-5"/></svg>
                Connected
            </span>
        </div>`;
    } else {
        // Google is NOT linked — show link button
        html = `<div style="display:flex; justify-content:space-between; align-items:center; padding:16px; background:#f9fafb; border:1px solid #eaecf0; border-radius:8px;">
            <div style="display:flex; gap:12px; align-items:center;">
                <div style="width:36px; height:36px; background:#fff; border:1px solid #eaecf0; border-radius:8px; display:flex; align-items:center; justify-content:center;">
                    <img src="/static/images/social-google.svg" alt="Google" style="width:20px; height:20px;">
                </div>
                <div>
                    <div style="font-weight:600; font-size:14px; color:#101828;">Google</div>
                    <div style="font-size:13px; color:#98a2b3; margin-top:2px;">Not connected</div>
                </div>
            </div>
            <a href="/auth/google" class="settings-btn settings-btn--secondary" style="display:inline-flex; align-items:center; gap:6px; font-size:13px; padding:6px 14px; text-decoration:none;">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>
                Link Account
            </a>
        </div>`;
    }
    
    list.innerHTML = html;
}

function escapeHtml(str) {
    if (!str) return '';
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

function populateConsent(consent) {
    document.getElementById('settings-terms-version').innerText = `Terms Version: ${consent.terms_version || '1.0'}`;
    document.getElementById('settings-terms-date').innerText = `Agreed on: ${new Date(consent.agreed_at || Date.now()).toLocaleDateString()}`;
}

/**
 * Initializes Tab navigation for Settings view
 */
function initTabs() {
    const sections = document.querySelectorAll('.settings-section');
    const tabLinks = document.querySelectorAll('.settings-tab-link');

    if (sections.length === 0 || tabLinks.length === 0) return;

    // Show initial section based on URL hash or default to first
    let activeId = window.location.hash.substring(1) || 'section-identity';
    
    function switchTab(targetId) {
        tabLinks.forEach(l => l.classList.remove('active'));
        sections.forEach(s => s.classList.remove('active'));

        const targetTab = document.querySelector(`.settings-tab-link[href="#${targetId}"]`);
        const targetSection = document.getElementById(targetId);

        if (targetTab && targetSection) {
            targetTab.classList.add('active');
            targetSection.classList.add('active');
            window.history.pushState(null, '', `#${targetId}`);
        }
    }

    switchTab(activeId);

    tabLinks.forEach(link => {
        link.addEventListener('click', (e) => {
            e.preventDefault();
            const targetId = link.getAttribute('href').substring(1);
            switchTab(targetId);
            window.scrollTo({top: 0, behavior: 'smooth'});
        });
    });
}

/**
 * Initializes the Read-to-Edit Morphing logic for cards
 */
function initMorphForms() {
    const morphGroups = document.querySelectorAll('.settings-morph-group');
    
    morphGroups.forEach(group => {
        const editBtn = group.querySelector('.js-morph-edit');
        const cancelBtn = group.querySelector('.js-morph-cancel');
        const saveBtn = group.querySelector('.js-morph-save');
        
        if (editBtn) {
            editBtn.addEventListener('click', () => {
                // Store original values before editing
                const inputs = group.querySelectorAll('input, select');
                inputs.forEach(input => {
                    input.dataset.originalValue = input.value;
                });
                
                // Switch to Edit
                group.classList.remove('active-state-read');
                group.classList.add('active-state-edit');
            });
        }
        
        if (cancelBtn) {
            cancelBtn.addEventListener('click', () => {
                // Revert to original values
                const inputs = group.querySelectorAll('input, select');
                inputs.forEach(input => {
                    if (input.dataset.originalValue !== undefined) {
                        input.value = input.dataset.originalValue;
                    }
                });
                
                group.classList.remove('active-state-edit');
                group.classList.add('active-state-read');
            });
        }
        
        if (saveBtn) {
            saveBtn.addEventListener('click', async () => {
                const originalText = saveBtn.innerText;
                saveBtn.innerText = 'Saving...';
                saveBtn.disabled = true;
                
                try {
                    let payload = {};
                    
                    if(group.id === 'morph-core-profile') {
                        payload = {
                            first_name: document.getElementById('edit-first-name').value,
                            last_name: document.getElementById('edit-last-name').value
                        };
                    } else if (group.id === 'morph-address') {
                        payload = {
                            address_line1: document.getElementById('edit-address-1').value,
                            address_line2: document.getElementById('edit-address-2').value,
                            city: document.getElementById('edit-city').value,
                            state_province: document.getElementById('edit-state').value,
                            postal_code: document.getElementById('edit-postal').value
                        };
                    } else {
                        // Identity vault
                        payload = {
                            date_of_birth: document.getElementById('edit-dob').value,
                            nationality: document.getElementById('edit-nationality').value,
                            tax_id: document.getElementById('edit-tax-id').value
                        };
                    }
                    
                    const result = await SettingsDataService.saveProfile(payload);
                    
                    if (result && result.success !== false) {
                        // Re-sync read views
                        if(group.id === 'morph-core-profile') {
                            document.getElementById('read-name').innerText = `${document.getElementById('edit-first-name').value} ${document.getElementById('edit-last-name').value}`.trim() || 'Not provided';
                        } else if (group.id === 'morph-address') {
                            const parts = [document.getElementById('edit-address-1').value, document.getElementById('edit-city').value, document.getElementById('edit-postal').value].filter(Boolean);
                            document.getElementById('read-full-address').innerText = parts.length > 0 ? parts.join(', ') : 'Not provided';
                        } else {
                            document.getElementById('read-dob').innerText = document.getElementById('edit-dob').value || 'Not provided';
                            document.getElementById('read-nationality').innerText = document.getElementById('edit-nationality').value || 'Not provided';
                            const txid = document.getElementById('edit-tax-id').value;
                            document.getElementById('read-tax-id').innerText = txid ? '•••• ' + txid.slice(-4) : 'Not provided';
                        }
                        showToast('Updated successfully', 'success');
                    } else {
                        showToast(result?.message || 'Failed to save', 'error');
                    }
                } catch(err) {
                    console.error('Save error:', err);
                    showToast('An error occurred while saving', 'error');
                }
                
                // Revert to Read View
                saveBtn.innerText = originalText;
                saveBtn.disabled = false;
                group.classList.remove('active-state-edit');
                group.classList.add('active-state-read');
            });
        }
    });
}

// ═══════════════════════════════════════════════════════════════
// MODAL FUNCTIONS
// ═══════════════════════════════════════════════════════════════

function openChangeEmailModal() {
    const modal = document.getElementById('modal-change-email');
    if (modal) {
        modal.style.display = 'flex';
        const emailInput = document.getElementById('modal-new-email');
        if (emailInput) setTimeout(() => emailInput.focus(), 100);
    }
}

function openChangePasswordModal() {
    const modal = document.getElementById('modal-change-password');
    if (modal) {
        modal.style.display = 'flex';
        const pwInput = document.getElementById('modal-current-password');
        if (pwInput) setTimeout(() => pwInput.focus(), 100);
    }
}

function closeModal(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) {
        modal.style.display = 'none';
        // Clear inputs
        modal.querySelectorAll('input').forEach(i => i.value = '');
        // Remove error messages
        const err = modal.querySelector('.settings-modal__error');
        if (err) err.remove();
    }
}

async function submitChangeEmail() {
    const newEmail = document.getElementById('modal-new-email')?.value;
    const password = document.getElementById('modal-email-password')?.value;
    
    if (!newEmail || !password) {
        showModalError('modal-change-email', 'Please fill in all fields.');
        return;
    }
    
    const result = await SettingsDataService.changeEmail(newEmail, password);
    
    if (result && result.success !== false) {
        closeModal('modal-change-email');
        // Update the display
        const emailEl = document.getElementById('settings-security-email-text');
        if (emailEl) emailEl.innerText = newEmail;
        const readEmail = document.getElementById('read-email');
        if (readEmail) readEmail.innerText = newEmail;
        showToast('Email updated successfully', 'success');
    } else {
        showModalError('modal-change-email', result?.message || 'Failed to change email.');
    }
}

async function submitChangePassword() {
    const current = document.getElementById('modal-current-password')?.value;
    const newPw = document.getElementById('modal-new-password')?.value;
    const confirm = document.getElementById('modal-confirm-password')?.value;
    
    if (!current || !newPw || !confirm) {
        showModalError('modal-change-password', 'Please fill in all fields.');
        return;
    }
    
    if (newPw !== confirm) {
        showModalError('modal-change-password', 'New passwords do not match.');
        return;
    }
    
    if (newPw.length < 8) {
        showModalError('modal-change-password', 'Password must be at least 8 characters.');
        return;
    }
    
    const result = await SettingsDataService.changePassword(current, newPw, confirm);
    
    if (result && result.success !== false) {
        closeModal('modal-change-password');
        showToast('Password changed successfully', 'success');
    } else {
        showModalError('modal-change-password', result?.message || 'Failed to change password.');
    }
}

function showModalError(modalId, message) {
    const modal = document.getElementById(modalId);
    if (!modal) return;
    // Remove existing error
    const existing = modal.querySelector('.settings-modal__error');
    if (existing) existing.remove();
    // Add new error
    const errDiv = document.createElement('div');
    errDiv.className = 'settings-modal__error';
    errDiv.innerText = message;
    const actions = modal.querySelector('.settings-modal__actions');
    if (actions) actions.parentNode.insertBefore(errDiv, actions);
}

// ═══════════════════════════════════════════════════════════════
// SESSION & 2FA MANAGEMENT
// ═══════════════════════════════════════════════════════════════

async function revokeSession(sessionId, cardIndex) {
    if (!await pooolConfirm({ title: 'Revoke session', message: 'This device will be immediately logged out.', confirmText: 'Revoke', type: 'danger' })) return;
    
    try {
        const res = await fetch('/api/settings/sessions/revoke', {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': getSettingsCsrfToken() },
            body: JSON.stringify({ session_id: sessionId })
        });
        
        if (res.ok || res.status === 404) {
            // Remove card from UI
            const card = document.getElementById(`session-card-${cardIndex}`);
            if (card) {
                card.style.transition = 'opacity 0.3s ease, transform 0.3s ease';
                card.style.opacity = '0';
                card.style.transform = 'translateX(20px)';
                setTimeout(() => card.remove(), 300);
            }
            showToast('Session revoked successfully', 'success');
        } else {
            showToast('Failed to revoke session', 'error');
        }
    } catch (err) {
        console.error('Revoke session error:', err);
        showToast('Failed to revoke session', 'error');
    }
}

async function disable2FA() {
    if (!await pooolConfirm({ title: 'Disable Two-Factor Authentication', message: 'This will make your account less secure. Are you sure?', confirmText: 'Disable 2FA', type: 'danger' })) return;
    
    try {
        const result = await SettingsDataService.disable2FA();
        if (result && result.success !== false) {
            populate2FABadge(false);
            const actions = document.getElementById('settings-2fa-actions');
            if (actions) {
                actions.innerHTML = `<a href="/auth/2fa/setup" class="settings-btn settings-btn--primary">Enable 2FA</a>`;
            }
            showToast('2FA has been disabled', 'info');
        } else {
            showToast(result?.message || 'Failed to disable 2FA', 'error');
        }
    } catch (err) {
        console.error('Disable 2FA error:', err);
        showToast('Failed to disable 2FA', 'error');
    }
}

async function exportData() {
    try {
        const result = await SettingsDataService.requestDataExport();
        if (result && result.success !== false) {
            showToast(result.message || 'Data export requested. You will receive an email shortly.', 'success');
        } else {
            showToast(result?.message || 'Failed to request data export', 'error');
        }
    } catch (err) {
        console.error('Export data error:', err);
        showToast('Failed to request data export', 'error');
    }
}

// ═══════════════════════════════════════════════════════════════
// TOAST NOTIFICATIONS
// ═══════════════════════════════════════════════════════════════

function showToast(message, type = 'info') {
  if(window.showPooolToast) {
    window.showPooolToast(null, message, type);
  }
}

// ═══════════════════════════════════════════════════════════════
// INITIALIZATION HELPERS
// ═══════════════════════════════════════════════════════════════

function initSearchOverlay() {
    // Search overlay is initialized in the IIFE below
}

// Close modals on overlay click
document.addEventListener('click', (e) => {
    if (e.target.classList.contains('settings-modal-overlay')) {
        e.target.style.display = 'none';
        // Clear inputs
        e.target.querySelectorAll('input').forEach(i => i.value = '');
    }
});

// ═══════════════════════════════════════════════════════════════
// SEARCH FUNCTIONALITY
// ═══════════════════════════════════════════════════════════════

(function () {
    "use strict";

    // Search index: all searchable items on the settings page
    const SEARCH_INDEX = [
        // Identity & Contact
        { title: "Core Profile", subtitle: "Name, avatar, email, phone", section: "section-identity", group: "Identity & Contact", icon: "user" },
        { title: "Avatar / Profile Photo", subtitle: "Upload your profile picture", section: "section-identity", group: "Identity & Contact", icon: "user" },
        { title: "First Name", subtitle: "Your first name", section: "section-identity", group: "Identity & Contact", icon: "user" },
        { title: "Last Name", subtitle: "Your last name", section: "section-identity", group: "Identity & Contact", icon: "user" },
        { title: "Email Address", subtitle: "Your primary email address", section: "section-identity", group: "Identity & Contact", icon: "user" },
        { title: "Phone Number", subtitle: "Your contact phone number", section: "section-identity", group: "Identity & Contact", icon: "user" },
        { title: "Residential Address", subtitle: "Street, city, state, postal code", section: "section-identity", group: "Identity & Contact", icon: "map" },
        { title: "Address Line 1", subtitle: "Street address", section: "section-identity", group: "Identity & Contact", icon: "map" },
        { title: "City", subtitle: "City of residence", section: "section-identity", group: "Identity & Contact", icon: "map" },
        { title: "State / Province", subtitle: "State or province", section: "section-identity", group: "Identity & Contact", icon: "map" },
        { title: "Postal Code", subtitle: "ZIP or postal code", section: "section-identity", group: "Identity & Contact", icon: "map" },
        { title: "Identity Vault", subtitle: "Date of birth, nationality, tax ID", section: "section-identity", group: "Identity & Contact", icon: "shield" },
        { title: "Date of Birth", subtitle: "Your date of birth", section: "section-identity", group: "Identity & Contact", icon: "shield" },
        { title: "Nationality", subtitle: "Country of citizenship", section: "section-identity", group: "Identity & Contact", icon: "shield" },
        { title: "Tax ID / SSN", subtitle: "Tax identification number", section: "section-identity", group: "Identity & Contact", icon: "shield" },

        // Financial Profile
        { title: "Identity Verification", subtitle: "KYC status and verification", section: "section-financial", group: "Financial Profile", icon: "dollar" },
        { title: "KYC Status", subtitle: "Know Your Customer verification", section: "section-financial", group: "Financial Profile", icon: "dollar" },
        { title: "Current Tier", subtitle: "Your investor tier level", section: "section-financial", group: "Financial Profile", icon: "dollar" },
        { title: "Annual Investment Limit", subtitle: "Maximum annual investment amount", section: "section-financial", group: "Financial Profile", icon: "dollar" },
        { title: "Invested Last 12 Months", subtitle: "Total invested in past year", section: "section-financial", group: "Financial Profile", icon: "dollar" },
        { title: "Limit Available", subtitle: "Remaining investment capacity", section: "section-financial", group: "Financial Profile", icon: "dollar" },
        { title: "Referral Code", subtitle: "Your personal referral code", section: "section-financial", group: "Financial Profile", icon: "dollar" },

        // Security & Access
        { title: "Email Security", subtitle: "Change email, verification status", section: "section-security", group: "Security & Access", icon: "lock" },
        { title: "Password", subtitle: "Change your account password", section: "section-security", group: "Security & Access", icon: "lock" },
        { title: "Two-Factor Authentication", subtitle: "2FA / TOTP setup with Google Authenticator", section: "section-security", group: "Security & Access", icon: "lock" },
        { title: "2FA", subtitle: "Two-factor authentication", section: "section-security", group: "Security & Access", icon: "lock" },
        { title: "Active Sessions", subtitle: "View and manage logged-in devices", section: "section-security", group: "Security & Access", icon: "lock" },
        { title: "Linked Accounts", subtitle: "Google, Facebook OAuth connections", section: "section-security", group: "Security & Access", icon: "lock" },
        { title: "Consent & Terms", subtitle: "Terms of service acceptance status", section: "section-security", group: "Security & Access", icon: "lock" },
        { title: "Danger Zone", subtitle: "Export data, delete account", section: "section-security", group: "Security & Access", icon: "lock" },
        { title: "Delete Account", subtitle: "Permanently delete your account", section: "section-security", group: "Security & Access", icon: "lock" },
        { title: "Export My Data", subtitle: "Download all personal data", section: "section-security", group: "Security & Access", icon: "lock" },

        // Preferences & Alerts
        { title: "Timezone", subtitle: "Your preferred timezone", section: "section-preferences", group: "Preferences & Alerts", icon: "sliders" },
        { title: "Language", subtitle: "Display language preference", section: "section-preferences", group: "Preferences & Alerts", icon: "sliders" },
        { title: "Currency", subtitle: "Preferred display currency", section: "section-preferences", group: "Preferences & Alerts", icon: "sliders" },
        { title: "Email Alerts", subtitle: "Newsletter and tip notifications", section: "section-preferences", group: "Preferences & Alerts", icon: "sliders" },
        { title: "Push Notifications", subtitle: "Browser push notification preferences", section: "section-preferences", group: "Preferences & Alerts", icon: "sliders" },

        // Info & Learning
        { title: "Rate Us", subtitle: "Leave a review", section: "section-info", group: "Info & Learning", icon: "book" },
        { title: "Feedback", subtitle: "Share your thoughts", section: "section-info", group: "Info & Learning", icon: "book" },
        { title: "Refer a Friend", subtitle: "Invite friends and earn rewards", section: "section-info", group: "Info & Learning", icon: "book" },
        { title: "Glossary", subtitle: "Terms and concepts explained", section: "section-info", group: "Info & Learning", icon: "book" },
        { title: "Learn with POOOL", subtitle: "Blog posts and educational content", section: "section-info", group: "Info & Learning", icon: "book" },
    ];

    const ICON_SVGS = {
        user: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>',
        map: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"/><circle cx="12" cy="10" r="3"/></svg>',
        shield: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>',
        dollar: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/></svg>',
        lock: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>',
        sliders: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"/></svg>',
        book: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>',
    };

    let activeIndex = -1;
    let filteredResults = [];

    function openSettingsSearch() {
        const overlay = document.getElementById("settings-search-overlay");
        const input = document.getElementById("settings-search-input");
        if (!overlay) return;

        overlay.classList.add("active");
        document.body.style.overflow = "hidden";

        // Reset state
        input.value = "";
        activeIndex = -1;
        renderSearchResults("");

        setTimeout(() => input.focus(), 100);
    }

    function closeSettingsSearch() {
        const overlay = document.getElementById("settings-search-overlay");
        if (!overlay) return;

        overlay.classList.remove("active");
        document.body.style.overflow = "";
    }

    function highlightText(text, query) {
        if (!query || query.length < 1) return text;
        const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
        const re = new RegExp(`(${escaped})`, "gi");
        return text.replace(re, "<mark>$1</mark>");
    }

    function renderSearchResults(query) {
        const container = document.getElementById("settings-search-results");
        if (!container) return;

        if (!query || query.trim().length === 0) {
            // Show all items grouped
            filteredResults = [...SEARCH_INDEX];
        } else {
            const q = query.toLowerCase().trim();
            filteredResults = SEARCH_INDEX.filter(
                (item) =>
                    item.title.toLowerCase().includes(q) ||
                    item.subtitle.toLowerCase().includes(q) ||
                    item.group.toLowerCase().includes(q)
            );
        }

        if (filteredResults.length === 0) {
            container.innerHTML = `<div class="settings-search-no-results">No results found for "${escapeSearchHtml(query)}"</div>`;
            return;
        }

        // Group results
        const groups = {};
        filteredResults.forEach((item) => {
            if (!groups[item.group]) groups[item.group] = [];
            groups[item.group].push(item);
        });

        let html = "";
        let globalIdx = 0;
        for (const [groupName, items] of Object.entries(groups)) {
            html += `<div class="settings-search-result-group">${escapeSearchHtml(groupName)}</div>`;
            for (const item of items) {
                const isActive = globalIdx === activeIndex ? " active" : "";
                html += `<div class="settings-search-result-item${isActive}" data-idx="${globalIdx}" data-section="${item.section}">
                    <div class="settings-search-result-item__icon">${ICON_SVGS[item.icon] || ICON_SVGS.user}</div>
                    <div class="settings-search-result-item__text">
                        <span class="settings-search-result-item__title">${highlightText(escapeSearchHtml(item.title), query)}</span>
                        <span class="settings-search-result-item__subtitle">${highlightText(escapeSearchHtml(item.subtitle), query)}</span>
                    </div>
                </div>`;
                globalIdx++;
            }
        }

        container.innerHTML = html;

        // Attach click handlers
        container.querySelectorAll(".settings-search-result-item").forEach((el) => {
            el.addEventListener("click", () => {
                const sectionId = el.dataset.section;
                navigateToSection(sectionId);
            });
        });
    }

    function navigateToSection(sectionId) {
        closeSettingsSearch();
        const section = document.getElementById(sectionId);
        if (section) {
            const yOffset = -32;
            const y = section.getBoundingClientRect().top + window.pageYOffset + yOffset;
            window.scrollTo({ top: y, behavior: "smooth" });

            // Briefly highlight the section
            section.style.transition = "box-shadow 0.3s ease";
            section.style.boxShadow = "0 0 0 3px rgba(46, 46, 249, 0.15)";
            section.style.borderRadius = "12px";
            setTimeout(() => {
                section.style.boxShadow = "";
            }, 2000);
        }
    }

    // Event: Input filtering
    document.addEventListener("DOMContentLoaded", () => {
        const input = document.getElementById("settings-search-input");
        if (input) {
            input.addEventListener("input", (e) => {
                activeIndex = -1;
                renderSearchResults(e.target.value);
            });
        }

        // Click outside to close
        const overlay = document.getElementById("settings-search-overlay");
        if (overlay) {
            overlay.addEventListener("click", (e) => {
                if (e.target === overlay) {
                    closeSettingsSearch();
                }
            });
        }
    });

    // Event: Keyboard shortcuts
    document.addEventListener("keydown", (e) => {
        const overlay = document.getElementById("settings-search-overlay");
        const isOpen = overlay && overlay.classList.contains("active");

        // ⌘K / Ctrl+K to open
        if ((e.metaKey || e.ctrlKey) && e.key === "k") {
            e.preventDefault();
            if (isOpen) {
                closeSettingsSearch();
            } else {
                openSettingsSearch();
            }
            return;
        }

        if (!isOpen) return;

        // Escape to close
        if (e.key === "Escape") {
            e.preventDefault();
            closeSettingsSearch();
            return;
        }

        // Arrow navigation
        if (e.key === "ArrowDown") {
            e.preventDefault();
            if (filteredResults.length > 0) {
                activeIndex = (activeIndex + 1) % filteredResults.length;
                updateActiveItem();
            }
            return;
        }

        if (e.key === "ArrowUp") {
            e.preventDefault();
            if (filteredResults.length > 0) {
                activeIndex = activeIndex <= 0 ? filteredResults.length - 1 : activeIndex - 1;
                updateActiveItem();
            }
            return;
        }

        // Enter to select
        if (e.key === "Enter") {
            e.preventDefault();
            if (activeIndex >= 0 && activeIndex < filteredResults.length) {
                navigateToSection(filteredResults[activeIndex].section);
            }
            return;
        }
    });

    function updateActiveItem() {
        const container = document.getElementById("settings-search-results");
        if (!container) return;

        container.querySelectorAll(".settings-search-result-item").forEach((el) => {
            el.classList.remove("active");
        });

        const activeEl = container.querySelector(`[data-idx="${activeIndex}"]`);
        if (activeEl) {
            activeEl.classList.add("active");
            activeEl.scrollIntoView({ block: "nearest", behavior: "smooth" });
        }
    }

    // Expose to global scope for onclick handlers
    window.openSettingsSearch = openSettingsSearch;
    window.closeSettingsSearch = closeSettingsSearch;
})();

/**
 * Load and save leaderboard preferences
 */
function loadLeaderboardPreferences(data) {
    const vEl = document.getElementById('settings-lb-visible');
    const aEl = document.getElementById('settings-lb-avatar');
    const nEl = document.getElementById('settings-lb-display-name');
    if (vEl) vEl.checked = data.lb_visible || false;
    if (aEl) aEl.checked = data.lb_avatar || false;
    if (nEl) nEl.value = data.lb_display_name || '';
}

document.addEventListener('DOMContentLoaded', function() {
    const lbBtn = document.getElementById('save-leaderboard-btn');
    if (lbBtn) {
        lbBtn.addEventListener('click', async function() {
            lbBtn.disabled = true;
            lbBtn.textContent = 'Saving...';
            try {
                const body = {
                    visible: document.getElementById('settings-lb-visible')?.checked || false,
                    show_avatar: document.getElementById('settings-lb-avatar')?.checked || false,
                    display_name: document.getElementById('settings-lb-display-name')?.value || '',
                };
                const result = await SettingsDataService.saveLeaderboard(body);
                if (result && result.success !== false) {
                    showToast('Leaderboard settings saved', 'success');
                } else {
                    showToast(result?.message || 'Failed to save leaderboard settings', 'error');
                }
            } catch (e) {
                showToast('Network error', 'error');
            }
            lbBtn.disabled = false;
            lbBtn.textContent = 'Save Leaderboard Settings';
        });
    }

    const locBtn = document.getElementById('save-localization-btn');
    if (locBtn) {
        locBtn.addEventListener('click', async function() {
            locBtn.disabled = true;
            locBtn.textContent = 'Saving...';
            try {
                const prefsBody = {
                    language: 'en',
                    currency: document.getElementById('edit-currency')?.value || 'USD',
                };
                const notifsBody = {
                    email_notifications: document.getElementById('settings-notify-email')?.checked || false,
                    push_notifications: document.getElementById('settings-notify-push')?.checked || false,
                };
                const profileBody = {
                    timezone: document.getElementById('edit-timezone')?.value || 'America/Los_Angeles'
                };
                
                const r1 = await SettingsDataService.savePreferences(prefsBody);
                const r2 = await SettingsDataService.saveNotifications(notifsBody);
                const r3 = await SettingsDataService.saveProfile(profileBody); // timezone is in profile
                
                if (r1 && r1.success !== false && r2 && r2.success !== false && r3 && r3.success !== false) {
                    showToast('Preferences updated successfully', 'success');
                } else {
                    showToast(r1?.message || r2?.message || r3?.message || 'Failed to save preferences', 'error');
                }
            } catch (e) {
                showToast('Network error', 'error');
            }
            locBtn.disabled = false;
            locBtn.textContent = 'Update Preferences';
        });
    }
});

/**
 * Load and display user payment methods via API
 */
async function loadPaymentMethods() {
    const container = document.getElementById('settings-payment-methods-list');
    if (!container) return;

    try {
        const resp = await fetch('/api/payment-methods');
        if (!resp.ok) throw new Error('Failed to fetch');
        const data = await resp.json();
        const methods = data.payment_methods || [];

        if (methods.length === 0) {
            container.innerHTML = `
                <div style="display:flex; align-items:center; gap:16px; padding:16px; border:1px solid #eaecf0; border-radius:10px; background:#fafafa;">
                    <div style="width:40px; height:40px; border-radius:8px; background:#f2f4f7; display:flex; align-items:center; justify-content:center;">
                        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#98a2b3" stroke-width="2"><rect x="1" y="4" width="22" height="16" rx="2" ry="2"/><line x1="1" y1="10" x2="23" y2="10"/></svg>
                    </div>
                    <div style="flex:1;">
                        <div style="font-size:14px; font-weight:500; color:#344054;">No payment methods added</div>
                        <div style="font-size:12px; color:#98a2b3; margin-top:2px;">Add a credit card or bank account to make investments</div>
                    </div>
                </div>
            `;
            return;
        }

        let html = '';
        methods.forEach(pm => {
            const isCard = pm.method_type === 'card';
            const brand = (pm.brand || '').toLowerCase();
            
            // Brand-specific SVG icons
            let brandIcon;
            if (brand === 'visa') {
                brandIcon = `<svg width="40" height="26" viewBox="0 0 40 26" fill="none"><rect x=".5" y=".5" width="39" height="25" rx="3.5" fill="#1A1F71" stroke="#1A1F71"/><text x="6" y="18" fill="#fff" font-size="12" font-weight="700" font-family="Arial">VISA</text></svg>`;
            } else if (brand === 'mastercard') {
                brandIcon = `<svg width="40" height="26" viewBox="0 0 40 26" fill="none"><rect x=".5" y=".5" width="39" height="25" rx="3.5" fill="#252525" stroke="#252525"/><circle cx="15" cy="13" r="8" fill="#EB001B"/><circle cx="25" cy="13" r="8" fill="#F79E1B"/><path d="M20 6.2a8 8 0 010 13.6 8 8 0 000-13.6z" fill="#FF5F00"/></svg>`;
            } else if (isCard) {
                brandIcon = `<svg width="40" height="26" viewBox="0 0 40 26" fill="none"><rect x=".5" y=".5" width="39" height="25" rx="3.5" fill="#f2f4f7" stroke="#d5d7da"/><rect x="6" y="8" width="28" height="3" rx="1.5" fill="#98a2b3"/><rect x="6" y="15" width="14" height="2" rx="1" fill="#d5d7da"/></svg>`;
            } else {
                brandIcon = `<svg width="40" height="26" viewBox="0 0 40 26" fill="none"><rect x=".5" y=".5" width="39" height="25" rx="3.5" fill="#f0fdf4" stroke="#bbf7d0"/><svg x="10" y="3" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#16A34A" stroke-width="2"><line x1="3" y1="22" x2="21" y2="22"/><line x1="6" y1="18" x2="6" y2="11"/><line x1="10" y1="18" x2="10" y2="11"/><line x1="14" y1="18" x2="14" y2="11"/><line x1="18" y1="18" x2="18" y2="11"/><polygon points="12 2 20 7 4 7"/></svg></svg>`;
            }
            
            // Card display: masked number + expiry
            const last4 = pm.last_four || '****';
            const maskedNumber = isCard ? `•••• •••• •••• ${last4}` : `Account ending in ${last4}`;
            const expiry = (pm.expiry_month && pm.expiry_year) ? `${String(pm.expiry_month).padStart(2, '0')}/${String(pm.expiry_year).slice(-2)}` : '';
            const brandLabel = brand ? brand.charAt(0).toUpperCase() + brand.slice(1) : (isCard ? 'Card' : 'Bank');
            const holderName = pm.holder_name || '';
            
            html += `
                <div class="settings-pm-card" style="display:flex; align-items:center; gap:16px; padding:14px 16px; border:1px solid #eaecf0; border-radius:10px; background:#fff; transition: all 0.15s ease;">
                    <div style="flex-shrink:0;">${brandIcon}</div>
                    <div style="flex:1; min-width:0;">
                        <div style="display:flex; align-items:center; gap:8px; margin-bottom:2px;">
                            <span style="font-size:14px; font-weight:600; color:#181d27;">${escapeSearchHtml(brandLabel)}</span>
                            ${pm.is_default ? '<span class="settings-badge settings-badge--success" style="font-size:10px; padding:1px 6px;">Default</span>' : ''}
                        </div>
                        <div style="font-size:13px; color:#535862; font-family:'SF Mono','Menlo',monospace; letter-spacing:0.5px;">${maskedNumber}</div>
                        <div style="display:flex; gap:16px; margin-top:4px;">
                            ${holderName ? `<span style="font-size:12px; color:#98a2b3;">${escapeSearchHtml(holderName)}</span>` : ''}
                            ${expiry ? `<span style="font-size:12px; color:#98a2b3;">Expires ${expiry}</span>` : ''}
                        </div>
                    </div>
                    <button onclick="deletePaymentMethod('${pm.id}')" class="settings-pm-delete-btn" title="Delete" style="background:none; border:1px solid #eaecf0; border-radius:6px; cursor:pointer; padding:6px; color:#98a2b3; transition: all 0.15s ease; display:flex; align-items:center; justify-content:center;">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
                    </button>
                </div>
            `;
        });
        container.innerHTML = html;
    } catch (err) {
        console.error('Failed to load payment methods:', err);
        container.innerHTML = `
            <div style="display:flex; align-items:center; gap:16px; padding:16px; border:1px solid #eaecf0; border-radius:10px; background:#fafafa;">
                <div style="width:40px; height:40px; border-radius:8px; background:#f2f4f7; display:flex; align-items:center; justify-content:center;">
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#98a2b3" stroke-width="2"><rect x="1" y="4" width="22" height="16" rx="2" ry="2"/><line x1="1" y1="10" x2="23" y2="10"/></svg>
                </div>
                <div style="flex:1;">
                    <div style="font-size:14px; font-weight:500; color:#344054;">No payment methods added</div>
                    <div style="font-size:12px; color:#98a2b3; margin-top:2px;">Add a credit card or bank account to make investments</div>
                </div>
            </div>
        `;
    }
}
window.loadPaymentMethods = loadPaymentMethods;

/**
 * Delete a payment method
 */
async function deletePaymentMethod(id) {
    if (!await pooolConfirm({ title: 'Delete payment method', message: 'This payment method will be permanently removed from your account.', confirmText: 'Delete', type: 'danger' })) return;
    try {
        const resp = await fetch(`/api/payment-methods/${id}`, { method: 'DELETE', headers: { 'X-CSRF-Token': getSettingsCsrfToken() } });
        if (resp.ok) {
            showToast('Payment method deleted', 'success');
            loadPaymentMethods();
        } else {
            showToast('Failed to delete payment method', 'error');
        }
    } catch (e) {
        showToast('Network error', 'error');
    }
}
window.deletePaymentMethod = deletePaymentMethod;

/* ─── Card Modal ─── */
function openSettingsCardModal() {
    const m = document.getElementById('modal-add-card');
    if (m) { m.style.display = 'flex'; document.body.style.overflow = 'hidden'; }
}
function closeSettingsCardModal() {
    const m = document.getElementById('modal-add-card');
    if (m) { m.style.display = 'none'; document.body.style.overflow = ''; }
    const f = document.getElementById('settings-add-card-form');
    if (f) f.reset();
    const e = document.getElementById('settings-card-errors');
    if (e) e.textContent = '';
}
window.openSettingsCardModal = openSettingsCardModal;
window.closeSettingsCardModal = closeSettingsCardModal;

// Card number formatting
document.addEventListener('DOMContentLoaded', function() {
    const cardInput = document.getElementById('settings-card-number');
    if (cardInput) {
        cardInput.addEventListener('input', function(e) {
            let v = e.target.value.replace(/\s+/g, '').replace(/[^0-9]/g, '');
            let parts = v.match(/.{1,4}/g);
            e.target.value = parts ? parts.join(' ') : '';
        });
    }
    const expiryInput = document.getElementById('settings-card-expiry');
    if (expiryInput) {
        expiryInput.addEventListener('input', function(e) {
            let v = e.target.value.replace(/\s+/g, '').replace(/[^0-9]/g, '');
            if (v.length >= 2) e.target.value = v.substring(0, 2) + ' / ' + v.substring(2, 4);
        });
    }

    // Card form submission
    const cardForm = document.getElementById('settings-add-card-form');
    if (cardForm) {
        cardForm.addEventListener('submit', async function(e) {
            e.preventDefault();
            const btn = document.getElementById('settings-card-submit');
            const errEl = document.getElementById('settings-card-errors');
            const holderName = document.getElementById('settings-card-holder').value;
            const cardNumber = document.getElementById('settings-card-number').value.replace(/\s/g, '');

            if (cardNumber.length < 13) { errEl.textContent = 'Please enter a valid card number.'; return; }

            btn.disabled = true; btn.textContent = 'Processing...';
            errEl.textContent = '';

            // Detect brand
            const BRANDS = [
                { name: 'Visa', pattern: /^4/ },
                { name: 'Mastercard', pattern: /^5[1-5]|^2(2[2-9]|[3-6]|7[01]|720)/ },
                { name: 'Amex', pattern: /^3[47]/ },
                { name: 'Discover', pattern: /^6(?:011|5)/ },
            ];
            let brand = 'Card';
            for (const b of BRANDS) { if (b.pattern.test(cardNumber)) { brand = b.name; break; } }
            const last4 = cardNumber.slice(-4);
            const stripeId = 'manual_' + brand.toLowerCase() + '_' + last4 + '_' + Date.now();
            const label = brand + ' ending in ' + last4;

            const body = 'stripe_payment_method_id=' + encodeURIComponent(stripeId)
                + '&holder_name=' + encodeURIComponent(holderName)
                + '&label=' + encodeURIComponent(label);

            try {
                const res = await fetch('/api/payment-methods/card', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'X-CSRF-Token': getSettingsCsrfToken() },
                    body: body
                });
                if (res.ok) {
                    closeSettingsCardModal();
                    showToast('Card added successfully', 'success');
                    loadPaymentMethods();
                } else {
                    const t = await res.text();
                    errEl.textContent = t || 'Error saving card';
                }
            } catch (_) {
                errEl.textContent = 'A network error occurred.';
            }
            btn.disabled = false; btn.textContent = 'Save Card';
        });
    }
});

/* ─── Bank Modal ─── */
const SETTINGS_BANK_SYSTEMS = {
    US: { system: 'ach', fields: [
        { name: 'routing_code', label: 'Routing Number (ABA)', placeholder: 'e.g. 021000021', maxlen: 9 },
        { name: 'account_number', label: 'Account Number', placeholder: 'e.g. 1234567890', maxlen: 17 }
    ]},
    GB: { system: 'bacs', fields: [
        { name: 'routing_code', label: 'Sort Code', placeholder: 'e.g. 20-00-00', maxlen: 8 },
        { name: 'account_number', label: 'Account Number', placeholder: 'e.g. 12345678', maxlen: 8 }
    ]},
    AU: { system: 'bsb', fields: [
        { name: 'routing_code', label: 'BSB Number', placeholder: 'e.g. 062-000', maxlen: 7 },
        { name: 'account_number', label: 'Account Number', placeholder: 'e.g. 12345678', maxlen: 10 }
    ]},
    IN: { system: 'ifsc', fields: [
        { name: 'routing_code', label: 'IFSC Code', placeholder: 'e.g. HDFC0001234', maxlen: 11 },
        { name: 'account_number', label: 'Account Number', placeholder: 'e.g. 1234567890', maxlen: 18 }
    ]},
    SEPA: { system: 'sepa', fields: [
        { name: 'account_number', label: 'IBAN', placeholder: 'e.g. DE89370400440532013000', maxlen: 34 },
        { name: 'routing_code', label: 'BIC / SWIFT Code (optional)', placeholder: 'e.g. DEUTDEDB', maxlen: 11, optional: true }
    ]},
    SWIFT: { system: 'swift', fields: [
        { name: 'routing_code', label: 'SWIFT / BIC Code', placeholder: 'e.g. CHASUS33', maxlen: 11 },
        { name: 'account_number', label: 'IBAN or Account Number', placeholder: 'e.g. GB29NWBK60161331926819', maxlen: 34 }
    ]}
};
const SEPA_CODES = ['DE','FR','IT','ES','NL','BE','AT','PT','IE','FI','SE','DK','NO','CH','PL','CZ','HU','RO','GR','LU'];

function getSettingsBankConfig(cc) {
    if (SETTINGS_BANK_SYSTEMS[cc]) return SETTINGS_BANK_SYSTEMS[cc];
    if (SEPA_CODES.indexOf(cc) >= 0) return SETTINGS_BANK_SYSTEMS.SEPA;
    return SETTINGS_BANK_SYSTEMS.SWIFT;
}

function renderSettingsBankFields(cc) {
    const config = getSettingsBankConfig(cc);
    document.getElementById('settings-bank-system').value = config.system;
    const container = document.getElementById('settings-bank-dynamic-fields');
    container.innerHTML = '';
    config.fields.forEach(function(f) {
        const row = document.createElement('div');
        row.className = 'settings-form-row';
        row.style.marginBottom = '16px';
        const lbl = document.createElement('label');
        lbl.className = 'settings-label';
        lbl.innerHTML = f.label + (f.optional ? '' : ' <span style="color:#d92d20">*</span>');
        const input = document.createElement('input');
        input.type = 'text';
        input.className = 'settings-input';
        input.name = f.name;
        input.placeholder = f.placeholder || '';
        if (f.maxlen) input.maxLength = f.maxlen;
        if (!f.optional) input.required = true;
        row.appendChild(lbl);
        row.appendChild(input);
        container.appendChild(row);
    });
}

function openSettingsBankModal() {
    const m = document.getElementById('modal-add-bank');
    if (m) { m.style.display = 'flex'; document.body.style.overflow = 'hidden'; }
    renderSettingsBankFields(document.getElementById('settings-bank-country').value);
}
function closeSettingsBankModal() {
    const m = document.getElementById('modal-add-bank');
    if (m) { m.style.display = 'none'; document.body.style.overflow = ''; }
    const f = document.getElementById('settings-add-bank-form');
    if (f) f.reset();
    const e = document.getElementById('settings-bank-errors');
    if (e) e.textContent = '';
    document.getElementById('settings-bank-dynamic-fields').innerHTML = '';
}
window.openSettingsBankModal = openSettingsBankModal;
window.closeSettingsBankModal = closeSettingsBankModal;

document.addEventListener('DOMContentLoaded', function() {
    const countrySelect = document.getElementById('settings-bank-country');
    if (countrySelect) {
        countrySelect.addEventListener('change', function() {
            renderSettingsBankFields(this.value);
        });
    }

    const bankForm = document.getElementById('settings-add-bank-form');
    if (bankForm) {
        bankForm.addEventListener('submit', async function(e) {
            e.preventDefault();
            const btn = document.getElementById('settings-bank-submit');
            const errEl = document.getElementById('settings-bank-errors');
            btn.disabled = true; btn.textContent = 'Processing...';
            errEl.textContent = '';

            const fd = new FormData(bankForm);
            const body = new URLSearchParams(fd).toString();

            try {
                const res = await fetch('/api/payment-methods/bank', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'X-CSRF-Token': getSettingsCsrfToken() },
                    body: body
                });
                if (res.ok) {
                    closeSettingsBankModal();
                    showToast('Bank account added successfully', 'success');
                    loadPaymentMethods();
                } else {
                    const t = await res.text();
                    errEl.textContent = t || 'Error saving bank account';
                }
            } catch (_) {
                errEl.textContent = 'A network error occurred.';
            }
            btn.disabled = false; btn.textContent = 'Save Bank Account';
        });
    }
});
