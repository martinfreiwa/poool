# POOOL Kostenanalyse: Development und Produktion

Stand: 22. Mai 2026  
Projekt: `my-project-35266-489713` / `POOOL 35266`  
Region aktuell: `europe-west1` fuer Backend, Cloud SQL und aktiven Redis  
Billing-Konto: `01FB25-F8276E-81802E`, Waehrung VND  
Planungsumrechnung: 1 USD ~= 26.000 VND. Google rechnet in VND nach den Cloud Platform SKUs ab; USD-Zahlen sind zur Vergleichbarkeit gerundet.

## Executive Summary

Die aktuelle Google-Cloud-Installation ist fuer eine Development-Phase zu teuer, weil mehrere produktionsartige Managed Services dauerhaft laufen:

- Cloud SQL `poool-db`: 2 vCPU / 7,5 GiB RAM, 24/7, obwohl die Datenbank nur ca. 98 MiB Daten nutzt.
- Cloud Run `poool-backend`: `min-instances=0`, aber CPU ist dauerhaft allokiert (`cpu-throttling=false`), dadurch werden fast ganze Instanz-Tage abgerechnet.
- Cloud Build: seit 1. Mai 2026 insgesamt 203 Builds, davon fast alle auf `E2_HIGHCPU_8`; das ist fuer Development teuer.
- Memorystore Redis: zwei 1-GiB-Instanzen, eine in `europe-west1` und eine offenbar ungenutzte in `asia-southeast1`.
- Serverless VPC Access Connector: laeuft mit mindestens 2 `e2-micro`-Instanzen, weil Redis privat angebunden ist.
- Artifact Registry: ca. 56,3 GB Docker-Images; Free Tier ist nur 0,5 GB.

Aktuelle Hochrechnung ohne Trial-Credits: ca. 269-284 USD/Monat, also ca. 7,0-7,4 Mio. VND/Monat.  
Fuer die naechsten zwei Development-Monate: ca. 540-570 USD, also ca. 14,0-14,8 Mio. VND, wenn nichts geaendert wird.

Mit Google-Cloud-Optimierung, ohne komplette Migration:

- konservativ, Managed Services bleiben: ca. 75-90 USD/Monat
- aggressiv fuer Development, Redis/VPC weg und kleinste DB: ca. 15-35 USD/Monat
- mit Cloud SQL weiter 24/7, aber kleiner und Free-Tier-nah: ca. 30-85 USD/Monat je nach Redis-Entscheidung

Mit VPS:

- Spaceship Development: ca. 12-25 USD/Monat
- Spaceship kleine Produktion: ca. 30-60 USD/Monat, aber US/Singapore statt EU und komplett selbst betrieben
- Hetzner Development: ca. 10-25 EUR/Monat
- Hetzner kleine Produktion: ca. 25-80 EUR/Monat, je nach Backup-/Redundanzmodell

Empfehlung fuer die naechsten zwei Monate: Nicht sofort alles auf VPS migrieren, sondern Google Cloud fuer Development stark herunterskalieren. Die groessten Soforthebel sind Cloud Build, Cloud SQL, Redis/VPC und Cloud Run CPU Billing.

## Umgesetzte Development-Reduktion am 22. Mai 2026

Nach der Analyse wurden folgende Kostenbremsen direkt umgesetzt:

- Cloud-Build-GitHub-Trigger `2dd37a2f-2d10-4345-a367-b9cc5dd25959` geloescht, damit Pushes auf `main` keine automatischen High-CPU-Builds mehr starten.
- `cloudbuild.yaml` lokal auf `E2_STANDARD_2` geaendert, damit ein spaeter neu aktivierter Cloud-Build-Pfad in die 2.500-Free-Tier-Minuten-Spur faellt.
- Cloud Run `poool-backend` auf Development-Billing gestellt:
  - `min-instances=0`
  - `max-instances=1`
  - `cpu-throttling=true`
- Ungenutzten Redis `poool-redis` in `asia-southeast1` geloescht.
- Cloud SQL On-Demand-Backup vor der Verkleinerung erstellt: Backup ID `1779467554230`.
- Cloud SQL `poool-db` von `db-custom-2-7680` auf `db-g1-small` verkleinert.
- Artifact Registry Cleanup Policy fuer `cloud-run-source-deploy` aktiviert:
  - loescht Dev-Images aelter als 14 Tage
  - behaelt die letzten 5 Versionen

