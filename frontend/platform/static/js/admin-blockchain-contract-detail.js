// Admin Blockchain Contract Detail Controller

document.addEventListener("DOMContentLoaded", async () => {
    const urlParams = new URLSearchParams(window.location.search);
    const address = urlParams.get('address');
    
    if (!address) {
        document.getElementById('page-asset-title').textContent = "No Address Specified";
        document.getElementById('clone-address').textContent = "N/A";
        document.getElementById('holders-tbody').innerHTML = `<tr><td colspan="4" style="text-align:center; padding:40px; color:var(--admin-danger);">Invalid URL parameter.</td></tr>`;
        return;
    }

    // Set initial UI states
    document.getElementById('clone-address').textContent = address;
    document.getElementById('contract-link').href = `https://amoy.polygonscan.com/address/${address}`;

    // Danger Zone Wireup — Per-Clone Pause/Unpause (SPV isolation)
    document.getElementById("btn-freeze-transfers").addEventListener("click", async () => {
        const isPaused = document.getElementById("btn-freeze-transfers").dataset.isPaused === "true";
        const action = isPaused ? "UNPAUSE" : "FREEZE";
        const endpoint = isPaused
            ? `/api/admin/blockchain/contracts/${address}/unpause`
            : `/api/admin/blockchain/contracts/${address}/pause`;

        if (!confirm(`CRITICAL WARNING:\n\nAre you sure you want to ${action} ALL TOKEN TRANSFERS for contract ${address}?\n\nThis will ${isPaused ? 'resume' : 'halt'} trading on this specific EIP-1167 clone.`)) {
            return;
        }

        const btn = document.getElementById("btn-freeze-transfers");
        const originalText = btn.textContent;
        btn.disabled = true;
        btn.textContent = `${action === "FREEZE" ? "Pausing" : "Unpausing"}... sending TX`;

        try {
            const resp = await fetch(endpoint, { method: "POST" });
            if (!resp.ok) {
                const err = await resp.json().catch(() => ({}));
                throw new Error(err.error || `HTTP ${resp.status}`);
            }
            const result = await resp.json();
            alert(`${action} successful!\n\nTx Hash: ${result.tx_hash || "unknown"}`);
            window.location.reload(); // Reload to reflect new state
        } catch (e) {
            alert(`${action} failed: ${e.message}`);
            btn.disabled = false;
            btn.textContent = originalText;
        }
    });

    try {
        const response = await fetch(`/api/admin/blockchain/contracts/${address}/detail`);
        
        if (!response.ok) {
            throw new Error(`API Error: ${response.status}`);
        }

        const data = await response.json();
        
        // Populate Header
        document.getElementById('page-asset-title').textContent = data.title;
        
        // Populate Status Badge
        const statusBadge = document.getElementById('kpi-live-status');
        if (data.is_paused) {
            statusBadge.innerHTML = `<span class="admin-badge admin-badge--warning" style="padding: 6px 12px; font-size: 13px;"><span class="contract-status-dot contract-status-dot--paused"></span> Contract Paused</span>`;
            document.getElementById("btn-freeze-transfers").textContent = "Unfreeze Token Transfers (Activate)";
            document.getElementById("btn-freeze-transfers").classList.replace("admin-btn--danger", "admin-btn--success");
            document.getElementById("btn-freeze-transfers").dataset.isPaused = "true";
        } else {
            statusBadge.innerHTML = `<span class="admin-badge admin-badge--success" style="padding: 6px 12px; font-size: 13px;"><span class="contract-status-dot contract-status-dot--live"></span> Live Clone</span>`;
            document.getElementById("btn-freeze-transfers").dataset.isPaused = "false";
        }

        // Populate KPIs
        const totalSupply = data.total_supply || 0;
        const tokensSold = data.tokens_sold || 0;
        const percentSold = totalSupply > 0 ? ((tokensSold / totalSupply) * 100).toFixed(1) : 0;
        
        document.getElementById('kpi-supply').textContent = totalSupply.toLocaleString();
        document.getElementById('kpi-sold').innerHTML = `${tokensSold.toLocaleString()} <span style="font-size:16px; font-weight:600; color:var(--admin-text-muted);">(${percentSold}%)</span>`;
        document.getElementById('kpi-sold-bar').style.width = `${percentSold}%`;
        document.getElementById('kpi-holders-count').textContent = (data.holders ? data.holders.length : 0).toLocaleString();

        // Populate Holders Table
        const tbody = document.getElementById('holders-tbody');
        if (!data.holders || data.holders.length === 0) {
            tbody.innerHTML = `<tr><td colspan="4" style="text-align:center; padding:40px; color:var(--admin-text-muted);">No on-chain holders found for this contract.</td></tr>`;
            return;
        }

        tbody.innerHTML = data.holders.map(holder => {
            const shortWallet = `${holder.wallet_address.substring(0,8)}...${holder.wallet_address.substring(36)}`;
            const holderPercent = totalSupply > 0 ? ((holder.balance / totalSupply) * 100).toFixed(2) : 0;
            const blockExplorerUrl = `https://amoy.polygonscan.com/address/${holder.wallet_address}`;
            const avatarUrl = `https://api.dicebear.com/7.x/identicon/svg?seed=${holder.wallet_address}`; // Use blockie style pseudorandom avatar

            return `
                <tr>
                  <td>
                    <div style="display:flex; align-items:center; gap:12px;">
                        <img src="${avatarUrl}" alt="Avatar" style="width:24px; height:24px; border-radius:4px; opacity:0.8;">
                        <div>
                            <div class="wallet-address-display" style="padding:2px 6px; font-size:12px; border:none; background:none;">
                                <a href="${blockExplorerUrl}" target="_blank" class="basescan-link">${shortWallet}</a>
                                <button class="copy-btn" title="Copy Address" onclick="window._copyAddr('${holder.wallet_address}')">
                                    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                                </button>
                            </div>
                            <div style="font-size:11px; color:var(--admin-text-muted); margin-left:6px;">${holder.email}</div>
                        </div>
                    </div>
                  </td>
                  <td>
                    <div style="font-family:'SF Mono', monospace; font-size:14px; font-weight:700; color:var(--admin-text-primary);">
                        ${holder.balance.toLocaleString()}
                    </div>
                  </td>
                  <td>
                    <div style="display:flex; align-items:center; gap:8px;">
                        <span style="font-size:12px; font-weight:600; width:40px;">${holderPercent}%</span>
                        <div class="token-supply-bar" style="width:80px; margin-top:0;">
                            <div class="sold" style="width: ${holderPercent}%"></div>
                        </div>
                    </div>
                  </td>
                  <td>
                    <div style="font-size:12px; color:var(--admin-text-muted);">${holder.last_synced_at}</div>
                  </td>
                </tr>
            `;
        }).join("");

    } catch (e) {
        console.error("Failed to load clone detail:", e);
        document.getElementById('holders-tbody').innerHTML = `<tr><td colspan="4" style="text-align:center; padding:40px; color:var(--admin-danger);">Failed to load blockchain metadata: ${e.message}</td></tr>`;
        document.getElementById('page-asset-title').textContent = "Error Loading Contract";
    }
});
