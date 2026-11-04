import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { Map as MapLibreMap, Marker, type ExpressionSpecification, type GeoJSONSource, type RasterTileSource } from 'maplibre-gl';
import '../map/maplibreWorker';
import { basemapStyle } from '../map/basemapStyle';
import type { MapObject } from '../contract/MapObject';
import type { SpeedCamera } from '../contract/SpeedCamera';
import type { SuchlageView } from '../contract/SuchlageView';
import type { TrafficIncident } from '../contract/TrafficIncident';
import type { Vehicle } from '../contract/Vehicle';
import type { Warning } from '../contract/Warning';
import { useThemeMode } from '../theme/ThemeModeProvider';
import { lageFarbAusdruck, lageFarbe } from '../theme/palette';
import { farbenDunkel, farbenHell } from '../theme/tokens';
import {
  LAGEPUNKT_NAME_AB_ZOOM,
  incidentFeatures,
  lageBeschriftungen,
  lageFlaechen,
  lagePunkte,
  radarTiles,
  speedCameraFeatures,
  surroundingBounds,
  trafficTiles,
  vehicleMarks,
  warningAreas,
} from './mapData';
import { aircraftPoints, areas, boundsKey, coloredTracks, coveredByDrone, suchlageBounds } from './suchlageData';

interface Props {
  lat: number;
  lon: number;
  /** Zoomstufe ohne Einsatz, eingestellt in der Suite. */
  zoom: number;
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
  /** Fahrzeuge und feste Orte aus der Suite; das eigene Fahrzeug steht am GPS-Fix (`lat`/`lon`). Einen Standortpunkt gibt es nicht. */
  vehicles: Vehicle[];
  /** Feste Objekte der Lagekarte aus der Suite; ziehen die Karte nicht auf. */
  mapObjects: MapObject[];
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

/** Kantenlänge eines Fahrzeugzeichens; steht so auch in `monitor.css` (`.lm-fahrzeug img`). */
const FAHRZEUG_PX = 44;
/** Kantenlänge eines Zeichens der Lagekarte, etwas kleiner als ein Fahrzeug; steht so auch in `monitor.css` (`.lm-lagepunkt img`). */
const LAGEPUNKT_PX = 32;

/** Dünne Hülle um maplibre (läuft nicht in jsdom); die Logik steckt in `mapData.ts`. */
export default function MapPanel({
  lat,
  lon,
  zoom,
  offline,
  band,
  listed,
  radarAt,
  trafficFlow,
  nowMs,
  incidents,
  speedCameras,
  vehicles,
  mapObjects,
  suchlage,
  droneOpen,
  sideColumn,
  children,
}: Props) {
  const { effektiv } = useThemeMode();
  const container = useRef<HTMLDivElement>(null);
  const [map, setMap] = useState<MapLibreMap | null>(null);
  // Nur der Start der Karte; danach wandert der Punkt (GPS), ohne die Karte neu zu bauen.
  const start = useRef({ lat, lon });
  useEffect(() => {
    start.current = { lat, lon };
  }, [lat, lon]);

  // Neu erzeugt nur bei Theme, Kartenquelle oder Zoomstufe, nicht bei jedem Datenstand.
  useEffect(() => {
    const { lat, lon } = start.current;
    const f = effektiv === 'dark' ? farbenDunkel : farbenHell;
    const kindColor: ExpressionSpecification = ['match', ['get', 'kind'], 'band', f.alarm, f.achtung];
    const m = new MapLibreMap({
      container: container.current!,
      style: basemapStyle(effektiv, { offline }),
      center: [lon, lat],
      zoom,
      attributionControl: { compact: false },
    });
    // `style.load` statt `load`: `load` wartet auf die ersten Kacheln, und die hängen ohne Netz.
    m.once('style.load', () => {
      // Verkehrsfluss ganz unten: Regen, Warnungen und Einsatz liegen darüber.
      m.addSource('verkehr', { type: 'raster', tiles: [trafficTiles(effektiv === 'dark', 0)], tileSize: 256, maxzoom: 18, attribution: '© TomTom' });
      m.addLayer({ id: 'verkehr', type: 'raster', source: 'verkehr', layout: { visibility: 'none' }, paint: { 'raster-opacity': 0.8 } });
      m.addSource('radar', { type: 'raster', tiles: [radarTiles(0)], tileSize: 256, attribution: '© DWD' });
      m.addSource('warngebiete', { type: 'geojson', data: { type: 'FeatureCollection', features: [] } });
      m.addLayer({ id: 'radar', type: 'raster', source: 'radar', layout: { visibility: 'none' }, paint: { 'raster-opacity': 0.7 } });
      m.addLayer({ id: 'warngebiete-flaeche', type: 'fill', source: 'warngebiete', paint: { 'fill-color': kindColor, 'fill-opacity': 0.14 } });
      m.addLayer({
        id: 'warngebiete-kante',
        type: 'line',
        source: 'warngebiete',
        paint: { 'line-color': kindColor, 'line-width': 2, 'line-dasharray': [3, 2] },
      });
      // Flächen der Lagekarte über Radar und Warngebieten, unter Verkehr, Einsatz und Fahrzeugen; fein und
      // leicht gefüllt, damit sie den Hintergrund nicht zudecken. Die Namen stehen als HTML (Effekt unten).
      const lageTon = lageFarbAusdruck(effektiv === 'dark');
      m.addSource('lagekarte', { type: 'geojson', data: { type: 'FeatureCollection', features: [] } });
      m.addLayer({ id: 'lagekarte-flaeche', type: 'fill', source: 'lagekarte', paint: { 'fill-color': lageTon, 'fill-opacity': 0.14 } });
      m.addLayer({ id: 'lagekarte-kante', type: 'line', source: 'lagekarte', paint: { 'line-color': lageTon, 'line-width': 1.5 } });
      // Baustellen und Blitzer neutral (Text- und Grundfarbe): Rot und Gelb gehören den Warnungen,
      // Blau Suchlage. Eine Baustelle ist eine gestreifte Linie, eine Sperrung breiter.
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
  }, [effektiv, offline, zoom]);

  // `!map?.style`: Wechselt Theme, Kartenquelle oder Zoomstufe, entfernt der Effekt oben die alte Karte
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

  // Lagekarte: Flächen als Ebene, Namen und Punkte als HTML-Marker. HTML statt Kartenschrift, weil die Glyphen
  // nur online kommen (`basemapStyle.ts`) und eine Fläche ohne Namen nichts sagt. Neu gesetzt nur, wenn sich
  // etwas ändert; `map` und das Thema in den Abhängigkeiten, weil eine neu gebaute Karte leer beginnt.
  const lageKey = JSON.stringify(mapObjects);
  useEffect(() => {
    if (!map?.style) return;
    map.getSource<GeoJSONSource>('lagekarte')?.setData(lageFlaechen(mapObjects) as Data);
    const namen = lageBeschriftungen(mapObjects).map((b) => {
      const el = document.createElement('div');
      el.className = 'lm-lageflaeche';
      el.style.setProperty('--lm-lage', lageFarbe(b.farbe, dark));
      el.textContent = b.name;
      return new Marker({ element: el, anchor: 'center' }).setLngLat([b.lon, b.lat]).addTo(map);
    });
    const punkte = lagePunkte(mapObjects).map((p) => {
      const el = document.createElement('div');
      el.className = 'lm-lagepunkt';
      // Ohne Zeichen ein schlichter Punkt.
      const zeichen = document.createElement(p.src === null ? 'i' : 'img');
      if (p.src !== null) Object.assign(zeichen, { src: p.src, alt: '' });
      const name = document.createElement('span');
      name.textContent = p.name;
      el.append(zeichen, name);
      return new Marker({ element: el, anchor: 'top', offset: [0, -LAGEPUNKT_PX / 2] }).setLngLat([p.lon, p.lat]).addTo(map);
    });
    // Namen unter den Punkten erst nah dran; als Attribut an der Kartenfläche, `monitor.css` blendet sie ein.
    const flaeche = map.getContainer();
    const nah = () => flaeche.toggleAttribute('data-nah', map.getZoom() >= LAGEPUNKT_NAME_AB_ZOOM);
    nah();
    map.on('zoom', nah);
    return () => {
      map.off('zoom', nah);
      namen.forEach((m) => m.remove());
      punkte.forEach((m) => m.remove());
    };
    // `mapObjects` steckt in `lageKey`; ein neues Array je Abfrage soll die Marker nicht neu setzen.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [map, lageKey, dark]);

  // Fahrzeuge als HTML-Marker (Zeichen als `<img>`, Funkrufname darunter). Neu gesetzt nur, wenn sich etwas
  // Sichtbares ändert; `map` in den Abhängigkeiten, weil eine neu gebaute Karte (Theme, Zoom) leer beginnt.
  const marks = useMemo(() => vehicleMarks(vehicles, nowMs, { lat, lon }), [vehicles, nowMs, lat, lon]);
  const marksKey = JSON.stringify(marks);
  useEffect(() => {
    if (!map?.style) return;
    const markers = marks.map((v) => {
      const el = document.createElement('div');
      el.className = 'lm-fahrzeug';
      el.toggleAttribute('data-veraltet', v.stand !== null);
      const img = document.createElement('img');
      img.src = v.src;
      img.alt = '';
      const name = document.createElement('span');
      name.textContent = v.stand ? `${v.callSign} · ${v.stand}` : v.callSign;
      el.append(img, name);
      // Oben verankert und um die halbe Zeichenhöhe hoch: die Mitte des Zeichens liegt auf der Position.
      return new Marker({ element: el, anchor: 'top', offset: [0, -FAHRZEUG_PX / 2] }).setLngLat([v.lon, v.lat]).addTo(map);
    });
    return () => markers.forEach((m) => m.remove());
    // `marks` steckt in `marksKey`; ein neues Array je Abfrage soll die Marker nicht neu setzen.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [map, marksKey]);

  // Einpassen nur, wenn sich der Rahmen merklich ändert (neuer Einsatz, neues Gebiet) oder das
  // Drohnenbild ein- bzw. ausfährt – nicht bei jedem Spurpunkt.
  const bounds = suchlage ? suchlageBounds(suchlage) : null;
  const key = boundsKey(bounds);
  // Auf rund 100 m: Das GPS zittert, die Karte soll dabei stillhalten.
  const centerKey = `${lat.toFixed(3)},${lon.toFixed(3)}`;
  // Ohne Einsatz: um das eigene Zeichen, so weit heraus, dass die Zeichen im Umkreis mit drauf sind.
  const around = surroundingBounds(marks, { lat, lon });
  const aroundKey = JSON.stringify(around);
  useEffect(() => {
    if (!map?.style) return;
    const right = droneOpen ? Math.round(map.getContainer().clientWidth * coveredByDrone(sideColumn)) : 0;
    const padding = { top: 50, bottom: 50, left: 50, right: right + 50 };
    if (bounds) {
      map.fitBounds(bounds, { padding, maxZoom: 15, duration: 1200 });
      return;
    }
    // Rand für Zeichen und Funkrufnamen, die sonst halb aus dem Bild ragten.
    const fit = around ? map.cameraForBounds(around, { padding: { top: 70, bottom: 90, left: 120, right: right + 120 } }) : undefined;
    const z = Math.min(zoom, fit?.zoom ?? zoom);
    map.easeTo({ center: [lon, lat], zoom: z, padding: { top: 0, bottom: 0, left: 0, right }, duration: 800 });
    // `bounds` steckt in `key`, `around` in `aroundKey`; ein neues Array je Abfrage soll nicht neu einpassen.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [map, key, droneOpen, sideColumn, centerKey, zoom, aroundKey]);

  return (
    <section className="lm-paneel lm-karte">
      <div ref={container} className="lm-karte-flaeche" data-testid="map" />
      {children}
    </section>
  );
}
