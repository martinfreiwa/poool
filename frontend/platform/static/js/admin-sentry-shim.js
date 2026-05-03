/**
 * Admin Sentry init shim.
 *
 * Reads the DSN from a `<meta name="poool-sentry-dsn">` tag (preferred — server
 * can inject it from env without leaking it to source) or from
 * `window.POOOL_SENTRY_DSN`. If neither is present, this is a no-op.
 *
 * Loads the official Sentry browser SDK from a CDN only when a DSN is found;
 * otherwise nothing is fetched. After init the global `window.Sentry` is
 * available, which the orderbook (and other admin pages) already feature-detect
 * via `reportError(scope, err)` -> `Sentry.captureException`.
 */
(function () {
  "use strict";
  if (window.Sentry) return; // already initialized elsewhere

  const meta = document.querySelector('meta[name="poool-sentry-dsn"]');
  const dsn = (meta && meta.getAttribute("content")) || window.POOOL_SENTRY_DSN || "";
  if (!dsn || !/^https?:\/\//.test(dsn)) return;

  const env =
    (document.querySelector('meta[name="poool-env"]') || {}).content || "production";
  const release =
    (document.querySelector('meta[name="poool-release"]') || {}).content || "";

  const script = document.createElement("script");
  script.src = "https://browser.sentry-cdn.com/7.119.2/bundle.tracing.min.js";
  script.crossOrigin = "anonymous";
  script.onload = () => {
    if (!window.Sentry) return;
    try {
      window.Sentry.init({
        dsn,
        environment: env,
        release: release || undefined,
        tracesSampleRate: 0.05,
        ignoreErrors: ["ResizeObserver loop limit exceeded"],
      });
      window.Sentry.setTag("app", "poool-admin");
    } catch (err) {
      console.warn("[sentry-shim] init failed", err);
    }
  };
  script.onerror = () => console.warn("[sentry-shim] CDN load failed");
  document.head.appendChild(script);
})();
