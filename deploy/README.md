# Lagemonitor auf dem Server einrichten

Diese Anleitung ist für den Server im Führungskraftwagen. Du brauchst keine Programmierkenntnisse. Du tippst die Befehle der Reihe nach ein oder kopierst sie. Das Programm wird **nicht** auf dem Server gebaut. Es kommt fertig als „Image“ (ein Paket, das alles enthält) aus dem Internet.

Du brauchst:

- einen Ubuntu Server mit Internet (nur für die Einrichtung und für Updates)
- einen Zugang per SSH oder eine Tastatur am Server
- ein GitHub-Konto mit Zugriff auf das Repository `rubenvitt/lagemonitor`

**Wichtig vorab:** Diese Anleitung geht von einem normalen PC-Prozessor aus („amd64“). Prüfe das einmal:

```bash
uname -m
```

**So prüfst du es:** Die Ausgabe ist `x86_64`. Steht dort etwas anderes (zum Beispiel `aarch64`), höre auf und frage nach. Das Image passt dann nicht.

---

## 1. Docker installieren

Docker ist das Programm, das den Lagemonitor in seinem Paket startet. Wir nehmen die offizielle Quelle von Docker.

Zuerst die Grundlagen:

```bash
sudo apt update
sudo apt install -y ca-certificates curl
```

Dann den Schlüssel und die Paketquelle von Docker eintragen:

```bash
sudo install -m 0755 -d /etc/apt/keyrings
sudo curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
sudo chmod a+r /etc/apt/keyrings/docker.asc
```

```bash
sudo tee /etc/apt/sources.list.d/docker.sources >/dev/null <<EOT
Types: deb
URIs: https://download.docker.com/linux/ubuntu
Suites: $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}")
Components: stable
Architectures: $(dpkg --print-architecture)
Signed-By: /etc/apt/keyrings/docker.asc
EOT
```

Docker und das Zusatzprogramm `docker compose` installieren:

```bash
sudo apt update
sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
```

Deinen Benutzer in die Gruppe `docker` aufnehmen, damit du nicht jedes Mal `sudo` brauchst:

```bash
sudo usermod -aG docker $USER
```

Jetzt abmelden und wieder anmelden (bei SSH: Verbindung schließen und neu öffnen).

**So prüfst du es:**

```bash
docker run --rm hello-world
```

Du siehst den Text „Hello from Docker!“. Und:

```bash
docker compose version
```

zeigt eine Versionsnummer, zum Beispiel `Docker Compose version v2.…`.

---

## 2. Zugang zum Image einrichten

Das Image liegt in einem privaten Bereich bei GitHub. Dafür brauchst du einen „Token“ (eine Art Passwort nur zum Lesen).

1. Öffne im Browser <https://github.com/settings/tokens/new?scopes=read:packages&description=lagemonitor-server>. Falls die Seite den Haken nicht vorausfüllt: Wähle „Tokens (classic)“ und setze **nur** den Haken bei `read:packages`.
2. Wähle eine Laufzeit („No expiration“ ist bequem, ein Jahr ist sicherer).
3. Klicke „Generate token“ und kopiere den Token (er beginnt mit `ghp_`). Du siehst ihn nur einmal.
4. Lege den Token in 1Password ab, damit er nicht verloren geht.

Auf dem Server meldest du Docker damit an:

```bash
docker login ghcr.io -u rubenvitt
```

Als Passwort fügst du den Token ein. Beim Einfügen siehst du nichts, das ist normal. Bestätige mit Enter.

**So prüfst du es:** Es erscheint `Login Succeeded`.

---

## 3. Ordner und Dateien anlegen

Alles liegt im Ordner `/opt/lagemonitor`.

```bash
sudo mkdir -p /opt/lagemonitor/data
sudo chown -R $USER: /opt/lagemonitor
sudo chown 65532:65532 /opt/lagemonitor/data
cd /opt/lagemonitor
```

Die letzte `chown`-Zeile ist wichtig: Der Lagemonitor läuft aus Sicherheitsgründen als Benutzer mit der Nummer 65532 und darf nur in `data` schreiben, wenn der Ordner ihm gehört.

Jetzt brauchst du zwei Dateien: `compose.yaml` und `.env`. Es gibt zwei Wege.

**Weg A: Inhalt einfügen (einfach).** Kopiere den ganzen Block und füge ihn im Terminal ein:

