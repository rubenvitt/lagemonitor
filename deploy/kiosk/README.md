# Kiosk-Einrichtung auf Ubuntu Server

Nach dem Booten startet auf dem angeschlossenen Bildschirm (tty1) Chromium im Vollbild. Darunter läuft `cage` als Wayland-Kiosk-Compositor. Es gibt keinen Desktop und keine Anmeldung, SSH bleibt unberührt. Erprobt ist das für Ubuntu Server 26.04 LTS.

## 1. Pakete

```bash
sudo apt update
sudo apt install -y cage curl
sudo snap install chromium   # auf Ubuntu gibt es Chromium nur als Snap
```

Snaps aktualisieren sich selbst und starten den Browser dabei neu. Damit das nachts passiert und nicht mitten im Dienst:

```bash
sudo snap set system refresh.timer=04:00-05:00
```

## 2. Kiosk-Benutzer

Ein eigener Benutzer ohne Passwort und ohne Login-Shell. Er darf nur Bildschirm und Eingabegeräte benutzen.

```bash
sudo useradd -m -s /usr/sbin/nologin -G video,render,input kiosk
```

## 3. Adresse, die angezeigt wird

```bash
echo 'KIOSK_URL=http://127.0.0.1:8080' | sudo tee /etc/default/lagemonitor-kiosk
```

Das ist die Adresse des Lagemonitors auf demselben Server. Er muss vorher laufen, siehe [`../README.md`](../README.md). Falls du den Kiosk vorab testen willst, trage stattdessen eine beliebige Webseite ein, etwa `https://www.dwd.de`, und ändere die Zeile später. Danach startest du den Dienst neu (Schritt 6).

## 4. PAM-Profil

Damit bekommt der Dienst eine echte Sitzung mit Zugriff auf Bildschirm und GPU. Ohne sie bleibt der Bildschirm schwarz.

```bash
sudo tee /etc/pam.d/lagemonitor-kiosk >/dev/null <<'EOF'
auth     required pam_unix.so nullok
account  required pam_unix.so
session  required pam_unix.so
session  required pam_systemd.so
EOF
```

## 5. Dienst

```bash
sudo tee /etc/systemd/system/lagemonitor-kiosk.service >/dev/null <<'EOF'
[Unit]
Description=Lagemonitor-Kiosk (cage + Chromium) auf tty1
After=systemd-user-sessions.service plymouth-quit-wait.service network-online.target
Wants=network-online.target dbus.socket systemd-logind.service
After=dbus.socket systemd-logind.service
Conflicts=getty@tty1.service
After=getty@tty1.service
ConditionPathExists=/dev/tty1

[Service]
User=kiosk
PAMName=lagemonitor-kiosk
EnvironmentFile=/etc/default/lagemonitor-kiosk
ExecStartPre=/bin/sh -c 'until curl -sf http://127.0.0.1:8080/api/health >/dev/null; do sleep 2; done'
TimeoutStartSec=infinity
ExecStart=/usr/bin/cage -- /snap/bin/chromium --kiosk --incognito --no-first-run --noerrdialogs --disable-infobars --disable-translate --ozone-platform=wayland --check-for-update-interval=31536000 ${KIOSK_URL}
Restart=always
RestartSec=5
UtmpIdentifier=tty1
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes
TTYVTDisallocate=yes
StandardInput=tty-fail
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=graphical.target
EOF
```

`ExecStartPre` wartet, bis der Lagemonitor antwortet. So zeigt der Bildschirm nach dem Booten nie die Fehlerseite des Browsers, sondern bleibt schwarz, bis der Lagemonitor bereit ist. `TimeoutStartSec=infinity` erlaubt dieses Warten beliebig lange.

`--incognito` sorgt dafür, dass nach einem Stromausfall kein Hinweis „Seiten wiederherstellen?“ erscheint. Stürzt Chromium ab, startet systemd ihn nach 5 Sekunden neu.

## 6. Einschalten

```bash
sudo systemctl set-default graphical.target   # Ubuntu Server startet sonst nur bis multi-user
sudo systemctl daemon-reload
sudo systemctl enable --now lagemonitor-kiosk.service
```

Danach sollte der Bildschirm die Seite zeigen. Zur Kontrolle einmal neu starten: `sudo reboot`.

## 7. Bildschirm aus- und wieder einschalten

Viele Monitore melden sich beim Ausschalten ab. cage erkennt den Monitor beim Einschalten wieder, aber Chromium
zeichnet nach einer Zeit ganz ohne Bildschirm nicht mehr neu: Das Bild bleibt schwarz, auch wenn der Monitor
beim Booten aus war. Deshalb startet eine udev-Regel den Kiosk neu, sobald ein Bildschirm **neu verbunden** ist.
Ausschalten löst nichts aus.

```bash
cd deploy/kiosk
sudo install -m 0755 lagemonitor-kiosk-hotplug /usr/local/sbin/
sudo install -m 0644 lagemonitor-kiosk-hotplug.service /etc/systemd/system/
sudo install -m 0644 90-lagemonitor-kiosk-hotplug.rules /etc/udev/rules.d/
sudo systemctl daemon-reload
sudo udevadm control --reload
```

Zum Prüfen den Monitor aus- und wieder einschalten: Nach ein paar Sekunden ist das Bild wieder da, und
`journalctl -u lagemonitor-kiosk-hotplug -b` zeigt „Bildschirm neu verbunden: …“. Die Erkennungslogik prüft
`deploy/kiosk/hotplug-test.sh` ohne Server.

## Betrieb

| Was | Befehl |
|---|---|
| Log ansehen | `journalctl -u lagemonitor-kiosk -f` |
| Neu laden (z. B. nach URL-Änderung) | `sudo systemctl restart lagemonitor-kiosk` |
| Kiosk aus, Konsole zurück auf tty1 | `sudo systemctl disable --now lagemonitor-kiosk` |

## Wenn der Bildschirm schwarz bleibt

1. `journalctl -u lagemonitor-kiosk -b` lesen.
2. Steht dort etwas wie `Could not open DRM device`, `no GPU` oder `failed to create backend`: Ein Grafiktreiber fehlt. Das tritt vor allem bei NVIDIA auf und lässt sich mit `sudo ubuntu-drivers install` beheben, danach neu starten.
3. Steht dort `Permission denied` beim Öffnen von `/dev/dri` oder `/dev/input`: Prüfen, ob die Sitzung entsteht (`loginctl list-sessions` sollte `kiosk` auf `tty1` zeigen) und ob Schritt 4 stimmt.
4. Schließt Chromium sofort wieder: einmal von Hand testen mit `sudo -u kiosk /snap/bin/chromium --version`. Der erste Start eines Snaps richtet das Profil ein und kann dauern.
