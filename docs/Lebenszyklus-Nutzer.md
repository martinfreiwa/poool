# POOOL Nutzer-Lebenszyklen

Dieses Dokument beschreibt die fachlichen Lebenszyklen, auf denen robuste E2E-Workflows basieren sollen. Der Fokus liegt auf ineinandergreifenden Rollen: Investor, Developer und Admin. Tests duerfen fuer fragile Setup-Schritte deterministische API- oder DB-Hand-offs nutzen, muessen finanzielle Zustandswechsel aber immer in den echten Tabellen pruefen.

## Leitprinzipien fuer E2E-Workflows

- Alle Geldwerte werden in `BIGINT`-Cents validiert, nie als Float.
- Jede Einzahlung, jeder Kauf, jede Order und jeder Trade wird gegen Wallet-, Ledger-, Investment- und Audit-Tabellen geprueft.
- Browser-Routen werden dort besucht, wo sie Nutzer- oder Admin-Zustand sichtbar machen.
- KYC, Developer-Freigabe, Asset-Publikation und Settlement duerfen im ersten stabilen Workflow als kontrollierte Test-Hand-offs passieren, solange die nachgelagerten Invarianten exakt geprueft werden.
- Jeder Test erzeugt eigene Nutzer, Assets, Orders und Referenzen und raeumt sie anschliessend wieder auf.

## Investor Primary Lifecycle

Ziel: Ein neuer Investor zahlt Geld ein, ein Admin bestaetigt den Zahlungseingang, der Investor kauft Primary-Tokens und sieht die Position im Portfolio.

1. Investor registriert sich oder wird als frischer E2E-Nutzer angelegt.
2. KYC wird genehmigt, damit Wallet-, Checkout- und Trading-Gates nicht blockieren.
3. Investor oeffnet `/wallet` und startet eine manuelle Einzahlung ueber `/api/wallet/deposit/init`.
4. Investor laedt einen Proof of Payment ueber `/wallet/deposit/:id/submit` hoch.
5. Admin oeffnet `/admin/deposits` und bestaetigt die Einzahlung ueber den echten Admin-Confirm-Endpunkt.
6. Erwartete Datenlage:
   - `deposit_requests.status = 'paid'`
   - Cash-Wallet des Investors steigt exakt um `amount_cents`
   - `wallet_transactions` enthaelt die abgeschlossene Einzahlung
   - `audit_logs.action = 'deposit.confirmed'`
7. Investor oeffnet `/marketplace`, waehlt ein publiziertes Asset, legt Tokens in den Warenkorb und kauft per Wallet-Checkout.
8. Erwartete Datenlage:
   - Cash-Wallet sinkt exakt um den Kaufbetrag
   - `wallet_transactions.type = 'purchase'`
   - `orders.status` ist `completed` fuer Wallet-Checkout
   - `investments.tokens_owned` steigt fuer das Asset
   - `/portfolio` und `/transactions` rendern ohne kritische Browserfehler

## Developer Lifecycle

Ziel: Ein Nutzer wird Developer, reicht ein Asset ein, das Asset wird adminseitig freigegeben und der Developer liefert operative Daten.

1. Developer registriert sich oder wird als frischer E2E-Nutzer angelegt.
2. Developer ruft `/api/developer/apply` mit vollstaendiger Application-Payload auf.
3. KYC wird genehmigt.
4. Admin genehmigt die Developer-Application oder der Test setzt diese Freigabe deterministisch, wenn die Admin-UI nicht Teil des Fokus ist.
5. Developer erhaelt die Rolle `developer`.
6. Asset wird erstellt, mit Developer verknuepft und als `published = TRUE` sowie `funding_status = 'funding_open'` oder `funded` bereitgestellt.
7. Browser prueft mindestens `/developer/dashboard`, `/developer/submissions`, `/developer/assets` oder die relevante Asset-Detailansicht.
8. Developer reicht einen Operations-Log ueber `/api/developer/villas/:asset_id/operations` ein und submitet ihn.
9. Erwartete Datenlage:
   - `developer_applications.status = 'approved'`
   - `developer_asset_links` verknuepft Developer und Asset
   - Operations-Log ist gespeichert und im Status `submitted`

## Secondary Market Lifecycle

Ziel: Investor A verkauft Tokens, Investor B kauft diese Tokens, Settlement bewegt Cash, Fees, Holds und Token-Bestaende korrekt.

