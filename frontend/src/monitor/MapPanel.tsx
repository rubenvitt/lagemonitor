import { useEffect, useRef, useState } from 'react';
import { Map as MapLibreMap, type ExpressionSpecification, type GeoJSONSource, type RasterTileSource } from 'maplibre-gl';
import '../map/maplibreWorker';
import { basemapStyle } from '../map/basemapStyle';
import type { Warning } from '../contract/Warning';
import { useThemeMode } from '../theme/ThemeModeProvider';
import { farbenDunkel, farbenHell } from '../theme/tokens';
import { radarTiles, warningAreas } from './mapData';

interface Props {
  lat: number;
  lon: number;
  label: string;
  offline: boolean;
  band: Warning[];
  listed: Warning[];
  /** Stand des Wetters; `null` blendet das Radar aus (veraltet oder nie geladen). */
  radarAt: number | null;
}

/** Dünne Hülle um maplibre (läuft nicht in jsdom); die Logik steckt in `mapData.ts`. */
export default function MapPanel({ lat, lon, label, offline, band, listed, radarAt }: Props) {
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
    m.on('load', () => {
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
      m.addLayer({
        id: 'standort-ring',
        type: 'circle',
        source: 'standort',
        paint: { 'circle-radius': 22, 'circle-color': 'transparent', 'circle-stroke-color': f.bedien, 'circle-stroke-width': 1.5, 'circle-stroke-opacity': 0.5 },
      });
      m.addLayer({ id: 'standort-punkt', type: 'circle', source: 'standort', paint: { 'circle-radius': 9, 'circle-color': f.bedien } });
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

  const areas = band.length + listed.length > 0;
  return (
    <section className="lm-paneel lm-karte">
      <div ref={container} className="lm-karte-flaeche" data-testid="map" />
      <div className="lm-karte-legende">
        <div className="lm-augenbraue">
          {['Standort', areas && 'Warngebiete', radarAt !== null && 'Radar'].filter(Boolean).join(' + ')}
        </div>
        <div className="lm-zahl">{label}</div>
      </div>
    </section>
  );
}
