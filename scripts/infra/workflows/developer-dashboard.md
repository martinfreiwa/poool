---
description: Make the Developer Dashboard fully production-ready — replace all hardcoded values with dynamic DB data, implement missing API endpoints, fix UI/UX issues, verify calculations
---

# 🏗️ Developer Dashboard — Production-Ready Workflow

> **Ziel:** Die Seite `/developer/dashboard` von einem statischen Prototyp in ein voll funktionsfähiges, datenbankgestütztes Dashboard transformieren. Alle Hardcoded-Werte durch dynamische API-Daten ersetzen, fehlende Backend-Endpoints implementieren, Charts & Tabellen mit echten Daten versorgen und die gesamte UI/UX verifizieren.

---

## 📊 Seiten-Analyse: Aktueller Zustand

### Dateien die betroffen sind
| Datei | Zweck | Status |
|-------|-------|--------|
| `frontend/platform/developer/dashboard.html` | Haupt-HTML (6701 Zeilen) | 🔴 100% statisch, monolithisch, KEIN `{% include %}` |
| `frontend/platform/static/js/developer-dashboard.js` | JS-Logik (127 Zeilen) | ⚠️ Nur Animationen, kein API-Fetch |
| `frontend/platform/static/css/developer-dashboard.css` | Desktop CSS | ✅ Grundstruktur ok |
| `frontend/platform/static/css/mobile-developer-dashboard.css` | Mobile CSS | ⚠️ Prüfen |
| `backend/src/developer/mod.rs` | Backend Router | ⚠️ Nur Page-Routes, keine API-Routes |
| `backend/src/developer/routes.rs` | Route Handler | ⚠️ Nur `serve_protected`, keine Datenlogik |
| `tests/test_developer_dashboard.py` | Test Suite | ⚠️ Rudimentär, keine API-Tests |

### Relevante DB-Tabellen
- `developer_projects` — Projekte des Developers (status, total_raised_cents, investors_count, funding_progress_bps)
- `assets` — Verknüpfte Assets (title, total_value_cents, tokens_total, tokens_available, funding_status)
- `investments` — Investments pro Asset (tokens_owned, purchase_value_cents)
- `orders` / `order_items` — Abgeschlossene Käufe
- `users` / `user_profiles` — Developer-Profildaten
- `user_roles` / `roles` — Rollenprüfung (developer)

---

## 🚨 KRITISCHE ERKENNTNIS: Architektur-Abweichung

Die `developer/dashboard.html` weicht **fundamental** vom Industriestandard des Projekts ab:

| Aspekt | ✅ Standard (wallet.html, 857 Zeilen) | 🔴 Aktuell (dashboard.html, 6701 Zeilen) |
|--------|----------------------------------------|------------------------------------------|
| **Head/Meta** | `{% include "components/head.html" %}` | Manuell geschrieben (Zeile 1-55) |
| **Mobile Menu** | `{% include 'components/mobile-menu.html' %}` | ~900 Zeilen inline kopiert (×3 Templates!) |
| **KYC Banner** | `{% include 'components/mobile-kyc-banner.html' %}` | Inline kopiert |
| **Sidebar** | `{% include 'components/sidebar.html' %}` | ~2400 Zeilen inline inkl. Sidebar + Desktop Dropdown |
| **SSR Daten** | `{{ cash_balance }}`, `{{ tx.status_label }}` | **KEIN EINZIGES** `{{ }}` Template-Tag |
| **State Layers** | 4-Layer Pattern (Loading/Error/Empty/Content) | Keine State-Layer |
| **Rust Handler** | Daten-Queries + `template.render(context! { ... })` | Nur `serve_protected` (keine Daten) |
| **Code-Duplikation** | 0% — alles in Components | ~4800 Zeilen duplizierter Code aus anderen Seiten |
| **Dateigröße** | 857 Zeilen | **6701 Zeilen** (8× größer, 80% davon Duplikate) |

> **Fazit:** ~70% des Dateiinhalts (Sidebar, Mobile-Menu, Dropdown, Profile-Switcher) sind Copy-Paste-Duplikate, die bereits als Minijinja-Komponenten existieren. Die Seite muss **vor** der Dynamisierung auf die Komponentenarchitektur umgebaut werden.

---

## ✅ MASTER-CHECKLISTE

### Phase 0: Template-Architektur & Code-Deduplizierung (ZUERST!)
> **PRIORITÄT 1** — Ohne diese Phase ist jede Änderung in doppelter/dreifacher Arbeit.

#### 0.1 Datei auf Minijinja-Komponentensystem umstellen

- [ ] **0.1.1** Head-Section ersetzen
  - Aktuell: 55 Zeilen manueller `<head>` Inhalt (Zeile 1-55)
  - Soll: `{% with title="Developer Dashboard", extra_css=['developer-dashboard', 'sidebar-developer'], extra_js=['developer-dashboard'] %}{% include "components/head.html" %}{% endwith %}`
  - Entferne: Duplizierte CSS-Imports die `head.html` bereits liefert

- [ ] **0.1.2** Mobile-Menu entfernen und durch Component ersetzen
  - Aktuell: 3 komplette `<template>` Blöcke (Investor-Menu, Developer-Menu, Profile-Switcher) → **~1200 Zeilen**
  - Soll: `{% include 'components/mobile-menu.html' %}`
  - Developer-vs-Investor-Logik muss in die Komponente integriert werden (falls nicht schon)

- [ ] **0.1.3** KYC-Banner durch Component ersetzen
  - Aktuell: Inline geschrieben (~70 Zeilen)
  - Soll: `{% include 'components/mobile-kyc-banner.html' %}` + `{% include 'components/kyc-banner.html' %}`

- [ ] **0.1.4** Sidebar durch Component ersetzen
  - Aktuell: Kompletter Investor- + Developer-Sidebar + Desktop-Profile-Dropdown inline (~2400 Zeilen!)
  - Soll: `{% include 'components/sidebar.html' %}`
  - Developer-spezifische Sidebar-Einträge (Dashboard, Assets, Add Asset) über Template-Variablen steuern