```bash
cat > /opt/lagemonitor/compose.yaml <<'EOT'
services:
  lagemonitor:
    image: ghcr.io/rubenvitt/lagemonitor:latest
    # Host-Netz, damit LAGEMONITOR_BIND echte Adressen des Servers bindet.
    network_mode: host
    environment:
      LAGEMONITOR_BIND: 127.0.0.1:8080,${LAN_IP:?LAN_IP fehlt – siehe .env}:8080
    volumes:
      - ./data:/data
    restart: unless-stopped
EOT
```

Die Datei `.env` legen wir in Schritt 4 an.

**Weg B: vom Laptop kopieren.** Wenn du das Repository auf dem Laptop hast, kopierst du die Datei von dort (das geht nicht direkt von GitHub, weil das Repository privat ist). Auf dem **Laptop** im Ordner des Repositories:

```bash
scp deploy/compose.yaml deploy/.env.example BENUTZER@SERVERNAME:/opt/lagemonitor/
```

`BENUTZER` und `SERVERNAME` ersetzt du durch deine Angaben.

**So prüfst du es:**

```bash
ls -la /opt/lagemonitor
```

Du siehst die Datei `compose.yaml` und den Ordner `data`. Der Ordner `data` gehört `65532`.

---

## 4. Die Adresse des Servers eintragen

Der Lagemonitor soll auf zwei Adressen antworten: auf dem Server selbst (`127.0.0.1`) und im Netz des Fahrzeugs. Dafür brauchst du die Netz-Adresse des Servers (`LAN_IP`).

```bash
ip -4 addr
```

Suche die Zeile mit `inet`, die zum Netz des Fahrzeugs gehört. Sie sieht aus wie `inet 192.168.10.5/24`. Gemerkt wird nur der Teil vor dem `/`, hier `192.168.10.5`. Die Adresse `127.0.0.1` ignorierst du. Nimm **nicht** die Adresse der Mobilfunk- oder Satelliten-Verbindung.

Trage sie ein. Ersetze `192.168.10.5` durch deine Adresse:

```bash
echo 'LAN_IP=192.168.10.5' > /opt/lagemonitor/.env
```

**So prüfst du es:**

```bash
cat /opt/lagemonitor/.env
```

Du siehst genau eine Zeile `LAN_IP=…` mit deiner Adresse.

---

## 5. Starten

```bash
cd /opt/lagemonitor
docker compose up -d
```

Docker lädt das Image herunter (das dauert beim ersten Mal ein bis zwei Minuten) und startet den Lagemonitor im Hintergrund. Er startet nach einem Neustart des Servers von allein wieder.

**So prüfst du es:**

```bash
curl -s 127.0.0.1:8080/api/health
```

Die Ausgabe ist `{"status":"ok"}`. Zusätzlich zeigt

```bash
docker compose ps
```

den Dienst `lagemonitor` mit dem Status `running` (oder `Up`).

---

## 6. Den Bildschirm (Kiosk) auf den Lagemonitor stellen

Wenn der Kiosk-Bildschirm noch nicht eingerichtet ist, folge der Anleitung in [`kiosk/README.md`](kiosk/README.md). Dort trägst du in Schritt 3 diese Adresse ein:

```text
KIOSK_URL=http://127.0.0.1:8080
```

War der Kiosk schon eingerichtet, änderst du nur die Adresse (Schritt 3) und startest ihn neu (Schritt 6):

```bash
echo 'KIOSK_URL=http://127.0.0.1:8080' | sudo tee /etc/default/lagemonitor-kiosk
sudo systemctl restart lagemonitor-kiosk
```

**So prüfst du es:** Nach wenigen Sekunden zeigt der Bildschirm den Lagemonitor. Auf einem anderen Gerät im Fahrzeugnetz erreichst du ihn unter `http://<LAN_IP>:8080`.

---

## 7. Aktualisieren

Wenn es eine neue Version gibt (die CI baut sie nach jeder Änderung auf `main`):

```bash
cd /opt/lagemonitor
docker compose pull
docker compose up -d
```

**So prüfst du es:** `docker compose pull` zeigt `Pulled` (oder „Downloaded newer image“), `up -d` startet den Dienst neu. Danach liefert `curl -s 127.0.0.1:8080/api/health` wieder `{"status":"ok"}`. Deine Einstellungen in `data` bleiben erhalten.

Aufräumen, wenn alte Versionen Platz brauchen:

```bash
docker image prune -f
```

---

## 8. Offline-Karte (optional)

Ohne dieses Paket lädt die Karte ihre Kacheln aus dem Internet. Mit der Offline-Karte funktioniert sie auch ohne Verbindung.

