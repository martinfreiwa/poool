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
                    background: #181D27; /* Dark modern background */
                    border: 1px solid #344054;
                    border-radius: 12px;
                    padding: 16px 20px;
                    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.15), 0 0 0 1px rgba(255, 255, 255, 0.05);
                    font-family: var(--ds-font, 'TT Norms Pro', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif);
                    animation: pooolToastSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
                    position: relative;
                    overflow: hidden;
                    display: flex;
                    align-items: flex-start;
                    gap: 12px;
                    transition: all 0.2s ease;
                    cursor: pointer;
                }
                /* Glossy sheen overlay */
                .poool-toast-card::after {
                    content: '';
                    position: absolute;
                    top: 0; left: 0; right: 0; bottom: 0;
                    background: linear-gradient(135deg, rgba(255,255,255,0.08) 0%, rgba(255,255,255,0) 100%);
                    pointer-events: none;
                    border-radius: 12px;
                }
                .poool-toast-card:hover {
                    transform: translateY(-2px);
                    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.2), 0 0 0 1px rgba(255, 255, 255, 0.08);
                }
            `;
            document.head.appendChild(style);
        }

        const toast = document.createElement('div');
        toast.className = 'poool-toast-card';
        
        let iconHtml = '';
        if (finalType === 'success' || finalType.toLowerCase().includes('success')) {
            toast.style.borderLeft = '4px solid #03FF88'; /* Neon Green for success */
            iconHtml = `<div style="flex-shrink:0;width:24px;height:24px;border-radius:50%;background:rgba(3,255,136,0.1);display:flex;align-items:center;justify-content:center;color:#03FF88;">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
            </div>`;
        } else if (finalType === 'error' || finalType.toLowerCase().includes('fail')) {
            toast.style.borderLeft = '4px solid #F04438'; /* Red for errors */
            iconHtml = `<div style="flex-shrink:0;width:24px;height:24px;border-radius:50%;background:rgba(240,68,56,0.1);display:flex;align-items:center;justify-content:center;color:#F04438;">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
            </div>`;
        } else if (finalType === 'warning') {
            toast.style.borderLeft = '4px solid #F79009'; /* Orange for warning */
            iconHtml = `<div style="flex-shrink:0;width:24px;height:24px;border-radius:50%;background:rgba(247,144,9,0.1);display:flex;align-items:center;justify-content:center;color:#F79009;">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>
            </div>`;
        } else {
            toast.style.borderLeft = '4px solid #0000FF'; /* Electric Blue for info */
            iconHtml = `<div style="flex-shrink:0;width:24px;height:24px;border-radius:50%;background:rgba(0,0,255,0.1);display:flex;align-items:center;justify-content:center;color:#0000FF;">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line></svg>
            </div>`;
            toast.style.boxShadow = '0 12px 32px rgba(0, 0, 255, 0.15), 0 0 0 1px rgba(255, 255, 255, 0.05)';
        }
        
        let contentHtml = `<div style="flex:1;min-width:0;">`;
        if (finalTitle && finalTitle !== finalMessage) {
            contentHtml += `<div style="font-weight:600;font-size:15px;color:#FFFFFF;margin-bottom:4px;letter-spacing:-0.01em;">${finalTitle}</div>`;
        }
        contentHtml += `<div style="font-size:14px;color:#98A2B3;line-height:1.4;word-wrap:break-word;">${finalMessage}</div>`;
        contentHtml += `</div>`;
        
        // Add subtle close button
        contentHtml += `<div style="flex-shrink:0;opacity:0.6;margin-top:2px;cursor:pointer;"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#98A2B3" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg></div>`;

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
