// Übernommen aus Suchlage d621418 (27.10.2026): frontend/src/theme/schriften.ts
// Übernommen aus Lifeline Hub 3eb9115f2 (16.10.2026): frontend/src/theme/schriften.ts
/* Gestaltungssprache „E · Lagekarte nachts" (LFH-352 · A0) — Schriftrollen.

   LOKAL AUSGELIEFERT, NIE ÜBER EIN CDN. Der Fükw arbeitet ohne Netz; eine
   Schrift, die erst geladen werden muss, ist im Einsatz keine. Vite hasht die
   woff2 ins Bundle, rust-embed nimmt sie ins Binary.

   Alle drei Familien stehen unter SIL OFL 1.1; Herkunft und Lizenztexte in
   `src/assets/fonts/LIESMICH.md` bzw. `src/assets/fonts/lizenzen/`.
   Gesamtbudget 131,4 KB in 8 Schnitten — der Deckel aus LFH-352 liegt bei 200 KB.
   Archivo 600 und JetBrains Mono 500 kamen mit dem Neuentwurf „Instrumententafel"
   (21.09.2026) dazu: Seitentitel/Augenbraue tragen 600, Datenwerte Mono 500.

   ROLLEN (die Namen stehen auch in `tokens.ts` und `rollen.css`):
     LFH Archivo         Fließtext, Formulare, Listen
     LFH Archivo Narrow  Köpfe und Sektionsmarken — kartografisch, schmal
     LFH JetBrains Mono  alle fachlichen Zahlen (Stärke, DTG, Koordinaten, Nummern)

   Angemeldet über die FontFace-API, nicht per `@font-face` (LFH-1108, `druck/AGENTS.md`,
   Schriften im Druck).

   `display: 'swap'` ist Absicht: lieber sofort in der Systemschrift lesbar
   als eine Sekunde lang gar nichts — im Einsatz zählt der erste Blick. */

import archivo400 from '../assets/fonts/archivo-400.woff2';
import archivo500 from '../assets/fonts/archivo-500.woff2';
import archivo600 from '../assets/fonts/archivo-600.woff2';
import archivo700 from '../assets/fonts/archivo-700.woff2';
import archivoNarrow600 from '../assets/fonts/archivo-narrow-600.woff2';
import jetbrainsMono400 from '../assets/fonts/jetbrains-mono-400.woff2';
import jetbrainsMono500 from '../assets/fonts/jetbrains-mono-500.woff2';
import jetbrainsMono600 from '../assets/fonts/jetbrains-mono-600.woff2';

/** Ein Schnitt einer Schriftrolle: Familie, Gewicht und Adresse der woff2. */
export interface Schriftschnitt {
  familie: string;
  gewicht: number;
  datei: string;
}

export const SCHRIFTSCHNITTE: readonly Schriftschnitt[] = [
  { familie: 'LFH Archivo', gewicht: 400, datei: archivo400 },
  { familie: 'LFH Archivo', gewicht: 500, datei: archivo500 },
  { familie: 'LFH Archivo', gewicht: 600, datei: archivo600 },
  { familie: 'LFH Archivo', gewicht: 700, datei: archivo700 },
  { familie: 'LFH Archivo Narrow', gewicht: 600, datei: archivoNarrow600 },
  { familie: 'LFH JetBrains Mono', gewicht: 400, datei: jetbrainsMono400 },
  { familie: 'LFH JetBrains Mono', gewicht: 500, datei: jetbrainsMono500 },
  { familie: 'LFH JetBrains Mono', gewicht: 600, datei: jetbrainsMono600 },
];

/**
 * Meldet alle Schnitte an. Was der Bildschirm braucht, lädt der Browser bei Bedarf wie bei
 * `@font-face`; den Rest lädt die App, sobald die Seite geladen ist und der Browser Luft hat, denn
 * der Druck setzt ihn (Archivo 700 in Markdown-Fettdruck). Scheitert ein Schnitt, steht der Text
 * in der Ersatzschrift.
 */
export function meldeSchriftenAn(menge: FontFaceSet = document.fonts): void {
  const schnitte = SCHRIFTSCHNITTE.map((s) => {
    const schnitt = new FontFace(s.familie, `url(${JSON.stringify(s.datei)}) format('woff2')`, {
      weight: String(s.gewicht),
      style: 'normal',
      display: 'swap',
    });
    menge.add(schnitt);
    return schnitt;
  });
  const ladeAlle = () => {
    for (const schnitt of schnitte) schnitt.load().catch(() => undefined);
  };
  const nachLeerlauf = () =>
    typeof requestIdleCallback === 'function'
      ? requestIdleCallback(ladeAlle, { timeout: 5000 })
      : setTimeout(ladeAlle, 1000);
  if (document.readyState === 'complete') nachLeerlauf();
  else window.addEventListener('load', nachLeerlauf, { once: true });
}
