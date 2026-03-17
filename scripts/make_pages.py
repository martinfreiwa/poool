import re

with open("frontend/platform/rewards.html", "r") as f:
    rewards_html = f.read()

with open("frontend/platform/tier.html", "r") as f:
    tier_html = f.read()

# --- REWARDS CONTENT ---
rewards_content = """
<div class="rewards-page-header">
    <div class="rewards-title-wrapper">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#414651" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="rewards-trophy-icon">
            <path d="M8 21h8M12 17v4M7 4h10M17 4v8a5 5 0 0 1-10 0V4M3 4h4v5H3zM17 4h4v5h-4z"/>
        </svg>
        <h1 class="rewards-title">Rewards</h1>
    </div>
</div>

<div class="rewards-card-container top-card">
    <div class="rewards-balance-section">
        <div class="balance-label-row">
            <span class="balance-label">Rewards balance</span>
            <svg class="help-icon" width="14" height="14" viewBox="0 0 14 14" fill="none"><circle cx="7" cy="7" r="6" stroke="#A4A7AE" stroke-width="1.33"/><path d="M7 9.5V7M7 4.5h.01" stroke="#A4A7AE" stroke-width="1.33" stroke-linecap="round"/></svg>
        </div>
        <div class="balance-amount">USD 357</div>
        <a href="#" class="view-balance-link">
            View current balance
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M6 12l4-4-4-4" stroke="#414651" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </a>
    </div>

    <div class="balance-icon-section">
        <div class="icon-outer-circle">
            <div class="icon-inner-circle">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" stroke="#0000FF" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </div>
        </div>
    </div>

    <div class="balance-breakdown-section">
        <div class="breakdown-row">
            <div class="breakdown-label">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M12 4H4C2.89543 4 2 4.89543 2 6V10C2 11.1046 2.89543 12 4 12H12C13.1046 12 14 11.1046 14 10V6C14 4.89543 13.1046 4 12 4Z" stroke="#414651" stroke-width="1.5" stroke-linecap="round"/><path d="M14 6L8 9L2 6" stroke="#414651" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
                <span>Cashback</span>
                <svg class="help-icon" width="14" height="14" viewBox="0 0 14 14" fill="none"><circle cx="7" cy="7" r="6" stroke="#A4A7AE" stroke-width="1.33"/><path d="M7 9.5V7M7 4.5h.01" stroke="#A4A7AE" stroke-width="1.33" stroke-linecap="round"/></svg>
            </div>
            <div class="breakdown-value">USD 1,300</div>
        </div>
        <div class="breakdown-separator"></div>
        <div class="breakdown-row">
            <div class="breakdown-label">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M10.6667 4.66667C10.6667 6.13943 9.47276 7.33333 8 7.33333C6.52724 7.33333 5.33333 6.13943 5.33333 4.66667C5.33333 3.19391 6.52724 2 8 2C9.47276 2 10.6667 3.19391 10.6667 4.66667Z" stroke="#414651" stroke-width="1.5" stroke-linecap="round"/><path d="M3.33333 14C3.33333 11.4227 5.42267 9.33333 8 9.33333C10.5773 9.33333 12.6667 11.4227 12.6667 14" stroke="#414651" stroke-width="1.5" stroke-linecap="round"/></svg>
                <span>Referrals</span>
                <svg class="help-icon" width="14" height="14" viewBox="0 0 14 14" fill="none"><circle cx="7" cy="7" r="6" stroke="#A4A7AE" stroke-width="1.33"/><path d="M7 9.5V7M7 4.5h.01" stroke="#A4A7AE" stroke-width="1.33" stroke-linecap="round"/></svg>
            </div>
            <div class="breakdown-value">USD 180</div>
        </div>
        <div class="breakdown-separator"></div>
        <div class="breakdown-row">
            <div class="breakdown-label">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 14A6 6 0 108 2a6 6 0 000 12zM8 5v4l2 2" stroke="#414651" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
                <span>Promotions</span>
                <svg class="help-icon" width="14" height="14" viewBox="0 0 14 14" fill="none"><circle cx="7" cy="7" r="6" stroke="#A4A7AE" stroke-width="1.33"/><path d="M7 9.5V7M7 4.5h.01" stroke="#A4A7AE" stroke-width="1.33" stroke-linecap="round"/></svg>
            </div>
            <div class="breakdown-value">USD 330</div>
        </div>
    </div>
</div>

<div class="rewards-bottom-grid">
    <a href="/tier" class="tier-progress-card">
        <div class="tier-card-header">
            <div class="tier-logo-wrap">
                <img src="/images/Logo Pool.svg" alt="POOOL" class="tp-logo">
                <span class="tp-badge premium">Premium</span>
            </div>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M6 12l4-4-4-4" stroke="#414651" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </div>
        <div class="tier-card-body">
            <div class="tp-invested-row">
                <span class="tp-amount">USD 12,500</span>
                <span class="tp-label">Invested in the last 12 months</span>
            </div>
            <div class="tp-progress-bar">
                <div class="tp-progress-fill" style="width: 50%;"></div>
            </div>
            <div class="tp-hint">
                Invest <strong>USD 25,000</strong> to reach Premium
            </div>
        </div>
    </a>

    <div class="refer-earn-card">
        <div class="refer-card-header">
            <div class="refer-title-wrap">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="#0000FF" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="8" width="14" height="9" rx="1"/><path d="M10 8v9M7.5 8c-1.5 0-2.5-1-2.5-2.5C5 4 7.5 4 7.5 4s1.5 1.5 2.5 4c1-2.5 2.5-4 2.5-4s2.5 0 2.5 1.5C15 7 14 8 12.5 8 11.5 8 10 8 10 8z"/></svg>
                <h3>Refer and earn</h3>
            </div>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M6 12l4-4-4-4" stroke="#414651" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </div>
        <p class="refer-text">
            Invite your friends and you'll both receive a rewards balance to invest in our properties!
        </p>
        <div class="refer-separator"></div>
        <ul class="refer-checklist">
            <li>
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="#5555FF" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M14 8A6 6 0 118 2a6 6 0 016 6z"/><path d="M5 8l2 2 4-4"/></svg>
                Friends get USD 30 upon signing up
            </li>
            <li>
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="#5555FF" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M14 8A6 6 0 118 2a6 6 0 016 6z"/><path d="M5 8l2 2 4-4"/></svg>
                You get USD 30 after they invest USD 1,000
            </li>
        </ul>
        <div class="refer-share-form">
            <label>Share your link</label>
            <div class="refer-input-group">
                <div class="refer-input-wrapper">
                    <input type="text" value="https://app.poool.com/rewards/1792..." readonly>
                    <button class="copy-icon-btn"><svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="#0000FF" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="5.33" y="5.33" width="8" height="8" rx="1.33"/><path d="M4 10.66H2.66A1.33 1.33 0 011.33 9.33V2.66h6.66A1.33 1.33 0 019.33 4"/></svg></button>
                </div>
                <button class="copy-link-btn">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="white" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="5.33" y="5.33" width="8" height="8" rx="1.33"/><path d="M4 10.66H2.66A1.33 1.33 0 011.33 9.33V2.66h6.66A1.33 1.33 0 019.33 4"/></svg>
                    Copy link
                </button>
            </div>
        </div>
    </div>
</div>
"""

