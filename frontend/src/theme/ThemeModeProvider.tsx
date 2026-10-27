// Übernommen aus Suchlage d621418 (27.10.2026): frontend/src/theme/ThemeModeProvider.tsx
// Übernommen aus Lifeline Hub 3eb9115f2 (16.10.2026): frontend/src/theme/ThemeModeProvider.tsx (Speicherschlüssel lagemonitor.*)
import { createContext, useCallback, useContext, useEffect, useId, useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import { ConfigProvider } from 'antd';
import deDE from 'antd/locale/de_DE';
import {
  antdAlgorithmus,
  antdKaestchen,
  antdKlappkopf,
  antdKnopf,
  antdKomponenten,
  antdOptionsknopf,
  antdToken,
  farbenDunkel,
  farbenHell,
  type Dichte,
} from './tokens';
import { sicherLesen, sicherSchreiben } from '../lib/sichererSpeicher';
import { DICHTE_DEFAULT, startDichte } from './dichte';
import {
  HELLIGKEIT_DEFAULT,
  abdunkelung,
  alsHelligkeit,
  wirksameHelligkeit,
  type Helligkeit,
} from './helligkeit';

/** Vom Nutzer wählbarer Modus. `system` folgt der OS-Einstellung. */
export type ThemeModus = 'system' | 'light' | 'dark';
/** Tatsächlich angewandtes Theme nach Auflösung von `system`. */
type EffektivesTheme = 'light' | 'dark';

const SPEICHER_SCHLUESSEL = 'lagemonitor.theme';
const DICHTE_SCHLUESSEL = 'lagemonitor.dichte';
/** Gespiegelt im Bootstrap-Skript von `index.html` (LFH-397) — wer eines ändert, ändert beide. */
const HELLIGKEIT_SCHLUESSEL = 'lagemonitor.helligkeit';

interface ThemeModeWert {
  modus: ThemeModus;
  effektiv: EffektivesTheme;
  setModus: (m: ThemeModus) => void;
  /** Gewählte Bediendichte. Kein zweiter Typ wie bei `ThemeModus`/`EffektivesTheme`: die Wahl
   *  IST die effektive Stufe. Die Zeigerart entscheidet nur, womit eine Sitzung ohne
   *  gespeicherte Wahl beginnt (`gespeicherteDichte`); ein `automatisch` könnte die Wahl
   *  überstimmen. */
  dichte: Dichte;
  setDichte: (d: Dichte) => void;
  /** Gewählte Helligkeit (LFH-397). Die WAHL — was wirkt, sagt `helligkeitWirksam`. */
  helligkeit: Helligkeit;
  setHelligkeit: (h: Helligkeit) => void;
  /** Die Stufe am Bildschirm: die Wahl, bei aktiver Warnung mindestens der Boden. */
  helligkeitWirksam: Helligkeit;
  warnungAktiv: boolean;
  /** Meldet eine Warnquelle an oder ab (nur über `useWarnsperre`). */
  meldeWarnung: (quelle: string, aktiv: boolean) => void;
}

const ThemeModeContext = createContext<ThemeModeWert | null>(null);

function istThemeModus(wert: string | null): wert is ThemeModus {
  return wert === 'system' || wert === 'light' || wert === 'dark';
}

/**
 * Modus ohne gespeicherte Wahl: NACHTBETRIEB. Gespiegelt im Bootstrap-Skript von `index.html`;
 * wer eines ändert, ändert beide.
 */
const MODUS_DEFAULT: ThemeModus = 'dark';

/** Ohne Speicher (gesperrt, LFH-942) gilt die Vorgabe wie ohne gespeicherte Wahl. */
function gespeicherterModus(): ThemeModus {
  const wert = sicherLesen(SPEICHER_SCHLUESSEL);
  return istThemeModus(wert) ? wert : MODUS_DEFAULT;
}

/**
 * Die Stufe, mit der eine Sitzung beginnt — die Regel steht in `./dichte` (LFH-724, Spec
 * `bedien-dichte`): Wahl → Zeigerart → `kompakt`. Hier nur die Quellen: der Speicher des
 * Geräts und die Zeigerfrage aus `useViewport` (`zeigerIstGrob`), kein eigenes `matchMedia`.
 * Gelesen EINMAL im `useState`-Initialisierer; ein Zuhörer auf die Zeigerart fehlt mit
 * Absicht (Spec: keine Umschaltung während der Sitzung).
 */
function gespeicherteDichte(): Dichte {
  return startDichte(sicherLesen(DICHTE_SCHLUESSEL), window.matchMedia('(pointer: coarse)').matches);
}

function gespeicherteHelligkeit(): Helligkeit {
  return alsHelligkeit(sicherLesen(HELLIGKEIT_SCHLUESSEL));
}

function systemBevorzugtDunkel(): boolean {
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

export function ThemeModeProvider({ children }: { children: ReactNode }) {
  const [modus, setModusState] = useState<ThemeModus>(gespeicherterModus);
  const [systemDunkel, setSystemDunkel] = useState<boolean>(systemBevorzugtDunkel);
  const [dichte, setDichteState] = useState<Dichte>(gespeicherteDichte);
  const [helligkeit, setHelligkeitState] = useState<Helligkeit>(gespeicherteHelligkeit);
  // Die angemeldeten Warnquellen (LFH-397). Eine MENGE statt eines Schalters: endet die
  // Warnung der einen Quelle, darf das die Sperre einer zweiten nicht mit aufheben.
  const [warnQuellen, setWarnQuellen] = useState<ReadonlySet<string>>(() => new Set());

  // OS-Einstellung live verfolgen — relevant, sobald der Modus `system` ist.
  useEffect(() => {
    const mql = window.matchMedia('(prefers-color-scheme: dark)');
    const aktualisiere = (e: MediaQueryListEvent) => setSystemDunkel(e.matches);
    mql.addEventListener('change', aktualisiere);
    return () => mql.removeEventListener('change', aktualisiere);
  }, []);

  const setModus = useCallback((m: ThemeModus) => {
    // Erst der State: Scheitert das Speichern, gilt die Wahl bis zum Neuladen (LFH-942).
    setModusState(m);
    sicherSchreiben(SPEICHER_SCHLUESSEL, m);
  }, []);

  const setDichte = useCallback((d: Dichte) => {
    setDichteState(d);
    sicherSchreiben(DICHTE_SCHLUESSEL, d);
  }, []);

  const setHelligkeit = useCallback((h: Helligkeit) => {
    setHelligkeitState(h);
    sicherSchreiben(HELLIGKEIT_SCHLUESSEL, String(h));
  }, []);

  const meldeWarnung = useCallback((quelle: string, aktiv: boolean) => {
    setWarnQuellen((vorher) => {
      // Unverändert → dieselbe Menge zurück, sonst rendert jeder Effektlauf den Baum neu.
      if (vorher.has(quelle) === aktiv) return vorher;
      const neu = new Set(vorher);
      if (aktiv) neu.add(quelle);
      else neu.delete(quelle);
      return neu;
    });
  }, []);

  const warnungAktiv = warnQuellen.size > 0;
  const helligkeitWirksam = wirksameHelligkeit(helligkeit, warnungAktiv);

  const effektiv: EffektivesTheme = modus === 'system' ? (systemDunkel ? 'dark' : 'light') : modus;

  // `data-theme` + `color-scheme` am <html> setzen, damit reines CSS
  // (z. B. LoginPage, ETB-Berichtigungszeilen) auf den Modus reagieren kann.
  useEffect(() => {
    document.documentElement.dataset.theme = effektiv;
    document.documentElement.style.colorScheme = effektiv;
  }, [effektiv]);

  // Das Merkmal am <html> schaltet die `[data-dichte='…']`-Blöcke in `rollen.css`; ohne es folgte
  // nur die antd-Fläche und das handgeschriebene CSS bliebe kompakt. Gesetzt für ALLE Stufen:
  // `kompakt` lebt unter `:root`. Wer einen `[data-dichte='kompakt']`-Block ergänzt, zieht
  // `DICHTE_BLOECKE` in `rollen.guard.test.ts` nach.
  useEffect(() => {
    document.documentElement.dataset.dichte = dichte;
  }, [dichte]);

  // Die Helligkeit wirkt ausschließlich über dieses Merkmal (LFH-397): die schwarze
  // Deckschicht in `rollen.css` liest `--lfh-abdunkelung`. Gesetzt wird die WIRKSAME Stufe,
  // nicht die Wahl — sonst dimmte die Anzeige an der Sperre vorbei. Paletten und
  // antd-Tokens bleiben unberührt (design.md D4).
  useEffect(() => {
    const wurzel = document.documentElement;
    wurzel.dataset.helligkeit = String(helligkeitWirksam);
    wurzel.style.setProperty('--lfh-abdunkelung', String(abdunkelung(helligkeitWirksam)));
  }, [helligkeitWirksam]);

  const knopf = useMemo(() => antdKnopf(dichte), [dichte]);
  const klappkopf = useMemo(() => antdKlappkopf(dichte), [dichte]);
  const kaestchen = useMemo(() => antdKaestchen(dichte), [dichte]);
  const optionsknopf = useMemo(() => antdOptionsknopf(dichte), [dichte]);

  const wert = useMemo<ThemeModeWert>(
    () => ({
      modus,
      effektiv,
      setModus,
      dichte,
      setDichte,
      helligkeit,
      setHelligkeit,
      helligkeitWirksam,
      warnungAktiv,
      meldeWarnung,
    }),
    [
      modus,
      effektiv,
      setModus,
      dichte,
      setDichte,
      helligkeit,
      setHelligkeit,
      helligkeitWirksam,
      warnungAktiv,
      meldeWarnung,
    ],
  );

  return (
    <ThemeModeContext.Provider value={wert}>
      <ConfigProvider
        locale={deDE}
        // Boden der kurzen Achse (LFH-381): die Breite eines beschrifteten Knopfs folgt der Staffel
        // nur über diesen Kontext-Stil.
        button={knopf}
        // Boden des Klappkopfs (LFH-653): antd rechnet den Kopf aus der Schrift, nicht aus
        // `controlHeight`; der Kontext erreicht jedes `Collapse` auf einmal.
        collapse={klappkopf}
        // Boden des beschrifteten Kästchens (LFH-907): antd hat für die Höhe des Labels kein
        // Komponenten-Token, es wäre nur so hoch wie seine Schrift.
        checkbox={kaestchen}
        // Boden des beschrifteten Optionsknopfs (LFH-1048): gebaut wie das Kästchen-Label.
        radio={optionsknopf}
        theme={{
          // Farbrollen je Modus aus derselben Quelle wie `rollen.css`; die Dichte hängt hier und nicht
          // an einer Größen-Prop je Element (die endet bei 40 px). `antdToken` setzt die kleine
          // Steuerhöhe mit, sonst rechnete antd sie unter den Boden aus A1 Gate 3.
          token: antdToken(effektiv === 'dark' ? farbenDunkel : farbenHell, dichte),
          // Nachts hält `antdAlgorithmus` die Signalfarben auf ihrem Rollenwert.
          algorithm: antdAlgorithmus(effektiv === 'dark'),
          // antds Switch rechnet seine Maße aus der Schrift statt aus `controlHeight` (LFH-380).
          components: antdKomponenten(effektiv === 'dark' ? farbenDunkel : farbenHell, dichte),
        }}
      >
        {children}
      </ConfigProvider>
    </ThemeModeContext.Provider>
  );
}

/**
 * Liefert den aktuellen Theme-Modus. Außerhalb des Providers (z. B. in isolierten Tests) wird
 * die Vorgabe `dark` zurückgegeben statt zu werfen, dieselbe, mit der die App startet.
 */
export function useThemeMode(): ThemeModeWert {
  const wert = useContext(ThemeModeContext);
  if (wert) return wert;
  return {
    modus: MODUS_DEFAULT,
    effektiv: 'dark',
    setModus: () => {},
    dichte: DICHTE_DEFAULT,
    setDichte: () => {},
    helligkeit: HELLIGKEIT_DEFAULT,
    setHelligkeit: () => {},
    helligkeitWirksam: HELLIGKEIT_DEFAULT,
    warnungAktiv: false,
    meldeWarnung: () => {},
  };
}

/**
 * Benannter Zugang zur Bediendichte. Bedienwege sind die Umschaltgruppe im Benutzermenü (der
 * einzige, der die aktive Stufe auch anzeigt) und die Kommandopalette.
 *
 * Das zurückgegebene Objekt ist je Aufruf frisch und gehört NICHT in ein Dependency-Array;
 * `dichte` und `setDichte` (identitätsstabil) gehören hinein.
 */
export function useDichte(): { dichte: Dichte; setDichte: (d: Dichte) => void } {
  const { dichte, setDichte } = useThemeMode();
  return { dichte, setDichte };
}

/**
 * Benannter Zugang zur Helligkeit (LFH-397), wie `useDichte` für die Dichte.
 *
 * `helligkeit` ist die WAHL (Anzeige im Menü, Speicher), `wirksam` die Stufe am
 * Bildschirm. Beide zu trennen ist die Aussage der Sperre: sie hebt an, was wirkt, und
 * lässt stehen, was gewählt ist.
 *
 * Wie bei `useDichte` ist das Objekt je Aufruf frisch — nicht in ein Dependency-Array.
 */
export function useHelligkeit(): {
  helligkeit: Helligkeit;
  setHelligkeit: (h: Helligkeit) => void;
  wirksam: Helligkeit;
  warnungAktiv: boolean;
} {
  const { helligkeit, setHelligkeit, helligkeitWirksam, warnungAktiv } = useThemeMode();
  return { helligkeit, setHelligkeit, wirksam: helligkeitWirksam, warnungAktiv };
}

/**
 * Meldet eine aktive Warnung an den Helligkeitsregler (LFH-397, design.md D3).
 *
 * Der Provider liegt über dem Router und kennt keinen Einsatz — die Quelle, die eine
 * Warnung erkennt, meldet sie hier. Beim Abbau (Einsatz verlassen) meldet sie sich
 * selbst ab; eine vergessene Abmeldung könnte die Sperre sonst nie lösen.
 */
export function useWarnsperre(aktiv: boolean): void {
  const quelle = useId();
  const { meldeWarnung } = useThemeMode();
  useEffect(() => {
    meldeWarnung(quelle, aktiv);
    return () => meldeWarnung(quelle, false);
  }, [quelle, aktiv, meldeWarnung]);
}
