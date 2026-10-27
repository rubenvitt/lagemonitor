import type { MonitorView } from '../contract/MonitorView';
import type { Severity } from '../contract/Severity';
import type { Warning } from '../contract/Warning';

export interface Layout {
  mode: 'ruhe' | 'warnung';
  /** `unknown`: nie ein Warnstand empfangen. `stale`: der letzte Stand ist veraltet. */
  warnState: 'known' | 'stale' | 'unknown';
  band: Warning[];
  listed: Warning[];
}

/** Solange nichts vom Server kam, weiß niemand, ob Warnungen anliegen. */
export const UNKNOWN_LAYOUT: Layout = { mode: 'ruhe', warnState: 'unknown', band: [], listed: [] };

const RANK: Record<Severity, number> = { Unknown: 0, Minor: 1, Moderate: 2, Severe: 3, Extreme: 4 };

const onsetMs = (w: Warning) => (w.onset ? Date.parse(w.onset) : Infinity);
const bySeverityThenOnset = (a: Warning, b: Warning) =>
  RANK[b.severity] - RANK[a.severity] || onsetMs(a) - onsetMs(b);

/**
 * Teilt die Warnungen in Band (ab Schwelle) und Liste (der Rest). `nowMs` ist die Uhr des
 * Bildschirms, nicht `view.nowMs`: Ist der Server weg, steht dessen Zeit still, und abgelaufene
 * Warnungen blieben sonst für immer stehen.
 */
export function selectLayout(view: MonitorView, nowMs: number): Layout {
  const { data, stale } = view.warnings;
  const active = (data ?? [])
    // Unlesbares `expires` (NaN) bleibt im Zweifel stehen.
    .filter((w) => !w.expires || !(Date.parse(w.expires) < nowMs))
    .sort(bySeverityThenOnset);
  const inBand = (w: Warning) => w.severity !== 'Unknown' && RANK[w.severity] >= RANK[view.warnThreshold];
  const band = active.filter(inBand);
  return {
    mode: band.length > 0 ? 'warnung' : 'ruhe',
    warnState: data === null ? 'unknown' : stale ? 'stale' : 'known',
    band,
    listed: active.filter((w) => !inBand(w)),
  };
}

/** Ohne frische Antwort seit so langer Zeit gilt der Stand auf dem Bildschirm als veraltet (drei Abrufe à 30 s). */
export const CLIENT_STALE_MS = 90_000;

/**
 * Bei einem Fehler oder einer pausierten Abfrage (Netz weg, `fetchStatus: 'paused'`, kein
 * `isError`) behält react-query den letzten Stand. Der darf dann nicht mehr als frisch gelten.
 */
export function isClientStale(isError: boolean, dataUpdatedAt: number, nowMs: number): boolean {
  return isError || nowMs - dataUpdatedAt > CLIENT_STALE_MS;
}

/** Markiert beide Quellen als veraltet. */
export function markStale(view: MonitorView, stale: boolean): MonitorView {
  if (!stale) return view;
  return {
    ...view,
    weather: { ...view.weather, stale: true },
    warnings: { ...view.warnings, stale: true },
  };
}
