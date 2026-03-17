const fs = require('fs');
const path = require('path');

const PLATFORM = '/Users/martin/Downloads/poool/platform.poool.app';

function sidebar(active) {
    const links = [
        { href: '/platform/marketplace', icon: '🏠', label: 'Marketplace' },
        { href: '/platform/commodities-marketplace', icon: '📦', label: 'Commodities' },
        { href: '/platform/portfolio', icon: '📊', label: 'Portfolio' },
        { href: '/platform/wallet', icon: '💰', label: 'Wallet' },
        { href: '/platform/cart', icon: '🛒', label: 'Cart' },
        { href: '/platform/support', icon: '💬', label: 'Support' },
        { href: '/platform/settings', icon: '⚙️', label: 'Settings' },
        { href: '/platform/kyc', icon: '🪪', label: 'KYC Verification' },
    ];
    return `
  <div class="sidebar">
    <div class="sidebar-logo"><img src="/platform/static/images/logo-pool.svg" alt="POOOL"></div>
    <nav class="sidebar-nav">
      ${links.map(l => `<a href="${l.href}" class="${active === l.label.toLowerCase().replace(' ', '-') ? 'active' : ''}"><span class="icon">${l.icon}</span> <span>${l.label}</span></a>`).join('\n      ')}
    </nav>
    <div class="sidebar-bottom">
      <a href="/platform/login">🚪 <span>Log out</span></a>
    </div>
  </div>`;
}

