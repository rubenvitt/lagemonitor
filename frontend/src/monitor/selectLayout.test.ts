import { isClientStale, markStale, selectLayout } from './selectLayout';
import { HOUR, NOW, makeView, makeWarning } from './testViews';

const withWarnings = (warnings: ReturnType<typeof makeWarning>[], threshold = 'Severe' as const) =>
  makeView({
    warnThreshold: threshold,
    warnings: { data: warnings, fetchedAtMs: NOW, stale: false, error: null },
  });

test('drops_expired_warnings', () => {
  const expired = makeWarning({ id: 'alt', severity: 'Extreme', expires: new Date(NOW - 1).toISOString() });
  const listedExpired = makeWarning({ id: 'alt2', severity: 'Minor', expires: new Date(NOW - HOUR).toISOString() });
  const open = makeWarning({ id: 'offen', severity: 'Minor', expires: null });
  const l = selectLayout(withWarnings([expired, listedExpired, open]), NOW);
  expect(l.band).toEqual([]);
  expect(l.listed.map((w) => w.id)).toEqual(['offen']);
  expect(l.mode).toBe('ruhe');
});

test('band_sorts_by_severity_and_counts_rest', () => {
  const severeLate = makeWarning({ id: 'severe-spaet', severity: 'Severe', onset: new Date(NOW).toISOString() });
  const severeEarly = makeWarning({ id: 'severe-frueh', severity: 'Severe', onset: new Date(NOW - 2 * HOUR).toISOString() });
  const extreme = makeWarning({ id: 'extrem', severity: 'Extreme' });
  const minor = makeWarning({ id: 'gering', severity: 'Minor' });
  const l = selectLayout(withWarnings([severeLate, minor, severeEarly, extreme]), NOW);
  expect(l.mode).toBe('warnung');
  expect(l.band.map((w) => w.id)).toEqual(['extrem', 'severe-frueh', 'severe-spaet']);
  expect(l.listed.map((w) => w.id)).toEqual(['gering']);
});

test('unknown_never_in_band', () => {
  const unknown = makeWarning({ id: 'unbekannt', severity: 'Unknown' });
  const l = selectLayout(withWarnings([unknown], 'Unknown' as never), NOW);
  expect(l.band).toEqual([]);
  expect(l.listed.map((w) => w.id)).toEqual(['unbekannt']);
});

test('threshold_moderate_includes_moderate', () => {
  const moderate = makeWarning({ id: 'mittel', severity: 'Moderate' });
  const minor = makeWarning({ id: 'gering', severity: 'Minor' });
  const l = selectLayout(withWarnings([minor, moderate], 'Moderate' as never), NOW);
  expect(l.band.map((w) => w.id)).toEqual(['mittel']);
  expect(l.listed.map((w) => w.id)).toEqual(['gering']);
});

test('ruhe_without_warnings', () => {
  const l = selectLayout(withWarnings([]), NOW);
  expect(l).toEqual({ mode: 'ruhe', warnState: 'known', band: [], listed: [] });
});

test('warn_state_unknown_and_stale', () => {
  const unknown = makeView({ warnings: { data: null, fetchedAtMs: null, stale: true, error: null } });
  expect(selectLayout(unknown, NOW).warnState).toBe('unknown');
  const stale = makeView({
    warnings: { data: [makeWarning({ severity: 'Extreme' })], fetchedAtMs: NOW - HOUR, stale: true, error: null },
  });
  const l = selectLayout(stale, NOW);
  expect(l.warnState).toBe('stale');
  // Ein Band aus einem veralteten Stand bleibt sichtbar.
  expect(l.band).toHaveLength(1);
});

test('onset_null_sorts_last_within_severity', () => {
  const noOnset = makeWarning({ id: 'ohne', severity: 'Severe', onset: null });
  const withOnset = makeWarning({ id: 'mit', severity: 'Severe' });
  expect(selectLayout(withWarnings([noOnset, withOnset]), NOW).band.map((w) => w.id)).toEqual(['mit', 'ohne']);
});

test('mark_stale_marks_both_sources', () => {
  // Server nicht erreichbar: react-query behält den letzten Stand, der darf nicht mehr als frisch gelten.
  const view = makeView();
  expect(markStale(view, false)).toBe(view);
  const failed = markStale(view, true);
  expect(failed.warnings.stale).toBe(true);
  expect(failed.weather.stale).toBe(true);
  expect(selectLayout(failed, NOW).warnState).toBe('stale');
});

test('client_stale_after_90s_or_on_error', () => {
  expect(isClientStale(false, NOW - 90_000, NOW)).toBe(false);
  expect(isClientStale(false, NOW - 90_001, NOW)).toBe(true);
  expect(isClientStale(true, NOW, NOW)).toBe(true);
});
