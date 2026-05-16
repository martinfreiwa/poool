/* global window, document, BroadcastChannel */

/**
 * Community cross-tab sync — 2026-05-16.
 *
 * When a user takes a write action (join circle, leave circle, change role,
 * ban, etc.) in tab A, other open tabs of the same domain need to refresh
 * the affected views. Without sync, tab B keeps stale state until the
 * user manually reloads.
 *
 * Strategy: one BroadcastChannel per concern. Sender side fires
 * `window.communitySync.emit('circle.joined', { circle_id })`. Receiver
 * side listens via `window.communitySync.on('circle.*', handler)`.
 *
 * Falls back to `localStorage` `storage` events when BroadcastChannel is
 * unavailable (Safari <15.4, old Firefox). Both paths deliver the same
 * payload shape: `{ event: 'circle.joined', payload: { … }, ts: 1234… }`.
 */
(function () {
  'use strict';

  var CHANNEL_NAME = 'poool-community';
  var STORAGE_KEY = '__poool_community_sync';

  var listeners = []; // { pattern, handler }
  var channel = null;

  // ─── Channel setup ─────────────────────────────────────────────
  function initChannel() {
    if (typeof BroadcastChannel === 'function') {
      try {
        channel = new BroadcastChannel(CHANNEL_NAME);
        channel.onmessage = function (e) { dispatch(e.data); };
        return;
      } catch (_) { /* fall through to storage fallback */ }
    }
    // Fallback: localStorage `storage` events fire across tabs of the
    // same origin when a key is set with a *new* value.
    window.addEventListener('storage', function (e) {
      if (e.key !== STORAGE_KEY || !e.newValue) return;
      try { dispatch(JSON.parse(e.newValue)); } catch (_) {}
    });
  }

  function dispatch(msg) {
    if (!msg || typeof msg.event !== 'string') return;
    listeners.forEach(function (l) {
      if (matches(l.pattern, msg.event)) {
        try { l.handler(msg.payload || {}, msg.event); } catch (err) {
          // Never let a single listener kill the dispatch loop.
          console.error('[community-sync] listener threw', err);
        }
      }
    });
  }

  function matches(pattern, event) {
    if (pattern === '*' || pattern === event) return true;
    // `circle.*` matches `circle.joined`, `circle.left`, …
    if (pattern.endsWith('.*')) {
      return event.startsWith(pattern.slice(0, -1));
    }
    return false;
  }

  // ─── Public API ────────────────────────────────────────────────
  function emit(event, payload) {
    var msg = { event: event, payload: payload || {}, ts: Date.now() };
    if (channel) {
      try { channel.postMessage(msg); } catch (_) {}
    }
    // Always also write to localStorage so older browsers + same-tab
    // listeners (which BroadcastChannel skips) get the event.
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(msg));
      // Self-dispatch — BroadcastChannel only delivers cross-tab; we
      // also want intra-tab listeners on the same write.
      dispatch(msg);
    } catch (_) {}
  }

  function on(pattern, handler) {
    if (typeof handler !== 'function') return function () {};
    var entry = { pattern: pattern, handler: handler };
    listeners.push(entry);
    return function unsubscribe() {
      var idx = listeners.indexOf(entry);
      if (idx >= 0) listeners.splice(idx, 1);
    };
  }

  initChannel();
  window.communitySync = { emit: emit, on: on };
})();
