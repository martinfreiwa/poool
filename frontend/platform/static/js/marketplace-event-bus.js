// frontend/platform/static/js/marketplace-event-bus.js
// Leichtgewichtiger Event-Bus für Cross-Component State-Sync
// Ersetzt React State / Vue Reactive ohne Framework-Overhead

class MarketplaceEventBus extends EventTarget {
    /**
     * Event abonnieren
     * @param {string} eventName - z.B. 'trade:executed', 'orderbook:updated'
     * @param {Function} callback - Handler-Funktion
     */
    on(eventName, callback) {
        this.addEventListener(eventName, (e) => callback(e.detail));
    }

    /**
     * Event einmalig abonnieren (auto-unsubscribe nach erstem Aufruf)
     */
    once(eventName, callback) {
        this.addEventListener(eventName, (e) => callback(e.detail), { once: true });
    }

    /**
     * Event auslösen
     * @param {string} eventName - z.B. 'trade:executed'
     * @param {Object} data - Payload
     */
    emit(eventName, data) {
        this.dispatchEvent(new CustomEvent(eventName, { detail: data }));
    }

    /**
     * Event-Listener entfernen
     */
    off(eventName, callback) {
        this.removeEventListener(eventName, callback);
    }
}

// Singleton: Eine Instanz für die gesamte Seite
if (!window.marketBus) {
    window.marketBus = new MarketplaceEventBus();
}
