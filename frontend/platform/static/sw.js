/* eslint-disable no-restricted-globals */
/**
 * Legacy POOOL service worker — self-destructing stub.
 *
 * Background: until 2026-05-15 the platform registered its SW at
 * `/static/sw.js` (scope `/static/`). The new SW lives at `/sw.js`
 * (scope `/`). Browsers that registered the old script will keep it
 * active indefinitely unless we either (a) actively unregister from
 * the page, or (b) ship an update that unregisters itself.
 *
 * `pwa-install.js` already does (a) on every page load, but (b) makes
 * the migration robust against stale tabs, blocked scripts, or users
 * who don't navigate through HTML before the cache TTL expires. When
 * an old client checks for updates, it pulls this file, sees a byte
 * diff, installs it, activates it, and the activate handler:
 *   1. unregisters this registration
 *   2. reloads any pages still under its control
 *
 * After the reload, `pwa-install.js` runs cold and registers the new
 * root-scope `/sw.js`. The legacy file can be deleted once telemetry
 * confirms there are no remaining `/static/sw.js` registrations.
 */
self.addEventListener('install', (event) => {
  event.waitUntil(self.skipWaiting());
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      try {
        await self.registration.unregister();
        const clients = await self.clients.matchAll({ type: 'window' });
        for (const client of clients) {
          try {
            client.navigate(client.url);
          } catch (_) {
            /* navigate is best-effort; pwa-install.js still handles the
               fallback path. */
          }
        }
      } catch (_) {
        /* swallow — the failure mode is "old SW stays active", which
           pwa-install.js will then unregister on the next page load. */
      }
    })()
  );
});

// Pass-through fetch so we never interfere with requests during the
// install/activate window.
self.addEventListener('fetch', () => {});
