// Übernommen aus Suchlage d621418 (27.10.2026): frontend/src/map/basemapStyle.test.ts
import { basemapStyle } from './basemapStyle';

const ONLINE = { offline: false };

const tileUrls = (offline: boolean) =>
  (basemapStyle('dark', { offline }).sources.basemap as { tiles: string[] }).tiles;

test('offline: Kacheln kommen vom eigenen Backend', () => {
  expect(tileUrls(true)).toEqual(['/api/tiles/{z}/{x}/{y}']);
});

test('online: Kacheln kommen von VersaTiles', () => {
  expect(tileUrls(false)).toEqual(['https://tiles.versatiles.org/tiles/osm/{z}/{x}/{y}']);
});

// Ein falscher Fontstack fällt nur draußen auf: Die Karte zeichnet, die Beschriftung fehlt still.
test.each(['dark', 'light'] as const)('%s: Beschriftung nutzt den Fontstack von VersaTiles', (theme) => {
  const style = basemapStyle(theme, ONLINE);
  expect(style.glyphs).toBe('https://tiles.versatiles.org/assets/glyphs/{fontstack}/{range}.pbf');
  const fonts = style.layers.flatMap((l) => {
    const font = (l.layout as { 'text-font'?: string[] } | undefined)?.['text-font'];
    return font ?? [];
  });
  expect(fonts.length).toBeGreaterThan(0);
  expect(new Set(fonts)).toEqual(new Set(['noto_sans_regular']));
});

test('Hell und Dunkel unterscheiden sich, Quelle und Namen bleiben gleich', () => {
  const dark = basemapStyle('dark', ONLINE);
  const light = basemapStyle('light', ONLINE);
  expect(dark.layers.map((l) => l.id)).toEqual(light.layers.map((l) => l.id));
  expect(dark.layers[0]).not.toEqual(light.layers[0]);
  expect(dark.sources).toEqual(light.sources);
  expect(new Set(dark.layers.map((l) => l.id)).size).toBe(dark.layers.length);
});
