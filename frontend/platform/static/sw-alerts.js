/**
 * Alerts Service Worker — minimal push handler.
 * Receives push events from the server and shows a notification.
 *
 * Subscribe flow lives in mp-alerts.js (subscribeWebPush()):
 *   1. SW registered
 *   2. PushManager.subscribe() with VAPID public key
 *   3. POST subscription to /api/admin/marketplace/push-subscriptions
 *
 * Backend dispatch is the missing piece — wire web-push library to your
 * notification service (currently logged via tracing in escalation worker).
 */

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', e => e.waitUntil(self.clients.claim()));

self.addEventListener('push', event => {
  let data = { title: 'POOOL Alert', body: 'New alert detected' };
  try { if (event.data) data = { ...data, ...event.data.json() }; } catch { /* not json */ }
  event.waitUntil(
    self.registration.showNotification(data.title, {
      body: data.body,
      icon: '/static/images/icons/logo-pool.svg',
      badge: '/static/images/icons/logo-pool.svg',
      tag: data.tag || 'poool-alert',
      requireInteraction: data.severity === 'critical',
      data: { url: data.url || '/admin/marketplace/alerts.html' },
    })
  );
});

self.addEventListener('notificationclick', event => {
  event.notification.close();
  const url = (event.notification.data && event.notification.data.url) || '/admin/marketplace/alerts.html';
  event.waitUntil(self.clients.matchAll({ type: 'window' }).then(clientList => {
    for (const c of clientList) {
      if (c.url.includes('/admin/marketplace/alerts') && 'focus' in c) return c.focus();
    }
    if (self.clients.openWindow) return self.clients.openWindow(url);
  }));
});
