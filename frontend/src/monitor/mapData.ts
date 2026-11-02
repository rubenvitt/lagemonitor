import type { Sourced } from '../contract/Sourced';
import type { SpeedCamera } from '../contract/SpeedCamera';
import type { TrafficIncident } from '../contract/TrafficIncident';
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

/** So lange gilt eine Verkehrskachel auf dem Server (`src/web/traffic.rs`, `FLOW_TTL_MS`). */
export const TRAFFIC_TILE_MS = 30 * 60_000;

/**
 * Kacheln des Verkehrsflusses über den eigenen Server (der TomTom-Schlüssel bleibt dort). `stand`
 * wechselt alle 30 Minuten, dann holt die Karte neue Kacheln; alle Bildschirme im selben Takt.
 */
export function trafficTiles(dark: boolean, nowMs: number, base: string = document.baseURI): string {
  const stand = Math.floor(nowMs / TRAFFIC_TILE_MS);
  return `${new URL('api/verkehr/fluss/', base).href}${dark ? 'dunkel' : 'hell'}/{z}/{x}/{y}?stand=${stand}`;
}

type Collection = { type: 'FeatureCollection'; features: AreaFeature[] };

/** Baustellen und Sperrungen; `kind` und ein kurzer Text je Feature, Geometrie wie von TomTom. */
export function incidentFeatures(incidents: TrafficIncident[]): Collection {
  return {
    type: 'FeatureCollection',
    features: incidents
      .filter((i) => {
        const g = i.geometry as { type?: unknown } | null;
        return g?.type === 'LineString' || g?.type === 'Point';
      })
      .map((i) => ({ type: 'Feature', properties: { id: i.id, kind: i.kind }, geometry: i.geometry })),
  };
}

/** Feste Blitzer als Punkte, `maxspeed` als Beschriftung (leer, wenn OSM keine kennt). */
export function speedCameraFeatures(cameras: SpeedCamera[]): Collection {
  return {
    type: 'FeatureCollection',
    features: cameras.map((c) => ({
      type: 'Feature',
      properties: { id: c.id, maxspeed: c.maxspeed ?? '' },
      geometry: { type: 'Point', coordinates: [c.lon, c.lat] },
    })),
  };
}
