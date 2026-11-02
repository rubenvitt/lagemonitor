# Online-Instanz auf iuk-services

Neben dem Lagemonitor im FüKw läuft derselbe Code ein zweites Mal auf dem Server der IUK-Suite, für
Rechner außerhalb des Fahrzeugs. Beide holen ihre Einstellungen aus der Suite (jede mit eigenem
Zugang) und fragen Suchlage mit demselben Großbild-Link ab. Sie zeigen also dasselbe Lagebild.

| | FüKw | Online |
|---|---|---|
| Adresse | `http://127.0.0.1:8080` (Kiosk), `http://<LAN_IP>:8080` | `https://lagemonitor.iuk-ue.de/monitor/` |
| Anmeldung | keine (nur Loopback und Fahrzeugnetz) | Pocket ID oder Link aus der Suite |
| Listener | `LAGEMONITOR_BIND` | `LAGEMONITOR_AUTH_BIND` (`LAGEMONITOR_BIND` leer) |
| Image | amd64 | arm64 (dasselbe Tag, Docker wählt) |
| Rollout | CI → `deploy-poller` am FüKw | Minuten-Poller am Server folgt `:latest` |
| Kartenpaket | `data/karte.mbtiles` | keins, Kacheln online |

## Wer hineinkommt

- **Mit Konto:** Anmeldung über Pocket ID, Gruppe `lagemonitor-nutzer` oder `lagemonitor-admin`
  (`LAGEMONITOR_OIDC_GROUPS`). Die Sitzung hält 14 Tage und wird lokal geprüft; ein Gruppenentzug
  wirkt also erst mit ihrem Ablauf.
- **Mit Link:** In der Suite unter *Lagemonitor → Links für Rechner ohne Anmeldung* einen Link
  anlegen und auf dem Rechner einmal öffnen. Der Rechner bleibt angemeldet (Cookie, 400 Tage), bis
  der Link in der Suite widerrufen ist; das wirkt nach spätestens einer Minute. Der Schlüssel steht
  hinter `#` und erreicht weder Cloudflare- noch Traefik-Logs.
- `/monitor/api/health` ist offen, alles andere nicht.

## Einrichtung

Pfade wie auf iuk-services (`~/server/compose/<stack>`).

1. **Pocket ID:** Gruppe `lagemonitor-nutzer` anlegen. OIDC-Client „Lagemonitor online“ mit
   Callback `https://lagemonitor.iuk-ue.de/monitor/auth/callback`, PKCE an, erlaubte Gruppen
   `lagemonitor-nutzer` und `lagemonitor-admin`.
2. **Suite:** unter *Lagemonitor → Zugänge für den Monitor* einen Zugang „Lagemonitor online“
   anlegen. In die `.env` der Suite `LAGEMONITOR_ONLINE_URL=https://lagemonitor.iuk-ue.de/monitor`
   eintragen (sonst bietet die Verwaltung keine Links an) und die Suite neu starten.
3. **Stack:** `compose.yaml` und `.env.example` nach `~/server/compose/lagemonitor/`, `.env` daraus
   füllen (`chmod 600`), `mkdir data && sudo chown 65532:65532 data`, dann `docker compose up -d`.
   Ist das Paket privat, braucht Docker dort einen `docker login ghcr.io` mit `read:packages`.
4. **Hostname:** keiner nötig, `lagemonitor.iuk-ue.de` ist schon im Tunnel. Die Regel in
   `compose.yaml` nimmt nur `/monitor`; alles andere auf dem Host bleibt bei der Suite.
5. **Rollout:** ein Poller wie bei Suchlage (`~/server/auto-update/`), der jede Minute den Digest
   von `:latest` vergleicht und bei Änderung `docker compose pull && docker compose up -d` fährt.

**So prüfst du es:** `curl -s https://lagemonitor.iuk-ue.de/monitor/api/health` gibt
`{"status":"ok"}`, `…/monitor/api/monitor` ohne Anmeldung `401`. Im Browser führt
`https://lagemonitor.iuk-ue.de/monitor/` zu Pocket ID und danach zum Monitor. In der Suite steht
beim Zugang nach spätestens einer Minute „zuletzt abgeholt“.
