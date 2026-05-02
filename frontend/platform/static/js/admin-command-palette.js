/**
 * Admin Command Palette (Cmd+K / Ctrl+K)
 * Static nav + actions list, optional remote search via /api/admin/search.
 */
(function () {
  "use strict";

  const COMMANDS = [
    // Navigation
    { kind: "nav", title: "Dashboard", subtitle: "Platform overview", url: "/admin/", keywords: "home overview kpi" },
    { kind: "nav", title: "Submissions", subtitle: "Asset submissions", url: "/admin/developer-submissions.html", keywords: "developer submit pending" },
    { kind: "nav", title: "Change Requests", subtitle: "Asset change review", url: "/admin/asset-change-requests.html", keywords: "change request" },
    { kind: "nav", title: "Live Assets", subtitle: "Published assets", url: "/admin/assets.html?published=true", keywords: "published live asset" },
    { kind: "nav", title: "Tokenize Asset", subtitle: "Mint asset tokens", url: "/admin/asset-tokenize.html", keywords: "tokenize mint blockchain" },
    { kind: "nav", title: "Orders", subtitle: "Marketplace orders", url: "/admin/orders.html", keywords: "purchase buy" },
    { kind: "nav", title: "Deposits", subtitle: "Deposit requests", url: "/admin/deposits.html", keywords: "fiat deposit money in" },
    { kind: "nav", title: "Treasury", subtitle: "Platform treasury", url: "/admin/blockchain-treasury.html", keywords: "treasury reserves" },
    { kind: "nav", title: "Live Contracts", subtitle: "Smart contracts", url: "/admin/blockchain-contracts.html", keywords: "contract chain web3" },
    { kind: "nav", title: "Web3 Sync & Health", subtitle: "Chain sync status", url: "/admin/blockchain-sync.html", keywords: "sync rpc health" },
    { kind: "nav", title: "Dividends", subtitle: "Dividend payouts", url: "/admin/dividends.html", keywords: "dividend payout distribution" },
    { kind: "nav", title: "Rewards", subtitle: "Reward balances", url: "/admin/rewards.html", keywords: "cashback referral promo" },
    { kind: "nav", title: "Pending Settlements", subtitle: "Settlement queue", url: "/admin/pending-settlements.html", keywords: "settle clearance" },
    { kind: "nav", title: "Affiliate Apps", subtitle: "Affiliate applications", url: "/admin/affiliate-applications.html", keywords: "affiliate apply partner" },
    { kind: "nav", title: "Affiliate Finance", subtitle: "Affiliate finance", url: "/admin/affiliate-finance.html", keywords: "affiliate payout commission" },
    { kind: "nav", title: "Syndicate Fraud", subtitle: "Fraud signals", url: "/admin/admin-affiliate-fraud.html", keywords: "fraud abuse syndicate" },
    { kind: "nav", title: "Audit Logs", subtitle: "All audit events", url: "/admin/audit-logs.html", keywords: "audit history events" },
    { kind: "nav", title: "Notifications", subtitle: "Send platform notification", url: "/admin/notifications.html", keywords: "notify push email broadcast" },
    { kind: "nav", title: "KYC", subtitle: "KYC submissions", url: "/admin/kyc.html", keywords: "identity verification kyc" },
    { kind: "nav", title: "Settings", subtitle: "Platform settings", url: "/admin/settings.html", keywords: "config preferences" },
    { kind: "nav", title: "Roles", subtitle: "Admin roles & permissions", url: "/admin/roles.html", keywords: "rbac role permission" },
    { kind: "nav", title: "Reports", subtitle: "Reports & exports", url: "/admin/reports.html", keywords: "report export" },

    // Actions
    { kind: "action", title: "Review pending KYC", subtitle: "Open KYC queue", url: "/admin/kyc.html?status=pending", keywords: "approve kyc verify" },
    { kind: "action", title: "Confirm pending deposits", subtitle: "Open deposits queue", url: "/admin/deposits.html?status=pending", keywords: "confirm deposit fiat" },
    { kind: "action", title: "Run fraud scan", subtitle: "Affiliate fraud signals", url: "/admin/admin-affiliate-fraud.html", keywords: "fraud scan abuse" },
    { kind: "action", title: "Send notification", subtitle: "Compose broadcast", url: "/admin/notifications.html", keywords: "send notify email push" },
    { kind: "action", title: "Tokenize new asset", subtitle: "Mint flow", url: "/admin/asset-tokenize.html", keywords: "mint tokenize asset" },
  ];

  let overlay, input, results, footerEl;
  let selectedIndex = 0;
  let currentItems = [];
  let remoteAbort = null;

  document.addEventListener("DOMContentLoaded", init);

  function init() {
    overlay = document.getElementById("cmdk-overlay");
    input = document.getElementById("cmdk-input");
    results = document.getElementById("cmdk-results");
    if (!overlay || !input || !results) return;

    document.addEventListener("keydown", (e) => {
      const isMac = navigator.platform.toLowerCase().includes("mac");
      const accel = isMac ? e.metaKey : e.ctrlKey;
      if (accel && (e.key === "k" || e.key === "K")) {
        e.preventDefault();
        open();
      } else if (e.key === "Escape" && !overlay.hidden) {
        e.preventDefault();
        close();
      } else if (e.key === "/" && !isTypingTarget(e.target) && overlay.hidden) {
        e.preventDefault();
        open();
      }
    });

    overlay.addEventListener("click", (e) => { if (e.target === overlay) close(); });
    input.addEventListener("input", onInput);
    input.addEventListener("keydown", onInputKey);

    const triggers = ["open-command-palette", "open-command-palette-2"];
    triggers.forEach((id) => {
      const btn = document.getElementById(id);
      if (btn) btn.addEventListener("click", open);
    });
  }

  function isTypingTarget(t) {
    if (!t) return false;
    const tag = (t.tagName || "").toLowerCase();
    return tag === "input" || tag === "textarea" || t.isContentEditable;
  }

  function open() {
    overlay.hidden = false;
    input.value = "";
    selectedIndex = 0;
    render("");
    setTimeout(() => input.focus(), 0);
  }

  function close() {
    overlay.hidden = true;
    if (remoteAbort) { remoteAbort.abort(); remoteAbort = null; }
  }

  function onInput() {
    selectedIndex = 0;
    render(input.value.trim());
    if (input.value.trim().length >= 2) fetchRemote(input.value.trim());
  }

  function onInputKey(e) {
    if (e.key === "ArrowDown") { e.preventDefault(); move(1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); move(-1); }
    else if (e.key === "Enter") { e.preventDefault(); activate(); }
  }

  function move(d) {
    if (currentItems.length === 0) return;
    selectedIndex = (selectedIndex + d + currentItems.length) % currentItems.length;
    updateSelection();
  }

  function activate() {
    const item = currentItems[selectedIndex];
    if (!item) return;
    window.location.href = item.url;
  }

  function render(query) {
    const q = query.toLowerCase();
    let items;
    if (!q) {
      items = COMMANDS.slice(0, 12);
    } else {
      items = COMMANDS
        .map((c) => ({ c, score: score(c, q) }))
        .filter((x) => x.score > 0)
        .sort((a, b) => b.score - a.score)
        .slice(0, 20)
        .map((x) => x.c);
    }
    currentItems = items;
    paintResults(items, q);
  }

  function score(c, q) {
    const hay = `${c.title} ${c.subtitle || ""} ${c.keywords || ""}`.toLowerCase();
    if (!hay.includes(q)) return 0;
    let s = 1;
    if (c.title.toLowerCase().startsWith(q)) s += 5;
    if (c.title.toLowerCase().includes(q)) s += 3;
    return s;
  }

  function paintResults(items, query) {
    results.replaceChildren();
    if (items.length === 0) {
      const empty = document.createElement("div");
      empty.className = "admin-cmdk-empty";
      empty.textContent = query ? `No matches for "${query}"` : "Type to search…";
      results.appendChild(empty);
      return;
    }
    const groups = { action: [], nav: [], remote: [] };
    items.forEach((it) => groups[it.kind]?.push(it));
    const groupOrder = [["action", "Actions"], ["remote", "Search results"], ["nav", "Navigate"]];

    let flat = [];
    groupOrder.forEach(([key, label]) => {
      if (groups[key].length === 0) return;
      const heading = document.createElement("div");
      heading.className = "admin-cmdk-group";
      heading.textContent = label;
      results.appendChild(heading);
      groups[key].forEach((item) => {
        const idx = flat.length;
        flat.push(item);
        const row = document.createElement("button");
        row.type = "button";
        row.className = "admin-cmdk-row";
        row.dataset.idx = String(idx);
        row.setAttribute("role", "option");
        const main = document.createElement("div");
        main.className = "admin-cmdk-row-main";
        const t = document.createElement("div");
        t.className = "admin-cmdk-row-title";
        t.textContent = item.title;
        const s = document.createElement("div");
        s.className = "admin-cmdk-row-sub";
        s.textContent = item.subtitle || "";
        main.append(t, s);
        const tag = document.createElement("span");
        tag.className = `admin-cmdk-tag admin-cmdk-tag--${item.kind}`;
        tag.textContent = item.kind === "action" ? "Action" : item.kind === "remote" ? item.remoteType || "Result" : "Page";
        row.append(main, tag);
        row.addEventListener("mouseenter", () => { selectedIndex = idx; updateSelection(); });
        row.addEventListener("click", () => { selectedIndex = idx; activate(); });
        results.appendChild(row);
      });
    });
    currentItems = flat;
    updateSelection();
  }

  function updateSelection() {
    const rows = results.querySelectorAll(".admin-cmdk-row");
    rows.forEach((r) => r.classList.remove("admin-cmdk-row--active"));
    const active = results.querySelector(`.admin-cmdk-row[data-idx="${selectedIndex}"]`);
    if (active) {
      active.classList.add("admin-cmdk-row--active");
      active.scrollIntoView({ block: "nearest" });
    }
  }

  async function fetchRemote(query) {
    if (remoteAbort) remoteAbort.abort();
    remoteAbort = new AbortController();
    try {
      const resp = await fetch(`/api/admin/search?q=${encodeURIComponent(query)}`, { signal: remoteAbort.signal });
      if (!resp.ok) return;
      const data = await resp.json();
      if (!Array.isArray(data.results) || data.results.length === 0) return;
      const remote = data.results.slice(0, 8).map((r) => ({
        kind: "remote",
        title: r.title,
        subtitle: r.subtitle,
        url: r.url,
        keywords: "",
        remoteType: r.type,
      }));
      // Merge: keep the static query-filtered list, prepend remote results
      const q = input.value.trim().toLowerCase();
      const local = COMMANDS
        .map((c) => ({ c, score: score(c, q) }))
        .filter((x) => x.score > 0)
        .sort((a, b) => b.score - a.score)
        .slice(0, 12)
        .map((x) => x.c);
      const items = [...remote, ...local];
      currentItems = items;
      paintResults(items, q);
    } catch (e) {
      // ignore aborts and network errors
    }
  }
})();
