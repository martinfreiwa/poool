# POOOL Projekt - Knowledge & Context Handoff

Dieses Dokument fasst den gesamten bisherigen Fortschritt und alle Architekturentscheidungen für das Antigravity / KI-Gedächtnis zusammen, um nahtlos im neuen Workspace am Projekt weiterzuarbeiten.

## 1. Ausgangssituation & Kontext
- Das Projekt ist die "POOOL Plattform" (Finanz-/Tokenisierungs-Plattform für Immobilien/Rohstoffe: Fractional investment in RWA).
- Die alte Agentur kann nicht liefern; der Bruder des Nutzers benötigt nun Hilfe, die begonnene Plattform komplett funktionsfähig zu Ende zu bauen.
- Das Frontend lag ausschließlich in kompilierter Form vor.
- **Erfolg:** Wir haben alle Frontend-Assets (HTML, CSS, JS, Bilder) von der Live-Seite erfolgreich heruntergeladen, offline lauffähig gemacht und bereinigt (inklusive Developer Dashboard und Investor Marketplace).

## 2. Projektstruktur
Das Projekt wurde sauber aus dem "Downloads"-Ordner entfernt und strukturiert nach:
`/Users/martin/Projects/poool/`
- `/frontend/platform/`: Das eigentliche Dashboard (HTMX, Custom CSS BEM, Alpine.js). Kein Framework wie React/Angular. Alles 100% Custom-Designed.
- `/frontend/www/`: Die statische (Angular/Tailwind) Marketing-Landingpage.
- `/backend/`: Unser neu initialisierter Rust Webserver (Axum).
- `/database/`: Zukünftige Schema-Dateien.
- `/docs/`: Dokumentationen und Entwicklungspläne.
- `/scripts/`: Python/Node/Shell Downloader- und Korrektur-Scripts aus der ersten Phase.

## 3. Technologie-Entscheidungen (Backend)
- **Programmiersprache:** Rust (Wunsch der originalen Entwickler und des Nutzers).
- **Web-Framework:** Axum.
- **Datenbank:** PostgreSQL (Wegen Finanzdaten-ACID-Konformität).
- **DB-Treiber:** SQLx.
- **Template Engine:** MiniJinja (um die statischen HTML-HTMX-Dateien aus `/frontend/platform/` mit Backend-Variablen anzureichern).
- **Architektur:** HTMX steuert das Frontend. Wir bauen Serverseitiges Rendering (SSR), das kleine HTML-Fragmente über Axum via HTTP an HTMX zurückliefert. Kein dicke JSON-API nötig.

## 4. Wie geht es jetzt weiter? (Aktueller Stand)
**Phase 1: Fundament & Datenbankaufbau** ist ✅ **abgeschlossen**.
- Das PostgreSQL Datenbankschema (`001_initial_schema.sql`) ist komplett entworfen, angelegt und einsatzbereit.
- Rust Backend (Axum) steht mit SQLx-Anbindung.
- **Sicheres Login, Registrierung & Session-Management** (inklusive HTMX Redirects und Argon2id-Password-Hashing) sind lauffähig.
- Dynamische Nutzerdaten werden bereits über `/api/me` bereitgestellt.

Wir befinden uns nun in **Phase 2 & 3: Assets, Identität & Kern-Funktionen**.
- Weitere Features wie KYC-Integration, Transaktions- und Investment-Logic sowie die vollständige Dynamisierung des Marktplatzes und Entwickler-Dashboards können nahtlos angeschlossen werden.
- Der Masterplan (in `/docs/MASTERPLAN.md`) dient als Referenz für die weitere Umsetzung.

> [!NOTE]
> KI-Regel für den Start im Workspace: Lies dieses Dokument kurz ein, greife nicht mehr auf `/Downloads` zu, sondern nutze ausschließlich `/Users/martin/Projects/poool/` als Basis. Schlage dem Nutzer als nächstes die Bearbeitung des nächsten Schritts aus der Roadmap vor (z. B. Dashboard-Komponenten anbinden oder Assets im Marktplatz laden).
