# Kiosk-Einrichtung auf Ubuntu Server

Nach dem Booten startet auf dem angeschlossenen Bildschirm (tty1) Chromium im Vollbild. Darunter läuft `cage` als Wayland-Kiosk-Compositor. Es gibt keinen Desktop und keine Anmeldung, SSH bleibt unberührt. Erprobt ist das für Ubuntu Server 26.04 LTS.

## 1. Pakete

```bash
sudo apt update
sudo apt install -y cage
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
echo 'KIOSK_URL=https://www.dwd.de/DE/wetter/warnungen_gemeinden/warnWetter_node.html' | sudo tee /etc/default/lagemonitor-kiosk
```

Zum Testen ist hier die DWD-Warnkarte eingetragen. Sobald der Lagemonitor läuft, ersetzt du sie durch `KIOSK_URL=http://127.0.0.1:<port>` und startest den Dienst neu (Schritt 6).

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

`--incognito` sorgt dafür, dass nach einem Stromausfall kein Hinweis „Seiten wiederherstellen?“ erscheint. Stürzt Chromium ab, startet systemd ihn nach 5 Sekunden neu.

## 6. Einschalten

```bash
sudo systemctl set-default graphical.target   # Ubuntu Server startet sonst nur bis multi-user
sudo systemctl daemon-reload
sudo systemctl enable --now lagemonitor-kiosk.service
```

Danach sollte der Bildschirm die Seite zeigen. Zur Kontrolle einmal neu starten: `sudo reboot`.

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
