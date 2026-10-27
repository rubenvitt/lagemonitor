// Übernommen aus Suchlage d621418 (27.10.2026): frontend/src/theme/helligkeit.ts
// Übernommen aus Lifeline Hub 3eb9115f2 (16.10.2026): frontend/src/theme/helligkeit.ts
/**
 * Die Helligkeitsachse und ihre Warnsperre (LFH-397, Kriterium 8 der Prüfliste
 * Einsatztauglichkeit: „1 Regler, 1 Sperre", MIL 5.2.2.1.9, 5.2.4.2.2.3).
 * Herleitung: `openspec/changes/archive/2026-09-29-lfh-397-helligkeitsregler-warnsperre/design.md`.
 *
 * EIN REGLER: Helligkeit, kein getrennter Kontrast. Den Kontrast trägt die Palette
 * (Kriterium 5); die Abdunklung senkt ihn — genau das begrenzt die Sperre.
 *
 * AUS IST KEINE STUFE. Die kleinste Stufe ist 20 %; im dunkeladaptierten Fükw liest man
 * dort noch. Über 100 % gibt es nichts: Software macht ein Display nicht heller, ein
 * `brightness(>1)` bliche nur die Farben aus.
 */

export const HELLIGKEIT_STUFEN = [100, 80, 60, 40, 20] as const;

export type Helligkeit = (typeof HELLIGKEIT_STUFEN)[number];

/** Ohne gespeicherte Wahl: volle Helligkeit. */
export const HELLIGKEIT_DEFAULT: Helligkeit = 100;

/**
 * Untergrenze, solange eine Warnung aktiv ist.
 *
 * ABGELEITET, NICHT GESETZT: die kleinste Stufe, bei der `alarmText` auf `grund` in
 * BEIDEN Paletten ≥ 4,5 : 1 hält (Kriterium 5, „nie < 4,5 : 1"; nachts 4,81 : 1, bei 60 %
 * wären es 3,07 : 1). `helligkeit.test.ts` rechnet das aus den Tokens nach — wer die
 * Palette ändert, bekommt dort den neuen Boden genannt.
 *
 * Für die FREIE Wahl taugt dieselbe Formel nicht: sie rechnet mit dem Streulicht eines
 * hellen Raums, nicht mit einem dunkeladaptierten Auge. Deshalb gilt sie nur hier.
 */
export const HELLIGKEIT_BODEN_WARNUNG: Helligkeit = 80;

export function istHelligkeit(wert: string | null): boolean {
  return HELLIGKEIT_STUFEN.some((s) => String(s) === wert);
}

/** Liest einen gespeicherten Wert; alles, was keine Stufe ist, ist keine Wahl. */
export function alsHelligkeit(wert: string | null): Helligkeit {
  return HELLIGKEIT_STUFEN.find((s) => String(s) === wert) ?? HELLIGKEIT_DEFAULT;
}

/**
 * DIE SPERRE. Bei aktiver Warnung wirkt mindestens der Boden, sonst genau die Wahl.
 *
 * Die Wahl selbst wird NIE überschrieben — die Sperre sitzt an der wirksamen Stufe,
 * nicht am Speicher. Endet die Warnung, kehrt die gewählte Stufe ohne Nachstellen zurück.
 */
export function wirksameHelligkeit(wahl: Helligkeit, warnungAktiv: boolean): Helligkeit {
  return warnungAktiv ? (Math.max(wahl, HELLIGKEIT_BODEN_WARNUNG) as Helligkeit) : wahl;
}

/** Deckkraft der schwarzen Abdunklungsschicht (`theme/rollen.css`). */
export function abdunkelung(stufe: Helligkeit): number {
  // Erst die Differenz, dann teilen: `1 - 80 / 100` ergäbe 0,19999999999999996 im CSS.
  return (100 - stufe) / 100;
}
