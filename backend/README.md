# POOOL Platform Backend

Dies ist das Backend der POOOL-Plattform. Es ist in **Rust** geschrieben, nutzt **Axum** als Web-Framework und verwendet **PostgreSQL** für die Datenbank via **SQLx**. Als Templating-Engine für die HTMX-gesteuerte Frontend-Bereitstellung kommt **MiniJinja** zum Einsatz.

## 🚀 Quick Start

1. **Voraussetzungen installieren:**
   - [Rust & Cargo](https://rustup.rs/) (aktuelle stabile Version)
   - [PostgreSQL](https://www.postgresql.org/download/) 16 oder höher

2. **Datenbank starten & initialisieren:**
   Stelle sicher, dass PostgreSQL läuft und importiere das initiale Schema:
   ```bash
   # Beispiel (macOS mit Homebrew):
   brew services start postgresql@16
   
   # Datenbank erstellen und Schema importieren:
   createdb poool
   psql -d poool -f ../database/001_initial_schema.sql
   ```

3. **Umgebungsvariablen konfigurieren:**
   Kopiere die Beispiel-Konfiguration und passe sie ggf. an:
   ```bash
   cp .env.example .env
   ```

4. **Server starten:**
   Startet den Axum-Server unter `http://localhost:8888`.
   ```bash
   cargo run
   ```

## 📂 Code-Architektur

- `src/main.rs`: Einsprungspunkt, Axum Router und Middleware-Setup.
- `src/db.rs` / `src/config.rs`: Datenbankverbindung und Env-Config.
- `src/error.rs`: Zentrales AppError-Handling (Error zu HTML fürs HTMX-Frontend).
- `src/auth/`: Modul für Login, Sign-up, Sessions & OAuth2. Strikt getrennt in `routes`, `models` und `service`.

### Wichtige Dokumentation
Das Projekt ist extrem detailliert dokumentiert. Bitte lies:
- `../docs/MASTERPLAN.md` - Die Architektur der Plattform
- `../docs/DATABASE_SCHEMA.md` - Das gesamte SQL-Schema, Feld für Feld
- `../docs/KNOWLEDGE_HANDOFF.md` - Aktueller Projektstand und Setup-Kontext
