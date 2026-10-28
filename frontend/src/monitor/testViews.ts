// Testdaten für die Monitor-Ansicht. Erfundene Werte, keine Einsatzdaten.
import type { MonitorView } from '../contract/MonitorView';
import type { SuchlageView } from '../contract/SuchlageView';
import type { Warning } from '../contract/Warning';

export const NOW = Date.parse('2026-10-27T12:00:00Z');
export const HOUR = 3_600_000;
const iso = (ms: number) => new Date(ms).toISOString();

export function makeWarning(over: Partial<Warning> = {}): Warning {
  return {
    id: 'w1',
    provider: 'dwd',
    event: 'STURMBÖEN',
    headline: 'Amtliche WARNUNG vor STURMBÖEN',
    description: '',
    instruction: null,
    severity: 'Moderate',
    levelLabel: 'Stufe 2',
    onset: iso(NOW - HOUR),
    expires: iso(NOW + HOUR),
    senderName: 'DWD',
    geometry: null,
    ...over,
  };
}

export function makeView(over: Partial<MonitorView> = {}): MonitorView {
  return {
    nowMs: NOW,
    location: { lat: 52.96, lon: 10.56, label: 'Uelzen' },
    region: { ars: '033600000000', name: 'Landkreis Uelzen' },
    warnThreshold: 'Severe',
    weather: {
      data: {
        current: {
          timestamp: iso(NOW),
          temperature: 11.4,
          condition: 'dry',
          icon: 'cloudy',
          windSpeed: 18,
          windGustSpeed: 34,
          windDirection: 225,
          precipitation: 0,
        },
        hourly: [],
      },
      fetchedAtMs: NOW,
      stale: false,
      error: null,
    },
    warnings: { data: [], fetchedAtMs: NOW, stale: false, error: null },
    map: { offline: false },
    modules: { weather: true, radar: true, warningAreas: true, warningList: true, suchlage: false },
    config: { fromSuite: false, fetchedAtMs: null, error: null },
    suchlage: null,
    notices: [],
    recruitment: null,
    ...over,
  };
}

/** Laufender Suchlage-Einsatz: ein Suchgebiet, zwei Flüge (einer läuft), eine Drohne mit Livebild. */
export function makeSuchlage(over: Partial<SuchlageView> = {}): SuchlageView {
  const line = (id: number, running: boolean, coords: number[][]) => ({
    type: 'Feature',
    properties: { flightId: id, running },
    geometry: { type: 'MultiLineString', coordinates: [coords] },
  });
  return {
    mission: { name: 'Vermisste Person Oldenstadt', location: 'Oldenstadt', section: 'Abschnitt Nord', startedAtMs: NOW - HOUR },
    areas: {
      type: 'FeatureCollection',
      features: [
        {
          type: 'Feature',
          // Datenwert aus Suchlage (Farbe des Kartenelements), keine Gestaltung.
          // eslint-disable-next-line no-restricted-syntax
          properties: { name: 'Suchgebiet A', color: '#2D6CDF' },
          geometry: { type: 'Polygon', coordinates: [[[10.55, 52.97], [10.58, 52.97], [10.58, 52.985], [10.55, 52.985], [10.55, 52.97]]] },
        },
      ],
    },
    tracks: {
      type: 'FeatureCollection',
      features: [
        line(9, true, [[10.556, 52.972], [10.562, 52.976]]),
        line(7, false, [[10.552, 52.971], [10.575, 52.983]]),
      ],
    },
    aircraft: [
      { gatewaySn: 'RC1', label: 'Drohne 1', lat: 52.976, lon: 10.562, headingDeg: 90, elevationM: 60, batteryPercent: 71, airborne: true },
    ],
    video: { gatewaySn: 'RC1', label: 'Drohne 1' },
    ...over,
  };
}