function layout(title, activePage, content) {
    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${title} - POOOL</title>
  <link rel="stylesheet" href="/platform/static/css/dashboard.css">
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
</head>
<body>
<div class="dashboard">
  ${sidebar(activePage)}
  <div class="main">
    <div class="topbar">
      <h1>${title}</h1>
      <div class="topbar-right">
        <button class="notif-btn">🔔</button>
        <div class="user-info">
          <div class="avatar">JD</div>
          <span style="font-size:14px;font-weight:500;">John Doe</span>
        </div>
      </div>
    </div>
    <div class="content">
      ${content}
    </div>
  </div>
</div>
</body>
</html>`;
}

// MARKETPLACE
const marketplace = layout('Marketplace', 'marketplace', `
<div class="stats-grid">
  <div class="stat-card"><div class="label">Total Properties</div><div class="value">12</div><div class="change up">↑ 3 new this month</div></div>
  <div class="stat-card"><div class="label">Total Invested</div><div class="value">$24,500</div><div class="change up">↑ 12.5%</div></div>
  <div class="stat-card"><div class="label">Avg. ROI</div><div class="value">14.2%</div><div class="change up">↑ 2.1%</div></div>
  <div class="stat-card"><div class="label">Active Investors</div><div class="value">2,847</div><div class="change up">↑ 156 this week</div></div>
</div>

<div class="table-header" style="background:white;border-radius:12px 12px 0 0;border:1px solid #f0f0f0;border-bottom:none;">
  <h2>Available Properties</h2>
</div>
<div class="property-grid" style="margin-top:0;">
  <div class="property-card">
    <div class="image" style="background:linear-gradient(45deg,#0a1628,#1a3a5c);display:flex;align-items:center;justify-content:center;color:white;font-size:24px;">🏡</div>
    <div class="badge" style="position:relative;display:inline-block;margin:16px 0 0 20px;">Freehold</div>
    <div class="info">
      <div class="title">Nomad Palm Residence</div>
      <div class="location">📍 Canggu, Bali, Indonesia</div>
      <div class="details">
        <div class="detail"><div class="val">$420,000</div><div class="lbl">Price</div></div>
        <div class="detail"><div class="val">12.8%</div><div class="lbl">APR</div></div>
        <div class="detail"><div class="val">24%</div><div class="lbl">IRR</div></div>
      </div>
      <div class="progress-bar"><div class="fill" style="width:100%"></div></div>
      <div style="display:flex;justify-content:space-between;margin-top:8px;font-size:12px;color:#6b7280;"><span>646 investors</span><span>100% funded</span></div>
    </div>
  </div>
  <div class="property-card">
    <div class="image" style="background:linear-gradient(45deg,#1a3a5c,#2d5f8a);display:flex;align-items:center;justify-content:center;color:white;font-size:24px;">🏠</div>
    <div class="badge" style="position:relative;display:inline-block;margin:16px 0 0 20px;">Freehold</div>
    <div class="info">
      <div class="title">Luna Bay Villa</div>
      <div class="location">📍 Uluwatu, Bali, Indonesia</div>
      <div class="details">
        <div class="detail"><div class="val">$380,000</div><div class="lbl">Price</div></div>
        <div class="detail"><div class="val">11.5%</div><div class="lbl">APR</div></div>
        <div class="detail"><div class="val">22%</div><div class="lbl">IRR</div></div>
      </div>
      <div class="progress-bar"><div class="fill" style="width:72%"></div></div>
      <div style="display:flex;justify-content:space-between;margin-top:8px;font-size:12px;color:#6b7280;"><span>412 investors</span><span>72% funded</span></div>
    </div>
  </div>
  <div class="property-card">
    <div class="image" style="background:linear-gradient(45deg,#2d5f8a,#4a90c4);display:flex;align-items:center;justify-content:center;color:white;font-size:24px;">🏘️</div>
    <div class="badge" style="position:relative;display:inline-block;margin:16px 0 0 20px;">Leasehold</div>
    <div class="info">
      <div class="title">Coral Breeze Apartments</div>
      <div class="location">📍 Seminyak, Bali, Indonesia</div>
      <div class="details">
        <div class="detail"><div class="val">$250,000</div><div class="lbl">Price</div></div>
        <div class="detail"><div class="val">15.1%</div><div class="lbl">APR</div></div>
        <div class="detail"><div class="val">28%</div><div class="lbl">IRR</div></div>
      </div>
      <div class="progress-bar"><div class="fill" style="width:45%"></div></div>
      <div style="display:flex;justify-content:space-between;margin-top:8px;font-size:12px;color:#6b7280;"><span>189 investors</span><span>45% funded</span></div>
    </div>
  </div>
</div>
`);

// COMMODITIES
const commodities = layout('Commodities Marketplace', 'commodities', `
<div class="stats-grid">
  <div class="stat-card"><div class="label">Available Commodities</div><div class="value">8</div></div>
  <div class="stat-card"><div class="label">Total Market Cap</div><div class="value">$1.2M</div></div>
  <div class="stat-card"><div class="label">Avg. Return</div><div class="value">18.6%</div></div>
</div>
<div class="table-card">
  <div class="table-header"><h2>Commodities</h2></div>
  <table>
    <thead><tr><th>Commodity</th><th>Price</th><th>24h Change</th><th>Market Cap</th><th>Volume</th></tr></thead>
    <tbody>
      <tr><td>☕ Bali Coffee Premium</td><td>$24.50/kg</td><td style="color:#10b981">+3.2%</td><td>$450,000</td><td>$12,400</td></tr>
      <tr><td>🍫 Cacao Organic</td><td>$18.20/kg</td><td style="color:#10b981">+1.8%</td><td>$320,000</td><td>$8,900</td></tr>
      <tr><td>🌴 Coconut Oil Virgin</td><td>$6.80/L</td><td style="color:#ef4444">-0.5%</td><td>$180,000</td><td>$5,200</td></tr>
      <tr><td>🍚 Balinese Rice</td><td>$3.40/kg</td><td style="color:#10b981">+0.3%</td><td>$150,000</td><td>$4,100</td></tr>
      <tr><td>🌿 Vanilla Extract</td><td>$142.00/kg</td><td style="color:#10b981">+5.7%</td><td>$98,000</td><td>$3,800</td></tr>
    </tbody>
  </table>
</div>
`);

// PORTFOLIO
const portfolio = layout('Portfolio', 'portfolio', `
<div class="stats-grid">
  <div class="stat-card"><div class="label">Portfolio Value</div><div class="value">$12,450</div><div class="change up">↑ $1,245 (11.1%)</div></div>
  <div class="stat-card"><div class="label">Total Returns</div><div class="value">$1,680</div><div class="change up">↑ 15.6% all-time</div></div>
  <div class="stat-card"><div class="label">Active Investments</div><div class="value">4</div></div>
  <div class="stat-card"><div class="label">Monthly Income</div><div class="value">$185</div><div class="change up">↑ from 3 properties</div></div>
</div>
<div class="table-card">
  <div class="table-header"><h2>My Investments</h2></div>
  <table>
    <thead><tr><th>Property</th><th>Invested</th><th>Current Value</th><th>Return</th><th>Status</th></tr></thead>
    <tbody>
      <tr><td>🏡 Nomad Palm Residence</td><td>$5,000</td><td>$5,640</td><td style="color:#10b981">+12.8%</td><td><span style="background:#d1fae5;color:#059669;padding:4px 10px;border-radius:12px;font-size:12px;">Active</span></td></tr>
      <tr><td>🏠 Luna Bay Villa</td><td>$3,500</td><td>$3,902</td><td style="color:#10b981">+11.5%</td><td><span style="background:#d1fae5;color:#059669;padding:4px 10px;border-radius:12px;font-size:12px;">Active</span></td></tr>
      <tr><td>🏘️ Coral Breeze Apartments</td><td>$2,000</td><td>$2,302</td><td style="color:#10b981">+15.1%</td><td><span style="background:#fef3c7;color:#d97706;padding:4px 10px;border-radius:12px;font-size:12px;">Funding</span></td></tr>
      <tr><td>☕ Bali Coffee Premium</td><td>$1,950</td><td>$2,106</td><td style="color:#10b981">+8.0%</td><td><span style="background:#d1fae5;color:#059669;padding:4px 10px;border-radius:12px;font-size:12px;">Active</span></td></tr>
    </tbody>
  </table>
</div>
`);

// WALLET
const wallet = layout('Wallet', 'wallet', `
<div class="balance-card">
  <div class="balance-label">Available Balance</div>
  <div class="balance-amount">$8,240.00</div>
  <div class="balance-actions">
    <button class="btn-primary">+ Deposit</button>
    <button class="btn-outline">↑ Withdraw</button>
    <button class="btn-outline">↔ Transfer</button>
  </div>
</div>
<div class="stats-grid">
  <div class="stat-card"><div class="label">Total Deposited</div><div class="value">$15,000</div></div>
  <div class="stat-card"><div class="label">Total Earned</div><div class="value">$1,680</div></div>
  <div class="stat-card"><div class="label">Pending</div><div class="value">$0.00</div></div>
</div>
<div class="table-card">
  <div class="table-header"><h2>Transaction History</h2></div>
  <table>
    <thead><tr><th>Date</th><th>Type</th><th>Description</th><th>Amount</th><th>Status</th></tr></thead>
    <tbody>
      <tr><td>Mar 5, 2026</td><td>Income</td><td>Monthly rental - Nomad Palm</td><td style="color:#10b981">+$62.50</td><td><span style="background:#d1fae5;color:#059669;padding:4px 10px;border-radius:12px;font-size:12px;">Completed</span></td></tr>
      <tr><td>Mar 3, 2026</td><td>Income</td><td>Monthly rental - Luna Bay</td><td style="color:#10b981">+$48.00</td><td><span style="background:#d1fae5;color:#059669;padding:4px 10px;border-radius:12px;font-size:12px;">Completed</span></td></tr>
      <tr><td>Feb 28, 2026</td><td>Investment</td><td>Coral Breeze Apartments</td><td style="color:#ef4444">-$2,000</td><td><span style="background:#d1fae5;color:#059669;padding:4px 10px;border-radius:12px;font-size:12px;">Completed</span></td></tr>
      <tr><td>Feb 15, 2026</td><td>Deposit</td><td>Bank transfer</td><td style="color:#10b981">+$5,000</td><td><span style="background:#d1fae5;color:#059669;padding:4px 10px;border-radius:12px;font-size:12px;">Completed</span></td></tr>
      <tr><td>Jan 20, 2026</td><td>Investment</td><td>Nomad Palm Residence</td><td style="color:#ef4444">-$5,000</td><td><span style="background:#d1fae5;color:#059669;padding:4px 10px;border-radius:12px;font-size:12px;">Completed</span></td></tr>
    </tbody>
  </table>
</div>
`);

// CART
const cart = layout('Cart', 'cart', `
<div class="empty-state">
  <div class="emoji">🛒</div>
  <h3>Your cart is empty</h3>
  <p>Browse the marketplace and add properties to your cart to invest.</p>
  <a href="/platform/marketplace" class="btn btn-blue" style="display:inline-block;text-decoration:none;">Browse Marketplace</a>
</div>
`);

// SUPPORT
const support = layout('Support', 'support', `
<div class="stats-grid">
  <div class="stat-card"><div class="label">Open Tickets</div><div class="value">0</div></div>
  <div class="stat-card"><div class="label">Avg Response Time</div><div class="value">2h</div></div>
  <div class="stat-card"><div class="label">Resolved Tickets</div><div class="value">3</div></div>
</div>
<div class="settings-section">
  <h3>Contact Support</h3>
  <div class="form-group"><label>Subject</label><input type="text" placeholder="What do you need help with?"></div>
  <div class="form-group"><label>Category</label><select><option>General Question</option><option>Account Issue</option><option>Investment Question</option><option>Technical Problem</option><option>KYC/Verification</option></select></div>
  <div class="form-group"><label>Message</label><textarea style="width:100%;padding:10px 14px;border:1px solid #d1d5db;border-radius:8px;font-size:14px;min-height:120px;font-family:inherit;" placeholder="Describe your issue..."></textarea></div>
  <button class="btn btn-blue">Submit Ticket</button>
</div>
<div class="settings-section">
  <h3>Quick Links</h3>
  <div style="display:flex;gap:16px;flex-wrap:wrap;">
    <a href="https://t.me/itspoool" target="_blank" style="padding:12px 20px;background:#0088cc;color:white;border-radius:8px;text-decoration:none;font-weight:600;">💬 Telegram</a>
    <a href="https://wa.me/6281325817676" target="_blank" style="padding:12px 20px;background:#25d366;color:white;border-radius:8px;text-decoration:none;font-weight:600;">📱 WhatsApp</a>
    <a href="mailto:support@poool.app" style="padding:12px 20px;background:#6b7280;color:white;border-radius:8px;text-decoration:none;font-weight:600;">✉️ Email</a>
  </div>
</div>
`);

// SETTINGS
const settings = layout('Settings', 'settings', `
<div class="settings-section">
  <h3>Profile Information</h3>
  <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px;">
    <div class="form-group"><label>First Name</label><input type="text" value="John"></div>
    <div class="form-group"><label>Last Name</label><input type="text" value="Doe"></div>
    <div class="form-group"><label>Email</label><input type="email" value="john.doe@example.com"></div>
    <div class="form-group"><label>Phone</label><input type="tel" value="+1 234 567 890"></div>
  </div>
  <button class="btn btn-blue" style="margin-top:8px;">Save Changes</button>
</div>
<div class="settings-section">
  <h3>Security</h3>
  <div class="form-group"><label>Current Password</label><input type="password" placeholder="Enter current password"></div>
  <div class="form-group"><label>New Password</label><input type="password" placeholder="Enter new password"></div>
  <div class="form-group"><label>Confirm New Password</label><input type="password" placeholder="Confirm new password"></div>
  <button class="btn btn-blue">Update Password</button>
</div>
<div class="settings-section">
  <h3>Notifications</h3>
  <div style="display:flex;flex-direction:column;gap:12px;">
    <label style="display:flex;align-items:center;gap:12px;cursor:pointer;"><input type="checkbox" checked> Email notifications for investments</label>
    <label style="display:flex;align-items:center;gap:12px;cursor:pointer;"><input type="checkbox" checked> Monthly portfolio reports</label>
    <label style="display:flex;align-items:center;gap:12px;cursor:pointer;"><input type="checkbox"> Marketing emails</label>
    <label style="display:flex;align-items:center;gap:12px;cursor:pointer;"><input type="checkbox" checked> Security alerts</label>
  </div>
</div>
`);

// KYC
const kyc = layout('KYC Verification', 'kyc-verification', `
<div class="stats-grid">
  <div class="stat-card"><div class="label">Verification Status</div><div class="value" style="color:#d97706;font-size:20px;">⏳ Pending</div></div>
  <div class="stat-card"><div class="label">Steps Completed</div><div class="value">2 / 4</div></div>
</div>
<div class="kyc-steps">
  <div class="kyc-step done"><div class="step-num">✓</div><div class="step-content"><div class="step-title">Email Verification</div><div class="step-desc">Confirm your email address</div></div><span class="step-status completed">Completed</span></div>
  <div class="kyc-step done"><div class="step-num">✓</div><div class="step-content"><div class="step-title">Personal Information</div><div class="step-desc">Full name, date of birth, address</div></div><span class="step-status completed">Completed</span></div>
  <div class="kyc-step"><div class="step-num">3</div><div class="step-content"><div class="step-title">Identity Document</div><div class="step-desc">Upload a valid government-issued ID (passport, national ID, or driver's license)</div></div><span class="step-status pending">Pending</span></div>
  <div class="kyc-step"><div class="step-num">4</div><div class="step-content"><div class="step-title">Selfie Verification</div><div class="step-desc">Take a selfie holding your ID document</div></div><span class="step-status pending">Pending</span></div>
</div>
`);

// Write all files
const pages = {
    'marketplace.html': marketplace,
    'commodities-marketplace.html': commodities,
    'portfolio.html': portfolio,
    'wallet.html': wallet,
    'cart.html': cart,
    'support.html': support,
    'settings.html': settings,
    'kyc.html': kyc,
    'index.html': marketplace, // same as marketplace for root
};

for (const [file, content] of Object.entries(pages)) {
    fs.writeFileSync(path.join(PLATFORM, file), content);
    console.log(`Created: ${file}`);
}

console.log('\nAll dashboard pages generated!');
