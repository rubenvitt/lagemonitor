import type { Notice } from '../contract/Notice';
import { countdownText } from './countdown';

/**
 * Meldungen aus der IUK-Suite, etwa „Nächste Lagebesprechung · in 42 min“. Ruhiges Blau statt
 * Alarmrot; eine abgelaufene Meldung fällt mit der Uhr der Oberfläche weg, nicht erst mit dem Server.
 */
export default function NoticeBand({ notices, nowMs }: { notices: Notice[]; nowMs: number }) {
  const running = notices.filter((n) => n.expiresAtMs > nowMs);
  if (running.length === 0) return null;
  return (
    <section className="lm-meldungen" aria-label="Meldungen">
      {running.map((n) => (
        <div key={n.id} className="lm-meldung">
          <span className="lm-meldung-kuerzel">Meldung</span>
          <span className="lm-meldung-text">{n.text}</span>
          {n.countdownToMs !== null && <span className="lm-meldung-zeit">{countdownText(n.countdownToMs, nowMs)}</span>}
        </div>
      ))}
    </section>
  );
}
