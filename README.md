# Lagemonitor

Lagebild-Anzeige für den Server im Führungskraftwagen: Wetter, amtliche Warnungen und Karte als Widgets, die nach Relevanz Platz bekommen. Läuft als Kiosk auf dem Server-Bildschirm ohne Anmeldung und ist zusätzlich als Webapp erreichbar (von außen nur mit Anmeldung über Pocket ID).

**Stand:** v1 (Karte, DWD-Wetter, NINA-Warnungen). Design: `docs/superpowers/specs/2026-10-27-lagemonitor-design.md`.

## Installation und Nutzung

Die CI baut das Image `ghcr.io/rubenvitt/lagemonitor`. Der Server zieht es nur, er baut nichts. Schritt für Schritt: [`deploy/README.md`](deploy/README.md). Kiosk-Bildschirm: [`deploy/kiosk/README.md`](deploy/kiosk/README.md).

Standort, Landkreis und Warnschwelle stehen in der Tabelle `settings` der SQLite-Datei `data/lagemonitor.db`. Bis es eine Verwaltung gibt, ändert man sie mit `sqlite3` (Beispiel in `deploy/README.md`, Schritt 9).

## Entwicklung

`mise run dev` startet den Server, `mise run check` ist das Qualitäts-Gate, `mise run image` baut das Image lokal.
