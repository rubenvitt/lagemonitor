# Lagemonitor – Design

- **Stand:** 27.10.2026, aus dem Brainstorming freigegebene Fassung (Entwurf)
- **Projekt:** `lagemonitor`, Repo `rubenvitt/lagemonitor` (privat), lokal `~/dev/personal/drk/lagemonitor`
- **Verwandt:** Suchlage (`~/dev/personal/drk/suchlage`), Lifeline Hub (`~/dev/personal/lifeline-hub`), iuk-suite (`~/dev/personal/drk/iuk-suite`), LLH-1484

## 1. Wozu

Der FüKw 40-12-1 der DRK-Bereitschaft bekommt einen eigenen Server (Ubuntu Server LTS). Sein angeschlossener Bildschirm soll ohne Anmeldung ein Lagebild zeigen: den **Lagemonitor**. Er ist eine eigenständige App mit konfigurierbaren Widgets, die nach Relevanz Platz bekommen. Dieselbe Ansicht ist als Webapp erreichbar, im FüKw-Netz ohne Anmeldung, von außen nur mit Anmeldung.

Warum jetzt und ohne Lifeline Hub: Lifeline Hub (LLH) ist in früher Alpha. Der Monitor soll schon vorher nützlich sein, im Einsatz, im Übungsdienst und beim Tag der offenen Tür. Die Anbindung an LLH kommt später (LLH-1484).

**Erfolg v1:** Der Bildschirm im FüKw zeigt nach dem Einschalten ohne Zutun Wetter und Karte. Liegt eine amtliche Warnung für den Landkreis vor, sieht man sie sofort.

## 2. Ziele, Nicht-Ziele, Anti-Ziele

**Ziele v1**
- Kiosk-Anzeige auf dem Server-Bildschirm, startet beim Booten, keine Anmeldung
- Widgets: **Wetter** (aktuell und Kurzvorhersage), **Warnungen** (NINA/DWD für den Landkreis), **Karte** (Standort, Warngebiete, Regenradar)
- Relevanzlogik: Warnungen rücken nach vorn, solange sie gelten
- Drei Zustände wie in den Entwürfen: Ruhe, Warnung aktiv, Schaufenster (Tag der offenen Tür)
- Admin-UI: Standort setzen, Widgets pinnen, ausblenden und sortieren, Widget-Parameter (Region, Warnschwelle), Schaufenster an und aus, Ansicht erzwingen
- Designsystem „Instrumententafel“ aus Lifeline Hub, wie in Suchlage übernommen

**Später (nicht v1, aber Architektur darf es nicht verbauen)**
- Lagemeldung als Freitext (nur sichtbar, wenn eingetragen)
- Suchlage-Widget: UAV-Livebild und Sektoren, rückt bei aktivem Flug nach vorn
- LLH-Widgets: Lagekarte mit Einsatzzeichen, Kräfte, Betroffene, ETB-Auszug (LLH-1484)
- Standort automatisch per GPS des FüKw-Routers (Modell noch unbekannt)
- Kräfte von Hand gepflegt, bis LLH sie liefert

**Nicht-Ziele**
- Kein Regel-Editor. Wann ein Widget relevant ist, steckt im Widget-Code. Konfigurierbar sind nur Reihenfolge, Pinnen, Ausblenden, Parameter und das Erzwingen einer Ansicht.
- Kein Karussell und kein zeitgesteuertes Umblättern
- Keine Fireboard-Anbindung: Die Fireboard Suite bietet keine Live-Leseschnittstelle, nur XML/JSON-Export. Eine Anfrage beim Hersteller ist offen.
- Keine aPager/Alamos-Anbindung: Die API kann nur Rückmeldungen schreiben, nicht lesen.
- SSH und die allgemeine Server-Einrichtung gehören nicht zum Projekt. Die Kiosk-Einrichtung schon.

**Anti-Ziele (wäre ein Fehlschlag, auch wenn es technisch läuft)**
- Der Monitor zeigt veraltete Daten, ohne dass man es merkt
- Wer kurz hochschaut, trifft auf die falsche Seite, weil gerade umgeblättert wird
- Ohne Internet bleibt der Bildschirm leer oder zeigt eine Fehlerseite
- Der Monitor ist von außen ohne Anmeldung erreichbar

## 3. Rahmenbedingungen

- **Hardware:** FüKw-Server mit einem angeschlossenen Bildschirm. Auflösung und GPU sind noch unbekannt, das Ziel ist 16:9 in Full HD. Das Netz läuft über LTE und Sat und kann ausfallen.
- **OS:** Ubuntu Server 26.04 LTS ohne Desktop-Image
- **Fernzugang:** Der Server kommt ins Tailnet und/oder hinter einen Cloudflare Tunnel, das wird gerade getestet. Beide Wege müssen funktionieren.
- **Identität:** Pocket ID wie bei der iuk-suite. Die Admin-Rechte laufen über eine OIDC-Gruppe.
- **Datenschutz:** Der Monitor zeigt keine Personendaten. Repo und Beispieldaten sind synthetisch.
- **Derselbe Server** soll später auch Suchlage tragen (Suchlage-Meilenstein M4).

