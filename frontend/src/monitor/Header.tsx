import type { MonitorView } from '../contract/MonitorView';
import { clock } from './format';
import type { Layout } from './selectLayout';

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** Status als Wort und Ton. „keine Warnungen“ nur, wenn der Warnstand bekannt, frisch und leer ist. */
function status(layout: Layout, fetchedAtMs: number | null): { text: string; tone: 'normal' | 'achtung' | 'alarm' } {
  const { band, listed, warnState } = layout;
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
  const { text, tone } = status(layout, view?.warnings.fetchedAtMs ?? null);
  return (
    <header className="lm-kopf">
      <div className="lm-marke" aria-hidden>
        +
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
