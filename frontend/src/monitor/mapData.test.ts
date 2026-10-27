import { warningAreas } from './mapData';
import { makeWarning } from './testViews';

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