Neue erwartete Development-Kosten:

- ca. 80-95 USD/Monat, solange der aktive Redis in `europe-west1` und der VPC Connector bleiben.
- ca. 160-190 USD fuer die naechsten zwei Monate.
- Gegenueber dem vorherigen Zustand von ca. 269-284 USD/Monat entspricht das einer erwarteten Reduktion von ca. 65-70%.

Noch nicht umgesetzt:

- Aktiven Redis in `europe-west1` entfernen.
- VPC Connector entfernen.
- Cloud SQL auf `db-f1-micro` weiter verkleinern oder nur bei Bedarf starten.

Diese letzten Schritte wuerden auf ca. 15-30 USD/Monat zielen, benoetigen aber technische Anpassungen oder bewusst akzeptierte Funktions-Einschraenkungen in der Dev-Umgebung.

### Nachpruefung am 22. Mai 2026, 18:45 Berlin

Live verifiziert:

- Cloud Run `poool-backend` ist auf Revision `poool-backend-00435-5fn`.
- Cloud Run hat `autoscaling.knative.dev/maxScale=1`.
- Cloud Run hat `run.googleapis.com/cpu-throttling=true`.
- Cloud SQL `poool-db` ist `RUNNABLE` und laeuft auf `db-g1-small`.
- Cloud-Build-Triggerliste ist leer.
- Ongoing Cloud Builds: keine.
- Redis `asia-southeast1`: geloescht.
- Redis `europe-west1`: bleibt aktiv, 1 GiB Basic.
- VPC Connector `poool-connector`: bleibt aktiv, weil Redis privat angebunden ist.
- Compute Engine Instanzen, Disks, reservierte IPs: keine.
- GKE Cluster: keine.
- Artifact Registry Cleanup Policy ist aktiv; der Cleanup laeuft asynchron und kann laut Google bis etwa einen Tag brauchen.

Neue realistische Monatsbasis ab jetzt:

- Cloud SQL `db-g1-small`: ca. 27 USD/Monat.
- Redis `europe-west1`: ca. 36 USD/Monat.
- VPC Connector: ca. 10-15 USD/Monat.
- Cloud Run: voraussichtlich 0-5 USD/Monat bei aktueller Dev-Nutzung, weil Request-Zahl unter Free Tier liegt und CPU nicht mehr dauerhaft allokiert ist.
- Cloud Build: 0 USD, solange keine manuellen Builds laufen; bei spaeterer Reaktivierung mit `E2_STANDARD_2` voraussichtlich Free-Tier-nah.
- Artifact Registry/Storage/Logs/Misc: ca. 5-13 USD/Monat, sinkend nach Cleanup.

Damit liegt die aktive erwartete Monatsbasis bei ca. 77-96 USD/Monat, also grob 2,0-2,5 Mio. VND/Monat. Die Billing-Oberflaeche kann 24-48 Stunden nachlaufen; neue Tagesbalken sollten aber ab dem 23. Mai 2026 sichtbar niedriger sein.

Noch guenstiger waere:

- Redis `europe-west1` loeschen und Cloud Run ohne VPC Connector betreiben: spart ca. 45-50 USD/Monat, deaktiviert laut Code aber Marketplace Trading/Matching und Redis-gestuetzte Admin-/Marketplace-Funktionen.
- Cloud SQL weiter auf `db-f1-micro` verkleinern: spart ca. 18 USD/Monat, ist aber bei 614 MiB RAM deutlich bruchanfaelliger.

## Ausgangsdaten aus dem Projekt

### Billing-Screenshot

Screenshot-Zeitraum: 1.-22. Mai 2026

- Bruttokosten: 5,54 Mio. VND
- Ersparnisse/Credits: 4,83 Mio. VND
- Nettokosten: 701.527 VND
- Prognose im Screenshot: 383.146 VND, vermutlich unter Annahme vorhandener Credits/Rabatte

Interpretation:

Die sichtbare Rechnung ist noch stark durch Credits/Rabatte reduziert. Die Bruttokosten von 5,54 Mio. VND fuer 22 Tage entsprechen hochgerechnet ca. 7,55 Mio. VND pro 30-Tage-Monat. Das passt zu meiner rechnerischen Hochrechnung von ca. 7,0-7,4 Mio. VND/Monat.

### Trial-Credits