1. Baue das Kartenpaket auf dem Laptop im Projekt `lifeline-hub/karten-build`: `make tiles AREA=germany`.
2. Kopiere die erzeugte Datei auf den Server, zuerst nach `/tmp`. Direkt nach `data` geht es nicht, der Ordner gehört dem Benutzer 65532. Auf dem **Laptop**:

   ```bash
   scp karte.mbtiles BENUTZER@SERVERNAME:/tmp/karte.mbtiles
   ```

3. Auf dem **Server** die Datei an ihren Platz legen und dem Lagemonitor übergeben:

   ```bash
   sudo install -o 65532 -g 65532 -m 0644 /tmp/karte.mbtiles /opt/lagemonitor/data/karte.mbtiles
   rm /tmp/karte.mbtiles
   ```

4. Neu starten (die Karte wird nur beim Start gelesen):

   ```bash
   cd /opt/lagemonitor
   docker compose restart
   ```

**So prüfst du es:** In `ls -la /opt/lagemonitor/data` steht `karte.mbtiles` mit Besitzer `65532`. Dann:

```bash
curl -s 127.0.0.1:8080/api/monitor | grep -o '"map":{[^}]*}'
```

Die Ausgabe ist `"map":{"offline":true}`. Steht dort `false`, ist die Datei nicht lesbar. Der Lagemonitor läuft trotzdem weiter und lädt die Karte aus dem Internet. Den Grund findest du mit `docker compose logs | grep Kartendatei`. Nachweis ohne Internet: Ziehe testweise das Netzwerkkabel zum Uplink, die Karte bleibt sichtbar.

---

## 9. Einstellungen ändern (Standort, Landkreis, Warnschwelle)

Eine Verwaltungsseite gibt es noch nicht. Bis dahin änderst du die Werte direkt in der Datenbank.

**Zuerst:** Der Lagemonitor muss mindestens einmal gelaufen sein (Schritt 5), denn er legt die Datenbank selbst an. Legt `sudo sqlite3` sie zuerst an, gehört sie dem Benutzer `root` und der Lagemonitor kann sie nicht öffnen. Reparieren: `sudo chown 65532:65532 /opt/lagemonitor/data/lagemonitor.db*`, dann `docker compose restart`.

Du brauchst das Programm `sqlite3` auf dem **Server** (nicht im Docker-Paket):

```bash
sudo apt install -y sqlite3
```

Die Einstellungen sind drei Einträge:

| Name | Bedeutung | Beispielwert |
|---|---|---|
| `location` | Standort für Wetter und Karte | `{"lat":49.0069,"lon":8.4037,"label":"Karlsruhe"}` |
| `region` | Landkreis für Warnungen (ARS = amtlicher Regionalschlüssel, 12 Stellen) | `{"ars":"082120000000","name":"Karlsruhe"}` |
| `warn_threshold` | ab welcher Stärke eine Warnung groß angezeigt wird: `Minor`, `Moderate`, `Severe` oder `Extreme` | `"Severe"` |

Beispiel: Standort ändern. Ersetze die Zahlen und den Namen. Die Anführungszeichen müssen genau so bleiben:

```bash
sudo sqlite3 /opt/lagemonitor/data/lagemonitor.db "INSERT OR REPLACE INTO settings(key, value) VALUES ('location', '{\"lat\":49.0069,\"lon\":8.4037,\"label\":\"Karlsruhe\"}');"
```

Beispiel: Warnschwelle auf „Moderate“ setzen:

```bash
sudo sqlite3 /opt/lagemonitor/data/lagemonitor.db "INSERT OR REPLACE INTO settings(key, value) VALUES ('warn_threshold', '\"Moderate\"');"
```

Der Lagemonitor liest die Werte bei jedem Abruf und jeder Anzeige neu. Ein Neustart sorgt nur dafür, dass er die Daten für den neuen Ort sofort holt, statt bis zum nächsten Abruf zu warten (Warnungen jede Minute, Wetter alle 10 Minuten). Nach einer Änderung von Standort oder Landkreis zeigt der Bildschirm bis dahin „WARNLAGE UNBEKANNT“ bzw. „Noch keine Daten“, nie die Daten des alten Orts. Die Warnschwelle wirkt sofort.

```bash
cd /opt/lagemonitor
docker compose restart
```

`sudo` ist nötig, weil die Datei dem Benutzer 65532 gehört, nicht dir.

**So prüfst du es:** Alle Einstellungen anzeigen:

```bash
sudo sqlite3 /opt/lagemonitor/data/lagemonitor.db "SELECT key, value FROM settings;"
```

Die Ausgabe enthält deine neuen Werte, zum Beispiel `warn_threshold|"Moderate"`.

Dann prüfen, ob der Lagemonitor die Werte lesen kann:

```bash
curl -s 127.0.0.1:8080/api/monitor | head -c 200
```

