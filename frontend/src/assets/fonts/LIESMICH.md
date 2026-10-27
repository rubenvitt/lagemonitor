# Schriften — Herkunft und Lizenz

Die Schriftrollen der Gestaltungssprache **E · Lagekarte nachts** (LFH-352 · A0). Alle Schnitte
sind **lokal ausgeliefert** und werden von Vite ins Bundle gehasht, von rust-embed ins Binary.
**Kein Google-Fonts-CDN, kein externer Abruf zur Laufzeit** — der Fükw arbeitet ohne Netz.

Alle Familien stehen unter der **SIL Open Font License 1.1**. Der Lizenztext liegt je Familie
unter `lizenzen/` und muss bei jeder Weitergabe der Schriftdateien mitgehen — das verlangt die
OFL ausdrücklich; eine nackte `.woff2` im Repo erfüllt die Bedingung nicht.

| Rolle                                        | Familie        | Dateien                                              | Herkunft                  | Lizenz                            |
| -------------------------------------------- | -------------- | ---------------------------------------------------- | ------------------------- | --------------------------------- |
| **Text** — Fließtext, Formulare, Listen      | Archivo        | `archivo-{400,500,600,700}.woff2`                    | Omnibus-Type              | `lizenzen/OFL-Archivo.txt`        |
| **Display** — Köpfe, Sektionsmarken          | Archivo Narrow | `archivo-narrow-600.woff2`                           | Omnibus-Type              | `lizenzen/OFL-Archivo-Narrow.txt` |
| **Zahl** — Stärke, DTG, Koordinaten, Nummern | JetBrains Mono | `jetbrains-mono-{400,500,600}.woff2`                 | JetBrains s.r.o.          | `lizenzen/OFL-JetBrains-Mono.txt` |

Bezogen über den Fontsource-Spiegel (`cdn.jsdelivr.net/fontsource/fonts/…`) im `latin`-Subset
(U+0000–00FF — deutsche Diakritika und `ß` vollständig). **Archivo 600** und **JetBrains Mono 500**
kamen mit dem Neuentwurf „Instrumententafel" (21.09.2026) dazu — aus den npm-Paketen
`@fontsource/archivo@5.3.0` bzw. `@fontsource/jetbrains-mono@5.3.0` (`files/*-latin-{600,500}-normal.woff2`,
per `npm pack` entpackt, **nicht** als Projektabhängigkeit). Gegenprobe auf dieselbe Quelle:
die Bestandsdateien `archivo-500.woff2` und `jetbrains-mono-600.woff2` sind byte-gleich mit den
entsprechenden Dateien dieser Paketversionen. Lizenztexte aus dem
Google-Fonts-Repository (`github.com/google/fonts/ofl/<familie>/OFL.txt`).

## Budget

**131,4 KB in 8 Schnitten** (134 584 Byte, gemessen 27.10.2026; die Arimo-Datei aus Suchlage entfällt hier, weil Lagemonitor keine taktischen Zeichen setzt) — der Deckel aus
LFH-352 liegt bei 200 KB. Nachprüfbar mit
`ls -l frontend/src/assets/fonts/*.woff2 | awk '{s+=$5} END {print s}'`
— **nicht** mit `du`, das Blockgrößen zählt.

Die Kandidaten der Variantenrunde (IBM Plex, Atkinson Hyperlegible) sind mit der Entscheidung
entfernt worden. **Feineres Subsetting** — nur die tatsächlich benutzten Zeichen statt des
vollen `latin`-Bereichs — ist möglich und würde das Budget weiter drücken; es lohnt erst, wenn
der Zeichenbedarf der Anwendung stabil ist, und ist deshalb bewusst nicht gemacht.

## Warum diese Auswahl

Die Kriterien aus LFH-352 sind an einem gerenderten Specimen geprüft, nicht aus Beschreibungen
übernommen:

- **Echte tabellarische Ziffern.** JetBrains Mono trägt `tabular-nums` von Haus aus — bei
  Stärkeangaben (`5/6/24//35`), DTG-Zeiten, Koordinaten und Funkrufnamen ist Ziffernflattern
  ein Lesefehler, kein Schönheitsfehler.
- **Unverwechselbare Zeichen.** `1 l I`, `0 O`, `5 S`, `8 B` sind in JetBrains Mono über
  Strichstärke, Punkt in der Null und unterschiedliche Höhen sicher getrennt.
- **Vollständige deutsche Diakritika und `ß`.**
- **Mehrere Schnitte inkl. eines echten Halbfetten** für Kennzahlen (Archivo 500/600/700,
  JetBrains Mono 500 für Datenwerte).
- **Freie Lizenz** (SIL OFL 1.1).
