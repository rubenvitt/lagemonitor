// Testdaten für die Monitor-Ansicht. Erfundene Werte, keine Einsatzdaten.
import type { MonitorView } from '../contract/MonitorView';
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
    ...over,
  };
}
