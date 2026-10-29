import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Map as MapLibreMap, type ExpressionSpecification, type GeoJSONSource, type RasterTileSource } from 'maplibre-gl';
import '../map/maplibreWorker';
import { basemapStyle } from '../map/basemapStyle';
import type { SpeedCamera } from '../contract/SpeedCamera';
import type { SuchlageView } from '../contract/SuchlageView';
import type { TrafficIncident } from '../contract/TrafficIncident';
import type { Warning } from '../contract/Warning';
import { useThemeMode } from '../theme/ThemeModeProvider';
import { farbenDunkel, farbenHell } from '../theme/tokens';
import { incidentFeatures, radarTiles, speedCameraFeatures, trafficTiles, warningAreas } from './mapData';
import { aircraftPoints, areas, boundsKey, coloredTracks, coveredByDrone, suchlageBounds } from './suchlageData';

interface Props {
  lat: number;
  lon: number;
  offline: boolean;
  band: Warning[];
  listed: Warning[];
  /** Stand des Wetters; `null` blendet das Radar aus (veraltet oder nie geladen). */
  radarAt: number | null;
  /** Verkehrsfluss von TomTom zeigen. */
  trafficFlow: boolean;
  /** Serverzeit; bestimmt den Stand der Verkehrskacheln. */
  nowMs: number;
  /** Baustellen und Sperrungen; leer, wenn aus. */
  incidents: TrafficIncident[];
  /** Feste Blitzer (nur online); leer, wenn aus. */
  speedCameras: SpeedCamera[];
  /** Laufender Suchlage-Einsatz; `null` ohne Einsatz oder mit Modul aus. */
  suchlage: SuchlageView | null;
  /** Das Drohnenbild ist eingefahren und verdeckt den rechten Rand. */
  droneOpen: boolean;
  /** Rechts steht die Wetterspalte (bestimmt, wie viel das Drohnenbild verdeckt). */
  sideColumn: boolean;
  /** Liegt über der Karte, etwa der QR-Code fürs Ehrenamt. */
  children?: ReactNode;
}

type Data = Parameters<GeoJSONSource['setData']>[0];

