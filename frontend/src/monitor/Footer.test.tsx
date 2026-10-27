import { render, screen } from '@testing-library/react';
import Footer from './Footer';
import { HOUR, NOW, makeView } from './testViews';

test('shows_no_data_hint', () => {
  const view = makeView({
    weather: { data: null, fetchedAtMs: null, stale: true, error: null },
    warnings: { data: null, fetchedAtMs: null, stale: true, error: null },
  });
  render(<Footer view={view} />);
  expect(screen.getByText('Noch keine Daten')).toBeInTheDocument();
});

test('shows_stale', () => {
  const view = makeView({ warnings: { data: [], fetchedAtMs: NOW - HOUR, stale: true, error: 'Abruf fehlgeschlagen' } });
  render(<Footer view={view} />);
  expect(screen.getByText(/^Veraltet · Stand \d\d:\d\d$/)).toBeInTheDocument();
});

test('frischer Stand und Kartenquelle', () => {
  render(<Footer view={makeView({ map: { offline: true } })} />);
  expect(screen.getByText(/^DWD\/NINA · Stand \d\d:\d\d$/)).toBeInTheDocument();
  expect(screen.getByText('Karte offline')).toBeInTheDocument();
  expect(screen.queryByText(/Veraltet/)).toBeNull();
});
