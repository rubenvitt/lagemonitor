import { useEffect, useRef, useState } from 'react';
import { Map as MapLibreMap, type ExpressionSpecification, type GeoJSONSource, type RasterTileSource } from 'maplibre-gl';
import '../map/maplibreWorker';
import { basemapStyle } from '../map/basemapStyle';
import type { SuchlageView } from '../contract/SuchlageView';
import type { Warning } from '../contract/Warning';
import { useThemeMode } from '../theme/ThemeModeProvider';
import { farbenDunkel, farbenHell } from '../theme/tokens';
import { radarTiles, warningAreas } from './mapData';
import { aircraftPoints, areas, boundsKey, coloredTracks, coveredByDrone, suchlageBounds } from './suchlageData';

interface Props {
  lat: number;
  lon: number;
  label: string;
  offline: boolean;
  band: Warning[];
  listed: Warning[];
  /** Stand des Wetters; `null` blendet das Radar aus (veraltet oder nie geladen). */
  radarAt: number | null;
  /** Laufender Suchlage-Einsatz; `null` ohne Einsatz oder mit Modul aus. */
  suchlage: SuchlageView | null;
  /** Das Drohnenbild ist eingefahren und verdeckt den rechten Rand. */
  droneOpen: boolean;
  /** Rechts steht die Wetterspalte (bestimmt, wie viel das Drohnenbild verdeckt). */
  sideColumn: boolean;
}

type Data = Parameters<GeoJSONSource['setData']>[0];

/** Dünne Hülle um maplibre (läuft nicht in jsdom); die Logik steckt in `mapData.ts`. */
export default function MapPanel({ lat, lon, label, offline, band, listed, radarAt, suchlage, droneOpen, sideColumn }: Props) {
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

  useEffect(() => {
    map?.getSource<GeoJSONSource>('warngebiete')?.setData(warningAreas(band, listed) as Parameters<GeoJSONSource['setData']>[0]);
  }, [map, band, listed]);

  useEffect(() => {
    if (!map) return;
    if (radarAt !== null) map.getSource<RasterTileSource>('radar')?.setTiles([radarTiles(radarAt)]);
    map.setLayoutProperty('radar', 'visibility', radarAt === null ? 'none' : 'visible');
  }, [map, radarAt]);

  useEffect(() => {
    if (!map) return;
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
    if (!map) return;
    const right = droneOpen ? Math.round(map.getContainer().clientWidth * coveredByDrone(sideColumn)) : 0;
    const padding = { top: 80, bottom: 50, left: 50, right: right + 50 };
    if (bounds) map.fitBounds(bounds, { padding, maxZoom: 15, duration: 1200 });
    else map.easeTo({ center: [lon, lat], zoom: 9, padding: { top: 0, bottom: 0, left: 0, right }, duration: 800 });
    // `bounds` steckt in `key`; ein neues Array je Abfrage soll nicht neu einpassen.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [map, key, droneOpen, sideColumn, lat, lon]);

  const warnAreas = band.length + listed.length > 0;
  const hasTracks = suchlage !== null && coloredTracks(suchlage).features.length > 0;
  const hasSearchAreas = suchlage !== null && areas(suchlage).features.length > 0;
  return (
    <section className="lm-paneel lm-karte">
      <div ref={container} className="lm-karte-flaeche" data-testid="map" />
      <div className="lm-karte-legende">
        <div className="lm-augenbraue">
          {['Standort', warnAreas && 'Warngebiete', radarAt !== null && 'Radar', hasSearchAreas && 'Suchgebiete', hasTracks && 'Flugspuren']
            .filter(Boolean)
            .join(' + ')}
        </div>
        <div className="lm-zahl">{label}</div>
      </div>
    </section>
  );
}
