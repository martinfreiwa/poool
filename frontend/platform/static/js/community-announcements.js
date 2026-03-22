document.addEventListener('DOMContentLoaded', () => {
    const annContainer = document.getElementById('community-announcements-container');
    const filtersContainer = document.getElementById('ann-category-filters');
    if (!annContainer || !filtersContainer) return;

    let currentCategory = '';

    function renderSkeleton() {
        annContainer.innerHTML = `
            <div class="ann-card" style="opacity: 0.6; pointer-events: none;">
                <div class="ann-card-header">
                    <div class="ann-icon" style="background:#eaecf0;"></div>
                    <div class="ann-meta">
                        <div style="height: 14px; width: 100px; background: #eaecf0; border-radius: 4px; margin-bottom: 4px;"></div>
                        <div style="height: 10px; width: 60px; background: #f2f4f7; border-radius: 4px;"></div>
                    </div>
                </div>
                <div style="height: 18px; width: 70%; background: #eaecf0; border-radius: 4px; margin: 12px 0;"></div>
                <div style="height: 14px; width: 90%; background: #eaecf0; border-radius: 4px; margin-bottom: 6px;"></div>
                <div style="height: 14px; width: 80%; background: #eaecf0; border-radius: 4px;"></div>
            </div>
            <div class="ann-card" style="opacity: 0.4; pointer-events: none;">
                <div class="ann-card-header">
                    <div class="ann-icon" style="background:#eaecf0;"></div>
                    <div class="ann-meta">
                        <div style="height: 14px; width: 100px; background: #eaecf0; border-radius: 4px; margin-bottom: 4px;"></div>
                    </div>
                </div>
                <div style="height: 18px; width: 50%; background: #eaecf0; border-radius: 4px; margin: 12px 0;"></div>
                <div style="height: 14px; width: 60%; background: #eaecf0; border-radius: 4px;"></div>
            </div>
        `;
    }

    function renderEmptyState() {
        annContainer.innerHTML = `
            <div style="text-align: center; padding: 40px 20px; color: #667085;">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#D0D5DD" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="margin-bottom:16px;">
                    <path d="M4 22h14a2 2 0 0 0 2-2V7.5L14.5 2H6a2 2 0 0 0-2 2v4"></path>
                    <polyline points="14 2 14 8 20 8"></polyline>
                    <path d="M2 15h10"></path>
                    <path d="M9 18l3-3-3-3"></path>
                </svg>
                <div style="font-size: 16px; font-weight: 500; color: #101828; margin-bottom: 4px;">No announcements found</div>
                <div style="font-size: 14px;">There are no announcements matching this category.</div>
            </div>
        `;
    }

    function timeAgo(dateString) {
        const date = new Date(dateString);
        const options = { year: 'numeric', month: 'long', day: 'numeric' };
        return date.toLocaleDateString(undefined, options);
    }

    async function loadAnnouncements() {
        renderSkeleton();
        try {
            const url = currentCategory ? `/api/community/feed?category=${currentCategory}` : `/api/community/feed`;
            const res = await fetch(url);
            if (!res.ok) throw new Error("Failed to fetch announcements");
            const posts = await res.json();
            
            if (posts.length === 0) {
                renderEmptyState();
                return;
            }

            annContainer.innerHTML = '';
            for (const p of posts) {
                // Determine icon and color based on category
                let iconClass = 'ann-icon--platform';
                let iconContent = `<circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />`;
                let displayCategory = 'Platform Update';

                if (p.category === 'new_commodities') {
                    iconClass = 'ann-icon--commodity';
                    displayCategory = 'New Commodity';
                    iconContent = `<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />`;
                } else if (p.category === 'dividends') {
                    iconClass = 'ann-icon--dividend';
                    displayCategory = 'Dividend Update';
                    iconContent = `<line x1="12" y1="1" x2="12" y2="23" /><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />`;
                } else if (p.category === 'market_news') {
                    iconClass = 'ann-icon--market';
                    displayCategory = 'Market News';
                    iconContent = `<polyline points="22 7 13.5 15.5 8.5 10.5 2 17" /><polyline points="16 7 22 7 22 13" />`;
                }

                // Build card using DOM construction (FIX-F3: XSS-safe)
                const card = document.createElement('div');
                card.className = `ann-card ${p.is_pinned ? 'ann-card--pinned' : ''}`;

                // Pinned badge (static HTML, safe)
                let pinnedHtml = '';
                if (p.is_pinned) {
                    pinnedHtml = `<div class="ann-card-pin"><svg class="poool-icon-custom" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#03FF88" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="17" x2="12" y2="22" /><path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z" /></svg>Pinned</div>`;
                }

                // Build the header and footer as static HTML (no user content)
                const headerHtml = `
                    ${pinnedHtml}
                    <div class="ann-card-header">
                        <div class="ann-icon ${iconClass}">
                            <svg class="poool-icon-custom" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#03FF88" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                ${iconContent}
                            </svg>
                        </div>
                        <div class="ann-meta">
                            <span class="ann-category">${displayCategory}</span>
                            <span class="ann-date">${timeAgo(p.created_at)}</span>
                        </div>
                    </div>`;

                const footerHtml = `
                    <div class="ann-footer">
                        <span class="ann-read-more" style="cursor:pointer;">View in Feed →</span>
                        <span class="ann-reactions">🔥 ${p.reaction_count || 0} · 💬 ${p.comment_count || 0}</span>
                    </div>`;

                // Set static parts via innerHTML (these only contain developer-controlled strings)
                card.innerHTML = headerHtml;

                // Content body — use textContent (SAFE: XSS prevention)
                const bodyDiv = document.createElement('div');
                bodyDiv.className = 'ann-body';
                bodyDiv.textContent = p.content; // SAFE: textContent escapes HTML
                card.appendChild(bodyDiv);

                // Append footer
                const footerWrapper = document.createElement('div');
                footerWrapper.innerHTML = footerHtml;
                const footerEl = footerWrapper.firstElementChild;
                // Attach click handler to "View in Feed" link
                const readMore = footerEl.querySelector('.ann-read-more');
                if (readMore) {
                    readMore.addEventListener('click', () => {
                        const feedTab = document.querySelector('[data-tab=community-feed-tab]');
                        if (feedTab) switchCommunityTab(feedTab);
                    });
                }
                card.appendChild(footerEl);

                annContainer.appendChild(card);
            }
        } catch (e) {
            console.error(e);
            annContainer.innerHTML = `<div style="padding: 24px; color: #D92D20; text-align: center;">Failed to load announcements. Please try again.</div>`;
        }
    }

    // Attach click events
    filtersContainer.querySelectorAll('.ann-filter-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            filtersContainer.querySelectorAll('.ann-filter-btn').forEach(b => b.classList.remove('active'));
            e.target.classList.add('active');
            
            currentCategory = e.target.getAttribute('data-category');
            loadAnnouncements();
        });
    });

    // Initial load
    loadAnnouncements();
});
