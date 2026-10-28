import { describe, expect, it } from 'vitest';
import { taktisch, taktischText } from './format';

describe('taktisch', () => {
  it('schreibt Tag, Uhrzeit, Monat und Jahr ohne Trenner', () => {
    expect(taktischText(new Date(2026, 9, 28, 7, 9).getTime())).toBe('280709okt26');
  });

  it('füllt einstellige Werte mit Null auf und kennt den März', () => {
    expect(taktisch(new Date(2027, 2, 1, 0, 5).getTime())).toEqual({ tag: '01', zeit: '0005', monatJahr: 'mär27' });
  });

  it('zeigt einen unlesbaren Zeitpunkt nicht als NaN', () => {
    expect(taktischText('kein Datum')).toBe('unbekannt');
  });
});