- [ ] **0.1.5** Seitenstruktur an Standard angleichen
  ```html
  {% with title="Developer Dashboard", ... %}{% include "components/head.html" %}{% endwith %}
  <body id="developer-dashboard-body">
    {% include 'components/mobile-menu.html' %}
    {% include 'components/mobile-kyc-banner.html' %}
    <div class="developer-dashboard-page">
      <div class="developer-dashboard-sidebar">
        {% include 'components/sidebar.html' %}
      </div>
      <main class="developer-dashboard-main">
        {% include 'components/kyc-banner.html' %}
        <!-- State Layers hier -->
        <!-- Dashboard Content hier -->
      </main>
    </div>
  </body>
  ```

- [ ] **0.1.6** Ergebnis validieren: Datei sollte von ~6701 auf ~800-1200 Zeilen schrumpfen

#### 0.2 Rust Backend: SSR mit Daten-Injection (wie wallet.html)
> Die Wallet-Seite zeigt das Referenzmuster: Rust-Handler laden Daten und übergeben sie via `context!` an Minijinja

- [ ] **0.2.1** Developer Dashboard Handler umschreiben
  ```rust
  // Aktuell (developer/routes.rs, Zeile 9-11):
  pub async fn page_developer_dashboard(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
      crate::common::routes_helper::serve_protected(jar, &state, "developer/dashboard.html").await
  }
  
  // Soll (nach Refactoring):
  pub async fn page_developer_dashboard(
      jar: CookieJar,
      State(state): State<AppState>,
  ) -> impl IntoResponse {
      let user = match middleware::get_current_user(&jar, &state.db).await {
          Some(u) => u,
          None => return Redirect::to("/auth/login").into_response(),
      };
      
      // Developer-Rollenprüfung
      let is_developer = sqlx::query_scalar!(
          "SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = $1 AND r.name = 'developer')",
          user.id
      ).fetch_one(&state.db).await.unwrap_or(Some(false)).unwrap_or(false);
      
      if !is_developer {
          return Redirect::to("/marketplace").into_response();
      }
      
      // Dashboard-Daten laden
      let stats = fetch_developer_dashboard_stats(&state.db, user.id).await;
      let top_assets = fetch_developer_top_assets(&state.db, user.id, "all_time").await;
      
      // SSR-Rendering mit Daten
      match state.templates.get_template("developer/dashboard.html") {
          Ok(template) => {
              match template.render(context! {
                  user => user,
                  total_assets => stats.total_assets,
                  total_sales => stats.total_sales_formatted,
                  total_investors => stats.total_investors,
                  new_investors => stats.new_investors,
                  total_views => stats.total_views,
                  avg_conversion_rate => stats.avg_conversion_rate,
                  sold_out_ratio => stats.sold_out_ratio,
                  avg_investment_amount => stats.avg_investment_formatted,
                  has_assets => stats.total_assets > 0,
                  assets => top_assets,
              }) {
                  Ok(content) => Html(content).into_response(),
                  Err(e) => { /* error handling */ }
              }
          }
          Err(_) => { /* 404 */ }
      }
  }
  ```

- [ ] **0.2.2** Rust Structs für Dashboard-Daten definieren (in `developer/models.rs` oder `developer/routes.rs`)
  ```rust
  #[derive(Debug, Serialize)]
  pub struct DeveloperDashboardStats {
      pub total_assets: i64,
      pub total_sales_cents: i64,
      pub total_sales_formatted: String,
      pub total_investors: i64,
      pub new_investors: i64,
      pub total_views: i64,
      pub avg_conversion_rate: f64,
      pub sold_out_ratio: f64,
      pub avg_investment_cents: i64,
      pub avg_investment_formatted: String,
      // Per-metric changes
      pub total_assets_change: MetricChange,
      pub total_sales_change: MetricChange,
      // ... etc
  }
  
  #[derive(Debug, Serialize)]
  pub struct MetricChange {
      pub percentage: f64,
      pub trend: String,  // "up" or "down"
  }
  
  #[derive(Debug, Serialize)]
  pub struct DeveloperTopAsset {
      pub id: String,
      pub title: String,
      pub cover_image_url: Option<String>,
      pub sales_cents: i64,
      pub sales_formatted: String,
      pub sales_change_pct: f64,
      pub sales_trend: String,
      pub views: i64,
      pub conversion_rate: f64,
      pub funding_progress_pct: i64,
  }
  ```

- [ ] **0.2.3** `cargo sqlx prepare` nach Query-Änderungen ausführen
  - SQLx Offline-Mode Meta-Daten aktualisieren
  - `.sqlx` Verzeichnis committen für Cloud-Build-Kompatibilität

- [ ] **0.2.4** Alle Geldbeträge mit `format_usd()` Helper formatieren (einheitlich, Cents → USD)

#### 0.3 Developer-spezifische Sidebar-Komponente

- [ ] **0.3.1** Prüfen ob `components/sidebar.html` Developer-Modus unterstützt
  - Falls nicht: Conditional Rendering hinzufügen basierend auf `user.role`
  - Developer-Sidebar-Items: Dashboard, Assets, Add Asset, Ranking (Soon), Settings
  - Investor-Sidebar-Items: Marketplace, Portfolio, Wallet, Rewards, etc.

- [ ] **0.3.2** Profile-Dropdown in Sidebar-Component integrieren
  - Desktop Profile-Dropdown (~300 Zeilen inline in dashboard.html) muss Teil der Sidebar-Component sein
  - User-Daten (`{{ user.first_name }}`, `{{ user.email }}`, `{{ user.avatar_url }}`) statt hardcoded "Olivia Rhye"

---

### Phase 1: Backend API Endpoints erstellen
> Aktuell existieren **KEINE** `/api/developer/*` Endpoints. Alles muss neu implementiert werden.