Google beschreibt den Free Trial als 300 USD Welcome Credit fuer 90 Tage. Nach Ablauf der 90 Tage oder nach Verbrauch der 300 USD wird regulaer abgerechnet. Die monatlichen Free-Tier-Kontingente laufen danach weiter, aber nur fuer Produkte mit Free Tier.

Ich konnte per `gcloud` bestaetigen, dass das Billing-Konto offen ist und in VND abrechnet. Den exakten verbleibenden Trial-Credit-Saldo stellt `gcloud billing accounts describe` nicht bereit. Aus dem Screenshot ist aber klar: Im Mai wurden bereits 4,83 Mio. VND Credits/Rabatte gegen die Kosten gerechnet. Das sind grob 185 USD. Wenn die urspruenglichen 300 USD Trial-Credits die Quelle sind, ist der Spielraum klein oder bereits fast weg.

## Live-Ressourcen und Nutzung

### Cloud Run

Service: `poool-backend`  
Region: `europe-west1`  
Image: Artifact Registry / `cloud-run-source-deploy`  
CPU/RAM: 1 vCPU, 512 MiB  
Concurrency: 80  
Timeout: 300s  
Min instances: effektiv 0  
Max scale: Service-Annotation 3, Revision-Annotation 10  
CPU Billing: `run.googleapis.com/cpu-throttling=false`

Metriken 1.-22. Mai 2026:

- Requests: 221.373 gesamt
- Durchschnitt Requests: ca. 10.062/Tag
- Hochrechnung: ca. 302.000 Requests/Monat
- Billable instance time: 1.741.480 Sekunden in 22 Tagen
- Durchschnitt billable time: ca. 79.158 Sekunden/Tag
- Hochrechnung billable time: ca. 2.374.745 Sekunden/Monat

Bewertung:

Die Request-Menge liegt sehr deutlich unter Cloud Runs Free Tier von 2 Mio. Requests/Monat. Das Problem ist nicht die Request-Anzahl, sondern dass die Instanz wegen `cpu-throttling=false` fast dauerhaft billable ist.

Wichtig: Der Rust-Backend-Code enthaelt Hintergrundjobs. Wenn Cloud Run auf request-based CPU umgestellt wird, koennen diese Hintergrundjobs bei Inaktivitaet pausieren oder verzögert laufen. Fuer Development ist das wahrscheinlich akzeptabel; fuer Produktion sollte man die Hintergrundjobs sauber in Cloud Run Jobs + Scheduler auslagern.

### Cloud SQL

Instanz: `poool-db`  
Engine: PostgreSQL 15  
Region/Zone: `europe-west1-d`  
Tier: `db-custom-2-7680`  
vCPU/RAM: 2 vCPU, 7,5 GiB RAM  
Storage: 10 GB SSD, auto-resize aktiv  
Availability: Zonal  
Backups: deaktiviert  
Public IPv4: aktiviert  
Query Insights: aktiviert  
Activation policy: `ALWAYS`

Metriken 1.-22. Mai 2026:

- CPU-Tagesmittel: ca. 12,1%
- Max. CPU-Tagesmittel: ca. 13,9%
- Memory-Tagesmittel: ca. 69%
- Max. Disk used: ca. 98 MiB
- Max. Postgres Backends/Connections: 84
- Durchschnitt der Tagesmaxima Connections: ca. 41

Bewertung:

Fuer Development ist diese Instanz massiv ueberdimensioniert. Die CPU ist niedrig, der Datensatz winzig. Der Memory-Wert ist bei PostgreSQL nicht automatisch ein Problem, weil Datenbank-Caches Speicher nutzen, aber 7,5 GiB fuer eine 100-MiB-Dev-Datenbank sind nicht wirtschaftlich.

### Cloud Build

Trigger: GitHub Push auf `^main$`, Datei `cloudbuild.yaml`  
Build machine: `E2_HIGHCPU_8` laut `cloudbuild.yaml`

Metriken seit 1. Mai 2026:

- Builds gesamt: 203
- Erfolgreich: 161
- Fehlgeschlagen: 42
- Build-Minuten gesamt: 1.841,75
- `E2_HIGHCPU_8`-Minuten: 1.705,15
- Durchschnitt: ca. 9,07 Minuten/Build
- Hochrechnung aktuelles Tempo: ca. 2.511 Build-Minuten/Monat, davon ca. 2.325 `E2_HIGHCPU_8`-Minuten/Monat

Bewertung:

