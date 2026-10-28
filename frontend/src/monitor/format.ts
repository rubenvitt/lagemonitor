/** Uhrzeit in Ortszeit des Bildschirms, `HH:mm` bzw. `HH:mm:ss`. */
export function clock(at: number | string, seconds = false): string {
  return new Date(at).toLocaleTimeString('de-DE', {
    hour: '2-digit',
    minute: '2-digit',
    ...(seconds && { second: '2-digit' }),
  });
}

const MONATE = ['jan', 'feb', 'mär', 'apr', 'mai', 'jun', 'jul', 'aug', 'sep', 'okt', 'nov', 'dez'];

/** Taktische Zeit (Datum-Zeit-Gruppe) in Teilen, damit die Uhrzeit sich absetzen kann: `28` `0709` `okt26`. */
export interface TaktischeZeit {
  tag: string;
  zeit: string;
  monatJahr: string;
}

/** Taktische Zeit in Ortszeit des Bildschirms, etwa `280709okt26` für den 28.10.2026, 07:09 Uhr. */
export function taktisch(at: number | string): TaktischeZeit {
  const d = new Date(at);
  const zwei = (n: number) => String(n).padStart(2, '0');
  return {
    tag: zwei(d.getDate()),
    zeit: zwei(d.getHours()) + zwei(d.getMinutes()),
    monatJahr: MONATE[d.getMonth()] + zwei(d.getFullYear() % 100),
  };
}

/** Taktische Zeit als eine Zeichenkette, für Fließtext; ein unlesbarer Zeitpunkt heißt „unbekannt“. */
export function taktischText(at: number | string): string {
  if (Number.isNaN(new Date(at).getTime())) return 'unbekannt';
  const t = taktisch(at);
  return t.tag + t.zeit + t.monatJahr;
}

/** Zahl mit deutschem Komma. */
export function decimal(value: number, digits: number): string {
  return value.toLocaleString('de-DE', { minimumFractionDigits: digits, maximumFractionDigits: digits });
}
