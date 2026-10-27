// Übernommen aus Suchlage d621418 (27.10.2026): frontend/src/lib/sichererSpeicher.ts
// Übernommen aus Lifeline Hub 3eb9115f2 (16.10.2026): frontend/src/lib/sichererSpeicher.ts
/**
 * Der einzige Zugang zum Browserspeicher (`localStorage`), LFH-942,
 * `openspec/changes/archive/2026-10-06-lfh-942-browserspeicher-abgesichert/design.md` D1.
 *
 * Auf einem gehärteten Rechner (Firefox mit `dom.storage.enabled=false`) ist `localStorage`
 * `null`, in einer Einbettung wirft schon der Zugriff einen SecurityError, und ein volles
 * Kontingent wirft beim Schreiben. Jede Funktion hier fängt das ab und fällt zurück: Ohne
 * Speicher arbeitet die App mit ihren Vorgaben weiter, statt beim ersten Render auszufallen.
 * `sichererSpeicher.guard.test.ts` hält jeden anderen Zugriff fern.
 */

/** Der gespeicherte Wert, oder `null`, wenn es keinen gibt oder der Speicher gesperrt ist. */
export function sicherLesen(schluessel: string): string | null {
  try {
    return globalThis.localStorage.getItem(schluessel);
  } catch {
    return null;
  }
}

/** Schreibt den Wert; `false`, wenn der Speicher gesperrt oder voll ist. */
export function sicherSchreiben(schluessel: string, wert: string): boolean {
  try {
    globalThis.localStorage.setItem(schluessel, wert);
    return true;
  } catch {
    return false;
  }
}

/** Entfernt den Wert; `false`, wenn der Speicher gesperrt ist. */
export function sicherEntfernen(schluessel: string): boolean {
  try {
    globalThis.localStorage.removeItem(schluessel);
    return true;
  } catch {
    return false;
  }
}

/** Alle Schlüssel im Speicher, leer bei gesperrtem Speicher. */
export function sicherSchluessel(): string[] {
  try {
    const speicher = globalThis.localStorage;
    const schluessel: string[] = [];
    for (let i = 0; i < speicher.length; i++) {
      const k = speicher.key(i);
      if (k !== null) schluessel.push(k);
    }
    return schluessel;
  } catch {
    return [];
  }
}