Das ist fuer Development ein grosser vermeidbarer Kostentreiber. Google gibt 2.500 Free-Tier-Build-Minuten/Monat nur fuer `e2-standard-2` im Default Pool. Eure Builds laufen auf `e2-highcpu-8`, also an der kostenlosen Standardspur vorbei.

### Redis / Memorystore

Aktive/erkannte Redis-Instanzen:

1. `poool-redis`, `europe-west1`
   - Basic
   - 1 GiB
   - Redis 7.0
   - Auth aktiv
   - TLS/Server Auth aktiv
   - Wird vom Secret `poool-redis-url` referenziert und ist damit die aktive App-Instanz.

2. `poool-redis`, `asia-southeast1`
   - Basic
   - 1 GiB
   - Redis 7.2
   - Kein Transit Encryption
   - Wirkt ungenutzt fuer die aktuelle POOOL-App.

Bewertung:

Fuer Development ist eine Managed Redis Instanz schon teuer, zwei davon sind klar unnoetig. Die Asia-Instanz sollte geloescht werden, wenn sie nicht bewusst fuer ein anderes Projekt gebraucht wird.

### Serverless VPC Access

Connector: `poool-connector`  
Region: `europe-west1`  
Machine type: `e2-micro`  
Min instances: 2  
Max instances: 3  
Zweck: Private Verbindung von Cloud Run zu Redis

Bewertung:

Dieser Connector ist ein dauerhafter Nebenkostentraeger. Er ist noetig, solange Cloud Run auf den privaten Memorystore Redis zugreifen muss. Wenn Redis fuer Development entfernt oder anders geloest wird, kann auch der Connector weg.

### Artifact Registry

Repositories in `europe-west1`:

- `cloud-run-source-deploy`: ca. 55,14 GB
- `trafficgen`: ca. 1,16 GB
- `poool-repo`: vorhanden, keine relevante Groesse angezeigt

Bewertung:

Artifact Registry Free Tier ist 0,5 GB. Alles darueber kostet. Bei ca. 56,3 GB sind das rechnerisch ca. 5,58 USD/Monat. Nicht der groesste Posten, aber leicht zu optimieren per Cleanup Policy.

## Current Google Cloud Kosten: aktueller Zustand

Diese Hochrechnung nutzt die aktuelle Konfiguration und die Nutzung vom 1.-22. Mai 2026.

| Komponente | Annahme | Monatlich USD | Monatlich VND ca. |
|---|---:|---:|---:|
| Cloud SQL `db-custom-2-7680` | 2 vCPU + 7,5 GiB + 10 GB SSD, 730h | 100,32 | 2.608.000 |
| Cloud Run aktuell | Instanzbasierte Abrechnung, Free Tier grob abgezogen | 39,90 | 1.037.000 |
| Cloud Build aktuell | `E2_HIGHCPU_8`, 2.325 Min/Monat | 36,27 | 943.000 |
| Redis Europe | 1 GiB Basic | 35,77 | 930.000 |
| Redis Asia | 1 GiB Basic, vermutlich ungenutzt | 35,77 | 930.000 |
| VPC Connector | 2x `e2-micro`, geschaetzt | 10-15 | 260.000-390.000 |
| Artifact Registry | ca. 56,3 GB - 0,5 GB free | 5,58 | 145.000 |
| Sonstiges | Storage, Logs, Secrets, Monitoring, kleine Nebenkosten | 5-10 | 130.000-260.000 |
| **Summe** |  | **269-284** | **7.000.000-7.400.000** |

Zwei Monate Development im aktuellen Zustand:

- Ohne weitere Credits: ca. 540-570 USD
- In VND: ca. 14,0-14,8 Mio. VND
- In EUR grob: ca. 500-530 EUR, je nach Wechselkurs

Diese Zahl passt zur Screenshot-Bruttohochrechnung von ca. 7,55 Mio. VND/Monat.

## Google Cloud Free Tier: was funktioniert und was nicht