## 4. Was entschieden ist

### 4.1 Kiosk
Ubuntu Server ohne Desktop. Darauf läuft **`cage`** (Wayland-Kiosk-Compositor) mit **Chromium im Kiosk-Modus**, beides als systemd-Dienst auf tty1 unter einem eigenen Benutzer ohne Rechte. Chromium öffnet den lokalen Monitor-Port. Ubuntu Frame mit `wpe-webkit-mir-kiosk` wurde verworfen, weil der stable-Kanal des Browser-Snaps im Store als veraltet markiert ist.

### 4.2 Eine Quelle der Wahrheit im FüKw
Monitor, Widgets, Konfiguration und Datenabruf laufen auf dem FüKw-Server. Ohne Internet läuft der Monitor weiter. Externe Daten (Wetter, Warnungen, Radar) veralten dann sichtbar.

### 4.3 Zugriff
| Weg | Monitor | Admin-UI |
|---|---|---|
| Lokal: Kiosk-Bildschirm und Geräte im FüKw-Netz | ohne Anmeldung | nur mit Anmeldung |
| Von außen über Tailscale oder Cloudflare Tunnel | nur mit Anmeldung | nur mit Anmeldung |

- Ob ein Zugriff lokal ist, entscheidet der **Listener bzw. Port**, nicht die Quell-IP. Hinter cloudflared kommt jede Anfrage von `localhost`. Der Port ohne Anmeldung lauscht nur auf dem LAN-Interface und auf Loopback für den Kiosk. Tunnel und Tailnet zeigen ausschließlich auf den Port mit Anmeldung.
- *Entscheidung Claude:* Der Port ohne Anmeldung bindet ausdrücklich an die LAN-Adresse, nie an `0.0.0.0` oder `::`. Zusätzlich blockt eine Host-Firewall eingehend alles auf den Uplink-Interfaces, auch IPv6, denn Sat und LTE können öffentliche v6-Adressen weiterreichen. Der Plan enthält dafür eine Prüfung von außen.
- *Entscheidung Claude:* Den Monitor von außen sehen darf eine eigene OIDC-Gruppe (Name einstellbar). Admin ist eine zweite, engere Gruppe.
- Admin funktioniert nur bei bestehender Internetverbindung. Das reicht, weil die Sitzung lang hält, Richtwert 14 Tage. Sie wird lokal geprüft und braucht Pocket ID nur beim Anmelden.

### 4.4 Admin-UI
Die Admin-UI ist **Teil der iuk-suite** (Modul „Lagemonitor“), mit Vorschau samt Beispieldaten. *Geändert am 27.10.2026 auf Wunsch von Ruben (LGM-5);* vorher stand hier: Admin im Lagemonitor, in der Suite nur ein Link.

Der Einwand von damals bleibt richtig und ist so gelöst: Der Lagemonitor **holt** die Konfiguration jede Minute aus der Suite (`GET /api/konfiguration`, Zugangsschlüssel als Bearer) und schreibt sie in seine Tabelle `settings`. Ohne Internet läuft er mit dem letzten Stand weiter; der Fuß zeigt dann, dass die Abholung scheitert. Ohne Suite-Adresse gilt allein die lokale Tabelle.

*Ergänzt am 27.10.2026 auf Wunsch von Ruben (LGM-6):* Die Suite schickt dazu **Meldungen** (`notices`) fürs blaue Band unter dem Warnband, etwa „Nächste Lagebesprechung · in 42 min“. Jede hat ein Ablaufdatum; der Monitor lässt Abgelaufenes selbst weg, auch ohne Verbindung zur Suite. Und optional einen **QR-Code fürs Ehrenamt** (`recruitment`, Vorgabe aus), unten rechts auf der Karte, nur in ruhiger Lage: bei Warnung im Band oder laufendem Einsatz gehört der Bildschirm der Lage.

### 4.5 Relevanz statt Karussell
Jedes Widget meldet selbst, ob es gerade etwas zu sagen hat, und mit welcher Dringlichkeit. Das Layout gibt dringlichen Widgets mehr Platz und lässt Widgets ohne Inhalt zurücktreten. Der Rahmen (Kopfzeile mit Uhr und Status, Fußzeile mit Datenstand) ist immer sichtbar. Die Admin-UI kann pinnen, ausblenden, sortieren und eine Ansicht erzwingen. Ruhe und Schaufenster sind die Zustände, wenn nichts anliegt.

*Entscheidung Claude, nach dem Builder-Check:* **Amtliche Warnungen brechen immer durch.** Gilt eine Warnung oberhalb der Schwelle, erscheint das Warnband auch dann, wenn eine Ansicht erzwungen, das Schaufenster aktiv oder das Warnungs-Widget ausgeblendet ist. Pinnen, Ausblenden und Erzwingen wirken nur auf den Bereich unterhalb des Bands.

