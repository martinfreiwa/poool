/* eslint-disable no-restricted-globals */
/**
 * POOOL Service Worker
 *
 * Served from /sw.js (platform root) so scope defaults to "/". Registration
 * site is /static/js/pwa-install.js with { updateViaCache: 'none' } so the
 * browser revalidates this file on every navigation.
 *
 * Caching strategy is conservative because the platform is session-cookie
 * authenticated and most HTML responses are user-specific:
 *
 *   • Static assets under /static/, /uploads/public/, /images/   → stale-while-revalidate
 *   • Top-level branding (favicon, apple-touch icons, manifest)  → stale-while-revalidate
 *   • HTML navigation (mode === 'navigate', GET)                 → network-first → offline.html
 *   • Anything sensitive (/api, /auth, /admin, /developer,
 *     /payments, /wallet, /portfolio, /checkout, /kyc, /me)      → network-only
 *   • Any non-GET request                                        → network-only
 *
 * Responses with Set-Cookie or Vary: Cookie are never stored. POOOL ships
 * an `X-Pwa-No-Cache: 1` header on routes that should be opt-out at the
 * application layer — also respected.
 *
 * Cache version is bumped whenever this file changes; activate() purges
 * caches with a different version prefix.
 */

const CACHE_VERSION = 'v1';
const STATIC_CACHE = `poool-static-${CACHE_VERSION}`;
const RUNTIME_CACHE = `poool-runtime-${CACHE_VERSION}`;
const OFFLINE_URL = '/offline.html';

const PRECACHE_URLS = [
  OFFLINE_URL,
  '/static/manifest.webmanifest',
  '/static/favicon.ico',
  '/static/images/icons/logo-pool.svg',
  '/static/images/icons/pwa-192.png',
  '/static/images/icons/pwa-512.png',
  '/static/images/icons/apple-touch-icon.png',
];

const SENSITIVE_PREFIXES = [
  '/api/',
  '/auth/',
  '/admin/',
  '/developer/',
  '/payments/',
  '/wallet/',
  '/portfolio/',
  '/checkout/',
  '/kyc/',
  '/me/',
  '/csrf',
];

const STATIC_PREFIXES = ['/static/', '/uploads/public/', '/images/'];

function isSensitive(pathname) {
  return SENSITIVE_PREFIXES.some((p) => pathname === p.slice(0, -1) || pathname.startsWith(p));
}

function isStatic(pathname) {
  return STATIC_PREFIXES.some((p) => pathname.startsWith(p));
}

function isCacheableResponse(response) {
  if (!response || !response.ok) return false;
  if (response.type === 'opaque' || response.type === 'opaqueredirect') return false;
  if (response.headers.get('Set-Cookie')) return false;
  const vary = response.headers.get('Vary') || '';
  if (/cookie/i.test(vary)) return false;
  if (response.headers.get('X-Pwa-No-Cache')) return false;
  const cacheControl = response.headers.get('Cache-Control') || '';
  if (/(private|no-store)/i.test(cacheControl)) return false;
  return true;
}

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(STATIC_CACHE)
      .then((cache) => cache.addAll(PRECACHE_URLS).catch(() => undefined))
      .then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter((k) => k.startsWith('poool-') && k !== STATIC_CACHE && k !== RUNTIME_CACHE)
            .map((k) => caches.delete(k))
        )
      )
      .then(() => self.clients.claim())
  );
});

self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'SKIP_WAITING') {
    self.skipWaiting();
  }
});

async function networkFirstNavigation(request) {
  try {
    const response = await fetch(request);
    return response;
  } catch (err) {
    // A superseded navigation can abort while the user or a test runner is
    // moving quickly between pages. That is not an offline state; let the
    // browser treat it as a cancelled navigation instead of rendering the
    // offline shell over a healthy session.
    if (err && err.name === 'AbortError') {
      throw err;
    }
    const cache = await caches.open(STATIC_CACHE);
    const offline = await cache.match(OFFLINE_URL);
    if (offline) return offline;
    return new Response('Offline', { status: 503, statusText: 'Offline' });
  }
}

async function staleWhileRevalidate(request) {
  const cache = await caches.open(RUNTIME_CACHE);
  const cached = await cache.match(request);
  const network = fetch(request)
    .then((response) => {
      if (isCacheableResponse(response)) {
        cache.put(request, response.clone()).catch(() => undefined);
      }
      return response;
    })
    .catch(() => cached);
  return cached || network;
}

self.addEventListener('fetch', (event) => {
  const { request } = event;
  if (request.method !== 'GET') return;

  const url = new URL(request.url);

  // Same-origin only.
  if (url.origin !== self.location.origin) return;

  // Never touch the SW script itself.
  if (url.pathname === '/sw.js' || url.pathname === '/static/sw-alerts.js') return;

  if (isSensitive(url.pathname)) return; // network-only (browser default)

  // Treat HTML page loads as navigations.
  if (request.mode === 'navigate') {
    event.respondWith(networkFirstNavigation(request));
    return;
  }

  if (isStatic(url.pathname)) {
    event.respondWith(staleWhileRevalidate(request));
    return;
  }

  // Everything else: network-only.
});
