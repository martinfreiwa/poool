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
    toast.innerHTML = `
      <span class="mp-toast-icon">${icons[type] || '✅'}</span>
      <span class="mp-toast-message">${message}</span>
    `;
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
   * @param {string} opts.bodyHTML
   * @param {Function} opts.onConfirm  Called with modal element for data extraction.
   * @param {string} [opts.confirmLabel='Confirm']
   * @param {string} [opts.confirmClass='admin-btn--danger']
   */
  window.mpModal = function (opts) {
    const overlay = document.createElement('div');
    overlay.className = 'mp-modal-overlay';
    overlay.innerHTML = `
      <div class="mp-modal">
        <h2 class="mp-modal-title">${opts.title}</h2>
        <p class="mp-modal-subtitle">${opts.subtitle || ''}</p>
        <div class="mp-modal-body">${opts.bodyHTML || ''}</div>
        <div class="mp-modal-actions">
          <button class="admin-btn admin-btn--secondary mp-modal-cancel">Cancel</button>
          <button class="admin-btn ${opts.confirmClass || 'admin-btn--danger'} mp-modal-confirm">${opts.confirmLabel || 'Confirm'}</button>
        </div>
      </div>
    `;

    document.body.appendChild(overlay);

    const close = () => {
      overlay.classList.add('closing');
      overlay.addEventListener('animationend', () => overlay.remove());
    };

    overlay.querySelector('.mp-modal-cancel').addEventListener('click', close);
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) close();
    });

    overlay.querySelector('.mp-modal-confirm').addEventListener('click', () => {
      if (opts.onConfirm) {
        opts.onConfirm(overlay);
      }
      close();
    });
  };
})();
