import { RADAR_MAX_AGE_MS, TRAFFIC_TILE_MS, VEHICLE_STALE_MS, incidentFeatures, radarAt, speedCameraFeatures, trafficTiles, vehicleMarks, warningAreas } from './mapData';
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

test('Verkehrskacheln über den eigenen Server, neuer Stand alle 30 Minuten', () => {
  const base = 'https://lagemonitor.example/monitor/';
  const a = trafficTiles(true, NOW, base);
  expect(a).toBe(`https://lagemonitor.example/monitor/api/verkehr/fluss/dunkel/{z}/{x}/{y}?stand=${Math.floor(NOW / TRAFFIC_TILE_MS)}`);
  expect(trafficTiles(true, NOW + 60_000, base)).toBe(a);
  expect(trafficTiles(true, NOW + TRAFFIC_TILE_MS, base)).not.toBe(a);
  expect(trafficTiles(false, NOW, base)).toContain('/hell/');
});

test('Baustellen ohne brauchbare Geometrie fallen weg, Blitzer werden Punkte', () => {
  const incident = { id: 'i1', kind: 'roadWorks' as const, description: null, from: null, to: null, roadNumbers: [], endTime: null };
  const fc = incidentFeatures([
    { ...incident, geometry: { type: 'LineString', coordinates: [[10, 52], [10.1, 52]] } },
    { ...incident, id: 'i2', geometry: { type: 'Polygon', coordinates: [] } },
  ]);
  expect(fc.features.map((f) => f.properties)).toEqual([{ id: 'i1', kind: 'roadWorks' }]);
  const cams = speedCameraFeatures([{ id: 'c1', lat: 52.9, lon: 10.5, maxspeed: null }]);
  expect(cams.features[0]).toEqual({ type: 'Feature', properties: { id: 'c1', maxspeed: '' }, geometry: { type: 'Point', coordinates: [10.5, 52.9] } });
});

test('Fahrzeuge: das eigene am GPS-Fix, fremde an der Meldung, nach zehn Minuten veraltet mit Uhrzeit', () => {
  const svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"/>';
  const at = NOW - VEHICLE_STALE_MS;
  const fremd = { id: 'f', callSign: 'Rotkreuz Uelzen 83-1', symbol: svg, lat: 53, lon: 10.4, atMs: at, own: false };
  const eigen = { ...fremd, id: 'e', callSign: 'Rotkreuz Uelzen 40-12-1', atMs: at - 3_600_000, own: true };
  const [f, e] = vehicleMarks([fremd, eigen], NOW, { lat: 52.96, lon: 10.57 });
  expect(f).toMatchObject({ lat: 53, lon: 10.4, stand: null });
  expect(f.src).toBe(`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`);
  expect(e).toMatchObject({ lat: 52.96, lon: 10.57, stand: null });
  const [alt] = vehicleMarks([{ ...fremd, atMs: at - 1 }], NOW, { lat: 0, lon: 0 });
  expect(alt.stand).toMatch(/^Stand \d{2}:\d{2}$/);
});
