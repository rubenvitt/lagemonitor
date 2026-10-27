import { render, screen } from '@testing-library/react';
import { vi } from 'vitest';
import { useMonitor } from '../api';
import MonitorPage from './MonitorPage';
import { makeSuchlage, makeView, makeWarning } from './testViews';

// maplibre läuft nicht in jsdom; die Karte ist hier nicht Gegenstand.
// Was über der Karte liegt (QR-Code), kommt als `children` und wird gerendert.
vi.mock('./MapPanel', () => ({ default: ({ children }: { children?: React.ReactNode }) => children ?? null }));
vi.mock('../api', () => ({ useMonitor: vi.fn() }));
// WebRTC gibt es in jsdom nicht; der Player hat eigene Tests in Suchlage.
vi.mock('../video/usePlayer', () => ({ usePlayer: () => ({ kind: 'playing', transport: 'webrtc', attempt: 1 }) }));

type Query = ReturnType<typeof useMonitor>;
const answer = (q: Partial<Query>) =>
  vi.mocked(useMonitor).mockReturnValue({ data: undefined, isError: false, isPending: false, dataUpdatedAt: 0, ...q } as Query);

test('lädt: Warnlage unbekannt', () => {
  answer({ isPending: true });
  render(<MonitorPage />);
  expect(screen.getByText('Lädt …')).toBeInTheDocument();
  expect(screen.getByText('WARNLAGE UNBEKANNT · lädt')).toBeInTheDocument();
});

test('Fehler ohne Daten: Server nicht erreichbar, nie „keine Warnungen“', () => {
  answer({ isError: true });
  render(<MonitorPage />);
  expect(screen.getByText('WARNLAGE UNBEKANNT · Server nicht erreichbar')).toBeInTheDocument();
  expect(screen.queryByText(/keine Warnungen/)).toBeNull();
});

test('frischer Stand: Ruhe', () => {
  answer({ data: makeView(), dataUpdatedAt: Date.now() });
  render(<MonitorPage />);
  expect(screen.getByText('RUHE · keine Warnungen')).toBeInTheDocument();
});

test('Fehler mit altem Stand: veraltet statt Ruhe', () => {
  answer({ data: makeView(), isError: true, dataUpdatedAt: Date.now() });
  render(<MonitorPage />);
  expect(screen.getByText(/^Warnstand veraltet/)).toBeInTheDocument();
  expect(screen.queryByText(/keine Warnungen/)).toBeNull();
});

test('pausierte Abfrage (Netz weg, kein Fehler): nach 90 s veraltet', () => {
  answer({ data: makeView(), dataUpdatedAt: Date.now() - 91_000 });
  render(<MonitorPage />);
  expect(screen.getByText(/^Warnstand veraltet/)).toBeInTheDocument();
  expect(screen.getByText(/^Veraltet · Stand/)).toBeInTheDocument();
});

const withSuchlage = (over: Parameters<typeof makeSuchlage>[0] = {}, stale = false) =>
  makeView({
    modules: { weather: true, radar: true, warningAreas: true, warningList: true, suchlage: true },
    suchlage: { data: makeSuchlage(over), fetchedAtMs: Date.now(), stale, error: null },
  });

test('laufender Suchlage-Einsatz: Status nennt ihn, Drohnenbild fährt ein', () => {
  answer({ data: withSuchlage(), dataUpdatedAt: Date.now() });
  render(<MonitorPage />);
  expect(screen.getByText('SUCHLAGE · Vermisste Person Oldenstadt')).toBeInTheDocument();
  const panel = screen.getByLabelText('Drohnenbild');
  expect(panel).toHaveAttribute('data-offen', 'true');
  expect(screen.getByText('Drohnenbild · Drohne 1')).toBeInTheDocument();
  expect(screen.getByText('Höhe 60 m')).toBeInTheDocument();
  expect(screen.getByText('Suchlage · Einsatz läuft')).toBeInTheDocument();
});

test('ohne Livebild bleibt das Drohnenbild draußen', () => {
  answer({ data: withSuchlage({ video: null }), dataUpdatedAt: Date.now() });
  render(<MonitorPage />);
  expect(screen.getByLabelText('Drohnenbild', { selector: 'section' })).toHaveAttribute('data-offen', 'false');
});

