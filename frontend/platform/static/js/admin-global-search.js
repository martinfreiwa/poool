/**
 * Admin Global Search — Unified search across Users, Assets, Orders, Transactions, Deposits.
 * Searches all relevant admin APIs and displays results in a dropdown overlay.
 * Loaded on every admin page via the #admin-global-search input.
 */
(function () {
  "use strict";

  let searchTimeout = null;
  let resultsEl = null;
  let isOpen = false;

  document.addEventListener("DOMContentLoaded", () => {
    const input = document.getElementById("admin-global-search");
    if (!input) return;

    // Create results dropdown
    resultsEl = document.createElement("div");
    resultsEl.className = "admin-global-search-results";
    resultsEl.style.cssText = `
            position:absolute;top:100%;left:0;right:0;z-index:1001;
            background:var(--admin-bg-card);border:1px solid var(--admin-border);
            border-radius:0 0 var(--admin-radius-lg) var(--admin-radius-lg);
            box-shadow:0 12px 40px rgba(0,0,0,0.15);max-height:420px;overflow-y:auto;
            display:none;
        `;
    input.parentElement.style.position = "relative";
    input.parentElement.appendChild(resultsEl);

    // Listen for typed input
    input.addEventListener("input", () => {
      clearTimeout(searchTimeout);
      const q = input.value.trim();
      if (q.length < 2) {
        closeResults();
        return;
      }
      searchTimeout = setTimeout(() => runSearch(q), 300);
    });

    // Keyboard navigation
    input.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        closeResults();
        input.blur();
      }
      if (e.key === "ArrowDown" && isOpen) {
        e.preventDefault();
        const first = resultsEl.querySelector(".admin-search-result-item");
        if (first) first.focus();
      }
    });

    // Close on outside click
    document.addEventListener("click", (e) => {
      if (!input.contains(e.target) && !resultsEl.contains(e.target))
        closeResults();
    });

    // Keyboard shortcut: Cmd+K or Ctrl+K to focus search
    document.addEventListener("keydown", (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        input.focus();
        input.select();
      }
    });
  });

  async function runSearch(query) {
    showLoading();
    const q = query.toLowerCase();

    // Parallel fetch from multiple endpoints
    const [users, assets, orders, deposits] = await Promise.allSettled([
      fetchSafe("/api/admin/users"),
      fetchSafe("/api/admin/assets"),
      fetchSafe("/api/admin/orders"),
      fetchSafe("/api/admin/deposits"),
    ]);

    const results = [];

    // Search Users (by email, name, UUID)
    const userList = extractArray(users);
    userList.forEach((u) => {
      const hay =
        `${u.email || ""} ${u.first_name || ""} ${u.last_name || ""} ${u.display_name || ""} ${u.id || ""}`.toLowerCase();
      if (hay.includes(q)) {
        results.push({
          type: "user",
          icon: "👤",
          title: u.display_name || u.first_name || u.email,
          subtitle: u.email,
          url: `/admin/user-details.html?id=${u.id}`,
          badge: u.kyc_status || "unknown",
        });
      }
    });

    // Search Assets (by title, slug)
    const assetList = extractArray(assets);
    assetList.forEach((a) => {
      const hay =
        `${a.title || ""} ${a.slug || ""} ${a.id || ""} ${a.asset_type || ""}`.toLowerCase();
      if (hay.includes(q)) {
        results.push({
          type: "asset",
          icon: "🏠",
          title: a.title,
          subtitle: `${a.asset_type || "asset"} · ${a.funding_status || ""}`,
          url: `/admin/asset-details.html?id=${a.id}`,
          badge: a.funding_status || "",
        });
      }
    });

    // Search Orders (by order_number)
    const orderList = extractArray(orders);
    orderList.forEach((o) => {
      const hay =
        `${o.order_number || ""} ${o.user_email || ""} ${o.id || ""}`.toLowerCase();
      if (hay.includes(q)) {
        results.push({
          type: "order",
          icon: "📋",
          title: o.order_number || `Order ${(o.id || "").substring(0, 8)}`,
          subtitle: `${o.user_email || ""} · ${formatCents(o.total_cents)}`,
          url: `/admin/orders.html?id=${o.id}`,
          badge: o.status || "",
        });
      }
    });

    // Search Deposits (by provider_reference, user)
    const depositList = extractArray(deposits);
    depositList.forEach((d) => {
      const hay =
        `${d.external_ref_id || ""} ${d.provider_reference || ""} ${d.user_email || ""} ${d.user_name || ""}`.toLowerCase();
      if (hay.includes(q)) {
        results.push({
          type: "deposit",
          icon: "💰",
          title: `Deposit ${d.external_ref_id || (d.id || "").substring(0, 8)}`,
          subtitle: `${d.user_email || d.user_name || ""} · ${formatCents(d.amount_cents)} ${d.currency || "USD"}`,
          url: `/admin/deposits.html`,
          badge: d.status || "",
        });
      }
    });

    renderResults(results.slice(0, 15), query);
  }

  function renderResults(results, query) {
    if (results.length === 0) {
      resultsEl.innerHTML = `
                <div style="padding:24px;text-align:center;color:var(--admin-text-muted);font-size:13px;">
                    <div style="font-size:20px;margin-bottom:8px;">🔍</div>
                    No results for "<strong>${esc(query)}</strong>"
                </div>`;
      openResults();
      return;
    }

    const grouped = {};
    results.forEach((r) => {
      if (!grouped[r.type]) grouped[r.type] = [];
      grouped[r.type].push(r);
    });

    const typeLabels = {
      user: "Users",
      asset: "Assets",
      order: "Orders",
      deposit: "Deposits",
    };

    let html = "";
    for (const [type, items] of Object.entries(grouped)) {
      html += `<div style="padding:6px 16px 2px;font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:0.5px;color:var(--admin-text-muted);">${typeLabels[type] || type}</div>`;
      items.forEach((item) => {
        html += `
                    <a href="${item.url}" class="admin-search-result-item" tabindex="0"
                       style="display:flex;align-items:center;gap:10px;padding:10px 16px;text-decoration:none;color:var(--admin-text-primary);transition:background 0.15s;"
                       onmouseover="this.style.background='var(--admin-bg-hover)'" onmouseout="this.style.background='transparent'"
                       onfocus="this.style.background='var(--admin-bg-hover)'" onblur="this.style.background='transparent'">
                        <span style="font-size:16px;width:24px;text-align:center;">${item.icon}</span>
                        <div style="flex:1;min-width:0;">
                            <div style="font-size:13px;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${esc(item.title)}</div>
                            <div style="font-size:11px;color:var(--admin-text-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${esc(item.subtitle)}</div>
                        </div>
                        ${item.badge ? `<span class="admin-badge admin-badge--${badgeClass(item.badge)}" style="font-size:10px;">${esc(item.badge)}</span>` : ""}
                    </a>`;
      });
    }

    html += `
            <div style="padding:8px 16px;border-top:1px solid var(--admin-border);text-align:center;">
                <span style="font-size:11px;color:var(--admin-text-muted);">${results.length} result${results.length !== 1 ? "s" : ""} · <kbd style="padding:1px 4px;border:1px solid var(--admin-border);border-radius:3px;font-size:10px;">⌘K</kbd> to search</span>
            </div>`;

    resultsEl.innerHTML = html;

    // Arrow key navigation between results
    const items = resultsEl.querySelectorAll(".admin-search-result-item");
    items.forEach((el, i) => {
      el.addEventListener("keydown", (e) => {
        if (e.key === "ArrowDown" && i < items.length - 1) {
          e.preventDefault();
          items[i + 1].focus();
        }
        if (e.key === "ArrowUp") {
          e.preventDefault();
          if (i > 0) items[i - 1].focus();
          else document.getElementById("admin-global-search").focus();
        }
        if (e.key === "Escape") {
          closeResults();
          document.getElementById("admin-global-search").focus();
        }
      });
    });

    openResults();
  }

  function showLoading() {
    resultsEl.innerHTML = `
            <div style="padding:24px;text-align:center;color:var(--admin-text-muted);font-size:13px;">
                <div style="width:18px;height:18px;border:2px solid var(--admin-border);border-top-color:var(--admin-accent);border-radius:50%;animation:spin 0.8s linear infinite;margin:0 auto 8px;"></div>
                Searching…
            </div>`;
    openResults();
  }

  function openResults() {
    resultsEl.style.display = "block";
    isOpen = true;
  }
  function closeResults() {
    resultsEl.style.display = "none";
    isOpen = false;
  }

  async function fetchSafe(url) {
    try {
      const r = await fetch(url);
      if (!r.ok) return [];
      return await r.json();
    } catch {
      return [];
    }
  }

  function extractArray(settled) {
    const val = settled.status === "fulfilled" ? settled.value : [];
    if (Array.isArray(val)) return val;
    // Try common wrapper keys
    for (const k of [
      "users",
      "assets",
      "orders",
      "deposits",
      "data",
      "items",
    ]) {
      if (Array.isArray(val[k])) return val[k];
    }
    return [];
  }

  function formatCents(c) {
    if (typeof c !== "number") return "$0";
    return (
      "$" +
      (Math.abs(c) / 100).toLocaleString("en-US", {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      })
    );
  }

  function badgeClass(status) {
    const s = (status || "").toLowerCase();
    if (["approved", "completed", "paid", "active", "funding_open"].includes(s))
      return "success";
    if (["pending", "processing", "in_review", "submitted"].includes(s))
      return "warning";
    if (["rejected", "failed", "cancelled", "expired", "suspended"].includes(s))
      return "danger";
    return "neutral";
  }

  function esc(s) {
    if (typeof s !== "string") return s || "";
    const d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }
})();