| Dienst | Free Tier | Funktioniert fuer euch? | Kommentar |
|---|---|---|---|
| Cloud Run Requests | 2 Mio. Requests/Monat | Ja | Aktuell hochgerechnet nur ca. 302.000 Requests/Monat. |
| Cloud Run CPU/RAM request-based | 180.000 vCPU-s + 360.000 GiB-s/Monat | Wahrscheinlich ja | Aber nur, wenn `cpu-throttling=true` und keine dauerhafte CPU allokiert wird. |
| Cloud Run instance-based | 240.000 vCPU-s + 450.000 GiB-s/Monat | Nein | Eure aktuelle Hochrechnung liegt bei ca. 2,37 Mio. Instanzsekunden/Monat. |
| Cloud Build | 2.500 Minuten/Monat fuer `e2-standard-2` | Ja, wenn umgestellt | Aktuell nutzt ihr `E2_HIGHCPU_8`, das ist der Fehler. |
| Cloud SQL | Kein sinnvoller Always-Free fuer Cloud SQL | Nein | Cloud SQL kostet, solange die Instanz laeuft. |
| Memorystore Redis | Kein Free Tier | Nein | Kostet 24/7 pro GiB. |
| VPC Access Connector | Kein echter Free Tier | Nein | Kostet, solange Instanzen reserviert sind. |
| Artifact Registry | 0,5 GB Storage | Teilweise | Ihr habt ca. 56,3 GB. |
| Cloud Storage | 5 GB nur in US-Regionen | Nur bedingt | Aktuelle Buckets/Regionen sind nicht darauf optimiert. |

Fazit: Free Tier kann Cloud Run und Cloud Build fast komplett abdecken. Die aktuelle Rechnung kommt aber von Diensten, die fuer Free Tier ungeeignet sind: Cloud SQL, Redis, VPC Connector und High-CPU-Builds.

## Google Cloud optimierte Development-Szenarien

### Szenario GCP-D0: Nichts aendern

Beschreibung:

- Alles bleibt wie jetzt.
- Cloud SQL gross.
- Zwei Redis-Instanzen.
- VPC Connector bleibt.
- Cloud Run CPU dauerhaft.
- High-CPU-Builds bleiben.

Kosten:

- 269-284 USD/Monat
- 538-568 USD fuer zwei Monate
- ca. 14,0-14,8 Mio. VND fuer zwei Monate

Bewertung:

Nicht sinnvoll fuer Development.

### Szenario GCP-D1: Schnelle, risikoarme Optimierung

Aenderungen:

- Cloud Build auf `e2-standard-2` oder Auto-Trigger deaktivieren.
- Artifact Registry Cleanup Policy setzen.
- Asia Redis loeschen, wenn nicht anderweitig gebraucht.
- Cloud Run auf request-based CPU umstellen, wenn Background-Job-Verzoegerung in Dev akzeptabel ist.
- Cloud SQL erstmal unveraendert lassen.

Kosten:

| Komponente | Monatlich USD ca. |
|---|---:|
| Cloud SQL current | 100 |
| Cloud Run request-based | 0-5 |
| Cloud Build Free Tier | 0-5 |
| Redis Europe | 36 |
| VPC Connector | 10-15 |
| Artifact/Storage/Logs | 5-10 |
| **Summe** | **151-171** |

Zwei Monate:

- 302-342 USD
- ca. 7,9-8,9 Mio. VND

Bewertung:

Spart ca. 40% gegenueber heute, aber Cloud SQL bleibt der groesste Block.

### Szenario GCP-D2: Sinnvolle Dev-Optimierung mit Managed Services

Aenderungen:

- Cloud SQL verkleinern, z.B. `db-g1-small` oder konservativer `db-custom-1-3840`.
- Cloud Run request-based.
- Build auf `e2-standard-2`.
- Asia Redis weg.
- Artifact Cleanup.
- Europe Redis bleibt.
- VPC Connector bleibt, weil Redis privat bleibt.

Kosten mit `db-g1-small`:

| Komponente | Monatlich USD ca. |
|---|---:|
| Cloud SQL `db-g1-small` + 10 GB SSD | 27 |
| Cloud Run request-based | 0-5 |
| Cloud Build Free Tier | 0-5 |
| Redis Europe | 36 |
| VPC Connector | 10-15 |
| Artifact/Storage/Logs | 3-7 |
| **Summe** | **76-95** |

Zwei Monate:

- 152-190 USD
- ca. 4,0-4,9 Mio. VND

Kosten mit `db-custom-1-3840` statt `db-g1-small`:

- Cloud SQL: ca. 51 USD/Monat
- Gesamt: ca. 100-120 USD/Monat

Bewertung:

Das ist mein bevorzugter Google-Cloud-Dev-Plan, wenn Redis im Backend weiter genutzt werden soll.

### Szenario GCP-D3: Aggressive Dev-Minimierung

Aenderungen:

- Asia Redis loeschen.
- Europe Redis fuer Dev entfernen oder durch lokalen/in-App-Fallback ersetzen.
- VPC Connector entfernen.
- Cloud SQL sehr klein (`db-f1-micro`) oder nur bei Bedarf starten.
- Cloud Run request-based.
- Builds auf `e2-standard-2`.
- Artifact Cleanup.

Kosten:

| Komponente | Monatlich USD ca. |
|---|---:|
| Cloud SQL `db-f1-micro` + 10 GB SSD | 9-12 |
| Cloud Run request-based | 0-3 |
| Cloud Build Free Tier | 0-5 |
| Redis | 0 |
| VPC Connector | 0 |
| Artifact/Storage/Logs | 3-10 |
| **Summe** | **15-30** |

Zwei Monate:

- 30-60 USD
- ca. 780.000-1,56 Mio. VND

Risiken:

- `db-f1-micro` ist fuer ernsthafte Produktion ungeeignet.
- Ohne Redis muss die App im Development-Modus sauber degradieren.
- Background-Worker und Marketplace-Features muessen fuer Dev eventuell entschärft werden.

Bewertung:

Gueltig, wenn in den naechsten zwei Monaten wirklich keine Nutzer produktiv auf die Plattform gehen.

## Google Cloud Produktionsszenarien

### GCP-P1: Kleine Launch-Produktion, kostenbewusst

Annahmen:

- Cloud Run request-based, min instances 0 oder 1 nur wenn noetig.
- Backgroundjobs ausgelagert in Cloud Run Jobs + Scheduler.
- Cloud SQL `db-custom-1-3840` oder aehnlich.
- Backups/PITR aktiv.
- Ein Redis 1 GiB.
- VPC Connector bleibt.
- Artifact Cleanup aktiv.

Kosten:

- Cloud SQL: 51-60 USD/Monat
- Cloud Run: 0-20 USD/Monat bei niedrigem Traffic
- Redis: 36 USD/Monat
- VPC Connector: 10-15 USD/Monat
- Logs/Storage/Artifact/Secrets: 5-20 USD/Monat
- Build: 0-10 USD/Monat

Summe:

- ca. 105-145 USD/Monat
- ca. 2,7-3,8 Mio. VND/Monat

Bewertung:

Guter Startpunkt fuer echte kleine Produktion mit kontrollierten Kosten.

### GCP-P2: Aktueller produktionsaehnlicher Betrieb

Annahmen:

- Cloud SQL bleibt `db-custom-2-7680`.
- Ein Redis bleibt.
- VPC Connector bleibt.
- Cloud Run bleibt eher dauerhaft CPU-allokiert.
- Build-Pipeline optimiert oder zumindest kontrolliert.

Kosten:

- ca. 190-260 USD/Monat
- ca. 4,9-6,8 Mio. VND/Monat

Bewertung:

Technisch bequem, aber fuer Early Stage zu teuer, solange kaum Nutzer existieren.

### GCP-P3: Robuste Produktion mit HA/Redundanz

Annahmen:

- Cloud SQL HA oder mindestens staerkerer Tier mit Backups/PITR.
- Redis Standard/HA.
- Cloud Run min instances fuer geringe Latenz.
- Monitoring/Logging/Budget Alerts/Security sauber aktiv.
- Optional Load Balancer/CDN/WAF.

Kosten:

- ca. 350-550+ USD/Monat
- ca. 9,1-14,3 Mio. VND/Monat

Bewertung:

Erst sinnvoll, wenn echte Nutzer, Umsatz, SLA-Anforderungen und Compliance-Druck da sind.

## VPS-/Spaceship-Alternative

### Technischer Zielzustand auf VPS

Ein einzelner VPS wuerde typischerweise laufen:

- Rust/Axum Backend als systemd service oder Docker container
- PostgreSQL lokal
- Redis lokal
- Caddy oder Nginx fuer TLS/Reverse Proxy
- lokale Uploads oder S3-kompatibler Object Storage
- Restic/Borg/pg_dump Backups auf externen Speicher
- Uptime Monitoring
- Logrotation
- Firewall, SSH Hardening, unattended security updates

Vorteil:

- Viel guenstiger.
- Fixe, vorhersehbare Kosten.
- Fuer Development sehr gut.

Nachteil:

- Kein Managed Cloud SQL.
- Kein automatisches Failover.
- Backup/Restore/Monitoring/Security liegen komplett bei euch.
- Deployments sind selbst zu bauen.
- Produktion auf einem Einzelserver ist ein Single Point of Failure.

