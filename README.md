# Lagemonitor

Lagebild-Anzeige für den Server im Führungskraftwagen: Wetter, amtliche Warnungen und Karte als Widgets, die nach Relevanz Platz bekommen. Läuft als Kiosk auf dem Server-Bildschirm ohne Anmeldung und ein zweites Mal online neben der IUK-Suite (`https://lagemonitor.iuk-ue.de/monitor/`), dort nur mit Anmeldung über Pocket ID oder mit einem Link aus der Suite.

**Stand:** v1 (Karte, DWD-Wetter, NINA-Warnungen). Design: `docs/superpowers/specs/2026-10-27-lagemonitor-design.md`.

## Installation und Nutzung

Die CI baut das Image `ghcr.io/rubenvitt/lagemonitor` für amd64 und arm64. Der Server zieht es nur, er baut nichts. FüKw Schritt für Schritt: [`deploy/README.md`](deploy/README.md). Kiosk-Bildschirm: [`deploy/kiosk/README.md`](deploy/kiosk/README.md). Online-Instanz: [`deploy/cloud/README.md`](deploy/cloud/README.md).

Verwaltet wird der Monitor in der IUK-Suite (Modul „Lagemonitor“): Standort, Landkreis, Warnschwelle, Module und die Verbindung zu Suchlage. Der Server holt die Einstellungen jede Minute ab (`LAGEMONITOR_SUITE_URL`, `LAGEMONITOR_SUITE_TOKEN`) und hält den letzten Stand in der Tabelle `settings`, damit er ohne Internet weiterläuft (`deploy/README.md`, Schritt 9).

Läuft in Suchlage ein Einsatz, zeigt die Karte seine Suchgebiete, die Flugspuren aller Flüge und die Drohnen; meldet Suchlage ein Livebild, fährt es von rechts ein. Der Server fragt Suchlage mit einem Großbild-Zugang (`/api/lagemonitor/v1`) und reicht das Video durch, der Kiosk-Browser sieht Suchlage nie.

Verkehrslage (in der Suite einzeln schaltbar): Verkehrsfluss und Baustellen/Sperrungen von TomTom, dazu feste Blitzer aus OpenStreetMap, die nur die Online-Instanz zeigt. Den TomTom-Schlüssel trägt man in der Suite ein; der Server holt alles selbst und speichert es lange, der Browser sieht den Schlüssel nie. Das Gratiskontingent (2.500 Störungsabfragen und 200.000 Kacheln im Monat) teilen sich FüKw und Online-Instanz: Störungen stündlich, jede Flusskachel höchstens alle 30 Minuten und je Instanz höchstens 3.000 Kacheln am Tag (`src/web/traffic.rs`); Blitzer einmal am Tag.

## Entwicklung

`mise run dev` startet den Server, `mise run check` ist das Qualitäts-Gate, `mise run image` baut das Image lokal.
