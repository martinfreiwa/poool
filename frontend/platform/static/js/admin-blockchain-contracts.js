/**
 * Live Contracts Page Controller
 * Fetches data from /api/admin/blockchain/treasury and populates the table.
 */
document.addEventListener("DOMContentLoaded", () => {
    initLiveContracts();
});

const NETWORK_NAMES = {
    polygon_amoy: "Polygon Amoy",
    polygon: "Polygon PoS",
};

const EXPLORER_ORIGINS = {
    polygon_amoy: "https://amoy.polygonscan.com",
    polygon: "https://polygonscan.com",
};

function createEl(tag, options = {}) {
    const el = document.createElement(tag);
    if (options.className) el.className = options.className;
    if (options.text !== undefined) el.textContent = options.text;
    if (options.cssText) el.style.cssText = options.cssText;
    if (options.attrs) {
        Object.entries(options.attrs).forEach(([name, value]) => {
            if (value !== undefined && value !== null) el.setAttribute(name, value);
        });
    }
    return el;
}

function appendChildren(parent, children) {
    children.forEach((child) => {
        if (child) parent.appendChild(child);
    });
    return parent;
}

function isValidContractAddress(value) {
    return typeof value === "string" && /^0x[a-fA-F0-9]{40}$/.test(value.trim());
}

function isValidTxHash(value) {
    return typeof value === "string" && /^0x[a-fA-F0-9]{64}$/.test(value.trim());
}

function safeInteger(value) {
    const number = Number(value);
    return Number.isInteger(number) && number >= 0 ? number : 0;
}

function explorerOrigin(network, fallbackUrl) {
    if (EXPLORER_ORIGINS[network]) return EXPLORER_ORIGINS[network];
    try {
        const parsed = new URL(fallbackUrl);
        if (parsed.origin === "https://polygonscan.com" || parsed.origin === "https://amoy.polygonscan.com") {
            return parsed.origin;
        }
    } catch (_err) {
        // Ignore invalid API values and fall back to Amoy.
    }
    return EXPLORER_ORIGINS.polygon_amoy;
}

function setStatus(message, type = "success") {
    const status = document.getElementById("contracts-status");
    if (!status) return;
    status.textContent = message;
    status.className = `contracts-status contracts-status--${type} is-visible`;
    window.clearTimeout(setStatus._timer);
    setStatus._timer = window.setTimeout(() => {
        status.classList.remove("is-visible");
    }, 2400);
}

async function initLiveContracts() {
    const tbody = document.getElementById("contracts-tbody");
    if (!tbody) return;

    try {
        const response = await fetch("/api/admin/blockchain/treasury");

        if (response.status === 401 || response.status === 403) {
            renderMessageRow(tbody, "Unauthorized. Insufficient permissions.", "error");
            return;
        }

        if (!response.ok) {
            throw new Error(`API Error: ${response.status}`);
        }

        const data = await response.json();
        const assets = Array.isArray(data.tokenized_assets) ? data.tokenized_assets : [];
        const totalCountBadge = document.getElementById("total-count-badge");
        if (totalCountBadge) totalCountBadge.textContent = `Total: ${assets.length} Clones`;

        renderKPIs(assets);
        renderContractsTable(assets, data.explorer_url || "https://amoy.polygonscan.com");
    } catch (e) {
        console.error("Failed to fetch live contracts:", e);
        renderMessageRow(tbody, `Failed to load blockchain data: ${e.message}`, "error");
    }
}

function renderKPIs(assets) {
    let totalSupply = 0;
    let totalDistributed = 0;

    assets.forEach((asset) => {
        if (isValidContractAddress(asset.chain_contract_address)) {
            const total = safeInteger(asset.tokens_total);
            const available = safeInteger(asset.tokens_available);
            totalSupply += total;
            totalDistributed += Math.max(total - available, 0);
        }
    });

    const activeClones = assets.filter((a) => isValidContractAddress(a.chain_contract_address)).length;
    const percent = totalSupply > 0 ? ((totalDistributed / totalSupply) * 100).toFixed(1) : "0.0";

    const active = document.getElementById("kpi-active-clones");
    const supply = document.getElementById("kpi-total-supply");
    const distributed = document.getElementById("kpi-distributed");
    const distributedSub = document.getElementById("kpi-distributed-sub");

    if (active) active.textContent = activeClones.toLocaleString();
    if (supply) supply.textContent = totalSupply.toLocaleString();
    if (distributed) distributed.textContent = totalDistributed.toLocaleString();
    if (distributedSub) distributedSub.textContent = `${percent}% of total supply`;
}