1. Investor A besitzt aktive Tokens in `investments`.
2. Investor B hat bestaetigtes Cash im Wallet.
3. Asset ist fuer Secondary Trading geeignet, etwa `funding_status = 'funded'` und `published = TRUE`.
4. Investor A platziert eine Sell-Order ueber `/api/marketplace/orders`.
5. Investor B platziert eine passende Buy-Order ueber `/api/marketplace/orders`.
6. Matching/Settlement laeuft ueber den Worker oder im stabilen E2E ueber einen deterministischen Settlement-Hand-off, der dieselben Tabelleninvarianten herstellt.
7. Erwartete Datenlage:
   - Buy- und Sell-Order sind `filled`
   - `trade_history` enthaelt genau den Trade
   - Buyer-Wallet ist um Kaufbetrag plus etwaige Buyer-Fee reduziert
   - Seller-Wallet ist um Verkaufserloes minus etwaige Seller-Fee erhoeht
   - Seller-`tokens_owned` sinkt, Seller-`held_tokens` ist wieder frei
   - Buyer-`tokens_owned` steigt
   - Keine Wallet hat negativen Saldo und Buyer/Seller sind nie dieselbe Person
8. Browser prueft `/marketplace-secondary`, `/admin/marketplace/orders`, `/portfolio` und `/transactions`.

## Admin Lifecycle

Ziel: Admin-Schritte bestaetigen externe oder kontrollpflichtige Zustaende, statt sie implizit im Investor- oder Developer-Flow zu verstecken.

1. Deposit Review:
   - Admin sieht pending/requested Deposits in `/admin/deposits`.
   - Confirm schreibt Wallet, Ledger und Audit.
   - Cancel/Extend bleiben separate negative Kontrollpfade.
2. Developer Review:
   - Admin prueft Application, KYC-Status und Unterlagen.
   - Approval setzt `developer_applications.status = 'approved'` und aktiviert die Rolle.
3. Asset Publication:
   - Admin oder deterministischer Test-Hand-off setzt Asset live.
   - Marketplace- und Developer-Routen muessen danach denselben Asset-Zustand anzeigen.
4. Marketplace Oversight:
   - Admin prueft Orders, Orderbook, Trades und Audit-Logs.
   - Admin-Cancel muss Holds freigeben und Audit schreiben.

## Negative und Control Lifecycles

- Einzahlung ohne Proof wird abgelehnt und darf kein Wallet-Guthaben erzeugen.
- Einzahlung mit Proof, aber ohne Admin-Confirm, darf Checkout/Trading nicht finanzieren.
- Nicht genehmigte KYC blockiert Checkout und Marketplace-Orders.
- Self-Trade wird blockiert, wenn ein Nutzer gegen eigene offene Orders handeln wuerde.
- Unzureichendes Cash blockiert Buy-Orders und Checkout.
- Unzureichende oder bereits gehaltene Tokens blockieren Sell-Orders.
- Admin-API-Mutations ohne CSRF werden abgelehnt.
- Admin ohne passende Permissions darf Deposit- und Marketplace-Mutations nicht ausfuehren.

## Empfohlene Test-Slices

1. `test_full_user_lifecycle_hybrid`: kompletter stabiler Pfad Investor -> Admin -> Developer -> Primary Buy -> Secondary Sell/Buy.
2. `test_deposit_requires_admin_confirmation`: Deposit mit Proof bleibt bis Admin-Confirm nicht als verfuegbares Guthaben nutzbar.
3. `test_secondary_market_controls`: Self-Trade, insufficient funds und insufficient tokens werden separat geprueft.
4. `test_admin_financial_audit_chain`: Admin-Confirm und Admin-Order-Cancel schreiben Audit-Logs und halten Wallet-Invarianten.

## Akzeptanzkriterien

- Der Happy Path laeuft gegen einen lokalen Backend-Server auf `BASE_URL` ohne manuelle Vorarbeit ausser laufender DB/Backend-Instanz.
- Jeder erzeugte Nutzer, jedes Asset, jede Order und jeder Trade ist ueber einen eindeutigen Marker identifizierbar.
- Cleanup entfernt Testdaten bestmoeglich, ohne fremde Daten anzufassen.
- Browser-Qualitaetschecks erfassen kritische JS-Fehler, HTTP-Fehler, blanke Seiten und Trace/Screenshot bei Fehlern.
