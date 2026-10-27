import type { Sourced } from '../contract/Sourced';
import type { Warning } from '../contract/Warning';
import type { Weather } from '../contract/Weather';

interface AreaFeature {
  type: 'Feature';
  properties: Record<string, unknown>;
  geometry: unknown;
}

/**
 * Warngebiete als eine FeatureCollection, jedes Feature mit `kind` (`band` oder `listed`) für die
 * Farbe. `geometry` kommt untypisiert aus NINA; was keine FeatureCollection ist, fällt weg.
 */
export function warningAreas(band: Warning[], listed: Warning[]): { type: 'FeatureCollection'; features: AreaFeature[] } {
  const features: AreaFeature[] = [];
  for (const [kind, warnings] of [['band', band], ['listed', listed]] as const) {
    for (const w of warnings) {
      const g = w.geometry as { type?: unknown; features?: unknown } | null;
      if (g?.type !== 'FeatureCollection' || !Array.isArray(g.features)) continue;
      for (const f of g.features as AreaFeature[]) features.push({ ...f, properties: { ...f.properties, kind } });
    }
  }
  return { type: 'FeatureCollection', features };
}

/** DWD-Niederschlagsradar (WMS, Layer per GetCapabilities geprüft). `stand` erzwingt neue Kacheln je Wetterstand. */
export function radarTiles(fetchedAtMs: number): string {
  return (
    'https://maps.dwd.de/geoserver/dwd/wms?service=WMS&version=1.1.1&request=GetMap' +
    '&layers=dwd:Niederschlagsradar&styles=&format=image/png&transparent=true' +
    `&srs=EPSG:3857&width=256&height=256&bbox={bbox-epsg-3857}&stand=${fetchedAtMs}`
  );
}

/** Älter als das zeigt das Radar zu alten Regen: Der DWD-Layer ist ohne Zeitangabe immer „jetzt“. */
export const RADAR_MAX_AGE_MS = 15 * 60_000;

/** Wetterstand, zu dem das Radar gezeigt wird, oder `null` (Radar aus). `nowMs` in Serverzeit. */
export function radarAt(weather: Sourced<Weather>, nowMs: number): number | null {
  const at = weather.fetchedAtMs;
  return weather.stale || at === null || nowMs - at > RADAR_MAX_AGE_MS ? null : at;
}