function renderMessageRow(tbody, message, type = "info", includeTokenizeLink = false) {
    tbody.replaceChildren();
    const row = document.createElement("tr");
    const cell = createEl("td", {
        attrs: { colspan: "6" },
        cssText: `text-align:center; padding:40px; color:var(${type === "error" ? "--admin-danger" : "--admin-text-muted"});`,
    });

    const title = createEl("span", {
        text: message,
        cssText: "display:block; font-size:16px; font-weight:600; color:var(--admin-text-primary); margin-bottom:8px;",
    });
    cell.appendChild(title);

    if (includeTokenizeLink) {
        const description = createEl("span", {
            text: "Deploy your first EIP-1167 Clone via the Tokenize interface.",
            cssText: "color:var(--admin-text-muted); font-size:13px;",
        });
        const link = createEl("a", {
            className: "admin-btn admin-btn--primary",
            text: "Tokenize Asset",
            attrs: { href: "/admin/asset-tokenize.html" },
        });
        link.style.marginTop = "16px";
        cell.append(description, document.createElement("br"), link);
    }

    row.appendChild(cell);
    tbody.appendChild(row);
}

function renderContractsTable(assets, explorerBaseUrl) {
    const tbody = document.getElementById("contracts-tbody");
    if (!tbody) return;

    if (!assets || assets.length === 0) {
        renderMessageRow(tbody, "No Smart Contracts Found", "info", true);
        return;
    }

    tbody.replaceChildren();
    assets.forEach((asset) => {
        tbody.appendChild(renderContractRow(asset, explorerBaseUrl));
    });
}

function renderContractRow(asset, explorerBaseUrl) {
    const row = document.createElement("tr");
    appendChildren(row, [
        renderAssetCell(asset),
        renderContractCell(asset, explorerBaseUrl),
        renderSupplyCell(asset),
        renderDistributionCell(asset),
        renderStatusCell(asset),
        renderActionsCell(asset, explorerBaseUrl),
    ]);
    return row;
}

function renderAssetCell(asset) {
    const cell = document.createElement("td");
    const wrapper = createEl("div", { cssText: "display:flex; align-items:center; gap:12px;" });
    const iconBox = createEl("div", {
        text: "◇",
        cssText: "width:32px; height:32px; border-radius:6px; background:rgba(56,189,248,0.1); border:1px solid rgba(56,189,248,0.2); display:flex; align-items:center; justify-content:center; color:var(--admin-accent); font-size:16px;",
    });
    const textWrap = document.createElement("div");
    const title = createEl("div", {
        text: asset.title || "Untitled asset",
        cssText: "font-weight:600; color:var(--admin-text-primary); margin-bottom:2px;",
    });
    const idPrefix = typeof asset.id === "string" && asset.id.length ? asset.id.split("-")[0] : "unknown";
    const id = createEl("div", {
        text: `ID: ${idPrefix}...`,
        cssText: "font-size:11px; color:var(--admin-text-muted); font-family:'SF Mono', monospace;",
    });
    textWrap.append(title, id);
    wrapper.append(iconBox, textWrap);
    cell.appendChild(wrapper);
    return cell;
}

function renderContractCell(asset, explorerBaseUrl) {
    const cell = document.createElement("td");
    const address = typeof asset.chain_contract_address === "string" ? asset.chain_contract_address.trim() : "";
    if (!isValidContractAddress(address)) {
        cell.appendChild(createEl("span", {
            text: "Not Tokenized",
            cssText: "color:var(--admin-text-muted); font-size:12px;",
        }));
        return cell;
    }

    const origin = explorerOrigin(asset.chain_network, explorerBaseUrl);
    const addressDisplay = createEl("div", { className: "wallet-address-display" });
    const link = createEl("a", {
        className: "basescan-link",
        text: `${address.substring(0, 8)}...${address.substring(36)}`,
        attrs: {
            href: `${origin}/address/${address}`,
            target: "_blank",
            rel: "noopener noreferrer",
            title: address,
        },
    });
    const copyButton = createEl("button", {
        className: "copy-btn",
        text: "Copy",
        attrs: { type: "button", "aria-label": `Copy contract address ${address}` },
    });
    copyButton.addEventListener("click", () => copyAddress(address));
    addressDisplay.append(link, copyButton);

    const network = createEl("div", {
        text: NETWORK_NAMES[asset.chain_network] || "Polygon Amoy",
        cssText: "font-size:11px; color:var(--admin-text-muted); margin-top:4px;",
    });
    cell.append(addressDisplay, network);
    return cell;
}

