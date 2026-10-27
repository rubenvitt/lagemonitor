import { render, screen } from '@testing-library/react';
import { vi } from 'vitest';
import { useMonitor } from '../api';
import MonitorPage from './MonitorPage';
import { makeView } from './testViews';

// maplibre läuft nicht in jsdom; die Karte ist hier nicht Gegenstand.
vi.mock('./MapPanel', () => ({ default: () => null }));
vi.mock('../api', () => ({ useMonitor: vi.fn() }));

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