/** Dünne Hülle um maplibre (läuft nicht in jsdom); die Logik steckt in `mapData.ts`. */
export default function MapPanel({
  lat,
  lon,
  offline,
  band,
  listed,
  radarAt,
  trafficFlow,
  nowMs,
  incidents,
  speedCameras,
  suchlage,
  droneOpen,
  sideColumn,
  children,
}: Props) {
  const { effektiv } = useThemeMode();
  const container = useRef<HTMLDivElement>(null);
  const [map, setMap] = useState<MapLibreMap | null>(null);

  // Neu erzeugt nur bei Theme, Kartenquelle oder Standort, nicht bei jedem Datenstand.
  useEffect(() => {
    const f = effektiv === 'dark' ? farbenDunkel : farbenHell;
    const kindColor: ExpressionSpecification = ['match', ['get', 'kind'], 'band', f.alarm, f.achtung];
    const m = new MapLibreMap({
      container: container.current!,
      style: basemapStyle(effektiv, { offline }),
      center: [lon, lat],
      zoom: 9,
      attributionControl: { compact: false },
    });
    // `style.load` statt `load`: `load` wartet auf die ersten Kacheln, und die hängen ohne Netz.
    m.once('style.load', () => {
      // Verkehrsfluss ganz unten: Regen, Warnungen und Einsatz liegen darüber.
      m.addSource('verkehr', { type: 'raster', tiles: [trafficTiles(effektiv === 'dark', 0)], tileSize: 256, maxzoom: 18, attribution: '© TomTom' });
      m.addLayer({ id: 'verkehr', type: 'raster', source: 'verkehr', layout: { visibility: 'none' }, paint: { 'raster-opacity': 0.8 } });
      m.addSource('radar', { type: 'raster', tiles: [radarTiles(0)], tileSize: 256, attribution: '© DWD' });
      m.addSource('warngebiete', { type: 'geojson', data: { type: 'FeatureCollection', features: [] } });
      m.addSource('standort', {
        type: 'geojson',
        data: { type: 'Feature', properties: {}, geometry: { type: 'Point', coordinates: [lon, lat] } },
      });
      m.addLayer({ id: 'radar', type: 'raster', source: 'radar', layout: { visibility: 'none' }, paint: { 'raster-opacity': 0.7 } });
      m.addLayer({ id: 'warngebiete-flaeche', type: 'fill', source: 'warngebiete', paint: { 'fill-color': kindColor, 'fill-opacity': 0.14 } });
      m.addLayer({
        id: 'warngebiete-kante',
        type: 'line',
        source: 'warngebiete',
        paint: { 'line-color': kindColor, 'line-width': 2, 'line-dasharray': [3, 2] },
      });
      // Baustellen und Blitzer neutral (Text- und Grundfarbe): Rot und Gelb gehören den Warnungen,
      // Blau dem Standort und Suchlage. Eine Baustelle ist eine gestreifte Linie, eine Sperrung breiter.
      m.addSource('baustellen', { type: 'geojson', data: { type: 'FeatureCollection', features: [] } });
      m.addSource('blitzer', {
        type: 'geojson',
        data: { type: 'FeatureCollection', features: [] },
        attribution: 'Blitzer © OpenStreetMap-Mitwirkende',
      });
      const closed: ExpressionSpecification = ['==', ['get', 'kind'], 'roadClosed'];
      const line: ExpressionSpecification = ['==', ['geometry-type'], 'LineString'];
      m.addLayer({
        id: 'baustellen-grund',
        type: 'line',
        source: 'baustellen',
        filter: line,
        layout: { 'line-cap': 'butt' },
        paint: { 'line-color': f.grund, 'line-width': ['case', closed, 8, 6] },
      });
      m.addLayer({
        id: 'baustellen-streifen',
        type: 'line',
        source: 'baustellen',
        filter: line,
        layout: { 'line-cap': 'butt' },
        paint: { 'line-color': f.text, 'line-width': ['case', closed, 5, 3], 'line-dasharray': [1.2, 1.2] },
      });
      m.addLayer({
        id: 'baustellen-punkt',
        type: 'circle',
        source: 'baustellen',
        filter: ['!', line],
        paint: { 'circle-radius': 6, 'circle-color': f.text, 'circle-stroke-color': f.grund, 'circle-stroke-width': 3 },
      });
      m.addLayer({
        id: 'blitzer',
        type: 'circle',
        source: 'blitzer',
        paint: { 'circle-radius': 7, 'circle-color': f.grund, 'circle-stroke-color': f.text, 'circle-stroke-width': 2.5 },
      });
      m.addLayer({
        id: 'blitzer-tempo',
        type: 'symbol',
        source: 'blitzer',
        minzoom: 11,
        layout: { 'text-field': ['get', 'maxspeed'], 'text-size': 12, 'text-offset': [0, 1.3], 'text-font': ['noto_sans_regular'] },
        paint: { 'text-color': f.text, 'text-halo-color': f.grund, 'text-halo-width': 2 },
      });
      // Suchlage über den Warngebieten, Drohnen ganz oben.
      for (const id of ['suchgebiete', 'flugspuren', 'drohnen']) {
        m.addSource(id, { type: 'geojson', data: { type: 'FeatureCollection', features: [] } });
      }
      m.addLayer({ id: 'suchgebiete-flaeche', type: 'fill', source: 'suchgebiete', paint: { 'fill-color': ['get', 'color'], 'fill-opacity': 0.16 } });
      m.addLayer({ id: 'suchgebiete-kante', type: 'line', source: 'suchgebiete', paint: { 'line-color': ['get', 'color'], 'line-width': 2.5 } });
      m.addLayer({
        id: 'flugspuren',
        type: 'line',
        source: 'flugspuren',
        layout: { 'line-cap': 'round', 'line-join': 'round' },
        paint: { 'line-color': ['get', 'color'], 'line-width': ['case', ['get', 'running'], 4, 3], 'line-opacity': 0.95 },
      });
      m.addLayer({
        id: 'standort-ring',
        type: 'circle',
        source: 'standort',
        paint: { 'circle-radius': 22, 'circle-color': 'transparent', 'circle-stroke-color': f.bedien, 'circle-stroke-width': 1.5, 'circle-stroke-opacity': 0.5 },
      });
      m.addLayer({ id: 'standort-punkt', type: 'circle', source: 'standort', paint: { 'circle-radius': 9, 'circle-color': f.bedien } });
      m.addLayer({
        id: 'drohnen',
        type: 'circle',
        source: 'drohnen',
        paint: {
          'circle-radius': 8,
          'circle-color': ['case', ['get', 'airborne'], f.text, f.gedaempft],
          'circle-stroke-color': f.grund,
          'circle-stroke-width': 3,
        },
      });
      setMap(m);
    });
    return () => {
      setMap(null);
      m.remove();
    };
  }, [effektiv, offline, lat, lon]);

  // `!map?.style`: Wechselt Theme, Kartenquelle oder Standort, entfernt der Effekt oben die alte Karte
  // (`remove()` löscht `style`), bevor `setMap(null)` ankommt. Die Effekte unten laufen im selben
  // Commit noch mit ihr, und ein Fehler dort leerte den ganzen Bildschirm (fuekw-server, 28.10.2026).
  useEffect(() => {
    if (!map?.style) return;
    map.getSource<GeoJSONSource>('warngebiete')?.setData(warningAreas(band, listed) as Parameters<GeoJSONSource['setData']>[0]);
  }, [map, band, listed]);

  useEffect(() => {
    if (!map?.style) return;
    if (radarAt !== null) map.getSource<RasterTileSource>('radar')?.setTiles([radarTiles(radarAt)]);
    map.setLayoutProperty('radar', 'visibility', radarAt === null ? 'none' : 'visible');
  }, [map, radarAt]);

  const dark = effektiv === 'dark';
  const flowTiles = trafficFlow ? trafficTiles(dark, nowMs) : null;
  useEffect(() => {
    if (!map?.style) return;
    if (flowTiles !== null) map.getSource<RasterTileSource>('verkehr')?.setTiles([flowTiles]);
    map.setLayoutProperty('verkehr', 'visibility', flowTiles === null ? 'none' : 'visible');
  }, [map, flowTiles]);

  useEffect(() => {
    if (!map?.style) return;
    map.getSource<GeoJSONSource>('baustellen')?.setData(incidentFeatures(incidents) as Data);
    map.getSource<GeoJSONSource>('blitzer')?.setData(speedCameraFeatures(speedCameras) as Data);
  }, [map, incidents, speedCameras]);

  useEffect(() => {
    if (!map?.style) return;
    const empty = { type: 'FeatureCollection', features: [] } as const;
    map.getSource<GeoJSONSource>('suchgebiete')?.setData((suchlage ? areas(suchlage) : empty) as Data);
    map.getSource<GeoJSONSource>('flugspuren')?.setData((suchlage ? coloredTracks(suchlage) : empty) as Data);
    map.getSource<GeoJSONSource>('drohnen')?.setData((suchlage ? aircraftPoints(suchlage.aircraft) : empty) as Data);
  }, [map, suchlage]);

  // Einpassen nur, wenn sich der Rahmen merklich ändert (neuer Einsatz, neues Gebiet) oder das
  // Drohnenbild ein- bzw. ausfährt – nicht bei jedem Spurpunkt.
  const bounds = suchlage ? suchlageBounds(suchlage) : null;
  const key = boundsKey(bounds);
  useEffect(() => {
    if (!map?.style) return;
    const right = droneOpen ? Math.round(map.getContainer().clientWidth * coveredByDrone(sideColumn)) : 0;
    const padding = { top: 50, bottom: 50, left: 50, right: right + 50 };
    if (bounds) map.fitBounds(bounds, { padding, maxZoom: 15, duration: 1200 });
    else map.easeTo({ center: [lon, lat], zoom: 9, padding: { top: 0, bottom: 0, left: 0, right }, duration: 800 });
    // `bounds` steckt in `key`; ein neues Array je Abfrage soll nicht neu einpassen.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [map, key, droneOpen, sideColumn, lat, lon]);

  return (
    <section className="lm-paneel lm-karte">
      <div ref={container} className="lm-karte-flaeche" data-testid="map" />
      {children}
    </section>
  );
}
