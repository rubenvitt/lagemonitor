import { render, screen } from '@testing-library/react';
import WarningBand from './WarningBand';
import { makeWarning } from './testViews';

test('zeigt levelLabel als Text (zweiter Kanal neben der Farbe)', () => {
  render(<WarningBand band={[makeWarning({ levelLabel: 'Stufe 3', headline: 'Orkanartige Böen' })]} />);
  expect(screen.getByText(/Stufe 3/)).toBeInTheDocument();
  expect(screen.getByText('Orkanartige Böen')).toBeInTheDocument();
  expect(screen.queryByText(/weitere/)).toBeNull();
});

test('„+1 weitere“ bei zwei Band-Warnungen', () => {
  render(<WarningBand band={[makeWarning({ id: 'a' }), makeWarning({ id: 'b' })]} />);
  expect(screen.getByText('+1 weitere')).toBeInTheDocument();
});
