import { useEffect, useMemo, useState } from 'react';
import { useMonitor } from '../api';
import { useThemeMode, useWarnsperre } from '../theme/ThemeModeProvider';
import DronePanel from './DronePanel';
import Footer from './Footer';
import Header from './Header';
import MapPanel from './MapPanel';
import NoticeBand from './NoticeBand';
import RecruitmentQr from './RecruitmentQr';
import WarningBand from './WarningBand';
import WeatherPanel from './WeatherPanel';
import { isDaylight } from './daylight';
import { radarAt } from './mapData';
import { UNKNOWN_LAYOUT, isClientStale, markStale, selectLayout, serverNow } from './selectLayout';
import './monitor.css';

// Ein festes leeres Feld: ein neues je Abfrage schriebe die Karte jedes Mal neu.
const NONE: never[] = [];

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
  const nowMs = view ? serverNow(view.nowMs, dataUpdatedAt, layoutNow) : layoutNow;
  const layout = useMemo(() => (view ? selectLayout(view, nowMs) : UNKNOWN_LAYOUT), [view, nowMs]);
  useWarnsperre(layout.mode === 'warnung');

  // Hell zwischen Sonnenauf- und -untergang am eingestellten Ort; ohne Daten bleibt der letzte Stand.
  const { setModus } = useThemeMode();
  const daylight = view ? isDaylight(view.location.lat, view.location.lon, layoutNow) : null;
  useEffect(() => {
    if (daylight !== null) setModus(daylight ? 'light' : 'dark');
  }, [daylight, setModus]);

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

  const { weather, location, modules, traffic } = view;
  // Ein Suchlage-Stand ohne Einsatz oder lange ohne Antwort zeigt nichts: Alte Spuren und ein
  // eingefrorenes Bild wären schlimmer als keine. Das Band oben bleibt davon unberührt.
  const einsatz = view.suchlage?.data?.mission ? view.suchlage.data : null;
  const listed = modules.warningList ? layout.listed : [];
  const sideColumn = modules.weather || listed.length > 0;
  const video = einsatz && !view.suchlage?.stale ? einsatz.video : null;
  // Der QR-Code fürs Ehrenamt nur in ruhiger Lage: bei Warnung oder Einsatz gehört der Bildschirm der Lage.
  const recruitment = layout.mode !== 'warnung' && !einsatz ? view.recruitment : null;
  return (
    <div className="lm">
      <Header view={view} layout={layout} nowMs={clockNow} />
      <div className="lm-rumpf">
        {layout.mode === 'warnung' && <WarningBand band={layout.band} />}
        <NoticeBand notices={view.notices} nowMs={serverNow(view.nowMs, dataUpdatedAt, clockNow)} />
        <div className="lm-spalten" data-seitenspalte={sideColumn}>
          <MapPanel
            lat={location.lat}
            lon={location.lon}
            offline={view.map.offline}
            band={modules.warningAreas ? layout.band : []}
            listed={modules.warningAreas ? layout.listed : []}
            radarAt={modules.radar ? radarAt(weather, nowMs) : null}
            trafficFlow={traffic.flow}
            nowMs={nowMs}
            incidents={traffic.incidents?.data ?? NONE}
            speedCameras={traffic.speedCameras?.data ?? NONE}
            suchlage={einsatz}
            droneOpen={video !== null}
            sideColumn={sideColumn}
          >
            {recruitment && <RecruitmentQr recruitment={recruitment} />}
          </MapPanel>
          {sideColumn && (
            <WeatherPanel weather={modules.weather ? weather : null} label={location.label} listed={listed} />
          )}
          {modules.suchlage && <DronePanel video={video} aircraft={einsatz?.aircraft ?? []} />}
        </div>
      </div>
      <Footer view={view} />
    </div>
  );
}
