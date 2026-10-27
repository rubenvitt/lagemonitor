import type { Sourced } from '../contract/Sourced';
import type { Warning } from '../contract/Warning';
import type { Weather } from '../contract/Weather';
import { clock, decimal } from './format';

/** Bright-Sky-`condition` und -`icon` als deutsches Wort; Unbekanntes bleibt Rohwert. */
const WORDS: Record<string, string> = {
  dry: 'trocken',
  fog: 'Nebel',
  rain: 'Regen',
  sleet: 'Schneeregen',
  snow: 'Schnee',
  hail: 'Hagel',
  thunderstorm: 'Gewitter',
  'clear-day': 'klar',
  'clear-night': 'klar',
  'partly-cloudy-day': 'heiter',
  'partly-cloudy-night': 'heiter',
  cloudy: 'bedeckt',
  wind: 'windig',
};

/** `condition` hat Vorrang; nur „trocken“ überlässt dem Symbol das Wort, weil es mehr sagt (bedeckt, klar). */
export function weatherWord(condition: string | null, icon: string | null): string {
  const key = condition && condition !== 'dry' ? condition : (icon ?? condition);
  return key === null ? '–' : (WORDS[key] ?? key);
}

const COMPASS = ['N', 'NO', 'O', 'SO', 'S', 'SW', 'W', 'NW'];
const compass = (deg: number) => COMPASS[Math.round(deg / 45) % 8];
const temp = (t: number | null) => (t === null ? '–' : `${Math.round(t)}°`);
const kmh = (v: number | null) => (v === null ? '–' : `${Math.round(v)} km/h`);

interface Props {
  /** `null`: Wetter ist ausgeschaltet, das Paneel trägt nur die Warnliste. */
  weather: Sourced<Weather> | null;
  label: string;
  /** Warnungen unter der Schwelle. */
  listed: Warning[];
}

export default function WeatherPanel({ weather, label, listed }: Props) {
  const data = weather?.data;
  const c = data?.current;
  return (
    <section className="lm-paneel lm-wetter">
      {weather && <div className="lm-augenbraue">Wetter · {label}</div>}
      {!weather ? null : !c ? (
        <div className="lm-leer">Noch keine Daten</div>
      ) : (
        <>
          <div className="lm-wetter-jetzt">
            <span className="lm-gross">{temp(c.temperature)}</span>
            <span>{weatherWord(c.condition, c.icon)}</span>
          </div>
          <div className="lm-zeile">
            <span>Wind</span>
            <span className="lm-zahl">
              {c.windDirection !== null && `${compass(c.windDirection)} `}
              {kmh(c.windSpeed)}
            </span>
          </div>
          <div className="lm-zeile">
            <span>Böen</span>
            <span className="lm-zahl">{kmh(c.windGustSpeed)}</span>
          </div>
          <div className="lm-zeile">
            <span>Niederschlag</span>
            {/* Bright Sky liefert die Summe der letzten 10 Minuten (precipitation_10), keine Rate. */}
            <span className="lm-zahl">{c.precipitation === null ? '–' : `${decimal(c.precipitation, 1)} mm/10 min`}</span>
          </div>
          <div className="lm-vorhersage">
            {data.hourly
              .filter((_, i) => i % 3 === 2)
              .map((h) => (
                <div key={h.timestamp} data-testid="forecast-hour">
                  <span>{clock(h.timestamp).slice(0, 2)}</span>
                  <span>{temp(h.temperature)}</span>
                </div>
              ))}
          </div>
        </>
      )}
      {listed.length > 0 && (
        <>
          <div className={weather ? 'lm-augenbraue lm-abstand' : 'lm-augenbraue'}>Weitere Warnungen</div>
          {listed.map((w) => (
            <div key={w.id} className="lm-zeile lm-zeile-achtung">
              <span>{w.event}</span>
              <span className="lm-zahl">{w.levelLabel}</span>
            </div>
          ))}
        </>
      )}
    </section>
  );
}
