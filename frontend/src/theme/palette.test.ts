import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import type { NoticeColor } from '../contract/NoticeColor';
import { LAGE_FARBEN, lageFarbAusdruck } from './palette';

const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), '../monitor/monitor.css'), 'utf-8');

/** `--lm-kat` einer Meldekategorie aus `monitor.css`, am Tag oder in der Nacht. */
function kategorie(farbe: NoticeColor, dunkel: boolean): string | undefined {
  const vor = dunkel ? "\\[data-theme='dark'\\] " : '^';
  return new RegExp(`${vor}\\.lm-meldung\\[data-farbe='${farbe}'\\] \\{ --lm-kat: (#[0-9a-f]{6});`, 'm').exec(css)?.[1];
}

test('Flächen tragen dieselben Töne wie die Meldekategorien', () => {
  for (const farbe of ['teal', 'green', 'ochre', 'orange', 'purple', 'grey'] as const) {
    expect(LAGE_FARBEN.hell[farbe]).toBe(kategorie(farbe, false));
    expect(LAGE_FARBEN.dunkel[farbe]).toBe(kategorie(farbe, true));
  }
});

test('Der Kartenausdruck kennt jeden Schlüssel und fällt auf Grau zurück', () => {
  const a = lageFarbAusdruck(true);
  expect(a.slice(0, 2)).toEqual(['match', ['get', 'farbe']]);
  expect(a).toContain('purple');
  expect(a.at(-1)).toBe(LAGE_FARBEN.dunkel.grey);
});
