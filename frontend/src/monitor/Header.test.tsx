import { render, screen } from '@testing-library/react';
import Header from './Header';
import { UNKNOWN_LAYOUT, selectLayout } from './selectLayout';
import { HOUR, NOW, makeView, makeWarning } from './testViews';

const renderHeader = (view = makeView()) =>
  render(<Header view={view} layout={selectLayout(view, NOW)} nowMs={NOW} />);

test('unknown_warnings_never_say_none', () => {
  renderHeader(makeView({ warnings: { data: null, fetchedAtMs: null, stale: true, error: null } }));
  expect(screen.getByText('WARNLAGE UNBEKANNT')).toBeInTheDocument();
  expect(screen.queryByText(/keine Warnungen/)).toBeNull();
});

test('stale_warnings_say_outdated', () => {
  renderHeader(makeView({ warnings: { data: [], fetchedAtMs: NOW - HOUR, stale: true, error: null } }));
  expect(screen.getByText(/^Warnstand veraltet · \d\d:\d\d$/)).toBeInTheDocument();
  expect(screen.queryByText(/keine Warnungen/)).toBeNull();
});

test('Ruhe nur bei bekanntem, leerem Warnstand', () => {
  renderHeader();
  expect(screen.getByText('RUHE · keine Warnungen')).toBeInTheDocument();
  expect(screen.getByText('LAGEMONITOR · Uelzen')).toBeInTheDocument();
});

test('Band zählt Warnungen, nennt die höchste Stufe und „veraltet“ bei altem Stand', () => {
  const warnings = [makeWarning({ id: 'a', severity: 'Extreme', levelLabel: 'Stufe 4' }), makeWarning({ id: 'b', severity: 'Severe' })];
  renderHeader(makeView({ warnings: { data: warnings, fetchedAtMs: NOW - HOUR, stale: true, error: null } }));
  expect(screen.getByText('2 WARNUNGEN · Stufe 4 · veraltet')).toBeInTheDocument();
});

test('Warnungen unter der Schwelle heißen nicht „keine Warnungen“', () => {
  renderHeader(makeView({ warnings: { data: [makeWarning({ severity: 'Minor' })], fetchedAtMs: NOW, stale: false, error: null } }));
  expect(screen.queryByText(/keine Warnungen/)).toBeNull();
  expect(screen.getByText('RUHE · 1 Warnung unter Schwelle')).toBeInTheDocument();
});

test('ohne Antwort vom Server: Warnlage unbekannt, mit Grund', () => {
  render(<Header layout={UNKNOWN_LAYOUT} nowMs={NOW} note="Server nicht erreichbar" />);
  expect(screen.getByText('WARNLAGE UNBEKANNT · Server nicht erreichbar')).toBeInTheDocument();
});