- [ ] **1.1** `GET /api/developer/dashboard/stats` — Haupt-KPI-Endpoint erstellen
  - [ ] Query: Zähle alle Assets des eingeloggten Developers aus `assets WHERE developer_user_id = $1`
  - [ ] Query: Summe `total_raised_cents` aus `developer_projects WHERE developer_id = $1`
  - [ ] Query: Summe aller `tokens_owned` aus `investments` für Assets des Developers → Total Investors (COUNT DISTINCT user_id)
  - [ ] Query: Neue Investoren im letzten Monat → New Investors
  - [ ] Berechnung: Avg. Conversion Rate = (total_tokens_sold / total_tokens) * 100
  - [ ] Berechnung: Sold Out Ratio = (fully funded assets / total assets) * 100
  - [ ] Berechnung: Avg. Investment Amount = total_raised_cents / total_investors
  - [ ] Berechnung: Veränderung vs. Vormonat für jede Metrik (% change)
  - [ ] Response-Format: JSON mit allen 8 Metriken + change + trend (up/down)

- [ ] **1.2** `GET /api/developer/dashboard/sales-chart` — Sales Chart Daten
  - [ ] Query: Monatliche Sales-Summen aus `order_items oi JOIN orders o ON o.id = oi.order_id JOIN assets a ON a.id = oi.asset_id WHERE a.developer_user_id = $1 AND o.status = 'completed' GROUP BY month`
  - [ ] Unterstütze Zeitrahmen via Query-Parameter: `?period=all_time|1y|30d|7d|24h`
  - [ ] Berechnung: Prozentuale Veränderung vs. Vorperiode
  - [ ] Response: Array von `{month: "Jan", value_cents: 12300000}` + `percentage_change`

- [ ] **1.3** `GET /api/developer/dashboard/top-assets` — Top Performing Assets
  - [ ] Query: Assets des Developers mit Sales, Views, Conversion Rate, Funding Progress
  - [ ] Sales = SUM(order_items.subtotal_cents) WHERE asset_id = X
  - [ ] Views = noch keine Tabelle → **NEUE Tabelle `asset_views` benötigt** oder Platzhalter
  - [ ] Conversion Rate = (investors_count / views) * 100
  - [ ] Funding Progress = ((tokens_total - tokens_available) / tokens_total) * 100
  - [ ] Unterstütze Zeitrahmen via Query-Parameter
  - [ ] Sortierung nach Sales (absteigend)
  - [ ] Response: Array von Asset-Objekten mit allen Spalten

- [ ] **1.4** Rollenprüfung in allen Endpoints
  - [ ] Middleware oder Guard: User muss `developer` Role haben
  - [ ] 403 zurückgeben wenn User kein Developer ist
  - [ ] Nur eigene Assets/Projekte zurückgeben (WHERE developer_user_id = current_user_id)

- [ ] **1.5** Routes in `backend/src/developer/mod.rs` registrieren
  - [ ] `.route("/api/developer/dashboard/stats", get(api_developer_dashboard_stats))`
  - [ ] `.route("/api/developer/dashboard/sales-chart", get(api_developer_sales_chart))`
  - [ ] `.route("/api/developer/dashboard/top-assets", get(api_developer_top_assets))`

- [ ] **1.6** Asset-View Tracking im Backend (Essential für "Total Views")
  - [ ] **Logik**: Jeder Aufruf von `page_property` und `page_commodity` in `backend/src/assets/routes.rs` muss einen View-Counter inkrementieren.
  - [ ] **Implementierung**: Funktion `track_asset_view(pool, asset_id, user_id, ip, user_agent)` erstellen.
  - [ ] **Query**: `INSERT INTO asset_views (asset_id, user_id, viewer_ip, user_agent) VALUES ($1, $2, $3, $4)`
  - [ ] **Integration**: In den Route-Handlern aufrufen (asynchron, ohne den Page-Load zu blockieren).

- [ ] **1.7** Conversion Rate Logik (Errechnet, nicht gespeichert)
  - [ ] **Definition**: `Conversion Rate = (Investoren Anzahl / Seitenaufrufe) * 100`
  - [ ] **Berechnung**: Im `stats` Endpoint:
    ```rust
    let views = sqlx::query_scalar!("SELECT COUNT(*) FROM asset_views WHERE asset_id = $1", asset_id).fetch_one(db).await?;
    let investors = sqlx::query_scalar!("SELECT COUNT(DISTINCT user_id) FROM investments WHERE asset_id = $1", asset_id).fetch_one(db).await?;
    let conv_rate = if views > 0 { (investors as f64 / views as f64) * 100.0 } else { 0.0 };
    ```

> [!TIP]
> **Expert-Tipp zum View-Tracking**: Um die "Total Views" nicht durch einfache Seiten-Refreshes zu verfälschen, sollte die `track_asset_view` Funktion eine Prüfung auf `viewer_ip` + `asset_id` innerhalb der letzten 24 Stunden implementieren (Unique Visits).

---

### Phase 2: Frontend — Hardcoded Werte durch API-Daten ersetzen

#### 2.1 Metric Cards (8 Stück — ALLE 100% hardcoded)

| Metrik | Hardcoded Wert | DB-Quelle | Formel |
|--------|---------------|-----------|--------|
| Total Assets | `12` | `COUNT(*) FROM assets WHERE developer_user_id = $1` | Direkt |
| Total Sales | `$138.4k` | `SUM(oi.subtotal_cents)` über Developer-Assets | Cents → USD |
| Total Investors | `650` | `COUNT(DISTINCT i.user_id) FROM investments i JOIN assets a ON a.id = i.asset_id WHERE a.developer_user_id = $1` | Direkt |
| New Investors | `37` | Gleiche Query + `AND i.purchased_at > NOW() - INTERVAL '30 days'` | Direkt |
| Total Views | `85,420` | ⚠️ **Keine Tabelle vorhanden** — `asset_views` muss erstellt werden oder Wert auf 0 | Aggregat |
| Avg. Conversion Rate | `8.5%` | views_count > 0 ? (investors / views * 100) : 0 | Berechnet |
| Sold Out Ratio | `67%` | (assets WHERE tokens_available = 0) / total_assets * 100 | Berechnet |
| Avg. Investment Amount | `$1.5k` | total_raised_cents / total_investors | Cents → USD |

