import type { MonitorView } from '../contract/MonitorView';
import { clock } from './format';
import type { Layout } from './selectLayout';

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

type Tone = 'normal' | 'achtung' | 'alarm' | 'einsatz';

/**
 * Status als Wort und Ton. „keine Warnungen“ nur, wenn der Warnstand bekannt, frisch und leer ist.
 * Eine Warnung über der Schwelle geht dem Suchlage-Einsatz vor, eine unbekannte Warnlage nicht.
 */
function status(layout: Layout, fetchedAtMs: number | null, einsatz: string | null): { text: string; tone: Tone } {
  const { band, listed, warnState } = layout;
  if (einsatz !== null && band.length === 0) return { text: `SUCHLAGE · ${einsatz}`, tone: 'einsatz' };
  if (band.length > 0) {
    const text = `${plural(band.length, 'WARNUNG', 'WARNUNGEN')} · ${band[0].levelLabel}`;
    return { text: warnState === 'stale' ? `${text} · veraltet` : text, tone: 'alarm' };
  }
  if (warnState === 'unknown') return { text: 'WARNLAGE UNBEKANNT', tone: 'achtung' };
  if (warnState === 'stale') {
    return { text: `Warnstand veraltet · ${fetchedAtMs === null ? '–' : clock(fetchedAtMs)}`, tone: 'achtung' };
  }
  if (listed.length > 0) return { text: `RUHE · ${plural(listed.length, 'Warnung', 'Warnungen')} unter Schwelle`, tone: 'normal' };
  return { text: 'RUHE · keine Warnungen', tone: 'normal' };
}

interface Props {
  /** Fehlt, solange nichts vom Server kam. */
  view?: MonitorView;
  layout: Layout;
  nowMs: number;
  /** Zusatz zum Status, etwa warum die Warnlage unbekannt ist. */
  note?: string;
}

export default function Header({ view, layout, nowMs, note }: Props) {
  const mission = view?.suchlage?.data?.mission;
  const { text, tone } = status(layout, view?.warnings.fetchedAtMs ?? null, mission ? mission.name : null);
  return (
    // Der Kopf liegt im Rahmen und bleibt dunkel; Statusfarben daher immer aus dem dunklen Satz.
    <header className="lm-kopf" data-theme="dark">
      <div className="lm-marke" aria-hidden>
        {/* Gleiches Zeichen wie public/favicon.svg: Bildschirm mit Kartennadel. */}
        <svg viewBox="0 0 24 24" width="26" height="26" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <rect x="2" y="3" width="20" height="14" rx="2" />
          <path d="M8 21h8M12 17v4" />
          <path d="M12 14.5s-3.5-3.2-3.5-5.8a3.5 3.5 0 0 1 7 0c0 2.6-3.5 5.8-3.5 5.8z" />
          <circle cx="12" cy="8.7" r="1" />
        </svg>
      </div>
      <span className="lm-wortmarke">{view ? `LAGEMONITOR · ${view.location.label}` : 'LAGEMONITOR'}</span>
      <span className={`lm-status lm-ton-${tone}`}>
        <span className="lm-punkt" aria-hidden />
        {note ? `${text} · ${note}` : text}
      </span>
      <span className="lm-uhr">{clock(nowMs, true)}</span>
    </header>
  );
}
