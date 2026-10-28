/**
 * Der Countdown einer Meldung in Worten: „in 42 min“, „in 1 h 5 min“, „jetzt“, „seit 3 min“. Gleich
 * gerechnet wie die Vorschau der IUK-Suite (`lagemonitor/_lib/vorschau.ts`, `countdownText`).
 */
export function countdownText(targetMs: number, nowMs: number): string {
  const d = targetMs - nowMs;
  if (d <= 0 && d > -60_000) return 'jetzt';
  const minutes = d > 0 ? Math.ceil(d / 60_000) : Math.floor(-d / 60_000);
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  const span = h === 0 ? `${m} min` : m === 0 ? `${h} h` : `${h} h ${m} min`;
  return d > 0 ? `in ${span}` : `seit ${span}`;
}
