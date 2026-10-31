import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { Notice } from '../contract/Notice';
import NoticeBand from './NoticeBand';

const now = Date.parse('2026-10-27T13:00:00Z');
const notice = (id: string, extra: Partial<Notice> = {}): Notice => ({
  id,
  text: `Meldung ${id}`,
  countdownToMs: null,
  expiresAtMs: now + 3_600_000,
  category: null,
  ...extra,
});

describe('NoticeBand', () => {
  it('zeigt laufende Meldungen mit Countdown', () => {
    render(
      <NoticeBand notices={[notice('a', { countdownToMs: now + 42 * 60_000 }), notice('b')]} nowMs={now} />,
    );
    expect(screen.getByText('Meldung a')).toBeInTheDocument();
    expect(screen.getByText('in 42 min')).toBeInTheDocument();
    expect(screen.getByText('Meldung b')).toBeInTheDocument();
  });

  it('lässt Abgelaufenes weg und rendert ohne Meldung nichts', () => {
    const { container } = render(
      <NoticeBand notices={[notice('a', { expiresAtMs: now - 1 })]} nowMs={now} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('zeigt die Kategorie mit Zeichen und Farbe, ohne Kategorie „Meldung“ in Blau', () => {
    const category = {
      name: 'Besprechung',
      color: 'purple' as const,
      icon: { viewBox: '0 0 48 48', paths: ['M14 4z'] },
      alertLeadMinutes: 10,
    };
    const { container } = render(
      <NoticeBand notices={[notice('a', { category }), notice('b')]} nowMs={now} />,
    );
    const [a, b] = container.querySelectorAll('.lm-meldung');
    expect(a).toHaveAttribute('data-farbe', 'purple');
    expect(a).toHaveTextContent('Besprechung');
    expect(a.querySelector('svg path')).toHaveAttribute('d', 'M14 4z');
    expect(b).toHaveAttribute('data-farbe', 'blue');
    expect(b).toHaveTextContent('Meldung');
    expect(b.querySelector('svg')).toBeNull();
  });

  it('blinkt ab dem Vorlauf der Kategorie, ohne Vorlauf nie', () => {
    const category = (alertLeadMinutes: number | null) => ({
      name: 'Besprechung',
      color: 'purple' as const,
      icon: null,
      alertLeadMinutes,
    });
    const { container } = render(
      <NoticeBand
        notices={[
          notice('bald', { countdownToMs: now + 4 * 60_000, category: category(5) }),
          notice('spaeter', { countdownToMs: now + 6 * 60_000, category: category(5) }),
          notice('nie', { countdownToMs: now + 60_000, category: category(null) }),
          notice('ohne', { countdownToMs: now + 60_000 }),
        ]}
        nowMs={now}
      />,
    );
    expect([...container.querySelectorAll('.lm-meldung')].map((e) => e.getAttribute('data-faellig'))).toEqual(
      ['true', 'false', 'false', 'false'],
    );
  });
});
