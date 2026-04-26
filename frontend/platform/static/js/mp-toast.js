/**
 * Marketplace Toast & Button Utility
 * Shared across all 12 marketplace admin pages.
 */
(function () {
  // Ensure toast container exists
  function getContainer() {
    let c = document.querySelector('.mp-toast-container');
    if (!c) {
      c = document.createElement('div');
      c.className = 'mp-toast-container';
      document.body.appendChild(c);
    }
    return c;
  }

  /**
   * Show a toast notification.
   * @param {string} message 
   * @param {'success'|'error'|'warning'|'info'} type 
   * @param {number} duration ms
   */
  window.mpToast = function (message, type = 'success', duration = 3000) {
    const icons = {
      success: '✅',
      error: '❌',
      warning: '⚠️',
      info: 'ℹ️'
    };
    const container = getContainer();
    const toast = document.createElement('div');
    toast.className = `mp-toast mp-toast--${type}`;
    const icon = document.createElement('span');
    icon.className = 'mp-toast-icon';
    icon.textContent = icons[type] || '✅';
    const text = document.createElement('span');
    text.className = 'mp-toast-message';
    text.textContent = message;
    toast.append(icon, text);
    container.appendChild(toast);

    setTimeout(() => {
      toast.classList.add('mp-toast-out');
      toast.addEventListener('animationend', () => toast.remove());
    }, duration);
  };

  /**
   * Set a button into loading state, then call callback, then restore + toast.
   * @param {HTMLElement} btn 
   * @param {string} successMsg 
   * @param {number} delay Fake latency in ms
   * @param {Function} [callback] Optional extra logic
   */
  window.mpButtonAction = function (btn, successMsg, delay = 1000, callback) {
    if (btn.classList.contains('mp-btn-loading')) return; // prevent double-click
    const originalHTML = btn.innerHTML;
    btn.classList.add('mp-btn-loading');
    btn.innerHTML = `<span class="mp-btn-text">${originalHTML}</span>`;

    setTimeout(() => {
      btn.classList.remove('mp-btn-loading');
      btn.innerHTML = originalHTML;
      if (callback) callback();
      mpToast(successMsg, 'success');
    }, delay);
  };

  /**
   * Opens a glassmorphism modal.
   * @param {object} opts
   * @param {string} opts.title
   * @param {string} opts.subtitle
   * @param {string} opts.bodyHTML Static developer-controlled HTML.
   * @param {Node} opts.bodyNode Dynamic body content built with DOM APIs.
   * @param {Function} opts.onConfirm  Called with modal element for data extraction.
   * @param {string} [opts.confirmLabel='Confirm']
   * @param {string} [opts.confirmClass='admin-btn--danger']
   */
  window.mpModal = function (opts) {
    const previouslyFocused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const overlay = document.createElement('div');
    overlay.className = 'mp-modal-overlay';

    const modal = document.createElement('div');
    modal.className = 'mp-modal';
    modal.setAttribute('role', 'dialog');
    modal.setAttribute('aria-modal', 'true');

    const title = document.createElement('h2');
    title.className = 'mp-modal-title';
    title.id = `mp-modal-title-${Date.now()}`;
    title.textContent = opts.title || 'Confirm';
    modal.setAttribute('aria-labelledby', title.id);

    const subtitle = document.createElement('p');
    subtitle.className = 'mp-modal-subtitle';
    subtitle.textContent = opts.subtitle || '';

    const body = document.createElement('div');
    body.className = 'mp-modal-body';
    if (opts.bodyNode instanceof Node) {
      body.appendChild(opts.bodyNode);
    } else if (opts.bodyHTML) {
      body.innerHTML = opts.bodyHTML;
    }

    const actions = document.createElement('div');
    actions.className = 'mp-modal-actions';

    const cancelBtn = document.createElement('button');
    cancelBtn.type = 'button';
    cancelBtn.className = 'admin-btn admin-btn--secondary mp-modal-cancel';
    cancelBtn.textContent = 'Cancel';

    const confirmBtn = document.createElement('button');
    confirmBtn.type = 'button';
    confirmBtn.className = `admin-btn ${opts.confirmClass || 'admin-btn--danger'} mp-modal-confirm`;
    confirmBtn.textContent = opts.confirmLabel || 'Confirm';

    actions.append(cancelBtn, confirmBtn);
    modal.append(title, subtitle, body, actions);
    overlay.appendChild(modal);

    document.body.appendChild(overlay);

    const focusableSelector = [
      'a[href]',
      'button:not([disabled])',
      'textarea:not([disabled])',
      'input:not([disabled])',
      'select:not([disabled])',
      '[tabindex]:not([tabindex="-1"])'
    ].join(',');

    const close = () => {
      document.removeEventListener('keydown', onKeydown);
      overlay.classList.add('closing');
      overlay.addEventListener('animationend', () => {
        overlay.remove();
        previouslyFocused?.focus?.();
      }, { once: true });
    };

    const onKeydown = (event) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        close();
        return;
      }

      if (event.key !== 'Tab') return;

      const focusable = Array.from(overlay.querySelectorAll(focusableSelector))
        .filter((el) => el instanceof HTMLElement && el.offsetParent !== null);
      if (!focusable.length) {
        event.preventDefault();
        modal.focus();
        return;
      }

      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener('keydown', onKeydown);

    cancelBtn.addEventListener('click', close);
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) close();
    });

    confirmBtn.addEventListener('click', async () => {
      if (opts.onConfirm) {
        const result = await opts.onConfirm(overlay);
        if (result === false) return;
      }
      close();
    });

    const firstInput = overlay.querySelector('input, textarea, select, button');
    if (firstInput) firstInput.focus();
  };
})();
