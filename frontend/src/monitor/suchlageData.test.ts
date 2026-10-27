import { TRACK_COLORS } from '../map/suchlageFarben';
import { aircraftPoints, boundsKey, coloredTracks, coveredByDrone, suchlageBounds } from './suchlageData';
import { makeSuchlage } from './testViews';

test('jeder Flug bekommt seine Farbe, nach Flug-ID', () => {
  const tracks = coloredTracks(makeSuchlage()).features;
  const color = (id: number) => tracks.find((f) => f.properties.flightId === id)?.properties.color;
  expect(color(7)).toBe(TRACK_COLORS[0]);
  expect(color(9)).toBe(TRACK_COLORS[1]);
});

test('Rahmen umfasst Gebiete, Spuren und Drohnen', () => {
  const b = suchlageBounds(makeSuchlage())!;
  expect(b[0][0]).toBeLessThanOrEqual(10.55);
  expect(b[1][1]).toBeGreaterThanOrEqual(52.985);
});

test('ohne Geometrie kein Rahmen', () => {
  const empty = makeSuchlage({ areas: { type: 'FeatureCollection', features: [] }, tracks: null, aircraft: [] });
  expect(suchlageBounds(empty)).toBeNull();
  expect(boundsKey(null)).toBeNull();
});

test('kleine Bewegungen ändern den Einpass-Schlüssel nicht', () => {
  const a = boundsKey([[10.5501, 52.97], [10.57, 52.98]]);
  const b = boundsKey([[10.5502, 52.97], [10.57, 52.98]]);
  expect(a).toBe(b);
});

test('Drohnen werden Punkte mit Bezeichnung', () => {
  const [p] = aircraftPoints(makeSuchlage().aircraft).features;
  expect(p.properties.label).toBe('Drohne 1');
  expect(p.geometry.coordinates).toEqual([10.562, 52.976]);
});

test('Drohnenbild verdeckt einen Teil der Karte, ohne Wetterspalte mehr', () => {
  expect(coveredByDrone(true)).toBeGreaterThan(0);
  expect(coveredByDrone(false)).toBeCloseTo(0.46);
  expect(coveredByDrone(false)).toBeGreaterThan(coveredByDrone(true));
});
