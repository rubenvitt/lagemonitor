import { TRACK_COLORS } from '../map/suchlageFarben';
import type { SuchlageAircraft } from '../contract/SuchlageAircraft';
import type { SuchlageView } from '../contract/SuchlageView';

interface Feature {
  type: 'Feature';
  properties: Record<string, unknown>;
  geometry: { type: string; coordinates: unknown };
}

export interface Collection {
  type: 'FeatureCollection';
  features: Feature[];
}

const EMPTY: Collection = { type: 'FeatureCollection', features: [] };

/** `areas`/`tracks` kommen untypisiert vom Server; was keine FeatureCollection ist, fällt weg. */
function asCollection(raw: unknown): Collection {
  const c = raw as { type?: unknown; features?: unknown } | null;
  return c?.type === 'FeatureCollection' && Array.isArray(c.features) ? (c as Collection) : EMPTY;
}

/** Spuren mit Farbe nach Flug (aufsteigend nach Flug-ID, damit ein Flug seine Farbe behält). */
export function coloredTracks(view: SuchlageView): Collection {
  const tracks = asCollection(view.tracks);
  const ids = [...new Set(tracks.features.map((f) => Number(f.properties.flightId)))].sort((a, b) => a - b);
  return {
    type: 'FeatureCollection',
    features: tracks.features.map((f) => ({
      ...f,
      properties: { ...f.properties, color: TRACK_COLORS[ids.indexOf(Number(f.properties.flightId)) % TRACK_COLORS.length] },
    })),
  };
}

export function areas(view: SuchlageView): Collection {
  return asCollection(view.areas);
}

export function aircraftPoints(aircraft: SuchlageAircraft[]): Collection {
  return {
    type: 'FeatureCollection',
    features: aircraft.map((a) => ({
      type: 'Feature',
      properties: { airborne: a.airborne, label: a.label ?? a.gatewaySn },
      geometry: { type: 'Point', coordinates: [a.lon, a.lat] },
    })),
  };
}

function* positions(coords: unknown): Generator<[number, number]> {
  if (!Array.isArray(coords)) return;
  if (typeof coords[0] === 'number' && typeof coords[1] === 'number') {
    yield [coords[0], coords[1]];
    return;
  }
  for (const c of coords) yield* positions(c);
}

export type Bounds = [[number, number], [number, number]];

/** Umgebendes Rechteck über Gebiete, Spuren und Drohnen; `null` ohne Geometrie. */
export function suchlageBounds(view: SuchlageView): Bounds | null {
  let minLon = Infinity;
  let minLat = Infinity;
  let maxLon = -Infinity;
  let maxLat = -Infinity;
  const all = [...areas(view).features, ...asCollection(view.tracks).features, ...aircraftPoints(view.aircraft).features];
  for (const f of all) {
    for (const [lon, lat] of positions(f.geometry?.coordinates)) {
      minLon = Math.min(minLon, lon);
      minLat = Math.min(minLat, lat);
      maxLon = Math.max(maxLon, lon);
      maxLat = Math.max(maxLat, lat);
    }
  }
  return minLon === Infinity ? null : [[minLon, minLat], [maxLon, maxLat]];
}

/**
 * Schlüssel, bei dessen Wechsel die Karte neu einpasst: gerundet auf etwa 500 m, sonst zöge eine
 * wachsende Spur die Karte alle fünf Sekunden nach.
 */
export function boundsKey(b: Bounds | null): string | null {
  if (!b) return null;
  return b.flat().map((v) => Math.round(v * 200)).join(',');
}

/** Breite des Drohnenbilds als Anteil der Inhaltsfläche (gleicher Wert wie `.lm-drohne` in `monitor.css`). */
export const DROHNE_ANTEIL = 0.46;
/** Anteil der Karte, solange die Wetterspalte daneben steht (`.lm-spalten`, 2.2fr : 1fr). */
export const KARTE_ANTEIL = 2.2 / 3.2;

/** So viel der Kartenbreite (Anteil 0–1) verdeckt das eingefahrene Drohnenbild. */
export function coveredByDrone(withSideColumn: boolean): number {
  const map = withSideColumn ? KARTE_ANTEIL : 1;
  return Math.max(0, DROHNE_ANTEIL - (1 - map)) / map;
}
