import { sicherLesen, sicherSchreiben } from './lib/sichererSpeicher';

/**
 * Neuer Stand nach einem Update: Der Server schreibt den Hash seiner eingebetteten Oberfläche in die
 * Seite (`<meta name="lagemonitor-stand">`) und in jede Antwort (`x-lagemonitor-stand`,
 * `src/static_files.rs`). Weicht die Antwort von der Seite ab, wurde neu ausgerollt, und eine offene
 * Seite (Kiosk im FüKw, Online-Instanz) lädt sich selbst neu. Ein Update nur am Server ändert den
 * Stand nicht und lädt nichts neu.
 */
export const STAND_HEADER = 'x-lagemonitor-stand';
const SPERRE_SCHLUESSEL = 'lagemonitor.neuladen';
/** Zeigt der Server nach dem Neuladen trotzdem den alten Stand, nicht im Kreis laden. */
export const SPERRE_MS = 10 * 60_000;

/** Der Stand, mit dem diese Seite geladen wurde; `null` im Vite-Dev-Server (dort vergleicht nichts). */
export function eigenerStand(doc: Document = document): string | null {
  return doc.querySelector<HTMLMetaElement>('meta[name="lagemonitor-stand"]')?.content || null;
}

/** Nur ein bekannter, abweichender Stand zählt. Fehlt einer, ist nichts zu entscheiden. */
export function istVeraltet(eigen: string | null, server: string | null): boolean {
  return !!eigen && !!server && eigen !== server;
}

/** Wurde für diesen Stand gerade schon neu geladen? */
export function gesperrt(ziel: string, jetzt: number, gespeichert: string | null): boolean {
  if (!gespeichert) return false;
  const [stand, zeit] = gespeichert.split('|');
  return stand === ziel && jetzt - Number(zeit) < SPERRE_MS;
}

/** Die gehashten Bundles, die eine `index.html` nachlädt. */
export function bundles(html: string): string[] {
  return [...html.matchAll(/(?:src|href)="(\.?\/?assets\/[^"]+)"/g)].map((m) => m[1]);
}

export interface Umgebung {
  holen: typeof fetch;
  neuLaden: () => void;
  jetzt: () => number;
  seite: string;
}

const browser = (): Umgebung => ({
  holen: (...a) => fetch(...a),
  neuLaden: () => window.location.reload(),
  jetzt: Date.now,
  seite: window.location.href,
});

let laeuft = false;

/**
 * Lädt die neue Seite und ihre Bundles vorab in den Cache und lädt erst dann neu: Das Neuladen geht
 * dann ohne Wartezeit, und es findet nur statt, wenn der Server die neue Seite wirklich ausliefert.
 * Die Sitzung (auch aus einem Link) hängt am Cookie, das `#`-Fragment wird dabei nicht gebraucht.
 */
export async function aufStandWechseln(ziel: string, u: Umgebung = browser()): Promise<boolean> {
  if (laeuft || gesperrt(ziel, u.jetzt(), sicherLesen(SPERRE_SCHLUESSEL))) return false;
  laeuft = true;
  try {
    const seite = await u.holen(u.seite, { cache: 'no-store' });
    if (!seite.ok) return false;
    const html = await seite.text();
    if (!html.includes(`content="${ziel}"`)) return false;
    await Promise.all(
      bundles(html).map((pfad) => u.holen(new URL(pfad, u.seite)).catch(() => undefined)),
    );
    sicherSchreiben(SPERRE_SCHLUESSEL, `${ziel}|${u.jetzt()}`);
    u.neuLaden();
    return true;
  } catch {
    return false;
  } finally {
    laeuft = false;
  }
}

/** Prüft eine Antwort des Servers und wechselt bei einem neuen Stand. */
export function standPruefen(res: Response, eigen: string | null = eigenerStand()): void {
  const server = res.headers.get(STAND_HEADER);
  if (server && istVeraltet(eigen, server)) void aufStandWechseln(server);
}