### Spaceship

Offizielle Spaceship-Starlight-VM-Preise:

- Standard 1: 1 Core, 2 GiB RAM, 25 GiB NVMe, 1 TiB Transfer, 4,90 USD/Monat
- Standard 2: 2 Cores, 4 GiB RAM, 60 GiB NVMe, 2 TiB Transfer, 11,90 USD/Monat
- Standard 3: 4 Cores, 8 GiB RAM, 160 GiB NVMe, 4 TiB Transfer, 25,89 USD/Monat
- Memory-optimized 1: 2 Cores, 8 GiB RAM, 50 GiB NVMe, 4 TiB Transfer, 22,89 USD/Monat

Wichtiger Standort-Hinweis:

Spaceship nennt aktuell Phoenix, USA und Singapore fuer Virtual Machines. Fuer europaeische Nutzer und moegliche Finanz-/KYC-Daten ist das gegenueber Google `europe-west1` oder Hetzner Deutschland ein relevanter Nachteil.

Spaceship Development:

- Minimal: Standard 1, 4,90 USD/Monat, aber 2 GiB RAM ist knapp fuer Postgres + Redis + App + Builds.
- Realistisch: Standard 2, 11,90 USD/Monat.
- Mit externem Backup/Storage/Monitoring: ca. 15-25 USD/Monat.
- Zwei Monate: ca. 30-50 USD.

Spaceship Produktion klein:

- Realistisch: Standard 3 oder Memory-optimized 1.
- Server: 22,89-25,89 USD/Monat.
- Backup/Storage/Monitoring: 5-20 USD/Monat.
- Summe Single VPS: ca. 30-50 USD/Monat.
- Stabilere Variante mit separatem DB/Backup/zweitem Server: ca. 60-120 USD/Monat.

Bewertung:

Sehr gut fuer billige Development-Umgebung. Fuer Produktion wegen Standort und Self-Managed-Ops nur sinnvoll, wenn bewusst ein Low-Cost-Single-Server-Betrieb akzeptiert wird.

### Hetzner / EU-VPS als bessere VPS-Alternative

Hetzner ist fuer POOOL naheliegender als Spaceship, weil EU/Deutschland/Finnland verfuegbar sind. Laut Hetzner-Preisadjustment ab 1. April 2026 liegen typische Cloud-Produkte nach offizieller Dokumentation grob in diesem Bereich:

- CAX11: ca. 5,34 EUR/Monat inkl. 19% MwSt.
- CAX21: ca. 9,51 EUR/Monat inkl. 19% MwSt.
- CAX31: ca. 19,03 EUR/Monat inkl. 19% MwSt.
- CCX13 Dedicated CPU: ca. 19,03 EUR/Monat inkl. 19% MwSt.

Hetzner Development:

- CAX21 oder aehnlich: ca. 10 EUR/Monat.
- Mit Backups/Storage: ca. 12-20 EUR/Monat.
- Zwei Monate: ca. 24-40 EUR.

Hetzner kleine Produktion:

- CAX31/CCX13: ca. 19-40 EUR/Monat je nach CPU/RAM/Architektur.
- Backups/Storage/Monitoring: 5-20 EUR/Monat.
- Single VPS: ca. 25-60 EUR/Monat.
- Redundanter Betrieb: ca. 60-150 EUR/Monat.

Bewertung:

Wenn VPS, dann fuer POOOL wahrscheinlich Hetzner vor Spaceship, weil EU-Standort, Datenschutz/Latency und sehr gutes Preis-Leistungs-Verhaeltnis. Spaceship ist billiger/okay fuer Dev, aber geografisch weniger passend.

## Direkter Vergleich

### Development: naechste zwei Monate

| Option | Monatlich | 2 Monate | Bewertung |
|---|---:|---:|---|
| GCP aktuell | 269-284 USD | 538-568 USD | Zu teuer fuer Dev |
| GCP D1, schnelle Optimierung | 151-171 USD | 302-342 USD | Besser, aber Cloud SQL bleibt teuer |
| GCP D2, sinnvoll managed | 76-120 USD | 152-240 USD | Guter Kompromiss |
| GCP D3, aggressive Dev-Minimierung | 15-30 USD | 30-60 USD | Billig, braucht technische Anpassung |
| Spaceship Dev | 15-25 USD | 30-50 USD | Sehr billig, aber US/Singapore |
| Hetzner Dev | 12-25 EUR | 24-50 EUR | Bester VPS-Dev-Kandidat |

