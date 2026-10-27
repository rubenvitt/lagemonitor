import { useEffect, useMemo, useState } from 'react';
import { useMonitor } from '../api';
import { useWarnsperre } from '../theme/ThemeModeProvider';
import Footer from './Footer';
import Header from './Header';
import MapPanel from './MapPanel';
import WarningBand from './WarningBand';
import WeatherPanel from './WeatherPanel';
import { UNKNOWN_LAYOUT, isClientStale, markStale, selectLayout } from './selectLayout';
import './monitor.css';

function useNow(intervalMs: number): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return now;
}

export default function MonitorPage() {
  const { data, isError, isPending, dataUpdatedAt } = useMonitor();
  const clockNow = useNow(1000);
  // Die Aufteilung nur alle 30 s neu rechnen: sonst bekäme die Karte jede Sekunde neue Warngebiete.
  const layoutNow = useNow(30_000);
  const clientStale = isClientStale(isError, dataUpdatedAt, clockNow);
  const view = useMemo(() => data && markStale(data, clientStale), [data, clientStale]);
  const layout = useMemo(() => (view ? selectLayout(view, layoutNow) : UNKNOWN_LAYOUT), [view, layoutNow]);
  useWarnsperre(layout.mode === 'warnung');

  if (!view) {
    const note = isPending ? 'lädt' : 'Server nicht erreichbar';
    return (
      <div className="lm">
        <Header layout={layout} nowMs={clockNow} note={note} />
        <div className="lm-hinweis">{isPending ? 'Lädt …' : 'Server nicht erreichbar'}</div>
        <Footer />
      </div>
    );
  }

  const { weather, location } = view;
  return (
    <div className="lm">
      <Header view={view} layout={layout} nowMs={clockNow} />
      <div className="lm-rumpf">
        {layout.mode === 'warnung' && <WarningBand band={layout.band} />}
        <div className="lm-spalten">
          <MapPanel
            lat={location.lat}
            lon={location.lon}
            label={location.label}
            offline={view.map.offline}
            band={layout.band}
            listed={layout.listed}
            radarAt={weather.stale ? null : weather.fetchedAtMs}
          />
          <WeatherPanel weather={weather} label={location.label} listed={layout.listed} />
        </div>
      </div>
      <Footer view={view} />
    </div>
  );
}
