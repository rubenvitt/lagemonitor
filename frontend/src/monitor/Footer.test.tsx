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

test('frischer Stand ohne Kartenquelle', () => {
  render(<Footer view={makeView({ map: { offline: true, zoom: 9 } })} />);
  expect(screen.getByText(/^DWD\/NINA · Stand \d\d:\d\d$/)).toBeInTheDocument();
  expect(screen.queryByText(/^Karte /)).toBeNull();
  expect(screen.queryByText(/Veraltet/)).toBeNull();
});

test('Verkehr steht nur da, wenn eine Ebene an ist, und ein Fehler fällt auf', () => {
  const { rerender } = render(<Footer view={makeView()} />);
  expect(screen.queryByText(/Verkehr/)).toBeNull();
  const incidents = { data: [], fetchedAtMs: NOW, stale: false, error: null };
  rerender(<Footer view={makeView({ traffic: { flow: true, flowError: null, incidents, speedCameras: null } })} />);
  expect(screen.getByText(/^Verkehr TomTom · Stand \d\d:\d\d$/)).toBeInTheDocument();
  rerender(<Footer view={makeView({ traffic: { flow: true, flowError: 'Verkehrsfluss nicht abrufbar', incidents, speedCameras: null } })} />);
  expect(screen.getByText('Verkehr: TomTom nicht abrufbar')).toHaveClass('lm-fuss-alarm');
});
