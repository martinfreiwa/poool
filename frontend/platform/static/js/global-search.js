/* global-search.js */
document.addEventListener('DOMContentLoaded', () => {
    const searchInput = document.getElementById('sidebar-search-input');
    if (!searchInput) return;

    // Create results container
    const resultsContainer = document.createElement('div');
    resultsContainer.className = 'global-search-results';
    // The search input is inside a sidebar-search div in sidebar.html
    const searchWrapper = searchInput.closest('.sidebar-search') || searchInput.parentNode;
    searchWrapper.style.position = 'relative';
    searchWrapper.appendChild(resultsContainer);

    const pages = [
        { title: 'Properties Marketplace', subtitle: 'Browse all available properties recorded on-chain', url: '/marketplace', icon: '🏠' },
        { title: 'Commodities Marketplace', subtitle: 'Invest in real-world agricultural & industrial assets', url: '/commodities-marketplace', icon: '🌽' },
        { title: 'Resale Market', subtitle: 'Secondary market for existing tokens', url: '/marketplace-secondary', icon: '🔄' },
        { title: 'Portfolio', subtitle: 'Real-time performance of your global RWA portfolio', url: '/portfolio', icon: '💼' },
        { title: 'My Wallet', subtitle: 'Manage your USDT balance and transactions', url: '/wallet', icon: '💳' },
        { title: 'Community Feed', subtitle: 'Insights and signals from the POOOL community', url: '/community/feed', icon: '👥' },
        { title: 'Leaderboard', subtitle: 'Top performers and community rankings', url: '/leaderboard', icon: '🏆' },
        { title: 'Referral Rewards', subtitle: 'Your lifetime earnings and level progression', url: '/rewards', icon: '🎁' },
        { title: 'Settings', subtitle: 'Security, privacy, and account management', url: '/settings', icon: '⚙️' },
        { title: 'Support', subtitle: 'Connect with our elite support collective', url: '/support', icon: '🎧' },
        { title: 'My Circles', subtitle: 'Private investment groups and DAO-gated chats', url: '/community/circles', icon: '🔘' },
        { title: 'Challenges', subtitle: 'Complete active quests to earn XP & Badges', url: '/community/challenges', icon: '✨' },
    ];

    let debounceTimer;

    searchInput.addEventListener('input', (e) => {
        const query = e.target.value.trim().toLowerCase();
        
        clearTimeout(debounceTimer);
        if (query.length < 2) {
            resultsContainer.classList.remove('active');
            return;
        }

        debounceTimer = setTimeout(async () => {
            const matchedPages = pages.filter(p => 
                p.title.toLowerCase().includes(query) || 
                p.subtitle.toLowerCase().includes(query)
            );

            try {
                const response = await fetch(`/api/assets/search?q=${encodeURIComponent(query)}`);
                const assets = await response.json();
                renderResults(matchedPages, assets, query);
            } catch (err) {
                console.error('Search failed', err);
                renderResults(matchedPages, [], query);
            }
        }, 200);
    });

    // Close on click outside
    document.addEventListener('click', (e) => {
        if (!searchInput.contains(e.target) && !resultsContainer.contains(e.target)) {
            resultsContainer.classList.remove('active');
        }
    });

    // Mirroring functionality for Marketplace (optional but helpful if user is on the page)
    // Actually the user wants them decoupled to avoid confusion, so let's stick to the dropdown
    // Unless they specifically press "Enter" without choosing a result
    
    searchInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            const query = e.target.value.trim();
            if (query.length > 0) {
                // If they press enter, default to marketplace search for assets
                window.location.href = `/marketplace?q=${encodeURIComponent(query)}`;
            }
        }
    });

    function renderResults(matchedPages, assets, query) {
        if (matchedPages.length === 0 && assets.length === 0) {
            resultsContainer.innerHTML = `<div class="search-no-results">No matches for "${query}".</div>`;
        } else {
            let html = '';
            
            if (matchedPages.length > 0) {
                html += '<div class="search-section-header">Pages & Tools</div>';
                matchedPages.forEach(p => {
                    html += createResultItem(p);
                });
            }

            if (assets.length > 0) {
                html += '<div class="search-section-header">Properties & Commodities</div>';
                assets.forEach(a => {
                    html += createResultItem(a);
                });
            }

            resultsContainer.innerHTML = html;
        }
        resultsContainer.classList.add('active');
    }

    function createResultItem(item) {
        return `
            <a href="${item.url}" class="search-result-item" onclick="document.querySelector('.global-search-results').classList.remove('active')">
                <div class="search-result-icon">${item.icon}</div>
                <div class="search-result-info">
                    <div class="search-result-title">${item.title}</div>
                    <div class="search-result-subtitle">${item.subtitle}</div>
                </div>
            </a>
        `;
    }
});
