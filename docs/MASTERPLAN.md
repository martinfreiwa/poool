# 🏛 POOOL Plattform – 10/10 Experten-Masterplan (Architektur & Implementierung)

Dieses Dokument dient als das zentrale "Single Source of Truth" (SSOT) für die Entwicklung, Skalierung und den Betrieb der POOOL-Plattform. Es definiert die gesamte technische und konzeptionelle Architektur für eine hochsichere, regulierte Finanzplattform (Fractional Investment in Real World Assets - RWA).

---

## 1. Executive Summary & Produktvision
POOOL demokratisiert den Zugang zu illiquiden Vermögenswerten (Immobilien, Edelmetalle, Kunst) durch Tokenisierung. Nutzer können kleine Anteile (Fractions) erwerben, halten und handeln. 
Da POOOL im stark regulierten Finanzsektor (FinTech) agiert, haben **Sicherheit (Security), Compliance (Regulatorik) und Transaktionsintegrität (ACID)** die absolut höchste Priorität. Ein einziger Fehler bei Transaktionen ist inakzeptabel.

---

## 2. Systemarchitektur & Tech-Stack

Um maximale Performance, Sicherheit und Wartbarkeit zu garantieren, setzen wir auf modernste Technologien.

### 2.1 Frontend (User Facing)
- **Marketing-Webseite (`/frontend/www/`):** Angular + Tailwind CSS. Auf blitzschnelles Laden und SEO optimiert.
- **Investor & Admin Dashboard (`/frontend/platform/`):** 
  - **Paradigma:** Server-Side Rendering (SSR) kombiniert mit **HTMX** und **Alpine.js**.
  - **Vorteil:** Keine komplexe Single-Page-Application-Logik, extreme Geschwindigkeit, maximale Sicherheit (keine sensible Business-Logik und keine empfindlichen API-Schlüssel im Client).

### 2.2 Backend (Core API & Business Logic)
- **Sprache & Framework:** **Rust** mit **Axum**.
- **Vorteile:** Rust bietet Memory-Safety (Vermeidung von Speicherlecks und Buffer Overflows zur Compile-Zeit) – essenziell für Finanzanwendungen. Axum garantiert asynchrone Hochleistung.
- **Templating:** **MiniJinja** (für das Rendern der sicheren HTML/HTMX-Views im Backend).

### 2.3 Datenhaltung & Infrastruktur
- **Relationale Datenbank:** **PostgreSQL 16+**. Primäre Quelle für alle Daten. Beherrschung komplexer Finanztransaktionen durch ACID-Garantie.
- **In-Memory Cache:** **Redis** (für Session-Speicherung, OTP-Codes, Rate-Limiting).
- **Deployment:** Containerisiert (Docker), gehostet in einer sicheren Cloud-Umgebung (z.B. AWS ECS, Google Cloud Run) mit strengen Virtual Private Cloud (VPC)-Regeln (Datenbank ist nie von außen erreichbar).

---

## 3. Kern-Module & User Journeys

### 3.1 Registrierung, Auth & KYC (Know Your Customer)
*Ein Nutzer darf erst investieren, wenn er vom System und regulatorisch geprüft wurde.*
1. **Sign-up:** Email + starkes Passwort + Double Opt-In.
2. **Sicherheit:** Erzwingung von **2FA** (Authenticator App / TOTP) für Anmeldungen und kritische Aktionen (z. B. Geld abheben).
3. **KYC-Prozess:** Tiefe API-Integration eines Tier-1 Anbieters (z. B. *SumSub*).
   - Liveness Check, Ausweisscan, Adressnachweis.
   - Automatisches Screening auf PEP- (Politically Exposed Persons) und Sanktionslisten.

### 3.2 Ein- und Auszahlungen (Fiat & Krypto)
*Geldströme müssen strikt getrennt und rechtssicher (BaFin-konform) gemanagt werden.*
- **Wallet-System:** Jeder Nutzer (und jedes Asset) erhält ein virtuelles Ledger-Konto.
- **Zahlungsanbieter (PSP):** Integration eines lizenzierten Payment-Service-Providers (z. B. *Mangopay* oder *Stripe Connect*), der E-Geld-Lizenzen und Escrow-Wallets (Treuhand) anbietet. POOOL darf Kundengelder rein rechtlich nicht direkt auf einem Firmenkonto halten.

### 3.3 Asset Marktplatz & Investment Engine
- **Asset Creation (Admin):** Anlage einer Immobilie mit zugehöriger SPV-Struktur (z.B. GmbH/UG), Gutachten, Finanzkennzahlen und Token-Metadaten (z. B. 10.000 Tokens à 100€).
- **Investment-Logik:** Nutzer klickt auf "Investieren". In einer *einzigen, atomaren Datenbanktransaktion* wird das Fiat-Wallet des Nutzers belastet und ihm der Ownership-Anteil (Tokens/Fractions) zugeschrieben. Schlägt ein Schritt fehl, wird alles zurückgerollt (Rollback).
- **Sekundärmarkt (Später):** Nutzer können untereinander Anteile handeln. Hierfür wird eine Matching-Engine benötigt.

---

## 4. Unabdingbare Sicherheits- und Compliance-Maßnahmen

Als FinTech-Plattform müssen wir uns so absichern, als ob wir morgen von Banken-Prüfern auditiert werden.

