/**
 * marketplace-event-bus.js
 * 
 * Lightweight event bus for marketplace components.
 * Uses EventTarget for native performance — no external dependencies.
 * 
 * Usage:
 *   import { MarketBus } from './marketplace-event-bus.js';
 *   MarketBus.on('orderbook:update', (data) => { ... });
 *   MarketBus.emit('orderbook:update', { bids, asks });
 *   MarketBus.once('trade', (data) => { ... });
 *   MarketBus.off('orderbook:update', handler);
 */

const _target = new EventTarget();
const _handlers = new WeakMap();

/**
 * Get or create a native event handler wrapper for a callback.
 * This allows us to remove the listener later with `off()`.
 */
function _wrap(callback) {
    if (!_handlers.has(callback)) {
        _handlers.set(callback, (e) => callback(e.detail));
    }
    return _handlers.get(callback);
}

const MarketBus = {
    /**
     * Subscribe to an event.
     * @param {string} event - Event name (e.g. 'orderbook:update', 'trade', 'ticker')
     * @param {Function} callback - Handler function receiving event data
     */
    on(event, callback) {
        _target.addEventListener(event, _wrap(callback));
    },

    /**
     * Subscribe to an event, but only fire once.
     * @param {string} event
     * @param {Function} callback
     */
    once(event, callback) {
        const wrapped = (e) => callback(e.detail);
        _target.addEventListener(event, wrapped, { once: true });
    },

    /**
     * Unsubscribe from an event.
     * @param {string} event
     * @param {Function} callback - Must be the same reference passed to `on()`
     */
    off(event, callback) {
        const wrapped = _handlers.get(callback);
        if (wrapped) {
            _target.removeEventListener(event, wrapped);
            _handlers.delete(callback);
        }
    },

    /**
     * Emit an event with data.
     * @param {string} event
     * @param {*} data - Any serializable data
     */
    emit(event, data) {
        _target.dispatchEvent(new CustomEvent(event, { detail: data }));
    },
};

// Freeze to prevent accidental mutation
Object.freeze(MarketBus);

// Export for use by other modules
window.MarketBus = MarketBus;
