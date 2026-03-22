/**
 * Live Contracts Page Controller
 * Fetches data from /api/admin/blockchain/treasury and populates the table
 */
document.addEventListener("DOMContentLoaded", () => {
    initLiveContracts();
});

async function initLiveContracts() {
    const tbody = document.getElementById("contracts-tbody");
    if (!tbody) return;

    try {
        const response = await fetch("/api/admin/blockchain/treasury");
        
        if (response.status === 401 || response.status === 403) {
            tbody.innerHTML = `<tr><td colspan="6" style="text-align:center; padding:40px; color:var(--admin-danger);">Unauthorized. Insufficient permissions.</td></tr>`;
            return;
        }
        
        if (!response.ok) {
            throw new Error(`API Error: ${response.status}`);
        }

        const data = await response.json();
        const assets = data.tokenized_assets || [];
        const explorerBaseUrl = data.explorer_url || "https://amoy.polygonscan.com";
        
        document.getElementById("total-count-badge").textContent = `Total: ${assets.length} Clones`;

        renderKPIs(assets);
        renderContractsTable(assets, explorerBaseUrl);
    } catch (e) {
        console.error("Failed to fetch live contracts:", e);
        tbody.innerHTML = `<tr><td colspan="6" style="text-align:center; padding:40px; color:var(--admin-danger);">Failed to load blockchain data: ${e.message}</td></tr>`;
    }
}

function renderKPIs(assets) {
    let totalSupply = 0;
    let totalDistributed = 0;

    assets.forEach(asset => {
        if (asset.chain_contract_address) {
            totalSupply += asset.tokens_total || 0;
            totalDistributed += (asset.tokens_total || 0) - (asset.tokens_available || 0);
        }
    });

    const activeClones = assets.filter(a => a.chain_contract_address).length;

    document.getElementById("kpi-active-clones").textContent = activeClones.toLocaleString();
    document.getElementById("kpi-total-supply").textContent = totalSupply.toLocaleString();
    document.getElementById("kpi-distributed").textContent = totalDistributed.toLocaleString();

    let percent = totalSupply > 0 ? ((totalDistributed / totalSupply) * 100).toFixed(1) : "0.0";
    document.getElementById("kpi-distributed-sub").textContent = `${percent}% of total supply`;
}

function renderContractsTable(assets, explorerBaseUrl) {
    const tbody = document.getElementById("contracts-tbody");
    const networkNames = {
        "polygon_amoy": "Polygon Amoy",
        "polygon": "Polygon PoS"
    };
    
    if (!assets || assets.length === 0) {
        tbody.innerHTML = `
            <tr>
                <td colspan="6" style="text-align:center; padding:40px;">
                    <span style="display:block; font-size:16px; font-weight:600; color:var(--admin-text-primary); margin-bottom:8px;">No Smart Contracts Found</span>
                    <span style="color:var(--admin-text-muted); font-size:13px;">Deploy your first EIP-1167 Clone via the Tokenize interface.</span>
                    <br><br>
                    <a href="/admin/asset-tokenize.html" class="admin-btn admin-btn--primary">Tokenize Asset</a>
                </td>
            </tr>
        `;
        return;
    }

    tbody.innerHTML = assets.map(asset => {
        const networkDisplay = networkNames[asset.chain_network] || asset.chain_network || "Polygon Amoy";
        const tokensSold = asset.tokens_total - asset.tokens_available;
        const percentSold = asset.tokens_total > 0 ? ((tokensSold / asset.tokens_total) * 100).toFixed(1) : 0;
        
        const contractLinkUrl = asset.chain_contract_address ? `${explorerBaseUrl}/address/${asset.chain_contract_address}` : "#";
        const shortContractAddress = asset.chain_contract_address ? `${asset.chain_contract_address.substring(0,8)}...${asset.chain_contract_address.substring(36)}` : "Pending...";
        
        const txLink = asset.chain_tx_hash ? `<a href="${explorerBaseUrl}/tx/${asset.chain_tx_hash}" target="_blank" class="admin-link">Tx History</a>` : "";

        return `
            <tr>
                <td>
                    <div style="display:flex; align-items:center; gap:12px;">
                        <div style="width: 32px; height: 32px; border-radius: 6px; background: rgba(56, 189, 248, 0.1); border: 1px solid rgba(56, 189, 248, 0.2); display: flex; align-items: center; justify-content: center; color: var(--admin-accent);">
                            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path></svg>
                        </div>
                        <div>
                            <div style="font-weight:600; color:var(--admin-text-primary); margin-bottom:2px;">${asset.title}</div>
                            <div style="font-size:11px; color:var(--admin-text-muted); font-family: 'SF Mono', monospace;">ID: ${asset.id.split('-')[0]}...</div>
                        </div>
                    </div>
                </td>
                <td>
                    ${asset.chain_contract_address ? `
                        <div class="wallet-address-display">
                            <a href="${contractLinkUrl}" target="_blank" class="basescan-link" title="${asset.chain_contract_address}">
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6M15 3h6v6M10 14L21 3"/></svg>
                                ${shortContractAddress}
                            </a>
                            <button class="copy-btn" title="Copy Address" onclick="window._copyAddr('${asset.chain_contract_address}')">
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                            </button>
                        </div>
                        <div style="font-size: 11px; color: var(--admin-text-muted); margin-top: 4px; display: flex; align-items: center; gap: 4px;">
                            <div style="width: 6px; height: 6px; border-radius: 50%; background: #a855f7;"></div>
                            ${networkDisplay}
                        </div>
                    ` : `<span style="color:var(--admin-text-muted); font-size: 12px;">Not Tokenized</span>`}
                </td>
                <td>
                    <div style="font-family:'SF Mono', monospace; font-size:14px; font-weight:700; color:var(--admin-text-primary);">
                        ${(asset.tokens_total || 0).toLocaleString()}
                    </div>
                    <div style="font-size: 11px; color: var(--admin-text-muted);">Tokens</div>
                </td>
                <td>
                    <div style="max-width: 140px;">
                        <div style="font-size:12px; font-weight:600; display:flex; justify-content:space-between; margin-bottom:4px;">
                            <span style="color:var(--admin-accent)">${tokensSold.toLocaleString()}</span>
                            <span style="color:var(--admin-text-muted)">${percentSold}%</span>
                        </div>
                        <div class="token-supply-bar">
                            <div class="sold" style="width: ${percentSold}%"></div>
                        </div>
                    </div>
                </td>
                <td>
                    ${asset.chain_contract_address ? '<span class="admin-badge admin-badge--success" style="padding: 4px 8px; font-weight: 600;"><span class="contract-status-dot contract-status-dot--live"></span> Live Clone</span>' : '<span class="admin-badge admin-badge--warning">Pending</span>'}
                </td>
                <td>
                    <div style="display:flex; flex-direction: column; gap:6px; align-items:flex-start;">
                        ${asset.chain_contract_address ? `<a href="/admin/blockchain-contract-detail.html?address=${asset.chain_contract_address}" class="admin-btn admin-btn--secondary" style="padding: 6px 12px; font-size: 11px;">View Clone</a>` : ''}
                        ${txLink}
                    </div>
                </td>
            </tr>
        `;
    }).join("");
}
