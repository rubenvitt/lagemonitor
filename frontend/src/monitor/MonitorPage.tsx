import { useEffect, useRef } from 'react';
import { Map } from 'maplibre-gl';
import '../map/maplibreWorker';
import { basemapStyle } from '../map/basemapStyle';
import { useMonitor } from '../api';
import { useThemeMode } from '../theme/ThemeModeProvider';

// Platzhalter: Vollbild-Karte am Standort. Task 6 baut Wetter, Warnband und Datenstand darauf.
export default function MonitorPage() {
  const { data } = useMonitor();
  const { effektiv } = useThemeMode();
  const container = useRef<HTMLDivElement>(null);
  const map = useRef<Map | null>(null);
  const lat = data?.location.lat;
  const lon = data?.location.lon;
  const offline = data?.map.offline;

  useEffect(() => {
    if (!container.current || lat === undefined || lon === undefined || offline === undefined) return;
    const m = new Map({
      container: container.current,
      style: basemapStyle(effektiv, { offline }),
      center: [lon, lat],
      zoom: 10,
    });
    map.current = m;
    return () => {
      m.remove();
      map.current = null;
    };
  }, [lat, lon, offline, effektiv]);

  return (
    <div style={{ position: 'fixed', inset: 0 }}>
      <div ref={container} data-testid="map" style={{ position: 'absolute', inset: 0 }} />
      {!data && (
        <div style={{ position: 'absolute', inset: 0, display: 'grid', placeItems: 'center' }}>
          Noch keine Daten
        </div>
      )}
    </div>
  );
}
