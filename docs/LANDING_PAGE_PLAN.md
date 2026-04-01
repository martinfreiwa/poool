# 🚀 Master-Plan: Public Marketplace & Guest-Cart Funnel (Expert Edition)

## Inhaltsverzeichnis
1. [Strategie & UX-Ziele](#1-strategie--ux-ziele)
2. [Expert Review 1: Frontend Developer (Die HTMX-Entscheidung)](#2-expert-review-1-frontend)
3. [Expert Review 2: Backend Developer (Rust/Axum & Server-Side Cart)](#3-expert-review-2-backend)
4. [Expert Review 3: Full-Stack Integrator (Der finale Flow)](#4-expert-review-3-integrator)
5. [Seitenarchitektur & Anpassungen](#5-seitenarchitektur--anpassungen)
6. [Implementierungs-Phasen](#6-arbeitsablauf--implementierungs-phasen)

---

## 1. Strategie & UX-Ziele
Inspiriert von Airbnb und mobile.de wird die Startseite zum **interaktiven Marktplatz**.
- **Zero-Click-Discovery:** Assets direkt auf der Startseite sichtbar.
- **Endowment-Effekt:** Reibungsloses Hinzufügen zum Warenkorb für Nicht-Eingeloggte (Guest Cart).
- **Soft vs. Hard Login Wall:** Login wird nur für tiefe Finanzdownloads (Exposés) oder beim finalen Klick auf "Zur Kasse" (Checkout) erzwungen.

---

## 2. Expert Review 1: Frontend Developer (Die HTMX-Entscheidung)
*"Macht Vanilla JS hier Sinn oder gibt es was Besseres für Rust/Axum?"*

**Fazit des FE-Experten:** Wir verwerfen reines Vanilla JS für die Warenkorb-Logik. Wie im Tech-Stack vorgegeben, ist **HTMX in Kombination mit MiniJinja** die mächtigste und schnellste Lösung für ein Rust-Backend.
- **Warum HTMX?** Anstatt JSON an Vanilla JS zu senden (was aufwendig ins DOM geparst werden muss), sendet Axum direkt fertiges HTML zurück. 
- **Live-Filtering (Marktplatz):** Wenn der Nutzer auf den Tab "Commodities" klickt, macht HTMX im Hintergrund einen Request (`hx-get="/api/assets?category=commodity"`). Axum rendert das Grid neu und HTMX tauscht das alte Grid butterweich (`hx-swap`) aus. Kein JS nötig.
- **Cart Updates:** Ein Klick auf "Add to Cart" feuert einen HTMX-Post-Request (`hx-post="/cart/add/123"`). Der Server schickt als Antwort einfach den aktualisierten Header-Warenkorb-Zähler als HTML zurück. Superschnell, winzige Payload.

---

## 3. Expert Review 2: Backend Developer (Rust/Axum & Server-Side Cart)
*"Wie verhindern wir die Sicherheitslücken von LocalStorage?"*

**Fazit des BE-Experten:** Die ursprünglich geplante `localStorage`-Lösung verwerfen wir. Das ist zu unsicher und fehleranfällig. Wir nutzen stattdessen die volle Power unserer server-seitigen Session (HTTP-only `poool_session`).
- **Anonymous Sessions:** Wenn ein Nutzer auf die Seite kommt (ohne Login), kreiert Axum im Hintergrund automatisch eine Gast-Session und setzt das Cookie.
- **Server-Side Guest Cart:** Wenn der Gast auf "Add to Cart" klickt, wird das Item sicher in unserer Datenbank/Redis gespeichert – verknüpft mit seiner *anonymen Session-ID*. Preise können nicht manipuliert werden, da alles im Rust-Backend abgefragt wird (DB Schema: `assets`).
- **Der elegante Cart-Merge:** Wenn der Nutzer zum Checkout geht und sich einloggt, überschreiben wir nicht die Session. Wir hängen einfach seine neu authentifizierte `user_id` an die bestehende Session. *Boom.* Der Gast-Warenkorb wird automatisch zu seinem echten Warenkorb. Es gibt keinen komplexen "Sync-Endpunkt" mehr. Das ist die sicherste und stabilste Architektur für E-Commerce.

---

## 4. Expert Review 3: Full-Stack Integrator (Der finale Flow)
*"Was bedeutet das für die existierenden Seiten?"*

**Fazit des Integrators:** Durch die Entscheidung des Backend-Experten (Anonymous Sessions) wird meine Arbeit extrem erleichtert.
- **Kein "Fake-Warenkorb" mehr:** Der Warenkorb auf `public-cart.html` nutzt exakt denselben Jinja-Code wie der eingeloggte Warenkorb. Wir blenden nur die Sidebar mithilfe eines Jinja-If-Statements (`{% if user_id %}`) aus.
- **Nahtloser Checkout:** Wenn der Nutzer auf `login.html` geht, schleifen wir `?redirect=/checkout` durch. Nach dem Einloggen landet er auf der bestehenden `checkout.html` (nun mit Sidebar). Da sein Warenkorb server-seitig via Session existiert, sind alle seine Items sofort da. 
- **Testing:** Ich werde E2E-Tests in Python (Pytest) schreiben, die exakt prüfen: Website aufrufen (Gast) -> Hinzufügen -> Cart aufrufen -> Login -> Checkout-Button. 

---

---

## 5. Detaillierte Seitenarchitektur & Visual Design (Der Bauplan)

Das gesamte Design bedient sich am `docs/DESIGN.md` (POOOL Platform Design System) und orientiert sich strikt an den bestehenden Farben und der Schriftart (TT Norms Pro). **Es gibt auf diesen Seiten keine Sidebar!**

### 5.1 Landing Page (`index.html`) – Das Schaufenster
Die Landing Page vereint starke Markenpräsenz (Brand) mit sofortiger Produktverfügbarkeit (Marketplace).
- **Header (Top Navigation):** Sticky. Wird von transparent (über dem Hero) zu deckend beim Scrollen. Log-in / Sign-up Buttons rechtsseitig platziert.
- **Sektion 1: Hero-Bereich (Der Hook):**
  - *Visuell:* Übernimmt das starke, animierte Design der aktuellen Landing Page. Background-Color oder Video, kräftige Typografie.
  - *Call-to-Action:* "Get Started" sowie die **neue Quick-Search Bar**! Direkt zentral positioniert. Die Suchleiste erlaubt Dropdowns wie "Immobilien", "Rohstoffe" oder "Gewünschte Rendite".
- **Sektion 2: Der Marktplatz (Airbnb/mobile.de Look):**
  - *Struktur:* 2 prominente Toggle-Tabs ("Property Assets", "Commodity Assets"). Dahinter ein responistves Raster (3–4 Spalten auf Desktop).
  - *Karten-Design (Asset Cards):* Flaches, hochmodernes Design ohne veraltete holographische Kanten! Weißer Hintergrund (`#ffffff`), sehr sanfter 1px-Border (`var(--border-color)`). Oben ein hochwertiges 16:9 Thumbnail, darunter Titel, Preis und "Erwartete Rendite p.a.". 
  - *Hover-State:* Zarter Schlagschatten und ein subtiler Zoom im Thumbnail-Bild, sobald die Maus darüber schwebt (Dynamic Design).
- **Sektion 3: How it Works & Trust:**
  - 3-Step-Layout (Registrieren -> Portfolio bauen -> Profitieren) und Social Proof (z.B. Investitionsvolumen).
- **Sektion 4: Footer:** Standard Footer mit allen rechtlichen Links.

### 5.2 Public Asset Detail Page (`public-asset.html`) – Die Entscheidung
Hier wird das Asset im Detail begutachtet.
- **Layout (Split-Screen auf Desktop):**
  - *Linke Spalte (Content):* Große Bilder-Galerie, Reiter für "Projektübersicht", "Standort (Map)" und "Timeline".
  - *Rechte Spalte (Sticky Card):* Wandert beim Scrollen mit. Enthält den Preis, Rendite, Funding-Fortschritt (Progress Bar) und den primären "Add to Cart"-Button.
- **Die Soft Wall (Download-Wall):** 
  - In der linken Spalte gibt es "Finanzielle Dokumente" (Exposé, Whitepaper, Business Plan).
  - *UX-Logik:* Klickt der Gast auf Download, öffnet sich per HTMX ein kleines Modal (`#login-modal`): *"Bitte logge dich ein oder erstelle in 30 Sekunden einen Account, um Zugriff auf interne Finanzdokumente zu erhalten."*

### 5.3 Public Cart (`public-cart.html`) – Das Commitment
Das Gegenstück zum Dashboard-Checkout, aber frei zugänglich.
- **Header:** Reduziert. Ohne tiefe Navigation, um Ablenkung zu vermeiden (Fokus auf Conversion).
- **Layout:**
  - *Links (Items):* Die via HTMX aus der serverseitigen Anonymous Session geladenen Asset-Karten. Clean & übersichtlich.
  - *Rechts (Summary Card):* Subtotal, geschätzte Gebühren, und die summierte erwartete Rendite (Endowment-Effekt!).
- **Die Hard Wall:** Der finale Button heißt "Proceed to Checkout". Er ist ein Anker/Formular, das den Gast via HTMX auf `/login?redirect=/checkout` weiterleitet.

### Anpassungen am Backend/Router
- **`login.html` & `signup.html`:** Weiterleitungsparameter `?redirect=/checkout` muss von Axum & Jinja ausgelesen und an das Formular übergeben werden.

---

## 6. Arbeitsablauf & Implementierungs-Phasen

**Phase 1: Backend & Security Blueprint (Rust/Axum)**
1. Axum Session-Middleware anpassen: Erlauben von anonymen Sessions für den Warenkorb.
2. Modifikation der Cart-Endpoints (`/api/cart`), damit sie auf `session_id` matchen, wenn `user_id` null ist.
3. Bereitstellen der Jinja-HTML-Fragmente für HTMX.

**Phase 2: Das Frontend (UI/HTMX)**
1. Erstellen der neuen `index.html` (inkl. CSS für Landing-Page & Flat-Design Asset Cards).
2. Einbau der HTMX-Attribute für Filterung und Cart-Actions.
3. Erstellen der `public-asset.html` und `public-cart.html`.

**Phase 3: Integration & Testing (Full-Stack)**
1. Anpassen des Login/Sign-up-Flows für nahtlose Redirects.
2. Erstellen umfassender Python E2E GUI-Tests (Pytest) für den gesamten Gast-Warenkorb-Zyklus.
