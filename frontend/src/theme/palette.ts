import type { ExpressionSpecification } from 'maplibre-gl';
import type { NoticeColor } from '../contract/NoticeColor';
import { farbenDunkel, farbenHell } from './tokens';

/**
 * Töne der Palette für die Flächen der Lagekarte, je Thema. Dieselben wie an den Meldekategorien
 * (`monitor.css`, `.lm-meldung[data-farbe]`, Kante und Kürzel), Blau ist dort `--lfh-bedien`;
 * `palette.test.ts` hält beide gleich. MapLibre kennt keine Custom Properties, darum die Werte hier.
 */
export const LAGE_FARBEN: Record<'hell' | 'dunkel', Record<NoticeColor, string>> = {
  hell: {
    blue: farbenHell.bedien,
    teal: '#0b6a6a',
    green: '#1c6640',
    ochre: '#7a5200',
    orange: '#a34700',
    purple: '#5b3a9e',
    grey: '#4a525b',
  },
  dunkel: {
    blue: farbenDunkel.bedien,
    teal: '#2fb8b1',
    green: '#5cbf7a',
    ochre: '#e8cc3a',
    orange: '#f29a4a',
    purple: '#b07cff',
    grey: '#9aa2ab',
  },
};

export function lageFarbe(farbe: NoticeColor, dark: boolean): string {
  return LAGE_FARBEN[dark ? 'dunkel' : 'hell'][farbe];
}

/** MapLibre-Ausdruck: Ton aus dem Palettenschlüssel `farbe` eines Features; Unbekanntes grau. */
export function lageFarbAusdruck(dark: boolean): ExpressionSpecification {
  const t = LAGE_FARBEN[dark ? 'dunkel' : 'hell'];
  const paare = (Object.keys(t) as NoticeColor[]).flatMap((k) => [k, t[k]]);
  return ['match', ['get', 'farbe'], ...paare, t.grey] as unknown as ExpressionSpecification;
}
