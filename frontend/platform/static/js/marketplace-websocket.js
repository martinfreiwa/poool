/**
 * marketplace-websocket.js
 * 
 * WebSocket client for real-time marketplace data.
 * Features:
 *   - Auto-reconnect with exponential backoff (1s → 2s → 4s → 8s → 16s → 30s max)
 *   - Heartbeat monitoring (expects server pings every 30s)
 *   - Event bus integration (emits to MarketBus)
 *   - Visibility API: disconnects when tab hidden, reconnects when visible
 *   - Connection state tracking
 *
 * Usage:
 *   MarketWS.connect('asset-uuid-here');
 *   MarketWS.disconnect();
 *   MarketWS.getState(); // 'connecting' | 'open' | 'closed' | 'reconnecting'
 */

const MarketWS = (function () {
    'use strict';

    let _ws = null;
    let _assetId = null;
    let _state = 'closed'; // 'connecting' | 'open' | 'closed' | 'reconnecting'
    let _reconnectAttempts = 0;
    let _reconnectTimer = null;
    let _heartbeatTimer = null;
    let _intentionalClose = false;

    const MAX_RECONNECT_DELAY = 30000; // 30s max
    const HEARTBEAT_TIMEOUT = 45000;   // 45s (server pings every 30s, 15s grace)
    const BASE_RECONNECT_DELAY = 1000; // 1s initial

    /**
     * Get the WebSocket URL for an asset.
     */
    function _wsUrl(assetId) {
        const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        return `${proto}//${window.location.host}/ws/market/${assetId}`;
    }

    /**
     * Calculate reconnect delay with exponential backoff + jitter.
     */
    function _reconnectDelay() {
        const delay = Math.min(
            BASE_RECONNECT_DELAY * Math.pow(2, _reconnectAttempts),
            MAX_RECONNECT_DELAY
        );
        // Add ±20% jitter to prevent thundering herd
        const jitter = delay * 0.2 * (Math.random() * 2 - 1);
        return Math.round(delay + jitter);
    }

    /**
     * Reset the heartbeat timer. Called when any message is received.
     */
    function _resetHeartbeat() {
        clearTimeout(_heartbeatTimer);
        _heartbeatTimer = setTimeout(() => {
            console.warn('[MarketWS] Heartbeat timeout — reconnecting');
            _reconnect();
        }, HEARTBEAT_TIMEOUT);
    }

    /**
     * Handle incoming WebSocket message.
     */
    function _onMessage(event) {
        _resetHeartbeat();

        // Skip ping/pong frames (handled by browser)
        if (!event.data || typeof event.data !== 'string') return;

        try {
            const msg = JSON.parse(event.data);
            const eventType = msg.event;

            if (!eventType) {
                console.warn('[MarketWS] Message without event type:', msg);
                return;
            }

            // Route to MarketBus based on event type
            switch (eventType) {
                case 'orderbook_update':
                    window.MarketBus?.emit('orderbook:update', msg);
                    break;
                case 'trade':
                    window.MarketBus?.emit('trade', msg);
                    break;
                case 'ticker':
                    window.MarketBus?.emit('ticker', msg);
                    break;
                default:
                    console.debug('[MarketWS] Unknown event:', eventType, msg);
            }
        } catch (err) {
            console.error('[MarketWS] Failed to parse message:', err, event.data);
        }
    }

    /**
     * Attempt to reconnect.
     */
    function _reconnect() {
        if (_intentionalClose || !_assetId) return;

        _setState('reconnecting');
        _cleanup();

        const delay = _reconnectDelay();
        _reconnectAttempts++;

        console.log(`[MarketWS] Reconnecting in ${delay}ms (attempt ${_reconnectAttempts})`);

        _reconnectTimer = setTimeout(() => {
            _doConnect(_assetId);
        }, delay);
    }

    /**
     * Clean up WebSocket and timers.
     */
    function _cleanup() {
        clearTimeout(_heartbeatTimer);
        clearTimeout(_reconnectTimer);

        if (_ws) {
            _ws.onopen = null;
            _ws.onmessage = null;
            _ws.onclose = null;
            _ws.onerror = null;

            if (_ws.readyState === WebSocket.OPEN || _ws.readyState === WebSocket.CONNECTING) {
                _ws.close(1000, 'cleanup');
            }
            _ws = null;
        }
    }

    /**
     * Set state and emit to event bus.
     */
    function _setState(newState) {
        _state = newState;
        window.MarketBus?.emit('ws:state', { state: newState, assetId: _assetId });
    }

    /**
     * Core connection logic.
     */
    function _doConnect(assetId) {
        _cleanup();
        _setState('connecting');

        const url = _wsUrl(assetId);
        console.log(`[MarketWS] Connecting to ${url}`);

        try {
            _ws = new WebSocket(url);
        } catch (err) {
            console.error('[MarketWS] WebSocket constructor failed:', err);
            _reconnect();
            return;
        }

        _ws.onopen = () => {
            console.log('[MarketWS] Connected');
            _setState('open');
            _reconnectAttempts = 0;
            _resetHeartbeat();
        };

        _ws.onmessage = _onMessage;

        _ws.onclose = (event) => {
            console.log(`[MarketWS] Closed: code=${event.code} reason=${event.reason}`);
            _setState('closed');

            if (!_intentionalClose) {
                _reconnect();
            }
        };

        _ws.onerror = (event) => {
            console.error('[MarketWS] Error:', event);
            // onclose will be called after onerror — reconnect happens there
        };
    }

    // ── Visibility API: pause when tab is hidden ──
    document.addEventListener('visibilitychange', () => {
        if (document.hidden) {
            // Tab hidden — disconnect to save resources
            if (_state === 'open' || _state === 'connecting') {
                console.log('[MarketWS] Tab hidden — disconnecting');
                _intentionalClose = true;
                _cleanup();
                _setState('closed');
                _intentionalClose = false; // Reset so we reconnect on visible
            }
        } else {
            // Tab visible — reconnect if we have an asset
            if (_assetId && _state !== 'open' && _state !== 'connecting') {
                console.log('[MarketWS] Tab visible — reconnecting');
                _reconnectAttempts = 0;
                _doConnect(_assetId);
            }
        }
    });

    // ── Public API ──
    return {
        /**
         * Connect to the WebSocket server for a specific asset.
         * @param {string} assetId - UUID of the asset
         */
        connect(assetId) {
            if (!assetId) {
                console.error('[MarketWS] assetId is required');
                return;
            }

            _intentionalClose = false;
            _assetId = assetId;
            _reconnectAttempts = 0;
            _doConnect(assetId);
        },

        /**
         * Intentionally disconnect. Will NOT auto-reconnect.
         */
        disconnect() {
            _intentionalClose = true;
            _assetId = null;
            _cleanup();
            _setState('closed');
        },

        /**
         * Get current connection state.
         * @returns {'connecting'|'open'|'closed'|'reconnecting'}
         */
        getState() {
            return _state;
        },

        /**
         * Check if connected.
         * @returns {boolean}
         */
        isConnected() {
            return _state === 'open';
        },
    };
})();

window.MarketWS = MarketWS;
