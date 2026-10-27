import { render, screen } from '@testing-library/react';
import WeatherPanel from './WeatherPanel';
import { makeView, makeWarning } from './testViews';

test('übersetzt condition „rain“ → „Regen“', () => {
  const view = makeView();
  view.weather.data!.current.condition = 'rain';
  view.weather.data!.current.icon = 'rain';
  render(<WeatherPanel weather={view.weather} label="Uelzen" listed={[]} />);
  expect(screen.getByText('Regen')).toBeInTheDocument();
});

test('bei trockenem Wetter sagt das Symbol mehr: „bedeckt“, unbekannt bleibt Rohwert', () => {
  const view = makeView();
  render(<WeatherPanel weather={view.weather} label="Uelzen" listed={[]} />);
  expect(screen.getByText('bedeckt')).toBeInTheDocument();
});

test('Vorhersage: jeder dritte Stundenwert, vier Stück', () => {
  const view = makeView();
  view.weather.data!.hourly = Array.from({ length: 12 }, (_, i) => ({
    timestamp: new Date(Date.UTC(2026, 9, 27, 13 + i)).toISOString(),
    temperature: i,
    icon: null,
    precipitation: null,
  }));
  render(<WeatherPanel weather={view.weather} label="Uelzen" listed={[]} />);
  expect(screen.getAllByTestId('forecast-hour').map((e) => e.lastChild?.textContent)).toEqual(['2°', '5°', '8°', '11°']);
});

test('weitere Warnungen mit Wort und Stufe; ohne Daten „Noch keine Daten“', () => {
  const { rerender } = render(
    <WeatherPanel weather={makeView().weather} label="Uelzen" listed={[makeWarning({ event: 'GEWITTER', levelLabel: 'Stufe 1' })]} />,
  );
  expect(screen.getByText('Weitere Warnungen')).toBeInTheDocument();
  expect(screen.getByText('GEWITTER')).toBeInTheDocument();
  expect(screen.getByText('Stufe 1')).toBeInTheDocument();
  rerender(<WeatherPanel weather={{ data: null, fetchedAtMs: null, stale: true, error: null }} label="Uelzen" listed={[]} />);
  expect(screen.getByText('Noch keine Daten')).toBeInTheDocument();
});