### 4.1 IT-Security
- **Authentifizierung:** Session-Handling via `HttpOnly`, `Secure`, `SameSite=Strict` Cookies. Wir speichern **niemals** JWTs oder sensible Tokens im unsicheren `localStorage` des Browsers.
- **Kryptographie:** Passwörter zwingend mit `Argon2id` hashen (Brute-Force sicher).
- **WAF & DDoS:** Einsatz einer Web Application Firewall (z.B. Cloudflare) zum Filtern von bösartigem Traffic, SQL-Injection, Cross-Site Scripting (XSS) und Bot-Angriffen.
- **Verschlüsselung:** "Encryption at Rest" (Festplatten-Verschlüsselung der Datenbank) und TLS 1.3 für alle Netzwerkverbindungen.

### 4.2 Compliance & Revisionssicherheit
- **Immutable Audit-Log:** *Jede* signifikante Veränderung an Daten (Kontostand, Berechtigungen, KYC-Status) wird irreversibel in einer Audit-Tabelle protokolliert (Wer, Wann, Was, Alte Daten, Neue Daten, IP-Adresse).
- **DSGVO / GDPR:** Architektur unterstützt das "Recht auf Löschen", verwendet aber bei Finanztransaktionen "Soft-Deletes" kombiniert mit Pseudonymisierung, um der 10-jährigen gesetzlichen Aufbewahrungspflicht für Finanzdaten nachzukommen.

---

## 5. Externe Integrationen & APIs

| Kategorie | Best-in-Class Empfehlung | Zweck |
| :--- | :--- | :--- |
| **KYC / AML** | **SumSub** oder IDnow | Echtzeit Identitätsprüfung & Sanktionslisten-Checks |
| **Payment (Fiat)** | **Mangopay** | E-Geld Lizenzen, IBANs für Nutzer, Treuhand (Escrow) |
| **E-Mail & SMS** | **Postmark** & Twilio | Verlässliche Zustellung von OTP-Codes und Bestätigungen |
| **Monitoring** | **Sentry** | Echtzeit-Erfassung von Backend-Abstürzen / Bugs |
| **Vertrags-Sig.** | **DocuSign** API | Rechtsgültige digitale Signatur für Investment-Verträge |

---

## 6. Datenbank-Design (Das relationale Fundament)

Die Datenbankstruktur muss fehlerfrei sein. Hier ein extrem durchdachter Grobentwurf der Tabellenstruktur auf Enterprise-Level:

1. `users` (id, email, password_hash, role [INVESTOR, DEVELOPER, ADMIN], 2fa_secret, status) -> *Unterstützung spezifischer Rollen.*
2. `user_profiles` (user_id, first_name, last_name, date_of_birth, address_line, tax_id)
3. `kyc_records` (id, user_id, provider, status, provider_reference, verified_at)
4. `wallets` (id, user_id, currency [EUR/USD], balance_cents) -> *Warum Cents? Weil Float-Zahlen in Finanzsystemen streng verboten sind.*
5. `assets` (id, title, description, asset_type, total_value_cents, tokens_total, tokens_available, token_price_cents, funding_status)
6. `asset_documents` (id, asset_id, document_type [expose, legal, financial], file_url)
7. `investments` (id, user_id, asset_id, tokens_owned, average_buy_price_cents)
8. `transactions` (id, wallet_id, type [DEPOSIT, WITHDRAWAL, BUY, SELL, DIVIDEND], amount_cents, status [PENDING, COMPLETED, FAILED], external_reference_id)
9. `audit_logs` (id, actor_user_id, action, table_name, record_id, previous_state, new_state, ip_address, created_at)
10. `idempotency_keys` (id, idempotency_key, user_id, request_path, response_status, response_body, created_at) -> *Verhindert doppelte Ausführungen (z.B. bei Zahlungen) durch Verbindungsabbrüche.*
11. `background_jobs` (id, queue_name, payload_json, status [PENDING, PROCESSING, COMPLETED, FAILED], attempts, run_at, created_at) -> *Für asynchrone Aufgaben (E-Mails, Vertrags-Generierung).*

---

## 7. Der Umsetzungs-Fahrplan (Roadmap / Next Steps)

Um dieses riesige Projekt erfolgreich zu stemmen, wird es agil in extrem fokussierten Phasen entwickelt:

### Phase 1: Die Festung (Fundament & Auth)
- Aufsetzen der Rust/Axum-Struktur inkl. Konfigurations-Management.
- Implementierung der PostgreSQL-Datenbank mit initialen Migrationen (Users, Wallets, Audit-Logs).
- Sicheres Login/Registrierung, E-Mail-Verifikation und Session-Management via HTMX.

### Phase 2: Identität & Nutzer (KYC & Profile)
- Evaluierung und Anbindung der REST-API von SumSub oder Mangopay für KYC.
- Ausarbeitung des User-Dashboards im Frontend (`/frontend/platform/profile`).

### Phase 3: Der Tresor (Asset-Verwaltung & Zahlungsverkehr)
- Anlage von Assets im System.
- Anzeige der Assets im Marktplatz für die Nutzer.
- Integration des Payment-Providers (Einzahlungen per Banküberweisung anzeigen und Buchen).

### Phase 4: Die Maschine (Investment Engine)
- Die Kernlogik: Nutzer kaufen Anteile an freigegebenen Assets.
- Transaktionsbuchungen, Generierung von Kaufverträgen.

### Phase 5: Go-Live & Audit
- Admin-Dashboard zur Kontrolle und Freigabe.
- Penetration Testing, Source Code Audit.
- Live-Deployment auf skalierbare Cloud-Server.

---
*Ende des Dokuments.*
