import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import NoticeBand from './NoticeBand';

const now = Date.parse('2026-10-27T13:00:00Z');
const notice = (id: string, extra: Partial<{ countdownToMs: number | null; expiresAtMs: number }> = {}) => ({
  id,
  text: `Meldung ${id}`,
  countdownToMs: null,
  expiresAtMs: now + 3_600_000,
  ...extra,
});

describe('NoticeBand', () => {
  it('zeigt laufende Meldungen mit Countdown', () => {
    render(<NoticeBand notices={[notice('a', { countdownToMs: now + 42 * 60_000 }), notice('b')]} nowMs={now} />);
    expect(screen.getByText('Meldung a')).toBeInTheDocument();
    expect(screen.getByText('in 42 min')).toBeInTheDocument();
    expect(screen.getByText('Meldung b')).toBeInTheDocument();
  });

  it('lässt Abgelaufenes weg und rendert ohne Meldung nichts', () => {
    const { container } = render(<NoticeBand notices={[notice('a', { expiresAtMs: now - 1 })]} nowMs={now} />);
    expect(container).toBeEmptyDOMElement();
  });
});