test('veralteter Suchlage-Stand: kein eingefrorenes Bild, Fuß meldet es', () => {
  answer({ data: withSuchlage({}, true), dataUpdatedAt: Date.now() });
  render(<MonitorPage />);
  expect(screen.getByLabelText('Drohnenbild', { selector: 'section' })).toHaveAttribute('data-offen', 'false');
  expect(screen.getByText(/^Suchlage veraltet/)).toBeInTheDocument();
});

test('eine Warnung über der Schwelle geht dem Einsatz im Status vor', () => {
  const view = withSuchlage();
  view.warnings.data = [makeWarning({ severity: 'Severe', levelLabel: 'Stufe 3' })];
  answer({ data: view, dataUpdatedAt: Date.now() });
  render(<MonitorPage />);
  expect(screen.getByText('1 WARNUNG · Stufe 3')).toBeInTheDocument();
});

test('ohne Einsatz kein Suchlage-Status', () => {
  answer({ data: withSuchlage({ mission: null, video: null }), dataUpdatedAt: Date.now() });
  render(<MonitorPage />);
  expect(screen.getByText('RUHE · keine Warnungen')).toBeInTheDocument();
  expect(screen.getByText('Suchlage · kein Einsatz')).toBeInTheDocument();
});

test('Wetter aus: keine Wetterspalte, Karte über die volle Breite', () => {
  answer({
    data: makeView({ modules: { weather: false, radar: true, warningAreas: true, warningList: true, suchlage: false } }),
    dataUpdatedAt: Date.now(),
  });
  const { container } = render(<MonitorPage />);
  expect(screen.queryByText(/^Wetter ·/)).toBeNull();
  expect(container.querySelector('.lm-spalten')).toHaveAttribute('data-seitenspalte', 'false');
  expect(screen.queryByLabelText('Drohnenbild')).toBeNull();
});

test('Konfiguration aus der Suite: Fuß nennt Herkunft, Fehler fällt auf', () => {
  answer({
    data: makeView({ config: { fromSuite: true, fetchedAtMs: Date.now(), error: 'Abruf fehlgeschlagen' } }),
    dataUpdatedAt: Date.now(),
  });
  render(<MonitorPage />);
  expect(screen.getByText(/^Konfiguration aus IUK-Suite nicht abholbar/)).toHaveClass('lm-fuss-alarm');
});

const QR = { url: 'https://drk.example.org/mitmachen', text: 'Lust mitzuhelfen?' };

test('Meldung aus der Suite steht als Band, mit Countdown', () => {
  const now = Date.now();
  answer({
    data: makeView({ nowMs: now, notices: [{ id: 'n1', text: 'Nächste Lagebesprechung', countdownToMs: now + 42 * 60_000, expiresAtMs: now + 3_600_000 }] }),
    dataUpdatedAt: now,
  });
  render(<MonitorPage />);
  expect(screen.getByRole('region', { name: 'Meldungen' })).toHaveTextContent('Nächste Lagebesprechung');
  expect(screen.getByText('in 42 min')).toBeInTheDocument();
});

test('QR-Code fürs Ehrenamt nur in ruhiger Lage', () => {
  answer({ data: makeView({ recruitment: QR }), dataUpdatedAt: Date.now() });
  const { unmount } = render(<MonitorPage />);
  expect(screen.getByRole('complementary', { name: 'Mitmachen im Ehrenamt' })).toHaveTextContent('Lust mitzuhelfen?');
  unmount();

  answer({ data: makeView({ recruitment: QR, warnings: { data: [makeWarning({ severity: 'Severe' })], fetchedAtMs: Date.now(), stale: false, error: null } }), dataUpdatedAt: Date.now() });
  const { unmount: weg } = render(<MonitorPage />);
  expect(screen.queryByRole('complementary', { name: 'Mitmachen im Ehrenamt' })).toBeNull();
  weg();

  answer({ data: { ...withSuchlage(), recruitment: QR }, dataUpdatedAt: Date.now() });
  render(<MonitorPage />);
  expect(screen.queryByRole('complementary', { name: 'Mitmachen im Ehrenamt' })).toBeNull();
});
