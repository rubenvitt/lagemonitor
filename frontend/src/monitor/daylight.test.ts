import { isDaylight, sunAltitude } from './daylight';

const UELZEN = [52.966, 10.558] as const;
const at = (iso: string) => Date.parse(iso);

test('Mittag im Sommer: Sonne hoch', () => {
  // Mittagshöhe am 21.06. in Uelzen: 90 − 52,97 + 23,44 ≈ 60,5°
  expect(sunAltitude(...UELZEN, at('2026-06-21T11:18:00Z'))).toBeCloseTo(60.5, 0);
});

test('27.10.: Tag am Nachmittag, Nacht nach Sonnenuntergang (≈ 15:50 UTC)', () => {
  expect(isDaylight(...UELZEN, at('2026-10-27T15:30:00Z'))).toBe(true);
  expect(isDaylight(...UELZEN, at('2026-10-27T16:10:00Z'))).toBe(false);
});

test('Wintermorgen vor Sonnenaufgang (≈ 07:30 UTC) ist Nacht', () => {
  expect(isDaylight(...UELZEN, at('2026-12-21T07:00:00Z'))).toBe(false);
  expect(isDaylight(...UELZEN, at('2026-12-21T08:00:00Z'))).toBe(true);
});
