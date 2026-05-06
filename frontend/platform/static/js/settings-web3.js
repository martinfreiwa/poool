/**
 * settings-web3.js — Settings → Web3 Wallet card
 *
 * Reads the user's currently bound chain wallet via GET /api/me/chain-wallet,
 * renders the connected/disconnected state, and runs the SIWE bind flow:
 *
 *   1. eth_requestAccounts          — get the active account
 *   2. POST /api/kyc/wallet/challenge {address} → {nonce, message}
 *   3. personal_sign(message)       — user signs via their wallet
 *   4. POST /api/kyc/wallet/bind {address, signature}
 *   5. Refresh state
 *
 * No gas, no transaction. The bound address is then picked up by the
 * KYC whitelist worker which calls setWhitelisted() on-chain.
 */
(function () {
  "use strict";

  const $ = (id) => document.getElementById(id);

  function csrfHeaders(extra) {
    const headers = Object.assign({ "Content-Type": "application/json" }, extra || {});
    const token = (typeof window.getCsrfToken === "function" && window.getCsrfToken()) ||
      (document.cookie.match(/(?:^|; )csrf_token=([^;]+)/) || [, ""])[1];
    if (token) headers["X-CSRF-Token"] = token;
    return headers;
  }

  const EXPLORERS = {
    polygon: "https://polygonscan.com",
    polygon_amoy: "https://amoy.polygonscan.com",
  };

  let currentNetwork = "polygon_amoy";

  function shorten(addr) {
    if (!addr || addr.length < 12) return addr || "";
    return addr.slice(0, 6) + "…" + addr.slice(-4);
  }

  function showError(msg) {
    const e = $("web3-error");
    if (!e) return;
    if (!msg) { e.hidden = true; e.textContent = ""; return; }
    e.hidden = false;
    e.textContent = msg;
  }

  function setView(view) {
    const map = { loading: "web3-loading", connected: "web3-connected", disconnected: "web3-disconnected" };
    Object.values(map).forEach((id) => { const el = $(id); if (el) el.hidden = true; });
    const target = $(map[view]);
    if (target) target.hidden = false;
  }

  function renderConnected(address, whitelistedAt, network) {
    currentNetwork = network || currentNetwork;
    const explorer = EXPLORERS[currentNetwork] || EXPLORERS.polygon;
    const addrEl = $("web3-address");
    if (addrEl) addrEl.textContent = address;
    const link = $("web3-explorer-link");
    if (link) link.href = explorer + "/address/" + address;
    const status = $("web3-whitelist-status");
    if (status) {
      if (whitelistedAt) {
        status.innerHTML = '<span style="color:#067647;">✓ Whitelisted</span> ' +
          '<span style="color:#667085; font-size:12px;">— your wallet is approved on-chain to receive property NFTs.</span>';
      } else {
        status.innerHTML = '<span style="color:#B54708;">⏳ Pending</span> ' +
          '<span style="color:#667085; font-size:12px;">— bound, awaiting next on-chain whitelist sync (runs every minute).</span>';
      }
    }
    setView("connected");
  }

  async function loadWalletStatus() {
    setView("loading");
    showError("");
    try {
      const res = await fetch("/api/me/chain-wallet", { credentials: "include" });
      if (!res.ok) {
        if (res.status === 401) { window.location.href = "/login"; return; }
        throw new Error("HTTP " + res.status);
      }
      const data = await res.json();
      currentNetwork = data.chain_network || "polygon_amoy";
      if (data.address) {
        renderConnected(data.address, data.whitelisted_at, data.chain_network);
      } else {
        setView("disconnected");
      }
    } catch (err) {
      console.error("web3 status load failed:", err);
      setView("disconnected");
      showError("Could not load wallet status. " + (err.message || ""));
    }
  }

  async function ensureNetwork() {
    const eth = window.ethereum;
    if (!eth) return;
    const target = currentNetwork === "polygon"
      ? { chainId: "0x89" }
      : { chainId: "0x13882" };
    try {
      const cur = await eth.request({ method: "eth_chainId" });
      if (cur && cur.toLowerCase() === target.chainId.toLowerCase()) return;
      await eth.request({ method: "wallet_switchEthereumChain", params: [target] });
    } catch (e) {
      if (e && (e.code === 4902 || (e.data && e.data.originalError && e.data.originalError.code === 4902))) {
        const params = currentNetwork === "polygon"
          ? {
              chainId: "0x89",
              chainName: "Polygon",
              nativeCurrency: { name: "POL", symbol: "POL", decimals: 18 },
              rpcUrls: ["https://polygon-rpc.com"],
              blockExplorerUrls: ["https://polygonscan.com"],
            }
          : {
              chainId: "0x13882",
              chainName: "Polygon Amoy Testnet",
              nativeCurrency: { name: "POL", symbol: "POL", decimals: 18 },
              rpcUrls: ["https://rpc-amoy.polygon.technology"],
              blockExplorerUrls: ["https://amoy.polygonscan.com"],
            };
        await eth.request({ method: "wallet_addEthereumChain", params: [params] });
      }
    }
  }

  async function connect() {
    showError("");
    const btn = $("btn-web3-connect");
    if (btn) { btn.disabled = true; btn.textContent = "Connecting…"; }
    try {
      const eth = window.ethereum;
      if (!eth || typeof eth.request !== "function") {
        throw new Error("No browser wallet detected. Install MetaMask first.");
      }
      // 1. Active account.
      const accounts = await eth.request({ method: "eth_requestAccounts" });
      if (!accounts || !accounts[0]) throw new Error("No account selected.");
      const address = accounts[0];

      // 2. Switch network so the user signs from the right chain.
      await ensureNetwork();

      // 3. Server challenge.
      const chRes = await fetch("/api/kyc/wallet/challenge", {
        method: "POST",
        credentials: "include",
        headers: csrfHeaders(),
        body: JSON.stringify({ address }),
      });
      if (!chRes.ok) {
        const j = await chRes.json().catch(() => ({}));
        throw new Error(j.error || ("Challenge failed (HTTP " + chRes.status + ")"));
      }
      const { message, address: serverAddress } = await chRes.json();

      // 4. Personal sign — user is prompted in MetaMask.
      const signature = await eth.request({
        method: "personal_sign",
        params: [message, serverAddress],
      });

      // 5. Bind.
      const bindRes = await fetch("/api/kyc/wallet/bind", {
        method: "POST",
        credentials: "include",
        headers: csrfHeaders(),
        body: JSON.stringify({ address: serverAddress, signature }),
      });
      if (!bindRes.ok) {
        const j = await bindRes.json().catch(() => ({}));
        throw new Error(j.error || ("Bind failed (HTTP " + bindRes.status + ")"));
      }

      await loadWalletStatus();
    } catch (err) {
      console.error("wallet connect failed:", err);
      const code = err && err.code;
      const msg = code === 4001
        ? "You rejected the request in your wallet."
        : (err && err.message) || String(err);
      showError(msg);
    } finally {
      if (btn) {
        btn.disabled = false;
        btn.innerHTML = '<svg width="16" height="16" viewBox="0 0 32 32" aria-hidden="true" style="margin-right:6px;vertical-align:middle;"><path d="M28.8 3.2 17.6 11.4l2.1-4.9z" fill="#E2761B"/><path d="M3.2 3.2 14.3 11.5 12.3 6.5z" fill="#E4761B"/><path d="M24.5 21.6l-3 4.6 6.4 1.8 1.8-6.3z" fill="#E4761B"/><path d="M2.4 21.7l1.8 6.3 6.4-1.8-3-4.6z" fill="#E4761B"/><path d="M10.2 14.4 8.5 17l6.3.3-.2-6.8z" fill="#E4761B"/><path d="M21.8 14.4 17.4 10.4l-.1 6.9 6.3-.3z" fill="#E4761B"/><path d="M10.6 26.2 14.4 24.3 11.1 21.8z" fill="#E4761B"/><path d="M17.6 24.3 21.4 26.2 20.9 21.8z" fill="#E4761B"/></svg> Connect Wallet';
      }
    }
  }

  async function copyAddress() {
    const addr = ($("web3-address") || {}).textContent || "";
    if (!addr) return;
    try {
      await navigator.clipboard.writeText(addr);
      const btn = $("btn-web3-copy");
      if (btn) {
        const original = btn.textContent;
        btn.textContent = "Copied!";
        setTimeout(() => { btn.textContent = original; }, 1200);
      }
    } catch (_) {}
  }

  function rebind() {
    if (!confirm(
      "Rebinding will replace the wallet associated with your account.\n\n" +
      "Tokens already held by your current wallet will NOT be moved automatically — " +
      "they remain at the old address. Continue?"
    )) return;
    setView("disconnected");
    showError("");
    // Don't actually clear the server-side address — the bind endpoint
    // will refuse to overwrite a different address by design. The user
    // must contact support to fully replace; this UI just lets them
    // re-confirm their existing address signature.
  }

  function init() {
    if (!$("settings-web3-card")) return; // section not on this page
    const connectBtn = $("btn-web3-connect");
    const copyBtn = $("btn-web3-copy");
    const rebindBtn = $("btn-web3-rebind");
    if (connectBtn) connectBtn.addEventListener("click", connect);
    if (copyBtn) copyBtn.addEventListener("click", copyAddress);
    if (rebindBtn) rebindBtn.addEventListener("click", rebind);
    loadWalletStatus();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
