/**
 * POOOL Platform - Overlays, Modals, and Toasts JS
 * Vanilla JS implementation managing dialog events and toast lifecycles.
 */

window.overlays = (function() {
    
    // --- Modals & Panels ---

    /**
     * Show a modal or slide-out panel by its ID.
     * Uses the native <dialog> API showModal() to lock the background.
     */
    function showModal(id) {
        const dialog = document.getElementById(id);
        if (dialog) {
            dialog.removeAttribute('closing');
            dialog.showModal();
            // Prevent body scrolling behind the modal
            document.body.style.overflow = 'hidden';
            
            // Close on backdrop click (click outside)
            dialog.addEventListener('click', _handleBackdropClick);
        }
    }

    /**
     * Show a Slide Out Panel. 
     * Uses the exact same dialog logic but named semantically for the API.
     */
    function showPanel(id) {
        showModal(id);
    }

    /**
     * Close a modal or slide-out panel with a smooth animation.
     */
    function closeModal(id) {
        const dialog = document.getElementById(id);
        if (dialog && dialog.open) {
            // Apply closing state for CSS animation targeting
            dialog.setAttribute('closing', '');
            
            // Wait for animation to finish before actually removing from layout
            dialog.addEventListener('animationend', function handler() {
                dialog.close();
                dialog.removeAttribute('closing');
                dialog.removeEventListener('animationend', handler);
                dialog.removeEventListener('click', _handleBackdropClick);
                document.body.style.overflow = ''; // Restore overall scroll
            }, { once: true });
        }
    }

    /**
     * Internal handler to check if the user clicked on the ::backdrop pseudo-element
     */
    function _handleBackdropClick(event) {
        const dialog = event.currentTarget;
        const rect = dialog.getBoundingClientRect();
        
        // If the click is outside the actual modal bounds (meaning on the backdrop)
        const isInDialog = (rect.top <= event.clientY && event.clientY <= rect.top + rect.height
          && rect.left <= event.clientX && event.clientX <= rect.left + rect.width);
        
        if (!isInDialog) {
            closeModal(dialog.id);
        }
    }


    // --- Toasts ---
    
    let toastContainer = null;

    /**
     * Ensures the container for floating toasts exists in the DOM.
     */
    function _ensureToastContainer() {
        if (!toastContainer) {
            toastContainer = document.createElement('div');
            toastContainer.className = 'ds-toast-container';
            document.body.appendChild(toastContainer);
        }
    }

    /**
     * Show a temporary toast notification using the holographic design system.
     * @param {string} type 'success', 'error', 'info', or 'warning'
     * @param {string} title Main toast heading
     * @param {string} message Descriptive text
     * @param {number} duration Milliseconds until auto-dismiss (default 5000ms)
     */
    function showToast(type, title, message, duration = 5000) {
        _ensureToastContainer();

        const toast = document.createElement('div');
        toast.className = `ds-toast ds-toast--${type}`;
        
        // SVG Icons based on type
        let iconSvg = '';
        switch(type) {
            case 'success':
                iconSvg = `<svg viewBox="0 0 24 24" fill="none" class="ds-toast__icon" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>`;
                break;
            case 'error':
                iconSvg = `<svg viewBox="0 0 24 24" fill="none" class="ds-toast__icon" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="15" y1="9" x2="9" y2="15"></line><line x1="9" y1="9" x2="15" y2="15"></line></svg>`;
                break;
            case 'info':
                iconSvg = `<svg viewBox="0 0 24 24" fill="none" class="ds-toast__icon" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line></svg>`;
                break;
            case 'warning':
                iconSvg = `<svg viewBox="0 0 24 24" fill="none" class="ds-toast__icon" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>`;
                break;
        }

        // Compose HTML
        toast.innerHTML = `
            ${iconSvg}
            <div class="ds-toast__content">
                <div class="ds-toast__title">${title}</div>
                <div class="ds-toast__desc">${message}</div>
            </div>
            <button class="ds-toast__close" aria-label="Dismiss Notification">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
            </button>
        `;

        // Render into container
        toastContainer.appendChild(toast);

        // Bind interactive close button
        const closeBtn = toast.querySelector('.ds-toast__close');
        
        const dismiss = () => {
            if (toast.classList.contains('closing')) return;
            toast.classList.add('closing');
            
            // Clean DOM element upon animation completion
            toast.addEventListener('animationend', () => {
                toast.remove();
            }, { once: true });
        };

        closeBtn.addEventListener('click', dismiss);

        // Standard Auto-dismiss
        if (duration > 0) {
            setTimeout(dismiss, duration);
        }
    }

    // --- Demo Interactions for Template Rendering ---
    
    /**
     * Demo utility to fake an API request for Trading Confrimation flows.
     */
    function simulateTrade() {
        // Toggle UI loader
        const modal = document.getElementById('trading-confirmation-modal');
        const loader = modal.querySelector('.ds-modal__loader');
        const confirmBtn = modal.querySelector('#trading-confirm-btn');
        const cancelBtn = modal.querySelector('#trading-cancel-btn');
        
        if (loader) {
            loader.classList.add('active');
            confirmBtn.disabled = true;
            cancelBtn.disabled = true;
        }

        // Simulate a 1.5s transaction
        setTimeout(() => {
            closeModal('trading-confirmation-modal');
            
            // Clean up state back to normal
            if (loader) {
                loader.classList.remove('active');
                confirmBtn.disabled = false;
                cancelBtn.disabled = false;
            }
            
            // Fire success toast
            showToast('success', 'Order Executed', 'Your buy order for 5 shares of The London Pearl has been successfully placed.', 6000);
        }, 1500);
    }

    // Explicit public api
    return {
        showModal,
        closeModal,
        showPanel, // alias
        closePanel: closeModal, // alias
        showToast,
        simulateTrade
    };

})();
