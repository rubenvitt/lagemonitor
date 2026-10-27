/** Uhrzeit in Ortszeit des Bildschirms, `HH:mm` bzw. `HH:mm:ss`. */
export function clock(at: number | string, seconds = false): string {
  return new Date(at).toLocaleTimeString('de-DE', {
    hour: '2-digit',
    minute: '2-digit',
    ...(seconds && { second: '2-digit' }),
  });
}

/** Zahl mit deutschem Komma. */
export function decimal(value: number, digits: number): string {
  return value.toLocaleString('de-DE', { minimumFractionDigits: digits, maximumFractionDigits: digits });
}
