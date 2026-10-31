import type { Notice } from '../contract/Notice';
import { countdownText, isDue } from './countdown';

/**
 * Meldungen aus der IUK-Suite, etwa „Besprechung · Nächste Lagebesprechung · in 42 min“. Vorn die
 * Kategorie mit Zeichen und Farbe (ohne Kategorie „Meldung“ in Blau); kurz vor der Uhrzeit blinkt das
 * Band. Kein Alarmrot; eine abgelaufene Meldung fällt mit der Uhr der Oberfläche weg, nicht erst mit
 * dem Server.
 */
export default function NoticeBand({ notices, nowMs }: { notices: Notice[]; nowMs: number }) {
  const running = notices.filter((n) => n.expiresAtMs > nowMs);
  if (running.length === 0) return null;
  return (
    <section className="lm-meldungen" aria-label="Meldungen">
      {running.map((n) => {
        const c = n.category;
        return (
          <div
            key={n.id}
            className="lm-meldung"
            data-farbe={c?.color ?? 'blue'}
            data-faellig={isDue(n.countdownToMs, c?.alertLeadMinutes ?? null, nowMs)}
          >
            <span className="lm-meldung-kuerzel">
              {c?.icon && (
                <svg
                  className="lm-meldung-zeichen"
                  viewBox={c.icon.viewBox}
                  aria-hidden="true"
                  focusable="false"
                >
                  {c.icon.paths.map((d, i) => (
                    <path key={i} d={d} />
                  ))}
                </svg>
              )}
              {c?.name ?? 'Meldung'}
            </span>
            <span className="lm-meldung-text">{n.text}</span>
            {n.countdownToMs !== null && (
              <span className="lm-meldung-zeit">{countdownText(n.countdownToMs, nowMs)}</span>
            )}
          </div>
        );
      })}
    </section>
  );
}