Die Ausgabe muss mit `{"nowMs"` beginnen. Beginnt sie stattdessen mit `{"error":"Einstellungen nicht lesbar: …"}`, ist der Wert mit dem genannten Namen falsch geschrieben. Der Bildschirm zeigt dann „Server nicht erreichbar“. Häufige Ursachen:

- ein Anführungszeichen fehlt: `warn_threshold` braucht `'"Moderate"'` (mit `\"` im Befehl oben), nicht `'Moderate'`
- ein Tippfehler im Stufennamen: erlaubt sind nur `Minor`, `Moderate`, `Severe`, `Extreme` (Groß- und Kleinschreibung ist egal)
- bei `location` ein Komma statt Punkt in der Zahl (`49,0069` statt `49.0069`) oder ein fehlendes Feld (`lat`, `lon`, `label`)
- bei `region` fehlt `ars` oder `name`

Korrigiere den Wert mit demselben `INSERT OR REPLACE`-Befehl und prüfe erneut.

---

## 10. Wenn etwas nicht klappt

Zuerst das Protokoll ansehen (mit `Strg+C` verlässt du es wieder):

```bash
cd /opt/lagemonitor
docker compose logs -f
```

| Meldung / Problem | Ursache und Lösung |
|---|---|
| `denied` oder `unauthorized` beim Herunterladen | Der Token ist abgelaufen oder falsch. Neuen Token anlegen (Schritt 2) und `docker login ghcr.io -u rubenvitt` wiederholen. |
| `LAN_IP fehlt` | Die Datei `.env` fehlt (Schritt 4). |
| `address already in use` | Auf Port 8080 läuft schon etwas anderes. Mit `sudo ss -ltnp \| grep 8080` siehst du, was. Das andere Programm beenden oder umstellen. |
| `Adresse nicht verfügbar, neuer Versuch` im Protokoll, im Fahrzeugnetz nicht erreichbar | Die Adresse aus `LAN_IP` ist (noch) nicht am Server. Der Bildschirm am Server läuft weiter, der Lagemonitor versucht es alle 10 Sekunden erneut. Steckt das Kabel und hat der Server seine Adresse, verschwindet die Meldung von allein. Sonst stimmt `LAN_IP` in `/opt/lagemonitor/.env` nicht: mit `ip -4 addr` die Adresse neu ermitteln, `.env` korrigieren, dann `docker compose up -d`. |
| `variable is not set` / `LAN_IP` leer | Die Datei `.env` fehlt oder liegt im falschen Ordner. Sie muss neben `compose.yaml` in `/opt/lagemonitor` liegen. |
| `unable to open database file` / `permission denied` bei `data` | Der Ordner gehört nicht dem Benutzer 65532. Reparieren: `sudo chown -R 65532:65532 /opt/lagemonitor/data`, dann `docker compose restart`. |
| Bildschirm zeigt „Noch keine Daten“ | Der Server hat noch kein Internet gehabt. Kurz warten, die Daten werden regelmäßig geholt. Letzte Stände bleiben auch ohne Internet erhalten. |

---

## Sicherheit

- Der Lagemonitor darf **nie** auf allen Adressen lauschen. Trage deshalb in `LAGEMONITOR_BIND` niemals `0.0.0.0` ein, nur `127.0.0.1` und die Adresse des Fahrzeugnetzes. Das ist in `compose.yaml` bereits so.
- Der Lagemonitor hat in dieser Version **keine Anmeldung**. Jeder im Fahrzeugnetz kann ihn sehen. Von außen (Mobilfunk, Satellit) darf er nicht erreichbar sein.
- Die Firewall `ufw` soll eingehende Verbindungen auf den Uplink-Schnittstellen sperren, für IPv4 und IPv6. Das Grundgerüst, mit eigenen Schnittstellennamen statt `UPLINK`:

  ```bash
  sudo ufw default deny incoming
  sudo ufw allow in on LAN_SCHNITTSTELLE to any port 8080 proto tcp
  sudo ufw allow OpenSSH
  sudo ufw enable
  ```

  Die genauen Schnittstellennamen (LTE, Satellit, Fahrzeugnetz) folgen aus LGM-3. Bis dahin: Befehle nicht blind kopieren.
  Wichtig: Docker umgeht `ufw` bei veröffentlichten Ports. Weil wir das Host-Netz benutzen, gibt es hier keine solchen Ports, und `ufw` greift normal.
- Der GitHub-Token hat nur Leserechte auf Pakete. Verliere ihn trotzdem nicht: Bei Verdacht auf Missbrauch löschst du ihn unter <https://github.com/settings/tokens>.
