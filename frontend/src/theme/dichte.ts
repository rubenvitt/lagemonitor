// Übernommen aus Suchlage d621418 (27.10.2026): frontend/src/theme/dichte.ts
// Übernommen aus Lifeline Hub 3eb9115f2 (16.10.2026): frontend/src/theme/dichte.ts
/**
 * Woraus die Bediendichte beim Sitzungsstart folgt (LFH-724, Spec `bedien-dichte`).
 * Herleitung: `openspec/changes/archive/2026-10-01-lfh-724-dichtestufe-aus-dem-geraet/design.md`.
 *
 * DER EINSATZKONTEXT IST DAS GERÄT, NICHT DIE PERSON (Entscheidung 01.10.2026). Die
 * Bedien-Leitlinie schneidet ihre vier Kontexte nach Gerät, Hand und Haltung; eine S-Funktion
 * sagt nichts darüber, ob jemand am Fükw sitzt oder mit Handschuh am Tablet steht, und die
 * Spec `bedien-arbeitsplatz` verbietet die Vorbelegung über den Arbeitsplatz. Deshalb kennt
 * dieses Modul weder Einsatz noch Rolle — `dichteQuelle.guard.test.ts` lässt nur `./tokens` als
 * Import zu.
 *
 * Reihenfolge: gespeicherte Wahl → Zeigerart (grob → `komfortabel`) → `kompakt`.
 * `handschuh` entsteht nie aus einer Ableitung, nur aus der Wahl: eine automatische
 * Erkennung ist nicht belegt, und ein Fehlschalten mitten im Einsatz kostet mehr als ein
 * Schalter. Gelesen wird einmal beim Sitzungsstart; der Provider hängt bewusst keinen
 * Zuhörer an die Zeigerart, sonst spränge das Layout unter dem Finger.
 */
import type { Dichte } from './tokens';

/** Ausgangsstufe ohne gespeicherte Wahl: der Fükw-Arbeitsplatz (A1 Festlegung 1). */
export const DICHTE_DEFAULT: Dichte = 'kompakt';

/** …und die Ausgangsstufe, wenn der primäre Zeiger grob ist. */
export const DICHTE_DEFAULT_BERUEHRUNG: Dichte = 'komfortabel';

export function istDichte(wert: string | null): wert is Dichte {
  return wert === 'kompakt' || wert === 'komfortabel' || wert === 'handschuh';
}

/**
 * Die Stufe, mit der eine Sitzung beginnt. Eine getroffene Wahl gewinnt IMMER, die Zeigerart
 * belegt nur vor; sonst drehte sich der Dichte-Umschalter beim Neuladen selbst zurück. Ein
 * unbekannter gespeicherter Wert ist keine Wahl und fällt auf das Zeigersignal, nicht
 * pauschal auf `kompakt`.
 */
export function startDichte(gespeichert: string | null, zeigerGrob: boolean): Dichte {
  if (istDichte(gespeichert)) return gespeichert;
  return zeigerGrob ? DICHTE_DEFAULT_BERUEHRUNG : DICHTE_DEFAULT;
}
