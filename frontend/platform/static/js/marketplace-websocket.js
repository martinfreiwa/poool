// frontend/platform/static/js/marketplace-websocket.js
// WebSocket-Client mit Auto-Reconnect, Heartbeat und Event-Bus-Integration

class MarketplaceWebSocket {
    constructor(assetId) {
        this.assetId = assetId;
        this.ws = null;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 10;
        this.reconnectDelay = 1000;    // Start: 1s, exponential backoff
        this.heartbeatInterval = null;
        this.isIntentionallyClosed = false;
    }

    connect() {
        const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
        const url = `${protocol}//${location.host}/ws/market/${this.assetId}`;

        console.log(`[WS] Attempting connection to ${url}`);
        
        try {
            this.ws = new WebSocket(url);
        } catch (e) {
            console.warn("[WS] Failed to construct WebSocket, running offline: ", e);
            this._updateStatusIndicator('failed');
            return;
        }

        this.ws.onopen = () => {
            console.log(`[WS] Connected to ${this.assetId}`);
            this.reconnectAttempts = 0;
            this.reconnectDelay = 1000;
            this.startHeartbeat();
            window.marketBus.emit('ws:connected', { url });

            this._updateStatusIndicator('connected');
        };

        this.ws.onmessage = (event) => {
            try {
                const msg = JSON.parse(event.data);
                this._handleMessage(msg);
            } catch (e) {
                console.warn('[WS] Invalid message:', event.data);
            }
        };

        this.ws.onclose = (event) => {
            console.log(`[WS] Disconnected: ${event.code} ${event.reason}`);
            this.stopHeartbeat();
            this._updateStatusIndicator('disconnected');
            window.marketBus.emit('ws:disconnected', { 
                code: event.code, reason: event.reason 
            });

            if (!this.isIntentionallyClosed) {
                this._reconnect();
            }
        };

        this.ws.onerror = (error) => {
            console.error('[WS] Error:', error);
        };
    }

    _handleMessage(msg) {
        switch (msg.type) {
            case 'trade':
                window.marketBus.emit('trade:executed', msg);
                break;
            case 'orderbook':
                window.marketBus.emit('orderbook:updated', msg);
                break;
            case 'ticker':
                window.marketBus.emit('ticker:updated', msg);
                break;
            case 'pong':
                break;
            case 'p2p_offer':
                window.marketBus.emit('p2p:offer_received', msg);
                break;
            default:
                console.warn('[WS] Unknown message type:', msg.type);
        }
    }

    _reconnect() {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.error('[WS] Max reconnect attempts reached');
            this._updateStatusIndicator('failed');
            this._showReconnectButton();
            return;
        }

        this.reconnectAttempts++;
        const delay = Math.min(this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1), 30000);
        console.log(`[WS] Reconnecting in ${delay}ms`);
        
        this._updateStatusIndicator('reconnecting');
        setTimeout(() => this.connect(), delay);
    }

    startHeartbeat() {
        this.heartbeatInterval = setInterval(() => {
            if (this.ws && this.ws.readyState === 1) {
                this.ws.send(JSON.stringify({ type: 'ping' }));
            }
        }, 25000);
    }

    stopHeartbeat() {
        if (this.heartbeatInterval) {
            clearInterval(this.heartbeatInterval);
            this.heartbeatInterval = null;
        }
    }

    disconnect() {
        this.isIntentionallyClosed = true;
        this.stopHeartbeat();
        if (this.ws) {
            this.ws.close(1000, 'User navigated away');
        }
    }

    _updateStatusIndicator(status) {
        const indicator = document.getElementById('ws-status');
        if (!indicator) return;

        const states = {
            connected:    { text: 'Live', class: 'status--live', icon: '🟢' },
            disconnected: { text: 'Offline', class: 'status--offline', icon: '🔴' },
            reconnecting: { text: 'Verbinde...', class: 'status--reconnecting', icon: '🟡' },
            failed:       { text: 'Fehler', class: 'status--failed', icon: '⚫' },
        };

        const s = states[status] || states.disconnected;
        indicator.textContent = `${s.icon} ${s.text}`;
        indicator.className = `ws-status ${s.class}`;
    }

    _showReconnectButton() {
        const container = document.getElementById('ws-reconnect-container');
        if (!container) return;
        container.innerHTML = `
            <button onclick="if(window.marketWs) { window.marketWs.reconnectAttempts=0; window.marketWs.connect(); }" 
                    class="btn btn--small btn--outline">
                🔄
            </button>`;
        container.style.display = 'block';
    }
}