### 4.6 Datenquellen v1
- **Warnungen:** NINA-API des BBK (`nina.api.bund.dev`), Abruf je Regionalschlüssel. Sie enthält DWD, MoWaS, KATWARN und Hochwasser. Warngebiete erscheinen als Fläche auf der Karte.
- **Wetter:** DWD-Daten, z. B. über Bright Sky (kein Schlüssel nötig)
- **Regenradar:** DWD-Radar als Kartenebene, nur bei bestehender Verbindung
- **Karte:** offline über ein lokales Kartenpaket, MapLibre wie in Suchlage und Lifeline. Der FüKw ist auch außerhalb des Landkreises im Einsatz, wenn auch selten. Deshalb umfasst das Paket **ganz Deutschland**. Gebaut wird es mit der Pipeline aus Lifeline Hub (`lifeline-hub/karten-build`, Shortbread-MBTiles über `versatiles-planetiler`, `make tiles AREA=germany`), damit es später zu den LLH-Karten passt. Ob der Lagemonitor die Datei selbst ausliefert oder den `karten-service` aus LLH mitnutzt, entscheidet der Builder.
- **Warnregion:** wird in der Admin-UI zusammen mit dem Standort eingestellt, als Regionalschlüssel bzw. Landkreis. Weil Einsätze außerhalb selten sind, folgt sie dem Standort in v1 nicht automatisch. Mit GPS-Standort kommt das später.
- Abrufe laufen über den Server, nicht direkt aus dem Browser. Der Server cached die Daten und liefert sie mit Zeitstempel aus.

*Entscheidung Claude – Warnstufen und Schwelle:*
- **Stufen** werden nicht über alle Quellen vereinheitlicht. Jede Warnung zeigt die Stufe ihrer Quelle als Wort, also DWD „Stufe 1–4“ und MoWaS/KATWARN „Gering … Extrem“. Die Farbe ergibt sich aus einer festen Zuordnung im Code.
- Die **Warnschwelle** bestimmt nur, welche Warnungen das Band oben bekommen und nach vorn rücken. Warnungen unterhalb der Schwelle verschwinden nicht, sie stehen als Liste im Warnungs-Widget. Alle amtlichen Warnungen bleiben also sichtbar.

*Entscheidung Claude – Standort außerhalb des Kartenpakets:* Wird ein Standort außerhalb des Kartenpakets eingestellt, zeigt die Kartenkachel einen ausdrücklichen Hinweis statt einer leeren Fläche. Wenn Internet da ist, nutzt sie Online-Kacheln als Rückfall.

### 4.7 Stack
Wie Suchlage: Backend in **Rust**, Frontend in **React/Vite mit antd 6** und dem Lifeline-Designsystem, Daten in **SQLite**. Die Übernahme des Designsystems ist beschrieben in `suchlage/docs/reference/lifeline-vorlage.md`. Der Rahmen wird nach den Lifeline-Maßen neu gebaut, nicht kopiert. Der Monitor hat keine Rail und kein Modulpanel.

### 4.8 Gestaltung
Die drei Zustände aus dem Brainstorming sind angenommen. Die Entwürfe liegen verbindlich in `docs/design/2026-10-27-zustaende.html`, die Daten darin sind erfunden:
- **Ruhe:** Karte groß mit Standort, rechts Wetter mit Kurzvorhersage
- **Warnung aktiv:** Die Warnung erscheint als Band oben, mit Stufe als Wort und Zahl, Inhalt, Gültigkeit und Quelle. Auf der Karte liegen Warngebiet und Radar.
- **Schaufenster:** Ruhe plus eine Kachel „Was ist das hier?“ mit QR-Code

Es gelten die Regeln der Instrumententafel: Nacht ist Vorgabe, Radius 0, Haarlinien, Status immer zusätzlich als Wort oder Zahl, Zahlen in JetBrains Mono. Die Statusleiste nennt den Datenstand. Ab einer Schwelle steht dort hervorgehoben „Veraltet“, so wie im Lifeline-Lagemonitor.

## 5. Dem Builder überlassen

- Schnitt der Widget-Schnittstelle (Relevanzmeldung, Datenabruf, Darstellung), solange Suchlage- und LLH-Widgets später ohne Umbau dazukommen
- Layout-Algorithmus für die Platzvergabe nach Dringlichkeit
- Abrufintervalle und Schwellen für „veraltet“
- Betriebsform des Monitor-Dienstes: Container wie bei Suchlage oder systemd. Die Kiosk-Schicht selbst ist host-seitig und läuft über systemd.
- Wie das Deutschland-Kartenpaket aktualisiert und auf den Server gebracht wird

## 6. Offen

- Router-Modell im FüKw: Davon hängt der automatische Standort ab
- Bildschirmauflösung und GPU des Servers. Das sollte früh getestet werden, ob Chromium unter `cage` auf dieser Hardware sauber läuft.
- Fireboard: Anfrage beim Hersteller, ob es einen Live-Leseweg gibt
- Tailscale oder Cloudflare Tunnel: wird gerade getestet. Das Design deckt beide ab.