# --- TIER CONTENT ---
tier_content = """
<div class="breadcrumbs">
    <a href="/rewards">Rewards</a>
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none"><path d="M4.5 9l3-3-3-3" stroke="#A4A7AE" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
    <span class="active-crumb">Tier</span>
</div>

<div class="rewards-page-header compact">
    <div class="rewards-title-wrapper">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#414651" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="tier-target-icon">
            <circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2"/>
        </svg>
        <h1 class="rewards-title">Tier</h1>
    </div>
</div>

<div class="tier-layout">
    <div class="tier-left-column">
        <div class="tier-progress-card intro-style active">
            <div class="tier-card-header">
                <div class="tier-logo-wrap">
                    <img src="/images/Logo Pool.svg" alt="POOOL" class="tp-logo filter-blue">
                    <span class="tp-badge intro italic text-blue">Intro</span>
                </div>
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M6 12l4-4-4-4" stroke="#414651" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </div>
            <div class="tier-card-body">
                <div class="tp-invested-row text-black">
                    <span class="tp-amount">USD 0</span>
                    <span class="tp-label">Invested in the last 12 months</span>
                </div>
                <div class="tp-progress-bar bg-white">
                    <div class="tp-progress-fill" style="width: 0%;"></div>
                </div>
                <div class="tp-hint text-black">
                    Invest <strong class="text-blue">USD 4,000</strong> to reach Plus
                </div>
            </div>
        </div>
    </div>

    <div class="tier-right-column">
        <div class="tier-stepper">
            
            <!-- INTRO -->
            <div class="stepper-item active">
                <div class="stepper-line"></div>
                <div class="stepper-icon active">
                    <div class="stepper-icon-inner"></div>
                </div>
                <div class="stepper-content">
                    <div class="stepper-header">
                        <span class="badge badge-intro-filled">INTRO</span>
                    </div>
                    <div class="stepper-body">
                        <h4 class="text-black">USD 5 for every qualified referral</h4>
                        <p class="text-gray mt-2">No payment processing fees</p>
                    </div>
                </div>
            </div>

            <!-- PLUS -->
            <div class="stepper-item inactive">
                <div class="stepper-line"></div>
                <div class="stepper-icon"></div>
                <div class="stepper-content border-top-none">
                    <div class="stepper-header space-between">
                        <span class="badge badge-plus-filled">PLUS</span>
                        <div class="stepper-lock"><svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="#A4A7AE" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="6" width="8" height="6" rx="1.5"/><path d="M4.5 6V4a2.5 2.5 0 115 0v2"/></svg> USD 3,500 to unlock</div>
                    </div>
                    <div class="stepper-body border-top">
                        <h4 class="text-black">USD 50 for every qualified referral</h4>
                        <p class="text-black mt-2">Access RentReinvest which allows you to automatically reinvest your rental income on USD investments</p>
                        <p class="text-gray mt-2">All the benefits you've unlocked so far</p>
                    </div>
                </div>
            </div>

            <!-- PRO -->
            <div class="stepper-item inactive">
                <div class="stepper-line"></div>
                <div class="stepper-icon"></div>
                <div class="stepper-content border-top-none">
                    <div class="stepper-header space-between">
                        <span class="badge badge-pro-filled">PRO</span>
                        <div class="stepper-lock"><svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="#A4A7AE" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="6" width="8" height="6" rx="1.5"/><path d="M4.5 6V4a2.5 2.5 0 115 0v2"/></svg> USD 10,000 to unlock</div>
                    </div>
                    <div class="stepper-body border-top">
                        <h4 class="text-black">USD 100 for every qualified referral</h4>
                        <p class="text-black mt-2">1% cashback on every investment for all USD investments</p>
                        <p class="text-black mt-2">0.5% cashback on every investment for all KSA investments</p>
                        <p class="text-gray mt-2">All the benefits you've unlocked so far</p>
                    </div>
                </div>
            </div>

            <!-- ELITE -->
            <div class="stepper-item inactive">
                <div class="stepper-line"></div>
                <div class="stepper-icon"></div>
                <div class="stepper-content border-top-none">
                    <div class="stepper-header space-between">
                        <span class="badge badge-elite-filled">ELITE</span>
                        <div class="stepper-lock"><svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="#A4A7AE" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="6" width="8" height="6" rx="1.5"/><path d="M4.5 6V4a2.5 2.5 0 115 0v2"/></svg> USD 30,000 to unlock</div>
                    </div>
                    <div class="stepper-body border-top">
                        <h4 class="text-black">USD 150 for every qualified referral</h4>
                        <p class="text-black mt-2">2% cashback on every investment for all Indonesian investments</p>
                        <p class="text-black mt-2">0.75% cashback on every investment for all KSA investments</p>
                        <p class="text-black mt-2">Gain exclusive early access to invest in funds in Indonesia before they go live to the general public</p>
                        <p class="text-gray mt-2">All the benefits you've unlocked so far</p>
                    </div>
                </div>
            </div>

            <!-- PREMIUM -->
            <div class="stepper-item inactive">
                <div class="stepper-icon"></div>
                <div class="stepper-content border-top-none">
                    <div class="stepper-header space-between">
                        <span class="badge badge-premium-filled">PREMIUM</span>
                        <div class="stepper-lock"><svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="#A4A7AE" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="6" width="8" height="6" rx="1.5"/><path d="M4.5 6V4a2.5 2.5 0 115 0v2"/></svg> USD 100,000 to unlock</div>
                    </div>
                    <div class="stepper-body border-top">
                        <h4 class="text-black">USD 200 for every qualified referral</h4>
                        <p class="text-black mt-2">3% cashback on every investment for all Indonesian investments</p>
                        <p class="text-black mt-2">1% cashback on every investment for all referral investments</p>
                        <p class="text-black mt-2">A dedicated account representative to help guide you through your investments.</p>
                        <p class="text-black mt-2">Free Bloomberg subscription</p>
                        <p class="text-black mt-2">Invitations to exclusive private investor dinners</p>
                        <p class="text-gray mt-2">All the benefits you've unlocked so far</p>
                    </div>
                </div>
            </div>

        </div>
    </div>
</div>
"""

rewards_html = rewards_html.replace("REWARDS_CONTENT_PLACEHOLDER", rewards_content)
tier_html = tier_html.replace("TIER_CONTENT_PLACEHOLDER", tier_content)

with open("frontend/platform/rewards.html", "w") as f:
    f.write(rewards_html)

with open("frontend/platform/tier.html", "w") as f:
    f.write(tier_html)

