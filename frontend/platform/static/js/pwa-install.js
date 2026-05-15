/**
 * POOOL PWA install + service worker lifecycle.
 *
 * Responsibilities:
 *   1. Migrate any legacy /static/sw.js registration to root /sw.js so the
 *      service worker scope is "/", not "/static/".
 *   2. Register /sw.js with { updateViaCache: 'none' } so the script
 *      revalidates on each navigation and updates land quickly.
 *   3. Show an "Install POOOL" button on Android/desktop when the browser
 *      fires beforeinstallprompt. Hide it after install or dismiss.
 *   4. On iOS Safari (no beforeinstallprompt), show a 3-step "Add to Home
 *      Screen" modal triggered by the same button.
 *   5. When a new SW version is waiting, show a non-blocking "Update
 *      available" toast that, on click, posts SKIP_WAITING and reloads.
 *
 * Sensitive pages (auth, admin, developer, payments, checkout, KYC) do not
 * show install UI — too intrusive in those contexts.
 */
(function () {
  'use strict';

  const STORAGE_KEY = 'poool:pwa:install-dismissed:v2';
  const IOS_MODAL_KEY = 'poool:pwa:ios-modal-dismissed:v2';
  const DISMISS_COOLDOWN_MS = 30 * 24 * 60 * 60 * 1000; // 30 days
  const IOS_COOLDOWN_MS = 7 * 24 * 60 * 60 * 1000; // 7 days

  const HIDDEN_PATH_PREFIXES = [
    '/auth/',
    '/admin/',
    '/developer/',
    '/payments/',
    '/checkout/',
    '/kyc/',
    '/api/',
  ];

  let deferredPrompt = null;

  function isStandalone() {
    return (
      window.matchMedia('(display-mode: standalone)').matches ||
      window.matchMedia('(display-mode: window-controls-overlay)').matches ||
      window.navigator.standalone === true
    );
  }

  function isIos() {
    const ua = window.navigator.userAgent || '';
    const isIosDevice = /iPhone|iPad|iPod/i.test(ua);
    // iPadOS reports as Mac with touch support
    const isIpadOs =
      /Macintosh/i.test(ua) &&
      typeof navigator.maxTouchPoints === 'number' &&
      navigator.maxTouchPoints > 1;
    return isIosDevice || isIpadOs;
  }

  function isIosSafari() {
    if (!isIos()) return false;
    const ua = window.navigator.userAgent || '';
    return /Safari/i.test(ua) && !/CriOS|FxiOS|EdgiOS|OPiOS/i.test(ua);
  }

  function pathHidden() {
    const p = location.pathname;
    return HIDDEN_PATH_PREFIXES.some((pre) => p === pre.slice(0, -1) || p.startsWith(pre));
  }

  function dismissedRecently(key, cooldown) {
    try {
      const raw = localStorage.getItem(key);
      if (!raw) return false;
      const ts = parseInt(raw, 10);
      if (!Number.isFinite(ts)) return false;
      return Date.now() - ts < cooldown;
    } catch (_) {
      return false;
    }
  }

  function markDismissed(key) {
    try {
      localStorage.setItem(key, String(Date.now()));
    } catch (_) {}
  }

  // ── Service worker registration ─────────────────────────────────────
  async function setupServiceWorker() {
    if (!('serviceWorker' in navigator)) return null;
    if (location.protocol !== 'https:' && location.hostname !== 'localhost' && location.hostname !== '127.0.0.1') {
      return null;
    }

    // Step 1: unregister any legacy /static/sw.js registration so the new
    // root-scope SW can take over cleanly.
    try {
      const regs = await navigator.serviceWorker.getRegistrations();
      await Promise.all(
        regs.map(async (reg) => {
          const url = reg.active && reg.active.scriptURL;
          if (url && /\/static\/sw\.js(\?|$)/.test(url)) {
            await reg.unregister();
          }
        })
      );
    } catch (_) {}

    // Step 2: register the new SW.
    let registration;
    try {
      registration = await navigator.serviceWorker.register('/sw.js', {
        scope: '/',
        updateViaCache: 'none',
      });
    } catch (e) {
      console.warn('SW registration failed', e);
      return null;
    }

    // Step 3: update detection → show toast when a new worker is ready.
    if (registration.waiting && navigator.serviceWorker.controller) {
      notifyUpdateAvailable(registration.waiting);
    }
    registration.addEventListener('updatefound', () => {
      const sw = registration.installing;
      if (!sw) return;
      sw.addEventListener('statechange', () => {
        if (sw.state === 'installed' && navigator.serviceWorker.controller) {
          notifyUpdateAvailable(sw);
        }
      });
    });

    // Reload exactly once when the new SW takes control.
    let didReload = false;
    navigator.serviceWorker.addEventListener('controllerchange', () => {
      if (didReload) return;
      didReload = true;
      window.location.reload();
    });

    return registration;
  }

  function notifyUpdateAvailable(worker) {
    if (window.poool && window.poool.__pwaUpdateNotified) return;
    window.poool = window.poool || {};
    window.poool.__pwaUpdateNotified = true;
    // Global toast lacks click handler support, so render a dedicated
    // clickable banner.
    renderUpdateBanner(worker);
  }

  function renderUpdateBanner(worker) {
    if (document.getElementById('poool-pwa-update-banner')) return;
    const bar = document.createElement('button');
    bar.id = 'poool-pwa-update-banner';
    bar.type = 'button';
    bar.style.cssText =
      'position:fixed; left:50%; bottom:20px; transform:translateX(-50%); z-index:1000; ' +
      'background:#03FF88; color:#001428; border:0; padding:12px 18px; border-radius:999px; ' +
      'font: 600 14px/1 -apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif; ' +
      'box-shadow:0 8px 24px rgba(0,0,0,0.25); cursor:pointer;';
    bar.textContent = '↻ Update available — tap to reload';
    bar.addEventListener('click', () => {
      worker.postMessage({ type: 'SKIP_WAITING' });
      bar.disabled = true;
      bar.textContent = 'Reloading…';
    });
    document.body.appendChild(bar);
  }

  // ── Install button (Android / desktop / iOS via fallback modal) ─────
  function maybeShowInstallButton() {
    if (document.getElementById('poool-pwa-install-btn')) return;
    if (isStandalone()) return;
    if (pathHidden()) return;

    const isIosFlow = isIosSafari() && !deferredPrompt;
    if (!deferredPrompt && !isIosFlow) return;

    if (isIosFlow) {
      if (dismissedRecently(IOS_MODAL_KEY, IOS_COOLDOWN_MS)) return;
    } else if (dismissedRecently(STORAGE_KEY, DISMISS_COOLDOWN_MS)) {
      return;
    }

    const btn = document.createElement('div');
    btn.id = 'poool-pwa-install-btn';
    btn.setAttribute('role', 'button');
    btn.tabIndex = 0;
    btn.style.cssText =
      'position:fixed; bottom:80px; right:20px; z-index:999; ' +
      'display:flex; align-items:center; gap:8px; ' +
      'background:#001DCA; color:#fff; border-radius:999px; ' +
      'padding:10px 14px 10px 12px; cursor:pointer; ' +
      'box-shadow:0 8px 24px rgba(0,29,202,0.35); ' +
      'font: 600 14px/1 -apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;';
    btn.innerHTML =
      '<span aria-hidden="true" style="font-size:18px;line-height:1;">⬇</span>' +
      '<span>Install POOOL</span>' +
      '<span data-dismiss="1" aria-label="Dismiss" ' +
      'style="margin-left:6px; opacity:.7; padding:2px 6px; border-radius:50%;">✕</span>';

    function dismiss() {
      markDismissed(isIosFlow ? IOS_MODAL_KEY : STORAGE_KEY);
      btn.remove();
    }

    btn.addEventListener('click', async (event) => {
      const target = event.target;
      if (target instanceof HTMLElement && target.dataset.dismiss === '1') {
        event.stopPropagation();
        dismiss();
        return;
      }
      if (isIosFlow) {
        showIosInstallModal();
        return;
      }
      if (!deferredPrompt) return;
      try {
        deferredPrompt.prompt();
        const { outcome } = await deferredPrompt.userChoice;
        if (outcome === 'accepted') {
          if (typeof window.showToast === 'function') {
            window.showToast('POOOL installed!', 'success');
          }
        } else {
          markDismissed(STORAGE_KEY);
        }
        btn.remove();
        deferredPrompt = null;
      } catch (e) {
        console.error('PWA install prompt failed', e);
      }
    });

    document.body.appendChild(btn);
  }

  // ── iOS Add-to-Home-Screen modal ────────────────────────────────────
  function showIosInstallModal() {
    if (document.getElementById('poool-pwa-ios-modal')) return;

    const overlay = document.createElement('div');
    overlay.id = 'poool-pwa-ios-modal';
    overlay.style.cssText =
      'position:fixed; inset:0; z-index:10000; background:rgba(10,11,46,0.6); ' +
      'backdrop-filter:blur(4px); display:flex; align-items:flex-end; ' +
      'justify-content:center; padding:16px;';

    const card = document.createElement('div');
    card.setAttribute('role', 'dialog');
    card.setAttribute('aria-labelledby', 'poool-ios-modal-title');
    card.style.cssText =
      'background:#fff; color:#0a0b2e; border-radius:20px; max-width:420px; width:100%; ' +
      'padding:24px 20px 16px; box-shadow:0 24px 60px rgba(0,0,0,0.35); ' +
      'font: 14px/1.5 -apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;';
    card.innerHTML =
      '<div style="display:flex;align-items:center;gap:12px;margin-bottom:12px;">' +
      '  <div style="width:48px;height:48px;border-radius:12px;background:#001DCA;display:flex;align-items:center;justify-content:center;">' +
      '    <img src="/static/images/icons/logo-pool.svg" alt="" style="width:32px;height:auto;filter:brightness(0) invert(1);" />' +
      '  </div>' +
      '  <div>' +
      '    <h2 id="poool-ios-modal-title" style="margin:0;font-size:17px;font-weight:600;">Install POOOL</h2>' +
      '    <p style="margin:2px 0 0;color:#555;">Add to your Home Screen for a full-screen, app-like experience.</p>' +
      '  </div>' +
      '</div>' +
      '<ol style="margin:12px 0 0;padding-left:0;list-style:none;display:grid;gap:10px;">' +
      '  <li style="display:flex;gap:10px;align-items:flex-start;">' +
      '    <span aria-hidden="true" style="flex:0 0 28px;height:28px;border-radius:50%;background:#03FF88;color:#001428;display:flex;align-items:center;justify-content:center;font-weight:700;">1</span>' +
      '    <span>Tap the <strong>Share</strong> button <span aria-hidden="true" style="display:inline-block;vertical-align:middle;">' +
      '      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#001DCA" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-3px"><path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"/><polyline points="16 6 12 2 8 6"/><line x1="12" y1="2" x2="12" y2="15"/></svg>' +
      '    </span> at the bottom of Safari.</span>' +
      '  </li>' +
      '  <li style="display:flex;gap:10px;align-items:flex-start;">' +
      '    <span aria-hidden="true" style="flex:0 0 28px;height:28px;border-radius:50%;background:#03FF88;color:#001428;display:flex;align-items:center;justify-content:center;font-weight:700;">2</span>' +
      '    <span>Scroll and choose <strong>Add to Home Screen</strong>.</span>' +
      '  </li>' +
      '  <li style="display:flex;gap:10px;align-items:flex-start;">' +
      '    <span aria-hidden="true" style="flex:0 0 28px;height:28px;border-radius:50%;background:#03FF88;color:#001428;display:flex;align-items:center;justify-content:center;font-weight:700;">3</span>' +
      '    <span>Confirm by tapping <strong>Add</strong> in the top-right.</span>' +
      '  </li>' +
      '</ol>' +
      '<button type="button" id="poool-ios-modal-close" ' +
      '  style="margin-top:18px;width:100%;padding:12px 16px;border:0;border-radius:12px;background:#001DCA;color:#fff;font-weight:600;cursor:pointer;">' +
      '  Got it' +
      '</button>';

    overlay.appendChild(card);
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) {
        markDismissed(IOS_MODAL_KEY);
        overlay.remove();
      }
    });
    card.querySelector('#poool-ios-modal-close').addEventListener('click', () => {
      markDismissed(IOS_MODAL_KEY);
      overlay.remove();
    });
    document.body.appendChild(overlay);
  }

  // ── Wire up ─────────────────────────────────────────────────────────
  window.addEventListener('beforeinstallprompt', (event) => {
    event.preventDefault();
    deferredPrompt = event;
    maybeShowInstallButton();
  });

  window.addEventListener('appinstalled', () => {
    deferredPrompt = null;
    const btn = document.getElementById('poool-pwa-install-btn');
    if (btn) btn.remove();
    markDismissed(STORAGE_KEY);
  });

  // iOS path: no beforeinstallprompt — show the button anyway so the
  // affordance is discoverable. Wait a moment so it doesn't fight FOUC.
  function ready(fn) {
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', fn);
    } else {
      fn();
    }
  }
  ready(() => {
    setupServiceWorker();
    if (isIosSafari() && !isStandalone()) {
      setTimeout(maybeShowInstallButton, 1500);
    }
  });
})();
