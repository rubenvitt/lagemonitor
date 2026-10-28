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

/** Suchlage nur, wenn verbunden; ein Fehler oder ein alter Stand fällt auf. */
function suchlageState({ suchlage }: MonitorView): { text: string; alert: boolean } | null {
  if (!suchlage) return null;
  if (suchlage.data === null) return { text: 'Suchlage nicht erreichbar', alert: true };
  if (suchlage.stale) {
    return { text: `Suchlage veraltet · Stand ${suchlage.fetchedAtMs === null ? '–' : clock(suchlage.fetchedAtMs)}`, alert: true };
  }
  return { text: suchlage.data.mission ? 'Suchlage · Einsatz läuft' : 'Suchlage · kein Einsatz', alert: false };
}

/** Verkehr nur, wenn eine Ebene an ist. Ein Fehler fällt auf, der Stand der Baustellen steht dabei. */
function trafficState({ traffic }: MonitorView): { text: string; alert: boolean } | null {
  const { flow, flowError, incidents, speedCameras } = traffic;
  if (!flow && !incidents && !speedCameras) return null;
  if (flowError !== null || incidents?.error) return { text: 'Verkehr: TomTom nicht abrufbar', alert: true };
  if (speedCameras?.error) return { text: 'Blitzer: OpenStreetMap nicht abrufbar', alert: true };
  const at = incidents?.fetchedAtMs;
  const sources = [(flow || incidents) && 'TomTom', speedCameras && 'OSM'].filter(Boolean).join('/');
  return { text: at ? `Verkehr ${sources} · Stand ${clock(at)}` : `Verkehr ${sources}`, alert: false };
}

/** Woher die Einstellungen stammen; scheitert die Abholung, gilt der letzte Stand und das steht da. */
function configState({ config }: MonitorView): { text: string; alert: boolean } | null {
  if (!config.fromSuite) return null;
  const at = config.fetchedAtMs === null ? '–' : clock(config.fetchedAtMs);
  if (config.error !== null) return { text: `Konfiguration aus IUK-Suite nicht abholbar · Stand ${at}`, alert: true };
  return { text: `Konfiguration IUK-Suite · ${at}`, alert: false };
}

export default function Footer({ view }: { view?: MonitorView }) {
  const states = view ? [dataState(view), suchlageState(view), trafficState(view), configState(view)] : [];
  return (
    <footer className="lm-fuss" data-theme="dark">
      {states.map(
        (s) =>
          s && (
            <span key={s.text} className={s.alert ? 'lm-fuss-alarm' : undefined}>
              {s.text}
            </span>
          ),
      )}
      <span>lokal · ohne Anmeldung</span>
    </footer>
  );
}
