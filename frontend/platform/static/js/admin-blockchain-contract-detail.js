// Admin Blockchain Contract Detail Controller

(function () {
  "use strict";

  const ADDRESS_RE = /^0x[a-fA-F0-9]{40}$/;

  const els = {};
  let contractAddress = "";

  document.addEventListener("DOMContentLoaded", () => {
    cacheElements();
    init().catch((err) => {
      console.error("Failed to load clone detail:", err);
      setTitle("Error Loading Contract");
      renderMessageRow("Failed to load blockchain metadata: " + getErrorMessage(err), "danger");
      setStatus("Contract metadata failed to load.");
    });
  });

  function cacheElements() {
    els.title = document.getElementById("page-asset-title");
    els.cloneAddress = document.getElementById("clone-address");
    els.copyCloneAddress = document.getElementById("copy-clone-address");
    els.contractLink = document.getElementById("contract-link");
    els.statusBadge = document.getElementById("kpi-live-status");
    els.kpiSupply = document.getElementById("kpi-supply");
    els.kpiSold = document.getElementById("kpi-sold");
    els.kpiSoldBar = document.getElementById("kpi-sold-bar");
    els.kpiHoldersCount = document.getElementById("kpi-holders-count");
    els.holdersTbody = document.getElementById("holders-tbody");
    els.freezeButton = document.getElementById("btn-freeze-transfers");
    els.refreshButton = document.getElementById("refresh-contract-detail");
    els.statusRegion = document.getElementById("contract-detail-status");
  }

  async function init() {
    const urlParams = new URLSearchParams(window.location.search);
    const address = (urlParams.get("address") || "").trim();

    if (!ADDRESS_RE.test(address)) {
      contractAddress = "";
      setTitle(address ? "Invalid Contract Address" : "No Address Specified");
      setText(els.cloneAddress, "N/A");
      renderMessageRow("Invalid URL parameter.", "danger");
      setStatus("Invalid contract address.");
      disableFreeze("Invalid Contract Address");
      return;
    }

    contractAddress = address.toLowerCase();
    setText(els.cloneAddress, contractAddress);
    setupExplorerLink(els.contractLink, contractAddress);
    setupCopyButton(els.copyCloneAddress, contractAddress);
    setupRefreshButton();
    setupFreezeButton();

    const response = await fetch(`/api/admin/blockchain/contracts/${contractAddress}/detail`);
    if (!response.ok) {
      const body = await response.json().catch(() => ({}));
      throw new Error(body.error || `API Error: ${response.status}`);
    }

    const data = await response.json();
    renderDetail(data);
  }

  function renderDetail(data) {
    setTitle(data.title || "Untitled Contract");

    const totalSupply = toSafeNumber(data.total_supply);
    const tokensSold = toSafeNumber(data.tokens_sold);
    const rawPercentSold = totalSupply > 0 ? (tokensSold / totalSupply) * 100 : 0;
    const percentSold = clamp(rawPercentSold, 0, 100);

    setText(els.kpiSupply, totalSupply.toLocaleString());
    renderSoldKpi(tokensSold, rawPercentSold);
    els.kpiSoldBar.style.width = `${percentSold.toFixed(1)}%`;
    setText(els.kpiHoldersCount, Array.isArray(data.holders) ? data.holders.length.toLocaleString() : "0");
    renderPauseState(data.pause_state || (data.is_paused ? "paused" : "unknown"));
    renderHolders(Array.isArray(data.holders) ? data.holders : [], totalSupply);
    setStatus(`Loaded contract ${data.title || contractAddress}.`);
  }

  function renderSoldKpi(tokensSold, rawPercentSold) {
    els.kpiSold.replaceChildren();
    els.kpiSold.appendChild(document.createTextNode(tokensSold.toLocaleString() + " "));

    const pct = document.createElement("span");
    pct.style.fontSize = "16px";
    pct.style.fontWeight = "600";
    pct.style.color = "var(--admin-text-muted)";
    pct.textContent = `(${rawPercentSold.toFixed(1)}%)`;
    els.kpiSold.appendChild(pct);
  }

  function renderPauseState(pauseState) {
    els.statusBadge.replaceChildren();
    const badge = document.createElement("span");
    badge.style.padding = "6px 12px";
    badge.style.fontSize = "13px";

    const dot = document.createElement("span");
    dot.className = "contract-status-dot";
    badge.appendChild(dot);

    if (pauseState === "paused") {
      badge.className = "admin-badge admin-badge--warning";
      dot.classList.add("contract-status-dot--paused");
      badge.appendChild(document.createTextNode(" Contract Paused"));
      els.statusBadge.appendChild(badge);
      els.freezeButton.textContent = "Unfreeze Token Transfers (Activate)";
      els.freezeButton.classList.remove("admin-btn--danger");
      els.freezeButton.classList.add("admin-btn--success");
      els.freezeButton.dataset.isPaused = "true";
      els.freezeButton.disabled = false;
      els.freezeButton.setAttribute("aria-disabled", "false");
      return;
    }

    if (pauseState === "live") {
      badge.className = "admin-badge admin-badge--success";
      dot.classList.add("contract-status-dot--live");
      badge.appendChild(document.createTextNode(" Live Clone"));
      els.statusBadge.appendChild(badge);
      els.freezeButton.textContent = "Freeze Token Transfers (Pause)";
      els.freezeButton.classList.remove("admin-btn--success");
      els.freezeButton.classList.add("admin-btn--danger");
      els.freezeButton.dataset.isPaused = "false";
      els.freezeButton.disabled = false;
      els.freezeButton.setAttribute("aria-disabled", "false");
      return;
    }

    badge.className = "admin-badge admin-badge--warning";
    dot.classList.add("contract-status-dot--paused");
    badge.appendChild(document.createTextNode(" State Unknown"));
    els.statusBadge.appendChild(badge);
    disableFreeze("Contract State Unknown");
  }

  function renderHolders(holders, totalSupply) {
    els.holdersTbody.replaceChildren();

    if (holders.length === 0) {
      renderMessageRow("No on-chain holders found for this contract.", "muted");
      return;
    }

    holders.forEach((holder) => {
      const wallet = typeof holder.wallet_address === "string" ? holder.wallet_address : "";
      const safeWallet = ADDRESS_RE.test(wallet) ? wallet : "";
      const balance = toSafeNumber(holder.balance);
      const holderPercent = totalSupply > 0 ? clamp((balance / totalSupply) * 100, 0, 100) : 0;
      const tr = document.createElement("tr");

      const walletTd = document.createElement("td");
      const walletWrap = document.createElement("div");
      walletWrap.style.cssText = "display:flex;align-items:center;gap:12px;";

      const avatar = document.createElement("div");
      avatar.setAttribute("aria-hidden", "true");
      avatar.style.cssText = "width:24px;height:24px;border-radius:4px;opacity:0.8;background:linear-gradient(135deg,var(--admin-accent),var(--admin-success));";

      const walletTextWrap = document.createElement("div");
      const addressWrap = document.createElement("div");
      addressWrap.className = "wallet-address-display";
      addressWrap.style.cssText = "padding:2px 6px;font-size:12px;border:none;background:none;";

      const walletLink = document.createElement("a");
      walletLink.className = "basescan-link";
      walletLink.target = "_blank";
      walletLink.rel = "noopener noreferrer";
      walletLink.textContent = safeWallet ? shortAddress(safeWallet) : "Invalid wallet";
      if (safeWallet) {
        walletLink.href = explorerUrl(safeWallet);
      }
      addressWrap.appendChild(walletLink);

      const copyBtn = createCopyButton(safeWallet, "Copy holder wallet address");
      addressWrap.appendChild(copyBtn);

      const email = document.createElement("div");
      email.style.cssText = "font-size:11px;color:var(--admin-text-muted);margin-left:6px;";
      email.textContent = holder.email || "Unknown holder";

      walletTextWrap.append(addressWrap, email);
      walletWrap.append(avatar, walletTextWrap);
      walletTd.appendChild(walletWrap);

      const balanceTd = document.createElement("td");
      const balanceDiv = document.createElement("div");
      balanceDiv.style.cssText = "font-family:'SF Mono',monospace;font-size:14px;font-weight:700;color:var(--admin-text-primary);";
      balanceDiv.textContent = balance.toLocaleString();
      balanceTd.appendChild(balanceDiv);

      const percentTd = document.createElement("td");
      const percentWrap = document.createElement("div");
      percentWrap.style.cssText = "display:flex;align-items:center;gap:8px;";
      const percentLabel = document.createElement("span");
      percentLabel.style.cssText = "font-size:12px;font-weight:600;width:40px;";
      percentLabel.textContent = `${holderPercent.toFixed(2)}%`;
      const bar = document.createElement("div");
      bar.className = "token-supply-bar";
      bar.style.cssText = "width:80px;margin-top:0;";
      const sold = document.createElement("div");
      sold.className = "sold";
      sold.style.width = `${holderPercent.toFixed(2)}%`;
      bar.appendChild(sold);
      percentWrap.append(percentLabel, bar);
      percentTd.appendChild(percentWrap);

      const syncTd = document.createElement("td");
      const sync = document.createElement("div");
      sync.style.cssText = "font-size:12px;color:var(--admin-text-muted);";
      sync.textContent = holder.last_synced_at || "Not synced";
      syncTd.appendChild(sync);

      tr.append(walletTd, balanceTd, percentTd, syncTd);
      els.holdersTbody.appendChild(tr);
    });
  }

  function setupFreezeButton() {
    els.freezeButton.addEventListener("click", async () => {
      if (!contractAddress || els.freezeButton.disabled) return;

      const isPaused = els.freezeButton.dataset.isPaused === "true";
      const action = isPaused ? "UNPAUSE" : "FREEZE";
      const endpoint = isPaused
        ? `/api/admin/blockchain/contracts/${contractAddress}/unpause`
        : `/api/admin/blockchain/contracts/${contractAddress}/pause`;

      const confirmed = await confirmAction({
        title: `${action === "FREEZE" ? "Freeze" : "Unfreeze"} token transfers`,
        message: `This will ${isPaused ? "resume" : "halt"} transfers for ${contractAddress}. Continue only for a verified operational, legal, or security reason.`,
        confirmText: action === "FREEZE" ? "Freeze Transfers" : "Unfreeze Transfers",
        type: action === "FREEZE" ? "danger" : "warning",
      });
      if (!confirmed) return;

      const originalText = els.freezeButton.textContent;
      els.freezeButton.disabled = true;
      els.freezeButton.setAttribute("aria-busy", "true");
      els.freezeButton.textContent = `${action === "FREEZE" ? "Pausing" : "Unpausing"}... sending TX`;
      setStatus(`${action} request submitted.`);

      try {
        const resp = await fetch(endpoint, { method: "POST" });
        if (!resp.ok) {
          const err = await resp.json().catch(() => ({}));
          throw new Error(err.error || `HTTP ${resp.status}`);
        }
        const result = await resp.json();
        showToast(`${action} successful. Tx Hash: ${result.tx_hash || "unknown"}`, "success");
        setStatus(`${action} successful.`);
        window.location.reload();
      } catch (err) {
        showToast(`${action} failed: ${getErrorMessage(err)}`, "error");
        setStatus(`${action} failed.`);
        els.freezeButton.disabled = false;
        els.freezeButton.removeAttribute("aria-busy");
        els.freezeButton.textContent = originalText;
      }
    });
  }

  function setupRefreshButton() {
    if (!els.refreshButton) return;
    els.refreshButton.addEventListener("click", () => {
      setStatus("Refreshing contract detail.");
      window.location.reload();
    });
  }

  async function confirmAction(options) {
    if (typeof window.pooolConfirm === "function") {
      return window.pooolConfirm(options);
    }
    showToast("Confirmation dialog unavailable. Please refresh and try again.", "error");
    return false;
  }

  function renderMessageRow(message, type) {
    if (!els.holdersTbody) return;
    els.holdersTbody.replaceChildren();
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = 4;
    td.style.cssText = "text-align:center;padding:40px;";
    td.style.color = type === "danger" ? "var(--admin-danger)" : "var(--admin-text-muted)";
    td.textContent = message;
    tr.appendChild(td);
    els.holdersTbody.appendChild(tr);
  }

  function disableFreeze(text) {
    els.freezeButton.textContent = text;
    els.freezeButton.disabled = true;
    els.freezeButton.setAttribute("aria-disabled", "true");
    els.freezeButton.dataset.isPaused = "";
  }

  function setupExplorerLink(link, address) {
    if (!link) return;
    link.href = explorerUrl(address);
    link.rel = "noopener noreferrer";
  }

  function explorerUrl(address) {
    return `https://amoy.polygonscan.com/address/${encodeURIComponent(address)}`;
  }

  function setupCopyButton(button, text) {
    if (!button) return;
    button.addEventListener("click", () => copyText(text));
  }

  function createCopyButton(text, label) {
    const button = document.createElement("button");
    button.className = "copy-btn";
    button.type = "button";
    button.title = label;
    button.setAttribute("aria-label", label);
    button.disabled = !text;
    button.appendChild(copyIcon(10));
    if (text) button.addEventListener("click", () => copyText(text));
    return button;
  }

  async function copyText(text) {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      showToast("Address copied", "success");
      setStatus("Address copied.");
    } catch (_err) {
      showToast("Copy failed. Select the address manually.", "error");
      setStatus("Address copy failed.");
    }
  }

  function copyIcon(size) {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("width", String(size));
    svg.setAttribute("height", String(size));
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("fill", "none");
    svg.setAttribute("stroke", "currentColor");
    svg.setAttribute("stroke-width", "2");
    svg.setAttribute("aria-hidden", "true");

    const rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    rect.setAttribute("x", "9");
    rect.setAttribute("y", "9");
    rect.setAttribute("width", "13");
    rect.setAttribute("height", "13");
    rect.setAttribute("rx", "2");
    rect.setAttribute("ry", "2");

    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute("d", "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1");

    svg.append(rect, path);
    return svg;
  }

  function showToast(message, type) {
    if (window.showPooolToast) {
      window.showPooolToast(null, message, type || "info");
      return;
    }

    let toast = document.getElementById("contract-detail-toast");
    if (!toast) {
      toast = document.createElement("div");
      toast.id = "contract-detail-toast";
      toast.setAttribute("role", "status");
      toast.style.cssText = "position:fixed;right:24px;bottom:24px;z-index:99999;padding:12px 16px;border-radius:8px;color:#fff;font-weight:600;box-shadow:0 12px 32px rgba(0,0,0,.18);";
      document.body.appendChild(toast);
    }
    toast.style.background = type === "error" ? "#D92D20" : "#027A48";
    toast.textContent = message;
    toast.style.display = "block";
    window.clearTimeout(toast._hideTimer);
    toast._hideTimer = window.setTimeout(() => {
      toast.style.display = "none";
    }, 4000);
  }

  function setStatus(message) {
    if (els.statusRegion) els.statusRegion.textContent = message;
  }

  function setTitle(title) {
    setText(els.title, title);
  }

  function setText(element, text) {
    if (element) element.textContent = text;
  }

  function toSafeNumber(value) {
    const number = Number(value);
    return Number.isFinite(number) ? number : 0;
  }

  function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
  }

  function shortAddress(address) {
    return `${address.substring(0, 8)}...${address.substring(36)}`;
  }

  function getErrorMessage(err) {
    return err && err.message ? err.message : "Unknown error";
  }
})();
