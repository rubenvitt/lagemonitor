import { describe, expect, it } from 'vitest';
import { ALERT_AFTER_MINUTES, countdownText, isDue } from './countdown';

describe('countdownText', () => {
  const t = Date.parse('2026-10-27T13:00:00Z');
  it('zählt auf die volle Minute aufgerundet, ab einer Stunde mit h', () => {
    expect(countdownText(t + 42 * 60_000, t)).toBe('in 42 min');
    expect(countdownText(t + 30_000, t)).toBe('in 1 min');
    expect(countdownText(t + 65 * 60_000, t)).toBe('in 1 h 5 min');
    expect(countdownText(t + 120 * 60_000, t)).toBe('in 2 h');
  });
  it('sagt eine Minute lang „jetzt“, danach „seit“', () => {
    expect(countdownText(t - 20_000, t)).toBe('jetzt');
    expect(countdownText(t - 3 * 60_000, t)).toBe('seit 3 min');
  });
});

describe('isDue', () => {
  const t = Date.parse('2026-10-27T13:00:00Z');
  it('blinkt ab dem Vorlauf bis zum Nachlauf', () => {
    expect(isDue(t, 5, t - 5 * 60_000 - 1)).toBe(false);
    expect(isDue(t, 5, t - 5 * 60_000)).toBe(true);
    expect(isDue(t, 5, t + ALERT_AFTER_MINUTES * 60_000 - 1)).toBe(true);
    expect(isDue(t, 5, t + ALERT_AFTER_MINUTES * 60_000)).toBe(false);
  });
  it('ohne Uhrzeit oder Vorlauf nie', () => {
    expect(isDue(null, 5, t)).toBe(false);
    expect(isDue(t, null, t)).toBe(false);
  });
});
