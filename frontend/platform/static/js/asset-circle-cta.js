(function () {
  "use strict";

  var cache = new Map();

  function escHtml(value) {
    if (value == null) return "";
    var d = document.createElement("div");
    d.appendChild(document.createTextNode(String(value)));
    return d.innerHTML;
  }

  function safeCircleUrl(value) {
    if (typeof value !== "string") return "";
    return value.indexOf("/community/circle/") === 0 ? value : "";
  }

  function circleMeta(data) {
    var circle = data && data.circle;
    if (!circle) return "";
    var parts = [];
    if (circle.is_official) parts.push("Official");
    if (circle.visibility) parts.push(circle.visibility.replace(/_/g, " "));
    if (circle.join_policy === "holder_only") parts.push("Holder-only");
    if (circle.member_count != null) parts.push(circle.member_count + " members");
    if (circle.recent_post_count != null) parts.push(circle.recent_post_count + " posts this week");
    return parts.filter(Boolean).join(" · ");
  }

  function actionLabel(accessState) {
    if (accessState === "open") return "Open circle";
    if (accessState === "join") return "Join circle";
    if (accessState === "request_access") return "Request access";
    if (accessState === "locked") return "View access";
    return "Open circle";
  }

  async function fetchAssetCircle(assetId) {
    if (!assetId) return null;
    var key = String(assetId);
    if (cache.has(key)) return cache.get(key);

    var promise = fetch("/api/community/assets/" + encodeURIComponent(key) + "/circle", {
      credentials: "same-origin",
      headers: { Accept: "application/json" },
    })
      .then(function (res) {
        if (res.status === 401 || res.status === 403 || res.status === 404) return null;
        if (!res.ok) throw new Error("asset circle " + res.status);
        return res.json();
      })
      .catch(function (err) {
        console.warn("[asset-circle] lookup failed", err);
        return null;
      });

    cache.set(key, promise);
    return promise;
  }

  function renderPropertyCta(container, data) {
    if (!container) return;
    var circle = data && data.circle;
    var url = safeCircleUrl(circle && circle.url);
    if (!circle || !url) {
      container.hidden = true;
      container.innerHTML = "";
      return;
    }

    var accessState = circle.access_state || "open";
    container.hidden = false;
    container.innerHTML =
      '<div class="asset-circle-cta">' +
        '<div class="asset-circle-cta__icon" aria-hidden="true">' +
          '<svg width="22" height="22" viewBox="0 0 24 24" fill="none">' +
            '<path d="M17 20a5 5 0 0 0-10 0" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>' +
            '<circle cx="12" cy="8" r="4" stroke="currentColor" stroke-width="1.8"/>' +
            '<path d="M20 10.5a3 3 0 0 1 0 6M4 10.5a3 3 0 0 0 0 6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>' +
          '</svg>' +
        '</div>' +
        '<div class="asset-circle-cta__copy">' +
          '<span class="asset-circle-cta__eyebrow">Investor discussion</span>' +
          '<h2 class="asset-circle-cta__title">' + escHtml(circle.name) + '</h2>' +
          '<p class="asset-circle-cta__meta">' + escHtml(circleMeta(data)) + '</p>' +
          '<p class="asset-circle-cta__body">Discuss official updates, Q&A, documents, reports, and risk topics connected to this asset.</p>' +
        '</div>' +
        '<a class="ds-btn ds-btn--primary asset-circle-cta__button" href="' + escHtml(url) + '">' +
          '<span>' + escHtml(actionLabel(accessState)) + '</span>' +
          '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">' +
            '<path d="M5 12h14m-6-7 7 7-7 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>' +
          '</svg>' +
        '</a>' +
      '</div>';
  }

  function renderActionButton(data, options) {
    var circle = data && data.circle;
    var url = safeCircleUrl(circle && circle.url);
    if (!circle || !url) return "";
    var variant = options && options.variant === "mobile" ? "mobile" : "desktop";
    var label = actionLabel(circle.access_state || "open");
    if (variant === "mobile") {
      return '<a class="mobile-asset-circle-link" href="' + escHtml(url) + '" onclick="event.stopPropagation();" aria-label="' + escHtml(label + " for " + circle.name) + '">' +
        '<span>Circle</span>' +
      '</a>';
    }

    return '<a class="portfolio-assets-detail-btn portfolio-assets-circle-btn" href="' + escHtml(url) + '" onclick="event.stopPropagation();" title="' + escHtml(label) + '" aria-label="' + escHtml(label + " for " + circle.name) + '">' +
      '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">' +
        '<path d="M17 20a5 5 0 0 0-10 0" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>' +
        '<circle cx="12" cy="8" r="4" stroke="currentColor" stroke-width="1.8"/>' +
        '<path d="M20 10.5a3 3 0 0 1 0 6M4 10.5a3 3 0 0 0 0 6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>' +
      '</svg>' +
    '</a>';
  }

  window.PooolAssetCircleCta = {
    fetchAssetCircle: fetchAssetCircle,
    renderPropertyCta: renderPropertyCta,
    renderActionButton: renderActionButton,
  };
})();
