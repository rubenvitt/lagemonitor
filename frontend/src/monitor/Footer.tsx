import type { MonitorView } from '../contract/MonitorView';
import { clock } from './format';

/** Datenstand beider Quellen; maßgeblich ist der ältere. */
function dataState({ weather, warnings }: MonitorView): { text: string; alert: boolean } {
  if (weather.data === null || warnings.data === null) return { text: 'Noch keine Daten', alert: true };
  const times = [weather.fetchedAtMs, warnings.fetchedAtMs].filter((t): t is number => t !== null);
  const at = times.length > 0 ? clock(Math.min(...times)) : '–';
  if (weather.stale || warnings.stale) return { text: `Veraltet · Stand ${at}`, alert: true };
  return { text: `DWD/NINA · Stand ${at}`, alert: false };
}

export default function Footer({ view }: { view?: MonitorView }) {
  const state = view && dataState(view);
  return (
    <footer className="lm-fuss">
      {state && <span className={state.alert ? 'lm-fuss-alarm' : undefined}>{state.text}</span>}
      <span>lokal · ohne Anmeldung</span>
    </footer>
  );
}