### Produktion spaeter

| Option | Monatlich | Geeignet fuer |
|---|---:|---|
| GCP P1 Lean Launch | 105-145 USD | Erste echte Nutzer, managed DB, kontrollierte Kosten |
| GCP P2 Produktiv komfortabel | 190-260 USD | Mehr Sicherheit/Komfort, aber noch ohne HA |
| GCP P3 robust/HA | 350-550+ USD | Ernsthafte Produktion mit SLA/Redundanz |
| Spaceship Single VPS | 30-50 USD | Low-cost, selbst betrieben, nicht ideal fuer EU/Finanzdaten |
| Hetzner Single VPS | 25-60 EUR | Low-cost EU, gut fuer kleine Produktion mit Ops-Disziplin |
| Hetzner redundant | 60-150 EUR | Preiswerter Produktionsbetrieb, aber weiterhin selbst verwaltet |

## Empfehlung

### Fuer die naechsten zwei Monate Development

Ich wuerde diese Reihenfolge nehmen:

1. Cloud Build sofort auf Free-Tier-nahe Nutzung bringen:
   - `E2_HIGHCPU_8` aus `cloudbuild.yaml` entfernen oder auf `E2_STANDARD_2` umstellen.
   - Auto-Trigger auf `main` deaktivieren oder loeschen/reversibel neu anlegen.

2. Unbenutzte Redis-Instanz in `asia-southeast1` loeschen:
   - spart ca. 36 USD/Monat.

3. Cloud Run request-based stellen:
   - spart ca. 35-40 USD/Monat.
   - fuer Dev akzeptabel, fuer Produktion vorher Backgroundjobs auslagern.

4. Cloud SQL verkleinern:
   - fuer Dev mindestens auf `db-g1-small`, wenn managed bleiben soll.
   - fuer aggressive Kostenreduktion `db-f1-micro` oder DB nur bei Bedarf starten.

5. Redis/VPC fuer Dev pruefen:
   - Wenn Redis im Dev nicht zwingend ist: Redis Europe entfernen, VPC Connector entfernen.
   - Das ist der Schritt, der GCP von ca. 75-90 USD/Monat auf ca. 15-30 USD/Monat bringen kann.

6. Artifact Registry Cleanup:
   - Docker-Images auf letzte 5-10 Versionen begrenzen.
   - spart nur ca. 5 USD/Monat, verhindert aber weiteres Wachstum.

### Migration zu VPS?

Fuer reine Development-Zeit ist ein VPS klar billiger. Die Frage ist nicht Preis, sondern Zeit und Risiko.

Wenn in den naechsten zwei Monaten viel deployed, ausprobiert und umgebaut wird, waere ein Hetzner- oder Spaceship-VPS fuer 12-25 USD/EUR monatlich die billigste Umgebung. Aber ihr muesst dann Deployment, Datenbank, Redis, Backups, TLS, Monitoring und Security selbst betreiben.

Mein pragmatischer Vorschlag:

- Kurzfristig: GCP auf D2 oder D3 optimieren, weil die Infrastruktur schon steht.
- Parallel: VPS-Deployment als Dev/Staging-Ziel vorbereiten.
- Produktion spaeter nicht blind auf Single VPS starten, wenn echte Finanz-/KYC-/Payment-Daten verarbeitet werden. Fuer Produktion ist GCP P1 oder ein gut administrierter Hetzner-Setup mit externen Backups realistischer.

## Quellen

- Google Cloud Free Trial und Free Tier: https://docs.cloud.google.com/free/docs/free-cloud-features
- Cloud Run Pricing: https://cloud.google.com/run/pricing
- Cloud Build Pricing: https://cloud.google.com/build/pricing
- Cloud SQL Pricing: https://cloud.google.com/sql/pricing
- Memorystore Redis Pricing: https://cloud.google.com/memorystore/docs/redis/pricing
- Serverless VPC Access / VPC Pricing: https://cloud.google.com/vpc/network-pricing
- Artifact Registry Pricing: https://cloud.google.com/artifact-registry/pricing
- Spaceship Starlight Virtual Machines: https://www.spaceship.com/hosting/virtual-machines/
- Hetzner Price Adjustment / Cloud Pricing: https://docs.hetzner.com/general/infrastructure-and-availability/price-adjustment/
