const RAD = Math.PI / 180;

/** Sonnenhöhe in Grad (Näherung nach dem Astronomical Almanac, genau auf etwa 0,1°). */
export function sunAltitude(lat: number, lon: number, atMs: number): number {
  const d = atMs / 86_400_000 - 10_957.5; // Tage seit J2000.0
  const g = (357.529 + 0.98560028 * d) * RAD;
  const l = (280.459 + 0.98564736 * d + 1.915 * Math.sin(g) + 0.02 * Math.sin(2 * g)) * RAD;
  const e = (23.439 - 0.00000036 * d) * RAD;
  const ra = Math.atan2(Math.cos(e) * Math.sin(l), Math.cos(l));
  const dec = Math.asin(Math.sin(e) * Math.sin(l));
  const hourAngle = (18.697374558 + 24.06570982441908 * d) * 15 * RAD + lon * RAD - ra;
  const phi = lat * RAD;
  return Math.asin(Math.sin(phi) * Math.sin(dec) + Math.cos(phi) * Math.cos(dec) * Math.cos(hourAngle)) / RAD;
}

/** Tag zwischen Sonnenauf- und -untergang (Oberrand am Horizont, mit Refraktion). */
export const isDaylight = (lat: number, lon: number, atMs: number) => sunAltitude(lat, lon, atMs) > -0.833;