function renderSupplyCell(asset) {
    const cell = document.createElement("td");
    const total = safeInteger(asset.tokens_total);
    const value = createEl("div", {
        text: total.toLocaleString(),
        cssText: "font-family:'SF Mono', monospace; font-size:14px; font-weight:700; color:var(--admin-text-primary);",
    });
    const label = createEl("div", {
        text: "Tokens",
        cssText: "font-size:11px; color:var(--admin-text-muted);",
    });
    cell.append(value, label);
    return cell;
}

function renderDistributionCell(asset) {
    const cell = document.createElement("td");
    const total = safeInteger(asset.tokens_total);
    const available = safeInteger(asset.tokens_available);
    const tokensSold = Math.max(total - available, 0);
    const percentSold = total > 0 ? Math.min((tokensSold / total) * 100, 100).toFixed(1) : "0.0";
    const wrapper = createEl("div", { cssText: "max-width:140px;" });
    const line = createEl("div", {
        cssText: "font-size:12px; font-weight:600; display:flex; justify-content:space-between; margin-bottom:4px;",
    });
    line.append(
        createEl("span", { text: tokensSold.toLocaleString(), cssText: "color:var(--admin-accent);" }),
        createEl("span", { text: `${percentSold}%`, cssText: "color:var(--admin-text-muted);" })
    );
    const bar = createEl("div", { className: "token-supply-bar" });
    const sold = createEl("div", { className: "sold" });
    sold.style.width = `${percentSold}%`;
    bar.appendChild(sold);
    wrapper.append(line, bar);
    cell.appendChild(wrapper);
    return cell;
}

function renderStatusCell(asset) {
    const cell = document.createElement("td");
    const live = isValidContractAddress(asset.chain_contract_address);
    const badge = createEl("span", {
        className: `admin-badge ${live ? "admin-badge--success" : "admin-badge--warning"}`,
        text: live ? "Live Clone" : "Pending",
    });
    badge.style.padding = "4px 8px";
    badge.style.fontWeight = "600";
    cell.appendChild(badge);
    return cell;
}

function renderActionsCell(asset, explorerBaseUrl) {
    const cell = document.createElement("td");
    const actions = createEl("div", {
        cssText: "display:flex; flex-direction:column; gap:6px; align-items:flex-start;",
    });
    const address = typeof asset.chain_contract_address === "string" ? asset.chain_contract_address.trim() : "";
    if (isValidContractAddress(address)) {
        actions.appendChild(createEl("a", {
            className: "admin-btn admin-btn--secondary",
            text: "View Clone",
            attrs: { href: `/admin/blockchain-contract-detail.html?address=${encodeURIComponent(address)}` },
            cssText: "padding:6px 12px; font-size:11px;",
        }));
    }
    if (isValidTxHash(asset.chain_tx_hash)) {
        const origin = explorerOrigin(asset.chain_network, explorerBaseUrl);
        actions.appendChild(createEl("a", {
            className: "admin-link",
            text: "Tx History",
            attrs: {
                href: `${origin}/tx/${asset.chain_tx_hash.trim()}`,
                target: "_blank",
                rel: "noopener noreferrer",
            },
        }));
    }
    cell.appendChild(actions);
    return cell;
}

async function copyAddress(address) {
    try {
        if (navigator.clipboard?.writeText) {
            await navigator.clipboard.writeText(address);
        } else {
            const textarea = createEl("textarea", { text: address });
            textarea.style.position = "fixed";
            textarea.style.opacity = "0";
            document.body.appendChild(textarea);
            textarea.select();
            document.execCommand("copy");
            textarea.remove();
        }
        setStatus("Contract address copied.", "success");
    } catch (err) {
        console.error("Copy failed", err);
        setStatus("Could not copy contract address.", "error");
    }
}
