/* global-search.js */
document.addEventListener('DOMContentLoaded', () => {
    const isDeveloperPage = window.location.pathname.startsWith('/developer');
    const pages = isDeveloperPage
        ? [
            { title: 'Developer Dashboard', subtitle: 'PAGE', url: '/developer/dashboard', iconType: 'chart' },
            { title: 'My Assets', subtitle: 'PAGE', url: '/developer/assets', iconType: 'home' },
            { title: 'My Submissions', subtitle: 'PAGE', url: '/developer/submissions', iconType: 'file' },
            { title: 'Add Asset', subtitle: 'PAGE', url: '/developer/add-asset', iconType: 'plus' },
            { title: 'Settings', subtitle: 'PAGE', url: '/developer/settings', iconType: 'settings' },
            { title: 'Support', subtitle: 'PAGE', url: '/developer/support', iconType: 'support' },
        ]
        : [
            { title: 'Properties Marketplace', subtitle: 'PAGE', url: '/marketplace', iconType: 'home' },
            { title: 'Commodities Marketplace', subtitle: 'PAGE', url: '/commodities-marketplace', iconType: 'commodity' },
            { title: 'Resale Market', subtitle: 'PAGE', url: '/marketplace-secondary', iconType: 'chart' },
            { title: 'Portfolio', subtitle: 'PAGE', url: '/portfolio', iconType: 'portfolio' },
            { title: 'My Wallet', subtitle: 'PAGE', url: '/wallet', iconType: 'wallet' },
            { title: 'Community Feed', subtitle: 'PAGE', url: '/community/feed', iconType: 'community' },
            { title: 'Leaderboard', subtitle: 'PAGE', url: '/leaderboard', iconType: 'award' },
            { title: 'Referral Rewards', subtitle: 'PAGE', url: '/rewards', iconType: 'star' },
            { title: 'Settings', subtitle: 'PAGE', url: '/settings', iconType: 'settings' },
            { title: 'Support', subtitle: 'PAGE', url: '/support', iconType: 'support' },
            { title: 'My Circles', subtitle: 'PAGE', url: '/community/circles', iconType: 'community' },
            { title: 'Challenges', subtitle: 'PAGE', url: '/community/challenges', iconType: 'star' },
        ];

    const inputs = document.querySelectorAll('[data-global-search-input]');
    inputs.forEach((input) => initGlobalSearchInput(input));

    document.addEventListener('keydown', (e) => {
        if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
            const firstInput = document.querySelector('[data-global-search-input]');
            if (!firstInput) return;
            e.preventDefault();
            firstInput.focus();
            firstInput.select();
        }
    });

    function initGlobalSearchInput(searchInput) {
        const resultsContainer = document.createElement('div');
        resultsContainer.className = 'global-search-results';

        const searchWrapper =
            searchInput.closest('.settings-topbar__search') ||
            searchInput.closest('.sidebar__search-wrapper') ||
            searchInput.closest('.sidebar-search-wrapper') ||
            searchInput.closest('.sidebar__search') ||
            searchInput.closest('.sidebar-search') ||
            searchInput.parentNode;
        if (!searchWrapper) return;

        searchWrapper.style.position = 'relative';
        searchWrapper.appendChild(resultsContainer);

        const clearButton = document.createElement('button');
        clearButton.type = 'button';
        clearButton.className = 'global-search-clear';
        clearButton.setAttribute('aria-label', 'Clear search');
        clearButton.appendChild(createClearIcon());
        searchInput.insertAdjacentElement('afterend', clearButton);

        let debounceTimer;

        searchInput.addEventListener('input', (e) => {
            const query = e.target.value.trim().toLowerCase();
            syncClearButton(searchInput, clearButton);

            clearTimeout(debounceTimer);
            if (query.length < 2) {
                resultsContainer.classList.remove('active');
                return;
            }

            debounceTimer = setTimeout(async () => {
                const matchedPages = pages.filter((p) =>
                    p.title.toLowerCase().includes(query) ||
                    p.subtitle.toLowerCase().includes(query)
                );

                try {
                    const response = await fetch(`/api/assets/search?q=${encodeURIComponent(query)}`);
                    const assets = response.ok ? await response.json() : [];
                    renderResults(resultsContainer, matchedPages, assets, query);
                } catch (err) {
                    console.error('Search failed', err);
                    renderResults(resultsContainer, matchedPages, [], query);
                }
            }, 200);
        });

        searchInput.addEventListener('keydown', (e) => {
            if (e.key !== 'Enter') return;

            const query = e.target.value.trim();
            if (!query) return;

            e.preventDefault();
            const firstResult = resultsContainer.querySelector('.search-result-item');
            if (firstResult) {
                window.location.href = firstResult.getAttribute('href');
                return;
            }

            window.location.href = isDeveloperPage
                ? `/developer/assets?q=${encodeURIComponent(query)}`
                : `/marketplace?q=${encodeURIComponent(query)}`;
        });

        clearButton.addEventListener('click', () => {
            searchInput.value = '';
            resultsContainer.classList.remove('active');
            syncClearButton(searchInput, clearButton);
            searchInput.focus();
        });

        document.addEventListener('click', (e) => {
            if (
                !searchInput.contains(e.target) &&
                !clearButton.contains(e.target) &&
                !resultsContainer.contains(e.target)
            ) {
                resultsContainer.classList.remove('active');
            }
        });

        syncClearButton(searchInput, clearButton);
    }

    function syncClearButton(searchInput, clearButton) {
        clearButton.classList.toggle('is-visible', searchInput.value.length > 0);
    }

    function renderResults(resultsContainer, matchedPages, assets, query) {
        const safeAssets = Array.isArray(assets) ? assets : [];
        resultsContainer.replaceChildren();

        if (matchedPages.length === 0 && safeAssets.length === 0) {
            const noResults = document.createElement('div');
            noResults.className = 'search-no-results';
            noResults.textContent = `No matches for "${query}".`;
            resultsContainer.appendChild(noResults);
        } else {
            if (matchedPages.length > 0) {
                resultsContainer.appendChild(createSectionHeader('Pages & Tools'));
                matchedPages.forEach((p) => {
                    resultsContainer.appendChild(createResultItem(p, resultsContainer));
                });
            }

            if (safeAssets.length > 0) {
                resultsContainer.appendChild(createSectionHeader('Properties & Commodities'));
                safeAssets.forEach((a) => {
                    resultsContainer.appendChild(createResultItem(a, resultsContainer));
                });
            }
        }

        resultsContainer.classList.add('active');
    }

    function createSectionHeader(label) {
        const header = document.createElement('div');
        header.className = 'search-section-header';
        header.textContent = label;
        return header;
    }

    function createResultItem(item, resultsContainer) {
        const link = document.createElement('a');
        link.href = item.url || '#';
        link.className = 'search-result-item';
        link.addEventListener('click', () => {
            resultsContainer.classList.remove('active');
        });

        const icon = document.createElement('div');
        icon.className = item.image_url ? 'search-result-icon search-result-icon--image' : 'search-result-icon';
        if (item.image_url) {
            const image = document.createElement('img');
            image.src = item.image_url;
            image.alt = '';
            image.loading = 'lazy';
            image.addEventListener('error', () => {
                icon.className = 'search-result-icon';
                icon.replaceChildren(createIcon(inferIconType(item)));
            }, { once: true });
            icon.appendChild(image);
        } else {
            icon.appendChild(createIcon(item.iconType || inferIconType(item)));
        }

        const info = document.createElement('div');
        info.className = 'search-result-info';

        const title = document.createElement('div');
        title.className = 'search-result-title';
        title.textContent = item.title || 'Untitled';

        const subtitle = document.createElement('div');
        subtitle.className = 'search-result-subtitle';
        subtitle.textContent = getResultLabel(item);

        const arrow = document.createElement('span');
        arrow.className = 'search-result-arrow';
        arrow.setAttribute('aria-hidden', 'true');
        arrow.appendChild(createChevronIcon());

        info.append(title, subtitle);
        link.append(icon, info, arrow);
        return link;
    }

    function getAssetLabel(item) {
        if (item.url && item.url.startsWith('/commodity/')) return 'COMMODITY';
        return 'REAL_ESTATE';
    }

    function getResultLabel(item) {
        if (item.url && (item.url.startsWith('/property/') || item.url.startsWith('/commodity/'))) {
            return getAssetLabel(item);
        }
        return item.subtitle || 'PAGE';
    }

    function inferIconType(item) {
        if (item.url && item.url.startsWith('/commodity/')) return 'commodity';
        if (item.url && item.url.startsWith('/property/')) return 'home';
        return 'search';
    }

    function createIcon(type) {
        const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        svg.setAttribute('viewBox', '0 0 24 24');
        svg.setAttribute('fill', 'none');
        svg.setAttribute('stroke', 'currentColor');
        svg.setAttribute('stroke-width', '2');
        svg.setAttribute('stroke-linecap', 'round');
        svg.setAttribute('stroke-linejoin', 'round');
        const paths = {
            award: ['M12 15l-3.5 2 1-4-3-2.8 4.1-.3L12 6l1.4 3.9 4.1.3-3 2.8 1 4z'],
            chart: ['M3 17l6-6 4 4 8-8', 'M14 7h7v7'],
            commodity: ['M12 3v18', 'M7 7c3 0 5 2 5 5', 'M17 7c-3 0-5 2-5 5'],
            community: ['M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2', 'M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8', 'M22 21v-2a4 4 0 0 0-3-3.9', 'M16 3.1a4 4 0 0 1 0 7.8'],
            file: ['M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z', 'M14 2v6h6', 'M16 13H8', 'M16 17H8'],
            home: ['M3 10.5L12 3l9 7.5', 'M5 10v10h14V10', 'M9 20v-6h6v6'],
            plus: ['M12 5v14', 'M5 12h14'],
            portfolio: ['M4 7h16v13H4z', 'M9 7V5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2'],
            search: ['M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16', 'M21 21l-4.35-4.35'],
            settings: ['M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7', 'M19.4 15a1.8 1.8 0 0 0 .36 1.98l.04.04a2 2 0 1 1-2.83 2.83l-.04-.04A1.8 1.8 0 0 0 15 19.4a1.8 1.8 0 0 0-1 .6 1.8 1.8 0 0 0-.4 1.1V21a2 2 0 1 1-4 0v-.06A1.8 1.8 0 0 0 8.6 19.4a1.8 1.8 0 0 0-1.98.36l-.04.04a2 2 0 1 1-2.83-2.83l.04-.04A1.8 1.8 0 0 0 4.6 15a1.8 1.8 0 0 0-.6-1 1.8 1.8 0 0 0-1.1-.4H3a2 2 0 1 1 0-4h.06A1.8 1.8 0 0 0 4.6 8.6a1.8 1.8 0 0 0-.36-1.98l-.04-.04a2 2 0 1 1 2.83-2.83l.04.04A1.8 1.8 0 0 0 9 4.6a1.8 1.8 0 0 0 1-.6 1.8 1.8 0 0 0 .4-1.1V3a2 2 0 1 1 4 0v.06A1.8 1.8 0 0 0 15.4 4.6a1.8 1.8 0 0 0 1.98-.36l.04-.04a2 2 0 1 1 2.83 2.83l-.04.04A1.8 1.8 0 0 0 19.4 9c.38.17.72.38 1 .6.3.27.6.68.6 1.1V11a2 2 0 1 1 0 4h-.06a1.8 1.8 0 0 0-1.54 0z'],
            star: ['M12 2l2.9 6 6.6.9-4.8 4.7 1.1 6.5L12 17l-5.8 3.1 1.1-6.5-4.8-4.7 6.6-.9z'],
            support: ['M21 11.5a8.4 8.4 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.4 8.4 0 0 1-3.8-.9L3 21l1.9-5.7a8.4 8.4 0 0 1-.9-3.8 8.5 8.5 0 1 1 17 0z'],
            wallet: ['M3 7h18v13H3z', 'M3 7l3-4h12l3 4', 'M17 13h2'],
        };
        (paths[type] || paths.search).forEach((d) => {
            const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
            path.setAttribute('d', d);
            svg.appendChild(path);
        });
        return svg;
    }

    function createClearIcon() {
        const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        svg.setAttribute('width', '14');
        svg.setAttribute('height', '14');
        svg.setAttribute('viewBox', '0 0 14 14');
        svg.setAttribute('fill', 'none');
        svg.setAttribute('aria-hidden', 'true');
        const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
        path.setAttribute('d', 'M10.5 3.5L3.5 10.5M3.5 3.5L10.5 10.5');
        path.setAttribute('stroke', 'currentColor');
        path.setAttribute('stroke-width', '1.75');
        path.setAttribute('stroke-linecap', 'round');
        svg.appendChild(path);
        return svg;
    }

    function createChevronIcon() {
        const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        svg.setAttribute('viewBox', '0 0 20 20');
        svg.setAttribute('fill', 'none');
        const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
        path.setAttribute('d', 'M7.5 5L12.5 10L7.5 15');
        path.setAttribute('stroke', 'currentColor');
        path.setAttribute('stroke-width', '1.75');
        path.setAttribute('stroke-linecap', 'round');
        path.setAttribute('stroke-linejoin', 'round');
        svg.appendChild(path);
        return svg;
    }
});