- [ ] **2.1.1** Service-Datei erstellen: `static/js/developer-dashboard-service.js`
  - [ ] `fetchDashboardStats()` → GET /api/developer/dashboard/stats
  - [ ] `fetchSalesChartData(period)` → GET /api/developer/dashboard/sales-chart?period=X
  - [ ] `fetchTopAssets(period)` → GET /api/developer/dashboard/top-assets?period=X
  - [ ] Fehlerbehandlung: try/catch, Retry-Logik
  - [ ] Formatierungs-Hilfsfunktionen: `formatCurrency(cents)`, `formatPercentage(bps)`, `formatNumber(n)`

- [ ] **2.1.2** Controller-Datei erweitern: `static/js/developer-dashboard.js`
  - [ ] `loadDashboardStats()` → API aufrufen → DOM-Elemente befüllen
  - [ ] Jede Metrik-Karte dynamisch aktualisieren (Wert, Change %, Trend up/down)
  - [ ] Mini-Chart SVG-Pfade dynamisch generieren (oder als statische Illustration belassen)
  - [ ] Animationen beibehalten (animate from 0 to API value)

- [ ] **2.1.3** Veränderungs-Badges (Change %) dynamisch setzen
  - [ ] Aktuell: Alle Pfeile sind hardcoded (up/down)
  - [ ] Soll: Basierend auf API-Response `.trend` Feld CSS-Klasse `.up` oder `.down` setzen
  - [ ] SVG-Pfeil Richtung dynamisch ändern (Arrow up vs Arrow down)
  - [ ] Farbe dynamisch: grün (#17B26A) für up, rot (#F04438) für down

#### 2.2 Sales Chart

- [ ] **2.2.1** Chart-SVG dynamisch generieren
  - [ ] Aktuell: Kompletter SVG-Pfad ist hardcoded (statische Bézier-Kurve)
  - [ ] Soll: SVG-Pfad aus API-Datenpunkten berechnen
  - [ ] Hilfsfunktion: `generateSVGPath(dataPoints, width, height)` implementieren
  - [ ] Gradient-Fill unter der Linie beibehalten

- [ ] **2.2.2** Y-Achse dynamisch
  - [ ] Aktuell: Hardcoded `300k, 200k, 150k, 100k, 50k, 0`
  - [ ] Soll: Automatische Skalierung basierend auf Max-Wert der Daten
  - [ ] Schöne Rundung (z.B. nächste 50k, 100k oider 500k)

- [ ] **2.2.3** X-Achse dynamisch
  - [ ] Aktuell: Hardcoded `Jan-Dec`
  - [ ] Soll: Dynamisch je nach gewähltem Zeitrahmen
  - [ ] "All time" → Monats-Labels, "30 days" → Tages-Labels, "7 days" → Tages-Labels, "24 hours" → Stunden-Labels

- [ ] **2.2.4** Tab-Wechsel funktional machen
  - [ ] Aktuell: Tabs wechseln nur CSS active-class, keine Daten ändern sich
  - [ ] Soll: Bei Tab-Klick API mit neuem `period` aufrufen und Chart neu rendern
  - [ ] Loading-State während API-Call anzeigen

- [ ] **2.2.5** Chart-Percentage dynamisch
  - [ ] Aktuell: Hardcoded `+17.6%`
  - [ ] Soll: Berechnet aus API-Response `percentage_change`

#### 2.3 Top Performing Assets Table

- [ ] **2.3.1** Tabellenzeilen dynamisch generieren
  - [ ] Aktuell: 6 hardcoded Zeilen mit identischen Asset-Namen ("1Bed in Sobha Hartland...")
  - [ ] Soll: Rows aus API `/api/developer/dashboard/top-assets` generieren
  - [ ] Template-Funktion `buildAssetRow(asset, index)` erstellen
  - [ ] Asset-Bild: Aus `asset_images` (cover_image) oder Fallback-Bild

- [ ] **2.3.2** Spalten korrekt befüllen
  - [ ] **Asset**: Name + Bild aus DB
  - [ ] **Sales**: `SUM(order_items.subtotal_cents)` formatiert als USD + Change %
  - [ ] **Views**: Aus `asset_views` oder "—" wenn nicht verfügbar
  - [ ] **Conversion Rate**: Berechnet (investors / views * 100) oder "—"
  - [ ] **Funding Progress**: `((tokens_total - tokens_available) / tokens_total * 100)` + Progress-Bar

- [ ] **2.3.3** Tab-Wechsel funktional machen
  - [ ] Aktuell: Tabs haben class `.table__tab` aber kein Event-Handler in JS
  - [ ] Die Tab-Buttons nutzen `table__tab` class, aber der JS sucht nach `.assets-tab` → **BUG**
  - [ ] Soll: Bei Tab-Klick API mit neuem `period` aufrufen und Tabelle neu rendern

- [ ] **2.3.4** "Show All" Button funktional machen
  - [ ] Aktuell: Button existiert, aber keine Funktionalität
  - [ ] Soll: Navigiert zu `/developer/assets` (Assets-Übersicht)

- [ ] **2.3.5** "View Details" Button pro Row
  - [ ] Aktuell: Buttons existieren, aber kein onClick
  - [ ] Soll: Navigiert zu `/developer/assets/:id` oder öffnet Detail-Modal

- [ ] **2.3.6** Sortierung implementieren
  - [ ] Sort-Icons in Table-Headers existieren (Chevron-SVGs)
  - [ ] Keine onclick-Handler → Sortier-Logik fehlt komplett
  - [ ] Implementiere: Click auf Header ↔ toggles ASC/DESC Sortierung

---

### Phase 3: UI/UX Bugs & Fehlende Funktionalität

#### 3.1 Identifizierte Bugs

- [ ] **BUG-01**: HTML enthält ungültiges `else` Attribut in Table-Cells
  - Zeilen: ~5989, 5991, 6011, 6013 etc. — `else` steht als HTML-Attribut → **ungültiges HTML**
  - Vermutlich Artefakt einer Template-Engine → muss bereinigt werden

- [ ] **BUG-02**: JS `.assets-tab` Selektor findet keine Elemente
  - `developer-dashboard.js` Zeile 14: `document.querySelectorAll(".assets-tab")` 
  - HTML nutzt aber die Klasse `table__tab` → **Tab-Event-Handler greift ins Leere**

- [ ] **BUG-03**: Duplizierte IDs in HTML
  - `developer-assets-header` existiert zweimal (Zeile 5784 und 5804)
  - Muss in eindeutige IDs geändert werden

- [ ] **BUG-04**: Metric Dropdown-Buttons (⋮) ohne Funktionalität
  - Alle 8 Metric-Cards haben einen 3-Dot-Dropdown-Button
  - Kein Event-Handler, kein Dropdown-Menü → Klick macht nichts

- [ ] **BUG-05**: Hardcoded User-Daten in Sidebar & Mobile Menu
  - "Olivia Rhye" und "olivia@poool.com" sind hardcoded
  - Muss durch echte User-Daten aus `/api/me` ersetzt werden
  - Avatar-Bild `/images/Image.webp` ist statisch

- [ ] **BUG-06**: Mobile KYC-Banner auf Developer Dashboard unpassend
  - KYC-Banner "Passport and address proof required" wird angezeigt
  - Auf dem Developer-Dashboard ist das irreführend → nur zeigen wenn KYC wirklich fehlt

#### 3.2 Fehlende UI-Elemente

- [ ] **UI-01**: Leerer Zustand (Empty State)
  - Was passiert wenn Developer noch keine Assets hat?
  - → Zeige "No assets yet. Add your first asset" CTA mit /developer/add-asset Link

- [ ] **UI-02**: Loading/Skeleton State
  - Keine Loading-Indikatoren während API-Calls
  - → Skeleton-Karten für Metriken, Chart-Placeholder und Tabellen-Skeleton implementieren

- [ ] **UI-03**: Error State
  - Keine Fehlerbehandlung wenn API-Calls fehlschlagen
  - → Error-Banner mit Retry-Button

- [ ] **UI-04**: Date Range Picker / Filter fehlt
  - Der Dashboard-Header hat nur den Titel "Dashboard"
  - → Optional: Datum-Bereich-Filter für alle Metriken hinzufügen

- [ ] **UI-05**: Notification Badge im Developer-Sidebar
  - Mobile-Menu zeigt "10" Notifications (hardcoded)
  - Sidebar-Desktop zeigt ebenfalls hardcoded Badge
  - → Dynamisch aus `/api/notifications/count` laden

- [ ] **UI-06**: Breadcrumb / Export-Button fehlt
  - Kein Export der Dashboard-Daten (PDF/CSV)
  - → Optional: Export-Button im Header hinzufügen

---

### Phase 4: Fehlende Seiten & Verlinkungen prüfen

- [ ] **4.1** `/developer/settings` — Seite existiert im Sidebar-Link, aber kein Route-Handler
  - Prüfen ob `developer/settings.html` existiert
  - Falls nicht: Route und Seite erstellen oder auf `/settings` umleiten

- [ ] **4.2** `/developer/notifications` — Im Mobile-Menu verlinkt
  - Prüfen ob Route und Seite existieren
  - Falls nicht: Implementieren oder "Coming Soon" State

- [ ] **4.3** `/developer/ranking` — "Soon" Badge im Mobile-Menu
  - Korrekt als "disabled" markiert → OK für jetzt

- [ ] **4.4** `/developer/assets/:id` — Asset-Detail-Seite für Developers
  - "View Details" Buttons in der Tabelle brauchen ein Ziel
  - Prüfen ob diese Seite existiert
  
- [ ] **4.5** Developer Support-Chat Widget
  - Mobile-Menu hat "Chat to support" Button
  - Prüfen ob /support Route für Developers korrekt funktioniert

---

### Phase 5: Berechnungslogik verifizieren

> **WICHTIG: Alle Geldbeträge in Cents (INTEGER) — niemals Floating Point!**

- [ ] **5.1** Total Sales Berechnung
  ```sql
  SELECT COALESCE(SUM(oi.subtotal_cents), 0) as total_sales_cents
  FROM order_items oi
  JOIN orders o ON o.id = oi.order_id
  JOIN assets a ON a.id = oi.asset_id
  WHERE a.developer_user_id = $1 AND o.status = 'completed'
  ```
  - [ ] Nur `completed` Orders zählen (nicht pending/failed)
  - [ ] Ergebnis in Cents → Frontend formatiert zu USD

- [ ] **5.2** Funding Progress Berechnung
  ```sql
  -- Per Asset:
  funding_progress = ((tokens_total - tokens_available)::float / tokens_total) * 100
  -- Gesamtdurchschnitt:
  AVG(((tokens_total - tokens_available)::float / tokens_total) * 100)
  ```
  - [ ] Division by Zero Guard wenn `tokens_total = 0`
  - [ ] Progress-Bar-Width muss dem berechneten % entsprechen

- [ ] **5.3** Avg. Investment Amount
  ```
  avg_investment_cents = total_sales_cents / NULLIF(total_investors, 0)
  ```
  - [ ] Division by Zero Guard
  - [ ] In Cents rechnen, im Frontend zu USD formatieren

- [ ] **5.4** Sold Out Ratio
  ```sql
  SELECT 
    COUNT(*) FILTER (WHERE tokens_available = 0)::float / NULLIF(COUNT(*), 0) * 100
  FROM assets WHERE developer_user_id = $1
  ```
  - [ ] Assets mit `tokens_available = 0` gelten als "sold out"

- [ ] **5.5** Monatliche Veränderung (% Change)
  ```
  change_pct = ((current_month - previous_month) / NULLIF(previous_month, 0)) * 100
  ```
  - [ ] Wenn Vormonats-Wert = 0 → zeige "+100%" oder "New"
  - [ ] Negativer Wert = "down" Trend, Positiver = "up" Trend

---

### Phase 6: Datensicherheit & RBAC

- [ ] **6.1** Developer Role Guard
  - Prüfe bei allen API-Endpoints: User hat `developer` Role
  - SQL: `EXISTS (SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = $1 AND r.name = 'developer')`

- [ ] **6.2** Daten-Isolation
  - Developer A darf nur SEINE Assets/Projekte/Stats sehen
  - NIEMALS `developer_user_id` aus dem Request nehmen — immer aus der Session

- [ ] **6.3** Rate Limiting
  - Dashboard-API kann viele Queries triggern
  - Cache-Strategie: Stats für 60 Sekunden cachen

---

### Phase 7: Tests erweitern

- [ ] **7.1** API-Endpoint-Tests
  - [ ] Test: `GET /api/developer/dashboard/stats` → 200 + korrektes JSON-Schema
  - [ ] Test: Unauthenticated → 401/403
  - [ ] Test: Non-Developer-User → 403
  - [ ] Test: Empty State (neuer Developer ohne Assets) → 200 mit Nullwerten
  - [ ] Test: Sales-Chart mit verschiedenen Period-Parametern

- [ ] **7.2** Berechnungs-Tests
  - [ ] Test: total_sales = SUM(order_items) für Developer-Assets (nicht fremde)
  - [ ] Test: Funding Progress = korrekt berechnet für verschiedene Token-Verhältnisse
  - [ ] Test: Division by Zero wird korrekt gehandled

- [ ] **7.3** UI-Integrationstests
  - [ ] Test: Dashboard lädt ohne JS-Fehler
  - [ ] Test: Alle Metric-Cards werden dynamisch befüllt
  - [ ] Test: Tab-Wechsel bei Chart und Table funktioniert
  - [ ] Test: "View Details" navigiert korrekt
  - [ ] Test: "Show All" navigiert zu /developer/assets

---

### Phase 8: Mobile-Responsiveness

- [ ] **8.1** Breakpoints prüfen (mobile-developer-dashboard.css)
  - [ ] Metric Cards: 1 Spalte auf Mobile, 2 auf Tablet, 3 auf Desktop
  - [ ] Sales Chart: Volle Breite auf Mobile, Achsenbeschriftungen angepasst
  - [ ] Assets Table: Horizontal scrollbar auf Mobile
  - [ ] Sidebar: Burger-Menu funktional

- [ ] **8.2** Touch-Interaktionen
  - [ ] Chart-Tabs sind tappbar (mind. 44x44px touch targets)
  - [ ] Table-Rows sind tappbar für "View Details"
  - [ ] Dropdown-Buttons funktionieren auf Touch

---

## 🗄️ Neue DB-Tabelle benötigt

### `asset_views` (Optional — für Total Views und Conversion Rate)
```sql
CREATE TABLE asset_views (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id    UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    viewer_ip   INET,
    user_agent  TEXT,
    user_id     UUID REFERENCES users(id),  -- NULL für anonyme Views
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_asset_views_asset ON asset_views(asset_id);
CREATE INDEX idx_asset_views_created ON asset_views(created_at DESC);
```
> **Alternativ:** Views-Tracking vorerst weglassen und Spalte "Views" mit "—" anzeigen.

---

## 📝 Zusammenfassung der Hardcoded Werte

| Element | Hardcoded Wert | Datei | Zeile(n) |
|---------|---------------|-------|----------|
| Total Assets | `12` | dashboard.html | ~4819 |
| Total Assets Change | `12%` | dashboard.html | ~4843 |
| Total Sales | `$138.4k` | dashboard.html | ~4916 |
| Total Sales Change | `2%` (down) | dashboard.html | ~4940 |
| Total Investors | `650` | dashboard.html | ~5016 |
| Total Investors Change | `2%` | dashboard.html | ~5040 |
| New Investors | `37` | dashboard.html | ~5119 |
| New Investors Change | `2%` | dashboard.html | ~5143 |
| Total Views | `85,420` | dashboard.html | ~5219 |
| Total Views Change | `12%` | dashboard.html | ~5243 |
| Avg. Conversion Rate | `8.5%` | dashboard.html | ~5325 |
| Avg. Conversion Rate Change | `2%` | dashboard.html | ~5349 |
| Sold Out Ratio | `67%` | dashboard.html | ~5434 |
| Sold Out Ratio Change | `12%` | dashboard.html | ~5458 |
| Avg. Investment Amount | `$1.5k` | dashboard.html | ~5546 |
| Avg. Investment Amount Change | `20%` | dashboard.html | ~5570 |
| Chart Percentage | `+17.6%` | dashboard.html | ~5645 |
| Chart SVG Path | Statische Bézier-Kurve | dashboard.html | ~5752-5758 |
| Y-Achse | `300k-0` | dashboard.html | ~5673-5674 |
| X-Achse | `Jan-Dec` | dashboard.html | ~5768-5771 |
| Table Row 1 | `$223k, 17,500, 9.2%, 78%` | dashboard.html | ~6016-6067 |
| Table Row 2 | `$120k, 28,237, 11%, 55%` | dashboard.html | ~6130-6183 |
| Table Row 3 | `$111k, 12,358, 5.7%, 63%` | dashboard.html | ~6244-6297 |
| Table Row 4 | `$97k, 11,961, 17.1%, 47%` | dashboard.html | ~6358-6410 |
| Table Row 5 | `$89k, 25,014, 8.4%, 24%` | dashboard.html | ~6472-6525 |
| Table Row 6 | `$77k, 11,599, 7.7%, 83%` | dashboard.html | ~6586-6638 |
| All Asset Names | "1Bed in Sobha Hartland..." | dashboard.html | alle Rows |
| All Asset Images | villa1-6.webp | dashboard.html | alle Rows |
| User Name | "Olivia Rhye" | dashboard.html | viele Stellen |
| User Email | "olivia@poool.com" | dashboard.html | viele Stellen |
| Mini-Chart SVGs | Statische Pfade (×8) | dashboard.html | je Metric Card |
| Notification Badge | "10" / "3" | dashboard.html | Mobile Header |

---

## 🚀 Empfohlene Reihenfolge der Umsetzung

1. **Phase 0** — Template-Architektur & Code-Deduplizierung (ZUERST!)
2. **Phase 0.2** — Rust SSR Handler + Daten-Injection  
3. **Phase 1** — Backend-APIs bauen (Fundament)
4. **Phase 2.1** — Service-Layer JS erstellen  
5. **Phase 2.2-2.3** — Metric Cards & Table dynamisch via SSR `{{ }}`
6. **Phase 3.1** — Bugs fixen (ungültiges HTML, JS-Selektoren)
7. **Phase 2.4** — Chart dynamisch machen (JS-seitig mit API-Fetch)
8. **Phase 3.2** — Empty/Loading/Error States (4-Layer Pattern)
9. **Phase 5** — Berechnungen verifizieren
10. **Phase 6** — Security & RBAC
11. **Phase 9** — Accessibility & SEO
12. **Phase 10** — Performance & Caching
13. **Phase 7** — Tests schreiben  
14. **Phase 4** — Fehlende Seiten prüfen  
15. **Phase 8** — Mobile Responsiveness  
16. **Phase 11** — Developer-Experience (Real-Time Updates)
17. **Phase 12** — Rust Code Quality & Compliance

---

### Phase 9: Accessibility & SEO (Experte)

- [ ] **9.1** Semantisches HTML prüfen
  - [ ] Nur ein `<h1>` pro Seite ("Dashboard")
  - [ ] Heading-Hierarchie korrekt: h1 → h2 (Section-Titel) → h3 (Card-Titel)
  - [ ] Metriken in `<section>` Elemente wrappen mit `aria-label`
  - [ ] Tabelle sollte `role="table"` oder echte `<table>` Tags nutzen (aktuell: div-basiert)

- [ ] **9.2** ARIA Labels & Rollen
  - [ ] Metric-Cards: `role="region" aria-label="Total Assets Metric"`
  - [ ] Chart: `role="img" aria-label="Sales chart showing monthly revenue"`
  - [ ] Tab-Panels: `role="tablist"` auf Tab-Container, `role="tab"` + `aria-selected` auf Tabs
  - [ ] Mini-Charts: `aria-hidden="true"` (dekorative SVGs)
  - [ ] Sort-Buttons in Table-Headers: `aria-label="Sort by Sales"` + `aria-sort`

- [ ] **9.3** Keyboard Navigation
  - [ ] Alle Tabs mit Pfeiltasten navigierbar
  - [ ] "View Details" Buttons per Tab erreichbar
  - [ ] Focus-Visible Style auf allen interaktiven Elementen (nicht nur `:hover`)
  - [ ] Skip-to-Content Link am Seitenanfang

- [ ] **9.4** SEO Meta-Tags (via `head.html` Component)
  - [ ] `<title>`: "Developer Dashboard - POOOL" ✅ (existiert)
  - [ ] `<meta name="robots" content="noindex">` — Dashboard ist privat, nicht indexieren

- [ ] **9.5** Color Contrast
  - [ ] Metrik-Werte (schwarz auf weiß) — OK
  - [ ] Chart-Linie (#98FB96 auf weiß) — ⚠️ WCAG AA Ratio prüfen (helles Grün könnte scheitern)
  - [ ] Change-Badges (grün/rot auf hellgrau) — Nicht nur Farbe, auch Pfeilrichtung als Indikator (✅ bereits vorhanden)

---

### Phase 10: Performance & Caching (Experte)

- [ ] **10.1** Backend-Caching für Dashboard-Stats
  - [ ] In-Memory Cache (z.B. `tokio::sync::RwLock<HashMap>`) für `/api/developer/dashboard/stats`
  - [ ] TTL: 60 Sekunden — Dashboard-Daten ändern sich nicht sekündlich
  - [ ] Cache-Key: `developer:{user_id}:stats`
  - [ ] Cache-Invalidierung bei neuem Investment/Order für Developer-Assets

- [ ] **10.2** SQL Query Optimierung
  - [ ] Alle Dashboard-Queries in EINER einzigen SQL-Abfrage zusammenfassen (CTE Pattern)
  ```sql
  WITH asset_stats AS (
      SELECT COUNT(*) as total_assets,
             COUNT(*) FILTER (WHERE tokens_available = 0) as sold_out_count
      FROM assets WHERE developer_user_id = $1
  ), investor_stats AS (
      SELECT COUNT(DISTINCT i.user_id) as total_investors,
             COUNT(DISTINCT i.user_id) FILTER (WHERE i.created_at > NOW() - INTERVAL '30 days') as new_investors,
             COALESCE(SUM(i.purchase_value_cents), 0) as total_raised_cents
      FROM investments i
      JOIN assets a ON a.id = i.asset_id
      WHERE a.developer_user_id = $1
  )
  SELECT * FROM asset_stats, investor_stats;
  ```
  - [ ] Vermeide N+1 Queries bei Top-Assets (Joins statt Loops)
  - [ ] `EXPLAIN ANALYZE` für alle Queries ausführen

- [ ] **10.3** Frontend Performance
  - [ ] CSS Critical Path: Dashboard-CSS inline im `<head>` für First Paint
  - [ ] Chart-SVG: Lazy-Load — erst rendern wenn im Viewport
  - [ ] Images in Asset-Table: `loading="lazy"` ✅ (existiert bereits)
  - [ ] JS-Bundle minimieren: Developer-Dashboard-JS nur auf dieser Seite laden (via `extra_js`)

- [ ] **10.4** HTTP Caching Headers
  - [ ] API-Responses: `Cache-Control: private, max-age=60`
  - [ ] Statische Assets (CSS/JS): `Cache-Control: public, max-age=86400, immutable`

---

### Phase 11: Developer-Experience & Echtzeit (Experte)

- [ ] **11.1** HTMX für Lazy-Loading Sektionen
  - [ ] Chart-Sektion: `hx-get="/api/developer/dashboard/chart-fragment" hx-trigger="load"`
  - [ ] Asset-Table: `hx-get="/api/developer/dashboard/assets-fragment" hx-trigger="load"`
  - [ ] Tab-Wechsel via HTMX: `hx-get="/api/developer/dashboard/chart-fragment?period=30d" hx-swap="innerHTML"`
  - [ ] Skeleton-Loading während der Requests (`hx-indicator`)

- [ ] **11.2** Refreshable Metriken
  - [ ] "Letzte Aktualisierung: vor X Minuten" Anzeige im Dashboard-Header
  - [ ] Manueller Refresh-Button (Reload-Icon) neben dem Titel
  - [ ] Optional: Auto-Refresh alle 5 Minuten

- [ ] **11.3** Toast/Notification bei Echtzeit-Events
  - [ ] Wenn ein neues Investment auf Developers Asset eingeht → Toast-Notification
  - [ ] Integration mit bestehendem Notifications-System (falls vorhanden)

- [ ] **11.4** Developer Onboarding
  - [ ] Beim ersten Login als Developer → Onboarding-Overlay/Welcome-Card
  - [ ] "Add your first asset" CTA prominent im Empty State
  - [ ] Tooltips für nicht offensichtliche Metriken (z.B. "Sold Out Ratio", "Conversion Rate")

---

### Phase 12: Rust Code Quality & Compliance (Experte)

> Entspricht den Projekt-Standards aus `standards_and_modularization.md`

- [ ] **12.1** Zero-Warning Policy
  - [ ] `cargo clippy --all-targets -- -D warnings` muss fehlerfrei durchlaufen
  - [ ] Alle neuen public Items brauchen `///` Rustdocs
  - [ ] Keine unused imports, dead code oder unhandled Results

- [ ] **12.2** Modul-Struktur konform
  - [ ] Developer-Module folgt dem Support-Module-Pattern: `handlers`, `service`, `db`, `models`
  - [ ] Oder mindestens: `mod.rs` (Router) + `routes.rs` (Handler) + ggf. `models.rs`
  - [ ] Imports über `crate::auth::routes::AppState` — kein Circular Import

- [ ] **12.3** SQLx Best Practices
  - [ ] `query_as!` für compile-time geprüfte Queries bevorzugen
  - [ ] LEFT JOIN Spalten mit `AS "column_name?"` für Nullable-Handling markieren
  - [ ] `cargo sqlx prepare` nach jeder Query-Änderung
  - [ ] `.sqlx/` Verzeichnis in Git versioniert

- [ ] **12.4** Error Handling
  - [ ] Alle DB-Queries mit expliziter Fehlerbehandlung (kein `.unwrap()`)
  - [ ] HTTP-angemessene Fehlercodes: 500 für DB-Fehler, 403 für Nicht-Developer, 404 für unbekannte Assets
  - [ ] `tracing::error!()` für unerwartete Fehler, `tracing::warn!()` für erwartete (z.B. "User hat keine Assets")

- [ ] **12.5** Kein Code-Duplikat im Backend
  - [ ] `format_usd()` Helper aus `common` nutzen — nicht neu implementieren
  - [ ] `get_current_user()` aus `auth::middleware` — nicht selbst Session-Cookies parsen
  - [ ] `serve_protected` Pattern für Pages nutzen ODER eigenen Handler mit Daten — nicht beides mischen

- [ ] **12.6** Testing Compliance
  - [ ] `cargo test` muss fehlerfrei durchlaufen
  - [ ] Neue Endpoints in Test-Suite aufnehmen
  - [ ] Edge Cases: Developer ohne Assets, Developer mit gelöschten Assets, abgelaufene Sessions

---

## 📋 Finale Qualitäts-Checkliste (vor Merge/Deploy)

- [ ] `cargo clippy -- -D warnings` ✅ 
- [ ] `cargo test` ✅
- [ ] `cargo sqlx prepare` ✅ (`.sqlx/` aktuell)
- [ ] `python3 tests/test_developer_dashboard.py` ✅
- [ ] HTML validiert (W3C Validator — keine `else` Attribute, keine doppelten IDs)
- [ ] Dashboard lädt ohne JS Console Errors
- [ ] Dashboard zeigt echte DB-Daten (keine Hardcoded Werte mehr)
- [ ] Empty State funktioniert (neuer Developer ohne Assets)
- [ ] Error State funktioniert (Backend nicht erreichbar)
- [ ] Mobile-Responsiveness getestet (320px - 1440px)
- [ ] Dateigröße `dashboard.html` < 1500 Zeilen (von ehemals 6701)
- [ ] Kein Code-Duplikat mit `components/*`
- [ ] Alle Tabs (Chart + Table) funktional mit API-Calls
- [ ] Developer sieht nur EIGENE Daten (RBAC verifiziert)
- [ ] Deployment: `gcloud run deploy` erfolgreich nach Änderungen

---

*Workflow erstellt: 2026-03-09 | Aktualisiert: 2026-03-09 | Kontext: POOOL Developer Dashboard*
*Erweitert um: Phase 0 (Architektur), Phase 9-12 (Experten-Phasen), Rust-Code-Standards, Finale QA-Checkliste*

