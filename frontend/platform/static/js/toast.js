(function() {
    window.showPooolToast = function(title, message, type) {
        // Resolve type from params
        let finalType = type || "info";
        let finalTitle = title;
        let finalMessage = message;

        // If title is null, but message is present, it means it's a 2-argument call
        if (title === null && message) {
           finalTitle = finalType.charAt(0).toUpperCase() + finalType.slice(1);
           finalMessage = message;
        }

        // Extract value if objects were accidentally passed due to destructuring or events
        if (typeof finalMessage === "object" && finalMessage !== null) {
            finalMessage = finalMessage.message || JSON.stringify(finalMessage);
        }

        // Create container if it doesn't exist
        let container = document.getElementById('poool-toast-container');
        if (!container) {
            container = document.createElement('div');
            container.id = 'poool-toast-container';
            container.style.cssText = 'position:fixed;bottom:24px;right:24px;z-index:99999;display:flex;flex-direction:column;gap:12px;width:100%;max-width:380px;pointer-events:none;';
            document.body.appendChild(container);
            
            // Inject keyframes for slideIn/fadeOut
            const style = document.createElement('style');
            style.textContent = `
                @keyframes pooolToastSlideIn {
                    from { transform: translateX(100%); opacity: 0; }
                    to { transform: translateX(0); opacity: 1; }
                }
                @keyframes pooolToastFadeOut {
                    from { opacity: 1; transform: scale(1); }
                    to { opacity: 0; transform: scale(0.95); }
                }
                .poool-toast-card {
                    pointer-events: auto;
                    background: #FFFFFF;
                    border: 1px solid #E5E7EB;
                    border-radius: 12px;
                    padding: 16px 20px;
                    box-shadow: 0 1px 2px rgba(10, 13, 18, 0.05), 0 12px 32px rgba(10, 13, 18, 0.10);
                    font-family: var(--ds-font, 'TT Norms Pro', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif);
                    animation: pooolToastSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
                    position: relative;
                    overflow: hidden;
                    display: flex;
                    align-items: flex-start;
                    gap: 12px;
                    transition: box-shadow 0.2s ease, transform 0.2s ease;
                    cursor: pointer;
                }
                /* Brand gradient bar at the top — matches every card on the dashboard. */
                .poool-toast-card::before {
                    content: '';
                    display: block;
                    position: absolute;
                    inset: 0 0 auto 0;
                    height: 4px;
                    background: linear-gradient(90deg, #0000FF 0%, #03FF88 100%);
                    border-radius: 12px 12px 0 0;
                    z-index: 1;
                    pointer-events: none;
                }
                .poool-toast-card[data-type="error"]::before {
                    background: linear-gradient(90deg, #B42318 0%, #F04438 100%);
                }
                .poool-toast-card[data-type="warning"]::before {
                    background: linear-gradient(90deg, #B45309 0%, #F79009 100%);
                }
                .poool-toast-card:hover {
                    transform: translateY(-2px);
                    box-shadow: 0 1px 2px rgba(10, 13, 18, 0.05), 0 16px 40px rgba(10, 13, 18, 0.14);
                }
                .poool-toast-card__title {
                    font-weight: 700;
                    font-size: 14px;
                    color: #101828;
                    margin-bottom: 2px;
                    letter-spacing: -0.01em;
                }
                .poool-toast-card__message {
                    font-size: 13px;
                    color: #475467;
                    line-height: 1.45;
                    word-wrap: break-word;
                }
                .poool-toast-card__icon {
                    flex-shrink: 0;
                    width: 28px;
                    height: 28px;
                    border-radius: 8px;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    margin-top: 2px;
                }
                .poool-toast-card__close {
                    flex-shrink: 0;
                    margin-top: 2px;
                    cursor: pointer;
                    color: #98A2B3;
                    transition: color 0.15s ease;
                }
                .poool-toast-card__close:hover { color: #475467; }
            `;
            document.head.appendChild(style);
        }

        const toast = document.createElement('div');
        toast.className = 'poool-toast-card';

        let iconBg, iconColor, iconSvg, normalizedType;
        if (finalType === 'success' || finalType.toLowerCase().includes('success')) {
            normalizedType = 'success';
            iconBg = '#ECFDF3';
            iconColor = '#039855';
            iconSvg = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>`;
        } else if (finalType === 'error' || finalType.toLowerCase().includes('fail')) {
            normalizedType = 'error';
            iconBg = '#FEF3F2';
            iconColor = '#B42318';
            iconSvg = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>`;
        } else if (finalType === 'warning') {
            normalizedType = 'warning';
            iconBg = '#FFFAEB';
            iconColor = '#B45309';
            iconSvg = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>`;
        } else {
            normalizedType = 'info';
            iconBg = '#F5F8FF';
            iconColor = '#0000FF';
            iconSvg = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line></svg>`;
        }
        toast.setAttribute('data-type', normalizedType);

        const iconHtml = `<div class="poool-toast-card__icon" style="background:${iconBg};color:${iconColor};">${iconSvg}</div>`;

        let contentHtml = `<div style="flex:1;min-width:0;">`;
        if (finalTitle && finalTitle !== finalMessage) {
            contentHtml += `<div class="poool-toast-card__title">${finalTitle}</div>`;
        }
        contentHtml += `<div class="poool-toast-card__message">${finalMessage}</div>`;
        contentHtml += `</div>`;

        contentHtml += `<div class="poool-toast-card__close" aria-label="Dismiss"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg></div>`;

        toast.innerHTML = iconHtml + contentHtml;
        container.appendChild(toast);

        // Auto remove
        setTimeout(() => {
            if (toast.parentElement) {
                toast.style.animation = 'pooolToastFadeOut 0.3s forwards ease';
                setTimeout(() => {
                    toast.remove();
                    if (container.childNodes.length === 0) container.remove();
                }, 300);
            }
        }, 5000);
        
        // Click to dismiss
        toast.addEventListener('click', () => {
            toast.style.animation = 'pooolToastFadeOut 0.2s forwards ease';
            setTimeout(() => {
                toast.remove();
                if (container.childNodes.length === 0) container.remove();
            }, 200);
        });
    };

    window.showToast = function(message, type) {
        return window.showPooolToast(null, message, type || 'info');
    };
})();
