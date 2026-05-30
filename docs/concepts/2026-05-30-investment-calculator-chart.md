# Investment Calculator Chart — Konzept

**Datum:** 2026-05-30
**Seite:** `/marketplace-trading-v3` → Investment Calculator
**Code:** [`marketplace-trading-v3.js:1798-2003`](../../frontend/platform/static/js/marketplace-trading-v3.js#L1798)
**Status:** Konzept — vor Implementierung freizugeben

---

## 1. Welche Frage beantwortet der Chart?

**Festgelegt: "Wie wächst mein Vermögen über die nächsten 5 Jahre?"**

Der Kunde investiert einen Betrag und will sehen, wie sich daraus über die Zeit
Vermögen aufbaut — Jahr für Jahr sichtbar steigend. Verkaufsorientiert: der
Chart soll Wachstum spürbar machen, ohne zu lügen.

Nicht beantwortet (bewusst): jährlicher Cashflow isoliert, Endwert-Donut.

---

## 2. Das Rechenmodell (Kern-Fix)

### Heute (falsch)
Jeder Balken = Prinzipal + Appreciation **nur dieses Jahres** + Miete **nur
dieses Jahres**. Ergebnis: Balken bleiben fast flach (122k → 127k), obwohl die
Headline einen kumulierten Gewinn von 121k zeigt. **Headline und Balken
widersprechen sich.**

### Neu (korrekt) — kumulatives Vermögen
```
Für Jahr N (1..5):
  Prinzipal            = Investment                       (konstant)
  kumul. Appreciation  = Σ jährl. Wertsteigerung bis N    (compoundet auf Immobilienwert)
  kumul. Miete         = Σ jährl. Miete bis N             (flach auf Investment)
  Balkenhöhe (Wert)    = Prinzipal + kumul. App + kumul. Miete
```

### Beispiel (100k, 10% Growth, 12% Yield)
| Jahr | Immobilienwert | kum. Appreciation | kum. Miete | **Balken (Gesamtwert)** |
|------|---------------:|------------------:|-----------:|------------------------:|
| 2026 | 100k → 110k    | 10,000            | 12,000     | **122,000** |
| 2027 | 110k → 121k    | 21,000            | 24,000     | **145,000** |
| 2028 | 121k → 133.1k  | 33,100            | 36,000     | **169,100** |
| 2029 | 133.1k → 146.4k| 46,410            | 48,000     | **194,410** |
| 2030 | 146.4k → 161.1k| 61,051            | 60,000     | **221,051** |

→ Balken steigen sichtbar 122k → 221k. Headline-Gewinn (121,051) = letzte
kum. App (61,051) + letzte kum. Miete (60,000). **Konsistent.**

### Modell-Annahmen (offen für Produkt-Entscheid, später)
- **Appreciation compoundet** auf den wachsenden Immobilienwert.
- **Miete ist flach** auf dem ursprünglichen Investment (wächst nicht mit).
  Inkonsistent, aber konservativ. Vorerst beibehalten — kein Blocker.

---

## 3. Parameter & Daten-Wiring

**Festgelegt: Parameter komplett frei, nur Defaults. Asset-Daten NICHT
vorbelegen** (außer Investment-Cap, s.u.).

| Slider | Min | Max | Default |
|--------|-----|-----|---------|
| Amount of Investment | $500 | **verfügbare Shares × tokenPrice** | min(100k, max) |
| Property value growth | 1% | 20% | 10% |
| Expected annual rental yield | 1% | 20% | 12% |

### Investment-Cap-Fix (festgelegt)
Heute: `slider.max = propertyValue` (voller Immobilienwert — unrealistisch).
Neu: `slider.max = available × tokenPrice`, wobei
`available = totalSupply − Σ offene Sell-Orders`
(bereits berechnet in [`:396`](../../frontend/platform/static/js/marketplace-trading-v3.js#L396)).
Der Kunde kann im Rechner nur so viel "investieren", wie real kaufbar ist.

### Bekannter Footgun (Mit-Fix, kostet nichts)
[`:1983-1985`](../../frontend/platform/static/js/marketplace-trading-v3.js#L1983):
`parseFloat(x) || default` schluckt einen legitimen Wert **0**. Ersetzen durch
`Number.isFinite(x) ? x : default`.

---

## 4. Design-Probleme heute → Ziel

Echte Brand-Farben (aus CSS): Investment `#000080`, Appreciation `#3A4BFF`,
Rental `#98FB96`, Brand-Blau `#0000FF`, Font `TT Norms Pro`.

| Problem | Ursache | Fix |
|---------|---------|-----|
| Balken fast flach, kein Profit-Gefühl | falsches Pro-Jahr-Modell | kumulatives Modell (§2) — löst ~80% |
| Toter Raum oben (Y bis 150k, Balken ~125k) | Skala passt nicht zu Daten | Auto-nice-max ([`:1880`](../../frontend/platform/static/js/marketplace-trading-v3.js#L1880)) greift mit höheren Werten automatisch |
| Value-Labels überlappen Gridlines | kein Headroom über Balken | Padding/Headroom + Label-Position |
| "Sieht billig aus / passt nicht zur Seite" | flaches Styling, Light-Green wirkt grell | Redesign — Variante via Mockup-Auswahl |

---

## 5. Offene Entscheidung: Chart-Stil

Drei Mockup-Varianten als Screenshots:
- **A** — Gestapelte Balken, aufpoliert (geringes Risiko, vertraut)
- **B** — Gestapelte Fläche / "Hockey-Stick" (max. Wow)
- **C** — Premium-Fläche mit Gewinn-Callout (+121% Chip)

→ Auswahl steht aus.

---

## 6. Umsetzungsreihenfolge (nach Freigabe)

1. Rechenmodell auf kumulativ ([`calculateReturns`](../../frontend/platform/static/js/marketplace-trading-v3.js#L1845) + [`updateStatsCard`](../../frontend/platform/static/js/marketplace-trading-v3.js#L1966)).
2. Investment-Cap auf verfügbare Shares ([`populateCalculator`](../../frontend/platform/static/js/marketplace-trading-v3.js#L958)).
3. Footgun-Fix ([`updateCalculator`](../../frontend/platform/static/js/marketplace-trading-v3.js#L1982)).
4. Gewählten Chart-Stil bauen (CSS + Render).
5. Im Preview verifizieren (Zahlen + Layout + responsive).
