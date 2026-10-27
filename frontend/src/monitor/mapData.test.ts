import { RADAR_MAX_AGE_MS, radarAt, warningAreas } from './mapData';
import { NOW, makeView, makeWarning } from './testViews';

const square = {
  type: 'FeatureCollection',
  features: [
    {
      type: 'Feature',
      properties: { name: 'LK' },
      geometry: { type: 'Polygon', coordinates: [[[10, 52], [11, 52], [11, 53], [10, 52]]] },
    },
  ],
};

test('Warngebiete tragen ihre Art, Warnungen ohne Geometrie fallen still weg', () => {
  const fc = warningAreas(
    [makeWarning({ geometry: square })],
    [makeWarning({ geometry: null }), makeWarning({ geometry: { type: 'Polygon' } }), makeWarning({ geometry: square })],
  );
  expect(fc.features.map((f) => f.properties)).toEqual([
    { name: 'LK', kind: 'band' },
    { name: 'LK', kind: 'listed' },
  ]);
});

test('Radar nur bei Wetterstand jünger als 15 Minuten', () => {
  const weather = makeView().weather;
  expect(radarAt({ ...weather, fetchedAtMs: NOW - RADAR_MAX_AGE_MS }, NOW)).toBe(NOW - RADAR_MAX_AGE_MS);
  expect(radarAt({ ...weather, fetchedAtMs: NOW - RADAR_MAX_AGE_MS - 1 }, NOW)).toBeNull();
  expect(radarAt({ ...weather, stale: true }, NOW)).toBeNull();
  expect(radarAt({ ...weather, data: null, fetchedAtMs: null }, NOW)).toBeNull();
});
