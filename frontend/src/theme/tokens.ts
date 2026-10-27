// Übernommen aus Suchlage d621418 (27.10.2026): frontend/src/theme/tokens.ts
// Übernommen aus Lifeline Hub 3eb9115f2 (16.10.2026): frontend/src/theme/tokens.ts (gekürzt: ohne Fachpaletten)
import { theme as antdTheme, type ConfigProviderProps, type ThemeConfig } from 'antd';
import type { MappingAlgorithm } from 'antd';

/**
 * Gestaltungssprache (LFH-352) in der Fassung des Neuentwurfs „Instrumententafel“.
 * Grundlage: `docs/superpowers/specs/2026-07-25-gestaltungssprache.md` und
 * `docs/design/2026-09-21-neuentwurf/umsetzung.md`; Abweichungen stehen mit Messwert an den
 * Werten (`farbenDunkel`).
 *
 * DIESE DATEI IST DIE TS-SEITE DER WAHRHEIT, die CSS-Seite steht als statische Custom
 * Properties in `rollen.css`; `rollen.guard.test.ts` hält beide deckungsgleich. antds
 * `--ant-*`-Variablen taugen nicht als Quelle: die meisten Rollen kennt antd nicht, und die
 * Namen gehören der Bibliothek.
 *
 * ROT BEDIENT NICHTS. `bedien` ist blau, `marke` ist rot, `alarm` ist rot in anderer Sättigung.
 */

/** Farbrollen eines Modus. Namen sind Rollen, nicht Farben. Jede Rolle existiert in BEIDEN
 *  Modi; was nur nachts gebraucht würde, wäre keine Rolle, sondern eine Ausnahme. */
export interface Farbrollen {
  grund: string;
  flaeche: string;
  flaeche2: string;
  linie: string;
  /** Kräftigere Haarlinie: Paneelkanten, Umrandung sekundärer Knöpfe. DEKORATIV; der Rahmen
   *  eines Steuerelements ist {@link Farbrollen.steuerRahmen}. */
  linieStark: string;
  rasterLinie: string;
  text: string;
  gedaempft: string;
  schwach: string;
  bedien: string;
  alarm: string;
  achtung: string;
  normal: string;
  marke: string;
  markeGlut: string;
  alarmFuellung: string;
  /** Zweite Intensität derselben Rolle, die obere Hälfte einer Flächenskala. KEIN neuer
   *  Farbton: `alarm` in stärkerer Füllung. */
  alarmFuellungStark: string;
  achtungFuellung: string;
  /** Zweite Intensität von `achtung` — Gegenstück zu {@link Farbrollen.alarmFuellungStark}. */
  achtungFuellungStark: string;
  normalFuellung: string;

  /** Kopfbänder IM INHALT: Tabellenkopf, Erfassungsleiste. Kommandoleiste und Rail lesen
   *  {@link rahmenFarben}, sie bleiben in beiden Modi dunkel. */
  kopf: string;
  /** Modulpanel, Seitenleisten, Kachel-Paneele — eine Stufe über `grund`. */
  paneel: string;
  /** Dritte Flächenstufe: aktive Zeile, leere Balkenspur, Zeilentrenner im Paneel. */
  flaeche3: string;
  /** Lauftext in Listen — zwischen `text` und `gedaempft`. */
  text2: string;
  /** Rahmen eines STEUERELEMENTS (antds `colorBorder`, Eingabefelder). Eigene Rolle,
   *  weil `linieStark` des Entwurfs WCAG 1.4.11 (≥ 3 : 1) nicht trägt — Messwerte am Wert. */
  steuerRahmen: string;
  /** Bedienfarbe unter dem Zeiger (antds `colorPrimaryHover`). */
  bedienHover: string;
  /** Text in Bedienfarbe AUF einer getönten Bedienfläche (`bedienFlaeche`, Sammelbanner). */
  bedienText: string;
  /** Vordergrund AUF einer satt gefüllten Bedien- oder Alarmfläche (Primär-/Gefahrknopf). */
  aufBedien: string;
  /** Text in Normalfarbe — nachts heller als die Füllfarbe `normal`, damit Zahl und Wort lesbar bleiben. */
  normalText: string;
  /** Text in Achtungsfarbe (Statuszahl, Hinweiswort). Am Tag dunkler als `achtung`, weil die
   *  Füllfarbe als Text den Tagesboden 7 : 1 nicht trägt; nachts gleich `achtung`. */
  achtungText: string;
  /** Text in Alarmfarbe — Gegenstück zu {@link Farbrollen.achtungText}. */
  alarmText: string;
  /** Gefahrrot unter dem Zeiger und beim Drücken (antds `colorErrorHover`/`colorErrorActive` am
   *  Knopf, LFH-693). Am Tag DUNKLER als `alarmText`: antd färbt mit demselben Ton auch die
   *  Schrift umrandeter Gefahrknöpfe, ein hellerer Ton trüge dort den Tagesboden nicht. */
  alarmHover: string;
  /** Deckende Statusflächen: „Ampel als Fläche, Zahl bleibt lesbar" (Statuszelle/-Chip). */
  normalFlaeche: string;
  achtungFlaeche: string;
  alarmFlaeche: string;
  bedienFlaeche: string;
  /** Fläche einer gewählten Option, eines gewählten Menüeintrags oder Knotens (antds
   *  `colorPrimaryBg`, LFH-984). Eigene Rolle statt `bedienFlaeche`: die hebt sich am Tag von der
   *  Zeigerspur und nachts von der Ruhe zu wenig ab (Messwerte über `farbenHell`/`farbenDunkel`,
   *  Vergleich in `openspec/changes/archive/2026-10-05-lfh-984-auswahlflaeche-rolle/design.md`, E1). */
  auswahlFlaeche: string;
  /** Sammelbanner („12 neue Meldungen"): Grund und Kante. */
  bannerGrund: string;
  bannerLinie: string;
  /** Zeilentönungen der Zeitachse: Berichtigung, Lücke, Problem. */
  berichtigungZeile: string;
  lueckeZeile: string;
  problemZeile: string;
  /** Platte unter einem taktischen Zeichen in Picker- und Inspector-Kacheln (LFH-836): die
   *  Bibliothek zeichnet mit schwarzer Tinte, nachts trüge das dunkle Paneel sie nicht. Am Tag
   *  durchsichtig (die Kachel sieht aus wie zuvor), nachts hell: Tinte ≥ 7 : 1, gegen `paneel`
   *  und `flaeche3` ≥ 3 : 1 (`tokens.test.ts`). Die Karte bleibt ohne Platte (LFH-833). */
  zeichenPlatte: string;
}

/**
 * Tagmodus, aus der Nachtpalette ABGELEITET. Statusrollen und die Füllungen behalten ihre
 * LFH-352-Werte; neu gestimmt sind Flächen- und Textstufen und `bedien`/`bedienHover`.
 *
 * Kontrast (WCAG; `grund` · `flaeche`): text 15,46 · 18,47 — text2 11,00 · 13,13 —
 * gedaempft 9,20 · 10,99 — schwach 7,53 · 8,99; steuerRahmen 3,30 · 3,95; normalText/
 * normalFlaeche 7,87, bedienText/bedienFlaeche 8,04.
 *
 * Primärknopf (LFH-661, Spec `farbrollen-kontrast`): die Beschriftung auf satter Bedienfläche
 * hält den TAGESBODEN 7 : 1, in Ruhe und unter dem Zeiger — KEIN eigener Knopfboden. Die
 * Knopfschrift misst 13,5 px, der große Anmelde-Knopf 16 px; WCAGs Großtext-Boden 4,5 gilt fett
 * erst ab 18,66 px. Weiß auf bedien 8,55 (vorher `#1a5fa0`, 6,59), auf bedienHover 7,32 (vorher
 * `#236aad`, 5,62): gleicher Ton und gleiche Sättigung, nur dunkler; der Zeiger hellt um
 * denselben Schritt auf wie zuvor (1,17). bedien auf grund 7,16. Gedrückt trägt den Zeigerton
 * (LFH-897), also ebenfalls 7,32 (antds Ableitung `#0a335e` lag bei 12,74). Gerechnet in
 * `bedienKontrast.test.ts`, gemessen in `e2e/primaerknopf-kontrast.spec.ts`.
 *
 * TEXTBODEN für JEDE Textstufe, auch den Tertiärtext `schwach` (Augenbraue, Meta,
 * Platzhalter): ≥ 7 : 1 auf jeder deckenden Fläche (LFH-643, Spec `textstufen-kontrast`). Die
 * dunkelste ist `flaeche3`: gedaempft 8,62, schwach 7,05. Die Rangfolge bleibt sichtbar,
 * benachbarte Stufen liegen ≥ 5 ΔL* auseinander (L* text 6,2 · text2 20,0 · gedaempft 25,4 ·
 * schwach 31,1). Gerechnet in `textstufen.test.ts`.
 *
 * `achtung`/`alarm` tragen als TEXT den Tagesboden nicht (auf ihrer Fläche 6,02 bzw. 5,52),
 * dafür stehen `achtungText`/`alarmText` (auf Weiß 9,22 bzw. 8,96, auf ihren Flächen und
 * Zeilentönungen ≥ 7,31, auf grund 7,72 bzw. 7,51). Die Füllfarben bleiben für Kante, Punkt
 * und Balken.
 *
 * Geerbter Text (LFH-652, `antdToken`): `bedienText` als Link auf grund 7,97 · flaeche 9,51 ·
 * kopf 8,33 · paneel 8,72; `gedaempft` als Beschreibung und Tabellenkopf auf kopf 9,62. Auch auf
 * der Hervorhebungsfläche `flaeche3` (Hover- und Aktivzeile) halten beide den Tagesboden: 7,46
 * bzw. 8,62 (LFH-702/LFH-877, Spec `textkontrast-rollen`). `bedienText` ist dafür gegenüber
 * LFH-652 um ein Zehntel dunkler (vorher `#164f86`, auf `flaeche3` 6,59), `gedaempft` trägt den
 * Wert aus LFH-643; `flaeche3` aufzuhellen hätte die Hervorhebung auf `grund` verschwinden
 * lassen. Gerechnet in `bedienKontrast.test.ts`.
 *
 * Gefahrrot (LFH-693, `antdKomponenten`): als Text `alarmText`, roter Menüeintrag auf flaeche2
 * 8,29, Gefahrknopf ohne Rahmen auf grund 7,51 (knappstes Paar); Weiß auf `alarmText` 8,96 (gefüllter Gefahrknopf in Ruhe, Menüeintrag unter dem Zeiger),
 * auf `alarmHover` 10,45 (Zeiger und Drücken); `alarmHover` als Schrift des umrandeten Knopfs auf
 * flaeche 10,45, auf grund 8,75, beim Drücken des Knopfs ohne Rahmen auf alarmFlaeche 8,52
 * (LFH-897; antds Tönung `#d69285` lag bei 4,13). `alarm` trüge hier nicht: als Text auf
 * flaeche2 6,27, Weiß darauf 6,78.
 *
 * Roter Fehlertext außerhalb von Formularen (LFH-874, `antdToken`: `colorErrorText`) ist
 * `alarmText`: ≥ 7,03 auf jeder deckenden Fläche (knappstes Paar flaeche3), auf der alarmierten
 * Karte (`alarmFlaeche`) 7,31. antds Ableitung `alarm` lag bei 5,31–6,78.
 *
 * Hinweisflächen (LFH-739, `antdKomponenten`: `Alert`) sind die Statusflächen; `steuerRahmen` als
 * Rand eines Knopfs darauf: bedienFlaeche 3,34 · achtungFlaeche 3,43 · alarmFlaeche 3,22 ·
 * normalFlaeche 3,39 (antds Ableitung lag bei 2,16 · 1,96 · 3,20 · 1,57). Gerechnet in
 * `hinweisKontrast.test.ts`.
 *
 * Auswahlfläche (LFH-984, `antdToken`: `colorPrimaryBg`): knappste Textstufe `schwach` 7,17,
 * `bedienText` 7,59, `steuerRahmen` 3,15; ΔE gegen `flaeche2` 9,2, gegen die Zeigerspur darauf
 * 7,5 (antds `#b9c1c4`: `schwach` 4,92, Rand 2,16). Gerechnet in `auswahlKontrast.test.ts`.
 */
export const farbenHell: Farbrollen = {
  grund: '#e9ebee',
  flaeche: '#ffffff',
  flaeche2: '#f5f6f8',
  linie: '#d4d8dd',
  linieStark: '#b9bfc6',
  rasterLinie: 'rgba(26, 95, 160, 0.07)',
  text: '#111418',
  gedaempft: '#363d45',
  schwach: '#424a53',
  bedien: '#154e84',
  alarm: '#b02318',
  achtung: '#7a5200',
  normal: '#1c6640',
  marke: '#a8071a',
  markeGlut: '0 0 10px 0 rgba(168, 7, 26, 0.3)',
  alarmFuellung: 'rgba(176, 35, 24, 0.07)',
  alarmFuellungStark: 'rgba(176, 35, 24, 0.2)',
  achtungFuellung: 'rgba(122, 82, 0, 0.08)',
  achtungFuellungStark: 'rgba(122, 82, 0, 0.2)',
  normalFuellung: 'rgba(28, 102, 64, 0.07)',
  kopf: '#eef0f2',
  paneel: '#f4f5f7',
  flaeche3: '#e1e4e8',
  text2: '#2b3138',
  steuerRahmen: '#79818a',
  bedienHover: '#185895',
  bedienText: '#144779',
  aufBedien: '#ffffff',
  normalText: '#155234',
  achtungText: '#604200',
  alarmText: '#8f1c12',
  alarmHover: '#7d1810',
  normalFlaeche: '#e3f1e8',
  achtungFlaeche: '#f7efd5',
  alarmFlaeche: '#f9e3e3',
  bedienFlaeche: '#e4edf7',
  auswahlFlaeche: '#dbe7f5',
  bannerGrund: '#e6eef8',
  bannerLinie: '#9dbbe0',
  berichtigungZeile: '#fbeaea',
  lueckeZeile: '#faf3da',
  problemZeile: '#f8eded',
  zeichenPlatte: 'transparent',
};

/**
 * Nachtbetrieb, die Vorgabe (`ThemeModeProvider`); Werte aus den Inline-Styles des
 * Neuentwurfs.
 *
 * Kontrast (WCAG; `grund` · `flaeche` · `flaeche2`): text 16,65 · 15,70 · 15,02 — text2 12,30 ·
 * 11,60 · 11,10 — gedaempft 7,71 · 7,27 · 6,96 — bedien 6,19 · 5,84 · 5,58 — alarm 7,18 · 6,77 ·
 * 6,48 — achtung 12,45 · 11,75 · 11,24 — normal 8,79 · 8,29 · 7,94 — aufBedien auf bedien 6,19,
 * auf alarm 7,18. Statusflächen: normalText 10,44, achtung 11,18, alarm 6,89, bedienText 9,65.
 * Gefahrrot (LFH-693): roter Menüeintrag auf flaeche2 6,48; aufBedien auf alarm 7,18 (gefüllter
 * Gefahrknopf, Menüeintrag unter dem Zeiger — vorher antds Weiß, 2,78), auf alarmHover 7,98
 * (Zeiger und Drücken, vorher gedrückt 3,74); alarmHover gedrückt ohne Rahmen auf alarmFlaeche
 * 7,66 (LFH-897, antds Tönung `#5b2e2e`: 4,49). Primärknopf gedrückt (LFH-897): aufBedien auf
 * bedienHover 9,00 (antds Ableitung `#396792`, dunkler als bedien: 3,35).
 * Fehlertext (LFH-874): `colorErrorText` = alarmText ≥ 6,35 auf jeder deckenden Fläche (flaeche3),
 * antds Ableitung `#dc5e5e` lag auf flaeche2 bei 4,97 und auf flaeche3 bei 4,88.
 * Geerbter Text (LFH-652): `bedienText` als Link ≥ 9,34 auf allen Flächenstufen, `gedaempft` als
 * Tabellenkopf auf kopf 7,48.
 * Hinweisflächen (LFH-739): `steuerRahmen` auf bedienFlaeche 3,32 · achtungFlaeche 3,26 ·
 * alarmFlaeche 3,48 · normalFlaeche 3,27 (antds Ableitung: 3,09 · 2,83 · 3,01 · 2,98).
 * Auswahlfläche (LFH-984): knappste Textstufe `schwach` 5,21, `bedienText` 9,53,
 * `steuerRahmen` 3,28; ΔE gegen `flaeche2` 12,3, gegen die Zeigerspur 16,2 (antds `#253a4e`:
 * `schwach` 3,39, Rand 2,13).
 *
 * Zwei bewusste Abweichungen vom Entwurf:
 * - `schwach` `#838b94` statt `#5f676f` (3,47 auf `grund`): die Rolle trägt über antds
 *   `colorTextTertiary`/`colorTextPlaceholder` echten Text und 10-px-Augenbrauen
 *   (`colorTextDescription` liest seit LFH-652 `gedaempft`) und hält deshalb den Nachtboden
 *   aller Textstufen, ≥ 5 : 1 auf jeder deckenden Fläche (LFH-643; 5,77 auf `grund`, 5,21 auf
 *   `flaeche2` = Dialog, 5,11 auf `flaeche3`).
 * - `steuerRahmen` `#626a73` trägt antds `colorBorder`, nicht `linieStark` (`#2e343a`, 1,49 auf
 *   `flaeche`). `#626a73` hält ≥ 3,21 auf allen Flächenstufen (WCAG 1.4.11).
 *
 * `marke` `#a8071a` liegt bei 2,57 : 1 auf `grund`: trägt als Dekoration (Logo, Rail-Marke),
 * nicht als Textfarbe.
 */
export const farbenDunkel: Farbrollen = {
  grund: '#08090b',
  flaeche: '#0f1215',
  flaeche2: '#14171b',
  linie: '#22262b',
  linieStark: '#2e343a',
  rasterLinie: 'rgba(77, 148, 214, 0.055)',
  text: '#e8ebee',
  gedaempft: '#9aa2ab',
  schwach: '#838b94',
  bedien: '#4d94d6',
  alarm: '#ff6b6b',
  achtung: '#e8cc3a',
  normal: '#52c41a',
  marke: '#a8071a',
  markeGlut: '0 0 12px 0 rgba(168, 7, 26, 0.55)',
  alarmFuellung: 'rgba(255, 107, 107, 0.1)',
  alarmFuellungStark: 'rgba(255, 107, 107, 0.24)',
  achtungFuellung: 'rgba(232, 204, 58, 0.1)',
  achtungFuellungStark: 'rgba(232, 204, 58, 0.24)',
  normalFuellung: 'rgba(82, 196, 26, 0.1)',
  kopf: '#0c0e11',
  paneel: '#0a0c0e',
  flaeche3: '#16191d',
  text2: '#c6ccd2',
  steuerRahmen: '#626a73',
  bedienHover: '#7db3e8',
  bedienText: '#8ec2f0',
  aufBedien: '#08090b',
  normalText: '#7ddc4a',
  achtungText: '#e8cc3a',
  alarmText: '#ff6b6b',
  alarmHover: '#e88a87',
  normalFlaeche: '#0d1a0a',
  achtungFlaeche: '#1c1705',
  alarmFlaeche: '#1c0a0d',
  bedienFlaeche: '#0d1620',
  auswahlFlaeche: '#08172b',
  bannerGrund: '#0d1520',
  bannerLinie: '#1d3a5c',
  berichtigungZeile: '#160d0f',
  lueckeZeile: '#161305',
  problemZeile: '#130f0f',
  zeichenPlatte: '#c9ced4',
};

/**
 * Der RAHMEN (Kommandoleiste 52 px, Rail 60 px) bleibt in BEIDEN Modi dunkel. Deshalb eine
 * modusunabhängige Palette statt weiterer `Farbrollen`: dort müsste der Nachtblock sie
 * wertgleich doppeln, und diese Redundanz verbietet `rollen.guard.test.ts`. Die Werte zeigen
 * auf die Nachtpalette statt sie zu kopieren. Rot ist hier `marke` (aktive Rail-Kategorie) und
 * `alarm` (Störungswort der SYNC-Zelle).
 *
 * Kontrast (LFH-434): der Rahmen wird auch bei Tageslicht gelesen, deshalb hält bedienbarer Text
 * die TAG-Schwelle ≥ 7 : 1 auf jedem Rahmengrund (`grund` · `feld` · `aktiv`): text 16,15 ·
 * 15,02 · 14,74 — gedaempft 8,04 · 7,47 · 7,33. Eine schwächere Textstufe gibt es im Rahmen nur
 * für Gesperrtes: `gesperrt` 5,60 auf `grund` (Boden 4,5; WCAG 1.4.3 nimmt inaktive Komponenten
 * aus), die Sperre trägt zusätzlich ein Zeichen ohne Farbe. Gerechnet in `rahmenKontrast.test.ts`.
 */
export const rahmenFarben = {
  grund: farbenDunkel.kopf,
  /** Aktive Rail-Kategorie. */
  aktiv: farbenDunkel.flaeche3,
  /** Suchfeld in der Kommandoleiste. */
  feld: farbenDunkel.flaeche2,
  linie: farbenDunkel.linie,
  text: farbenDunkel.text,
  /**
   * Zweite Textstufe: Uhr, Menüs, Suchfeld-Hinweis, inaktive Rail-Etiketten. Eigener Wert statt
   * `farbenDunkel.gedaempft` (`#9aa2ab`, 6,96 auf `feld`): der hielte im Suchfeld die
   * Tag-Schwelle nicht (LFH-434).
   */
  gedaempft: '#a0a8b1',
  /** NUR für gesperrte Einträge, nie für bedienbaren Text (LFH-434). */
  gesperrt: farbenDunkel.schwach,
  /**
   * Störungswort (GETRENNT/OFFLINE/PRÜFEN). Eigener Wert statt `farbenDunkel.alarm` (`#ff6b6b`,
   * 6,96 auf `grund`): hier 7,18 (LFH-434).
   */
  alarm: '#ff7070',
  marke: farbenDunkel.marke,
} as const;

/**
 * Abgedunkelte Maske hinter der Sprungpalette (`command-palette/CommandPalette.tsx`). Wie
 * {@link rahmenFarben} modusunabhängig und keine `Farbrolle`: in beiden Modi dieselbe
 * Abdunkelung, die Palette ist ein Fokusmoment, kein Farbträger. Hier statt in der Palette,
 * damit der Wert an der Farbquelle steht und nicht als Literal in einer Komponente (LFH-797).
 */
export const paletteMaske = 'rgba(5, 6, 8, 0.72)';

/** Abstandsraster einer Dichtestufe. Komponenten importieren diese Werte, statt Pixel zu
 *  erfinden. */
export interface Abstandsraster {
  xs: number;
  sm: number;
  md: number;
  lg: number;
}

/** Bediendichte nach A1 Festlegung 4. `kompakt` ist der Fükw und die ortsfeste
 *  Stelle, `komfortabel` das Führungs-Tablet und mobil, `handschuh` der Betrieb
 *  mit Einsatzhandschuh (72 px ≙ 19,05 mm, MIL-STD-1472F Fig. 12). */
export type Dichte = 'kompakt' | 'komfortabel' | 'handschuh';

interface Dichtestufe {
  /** `controlHeight` — trägt die Treffläche für alle Steuerelemente auf einmal. */
  zeilenhoehe: number;
  /**
   * `controlHeightSM`: die Höhe, die antd allen „kleinen“ Steuerelementen gibt. Steht hier, weil
   * antd sie sonst mit Faktor 0,75 ableitet und unter den Boden aus A1 Gate 3 fällt (siehe
   * {@link dichten}).
   */
  kleineZeilenhoehe: number;
  schriftgroesse: number;
  abstand: Abstandsraster;
}

/**
 * Die Staffel, Werte aus A1 Festlegung 4. `antdToken()` nimmt die Stufe als Parameter,
 * `rollen.css` spiegelt sie als `[data-dichte='…']`. `componentSize` scheidet als Träger aus:
 * dessen `large` endet bei 40 px. Die Grundschrift steigt nur einmal (13,5 → 15): der Handschuh
 * ändert die Hand, nicht das Auge.
 *
 * `kleineZeilenhoehe` steht hier statt abgeleitet zu werden: antd rechnet
 * `controlHeightSM = controlHeight × 0,75` (22,5 / 36 / 54) und unterschritte damit in jeder
 * Stufe den Boden aus Gate 3 (24 / 48 / 72). In `komfortabel` und `handschuh` fällt die kleine
 * Höhe deshalb mit der vollen zusammen: dort gibt es keine kleine Fläche. `controlHeightXS`/`LG`
 * bleiben abgeleitet, sie tragen keine Trefflächen-Aussage.
 */
export const dichten: Record<Dichte, Dichtestufe> = {
  kompakt: {
    zeilenhoehe: 30,
    kleineZeilenhoehe: 24,
    schriftgroesse: 13.5,
    // md/lg stehen so in A1; xs/sm sind [abgeleitet] (× 1,6 / × 2,4 der md-Stufe, gerundet).
    abstand: { xs: 3, sm: 7, md: 11, lg: 18 },
  },
  komfortabel: {
    zeilenhoehe: 48,
    kleineZeilenhoehe: 48,
    schriftgroesse: 15,
    // xs/sm [abgeleitet] nach demselben Faktor wie kompakt.
    abstand: { xs: 5, sm: 11, md: 18, lg: 28 },
  },
  handschuh: {
    zeilenhoehe: 72,
    kleineZeilenhoehe: 72,
    schriftgroesse: 15,
    // xs/sm [abgeleitet] nach demselben Faktor wie kompakt.
    abstand: { xs: 7, sm: 16, md: 26, lg: 44 },
  },
};

/** Das Abstandsraster der kompakten Stufe, für Konsumenten ohne Dichte. */
export const abstand: Abstandsraster = dichten.kompakt.abstand;

/**
 * Wiederkehrende Flächen- und Breitenmaße statt verstreuter Inline-Pixel. Norm für Neues und
 * ohnehin Angefasstes. Bewusst kein antd-Token: antd kennt weder Seitenbreite noch die Höhe
 * einer Ladefläche.
 */
export const flaeche = {
  /**
   * Lesebreite einer reinen Formular-/Editorseite (`breite="schmal"` an `EinsatzSeite` und
   * `AdminPage`). Listen, Übersichten und Zeitachsen füllen die ganze Inhaltsbreite.
   */
  seiteSchmal: 900,
  /** Abstand über einem Lade-/Fehlerzustand, damit er nicht am Kopf klebt. */
  zustandOben: 80,
  /** Mindestbreite einer Kachel im Kartenraster. */
  kachelMin: 260,
  /** …und die engere Variante für Nebenraster. */
  kachelMinKlein: 220,
} as const;

/**
 * Die Seitenrinne: der seitliche Rand einer Seite zum Fensterrand, `breit` am Fükw und am
 * Tablet, `schmal` auf dem Handschirm. Viewport-Achse, deshalb ein eigener Export und kein
 * Schlüssel in `flaeche` oder `dichten`.
 *
 * Wirksam wird sie als Custom-Property in `rollen.css` (Media-Regel an antds `md`-Schwelle), so
 * ist sie beim ersten Paint korrekt. `rollen.guard.test.ts` hält CSS- und TS-Seite gleich.
 */
export const seitenrinne = {
  /** Ab antds `md`-Schwelle (768 px) — Fükw, ortsfeste Stelle, Führungs-Tablet. */
  breit: 24,
  /** Darunter — mobil, ~390 px einhändig. */
  schmal: 12,
} as const;

/**
 * Breite des Navigations-Drawers, der den Einsatz-Rahmen unter antds `lg`-Schwelle ersetzt.
 * Viewport-Achse wie die Seitenrinne. Das Maß trägt, weil der Drawer ein flaches Akkordeon in
 * einer Spalte zeigt statt Rail plus Modulspalte.
 */
export const navDrawerBreite = 280;

/** Formrollen. Radius 0 ist eine Entscheidung: die Kachel trägt der Umrissrahmen, nicht eine
 *  weiche Ecke. `zeilenhoehe`/`schriftgroesse` sind Dichte und zeigen auf `dichten.kompakt`;
 *  wer eine Stufe braucht, liest `dichten`. */
export const form = {
  radiusFlaeche: 0,
  radiusSteuer: 0,
  radiusMarke: 0,
  zeilenhoehe: dichten.kompakt.zeilenhoehe,
  schriftgroesse: dichten.kompakt.schriftgroesse,
  versalSperrung: '2.2px',
  uebergang: '120ms cubic-bezier(0.2, 0.8, 0.2, 1)',
} as const;

/** Schriftrollen. Alle drei Familien werden lokal ausgeliefert (SIL OFL 1.1,
 *  `src/assets/fonts/`) — kein CDN, der Fükw arbeitet ohne Netz. */
export const schrift = {
  text: "'LFH Archivo', system-ui, -apple-system, sans-serif",
  display: "'LFH Archivo Narrow', 'LFH Archivo', system-ui, sans-serif",
  zahl: "'LFH JetBrains Mono', ui-monospace, 'SF Mono', Menlo, monospace",
} as const;

/** Die Schriftfamilie einer Stufe — ein Schlüssel in {@link schrift}, kein Familienname. */
type Schriftfamilie = keyof typeof schrift;

interface Schriftstufe {
  /** px */
  groesse: number;
  gewicht: 400 | 500 | 600 | 700;
  familie: Schriftfamilie;
  /** `letter-spacing` als CSS-Wert. Nur wo der Entwurf ihn setzt. */
  sperrung?: string;
  /** Versalien (`text-transform: uppercase`). */
  versal?: boolean;
}

/**
 * Schriftskala des Neuentwurfs („Kontrastsprung statt Abstufung“, `umsetzung.md`).
 *
 * Ergänzt die Dichte-Achse: antds Grundschrift bleibt `dichten[…].schriftgroesse`, die Skala
 * benennt die Stufen darüber und daneben, die handgebaute Bausteine setzen. Zahlen, Zeiten,
 * Funkrufnamen, Koordinaten und Nummern laufen immer in `zahl` (Mono, `tabular-nums`).
 *
 * CSS-Seite: `--lfh-typo-<stufe>-{groesse,gewicht,sperrung}` in `rollen.css`, deckungsgleich
 * gehalten von `rollen.guard.test.ts`; die Familie ist `--lfh-schrift-<familie>`.
 */
export const schriftskala = {
  /** Große Überschrift (Einstiegsflächen). */
  ueberschrift: { groesse: 30, gewicht: 600, familie: 'text', sperrung: '-0.02em' },
  /** Seitenkopf: Titel 14/600. */
  seitentitel: { groesse: 14, gewicht: 600, familie: 'text' },
  /** Lauftext-Grundmaß des Entwurfs. */
  text: { groesse: 14, gewicht: 400, familie: 'text' },
  /** Dichter Lauftext in Listen und Paneelen. */
  textKlein: { groesse: 12, gewicht: 400, familie: 'text' },
  /** Augenbraue: 10 px, 600, Versalien, Sperrung .14em, Farbe `schwach`. */
  augenbraue: { groesse: 10, gewicht: 600, familie: 'text', sperrung: '0.14em', versal: true },
  /** Etikett unter dem Rail-Icon: 9 px Versalien. */
  railEtikett: { groesse: 9, gewicht: 500, familie: 'text', sperrung: '0.06em', versal: true },
  /** Mono-Meta: Zeiten, Nummern, Zähler neben Text. */
  meta: { groesse: 11, gewicht: 400, familie: 'zahl' },
  /** Datenwert in Paneelzeilen und Kennzahlenbändern. */
  datenwertKlein: { groesse: 22, gewicht: 500, familie: 'zahl', sperrung: '-0.02em' },
  /** Datenwert einer Kennzahl-Kachel. */
  datenwert: { groesse: 32, gewicht: 500, familie: 'zahl', sperrung: '-0.02em' },
  /** Führende Kennzahl („Zahl führt"). */
  datenwertGross: { groesse: 40, gewicht: 500, familie: 'zahl', sperrung: '-0.02em' },
} as const satisfies Record<string, Schriftstufe>;

export type Schriftstufenname = keyof typeof schriftskala;

/**
 * Der antd-Algorithmus eines Modus.
 *
 * Nachts läuft zuerst antds `darkAlgorithm` (leitet Hover-, Aktiv-, Füll- und Randstufen ab),
 * danach {@link seedTreu}. `darkAlgorithm` rechnet auch die SEED-Farben um, und die lassen sich
 * über `token` nicht überschreiben (antd löscht Seed-Schlüssel aus dem Override,
 * `theme/util/alias.js`); ohne `seedTreu` trüge die antd-Fläche eine andere Farbe als
 * `rollen.css` daneben.
 */
export function antdAlgorithmus(dunkel: boolean): MappingAlgorithm | MappingAlgorithm[] {
  return dunkel ? [antdTheme.darkAlgorithm, seedTreu] : antdTheme.defaultAlgorithm;
}

/** Zweite Stufe des Nacht-Algorithmus: die Signalfarben tragen genau ihren Rollenwert. `colorLink`
 *  gehört dazu, weil er Text ist: die dunkle Palette verdunkelte ihn auf 4,55 : 1 (LFH-652). */
const seedTreu: MappingAlgorithm = (seed, abgeleitet) => ({
  ...(abgeleitet ?? antdTheme.darkAlgorithm(seed)),
  colorPrimary: seed.colorPrimary,
  colorLink: seed.colorLink,
  colorInfo: seed.colorInfo,
  colorError: seed.colorError,
  colorWarning: seed.colorWarning,
  colorSuccess: seed.colorSuccess,
});

/** antds fester Innenabstand der Schalterspur (`switch/style/index.js`: „Fixed value"). */
const SWITCH_SPURPOLSTER = 2;

/**
 * Die Maße des Kippschalters aus der Dichte-Staffel (LFH-380).
 *
 * antd leitet den `Switch` aus der Schrift ab, nicht aus `controlHeight`: die Spur ist
 * `fontSize × lineHeight` = `fontSize + 8` (`antd/es/switch/style/index.js`), also 21,5 / 23 /
 * 23 px. Boden ist `kleineZeilenhoehe` (24 / 48 / 72), die kurze Achse aus Gate 3.
 *
 * Gesetzt wird der GANZE abhängige Satz (Griff, Mindestbreite, Innenränder): antd rechnet ihn
 * aus der Schrift, und ein überschriebener Komponententoken zieht die übrigen nicht nach. Die
 * Formeln sind antds eigene mit der Spur als Eingang; ergibt 24 × 48 · 48 × 96 · 72 × 144. Die
 * `…SM`-Variante bleibt unberührt, `dichte.guard.test.ts` sperrt die Größen-Prop am `Switch`.
 *
 * Rein und exportiert, damit die Rechnung ohne Render prüfbar ist.
 */
export function switchMasse(stufe: Pick<Dichtestufe, 'kleineZeilenhoehe'>) {
  const trackHeight = stufe.kleineZeilenhoehe;
  const handleSize = trackHeight - 2 * SWITCH_SPURPOLSTER;
  return {
    trackHeight,
    trackPadding: SWITCH_SPURPOLSTER,
    handleSize,
    trackMinWidth: 2 * handleSize + 4 * SWITCH_SPURPOLSTER,
    innerMinMargin: handleSize / 2,
    innerMaxMargin: handleSize + 3 * SWITCH_SPURPOLSTER,
  };
}

/**
 * Senkrechtes Polster der Kopfzeile eines `Tabs` (LFH-724).
 *
 * antd rechnet den Tab aus der Schrift, nicht aus `controlHeight`: Höhe = `2 × paddingSM` +
 * Zeilenhöhe der Grundschrift (`fontSize + 8`, `getLineHeight`), also 35,5 / 45 / 55 px. In
 * `komfortabel` und `handschuh` lag damit jeder Tab (Aufträge/Befehle, UHS-Reiter) unter dem
 * Boden aus Gate 3. Das Polster wächst genau so weit, dass die Zeile die Steuerhöhe erreicht,
 * und nie unter `paddingSM`: `kompakt` bleibt unverändert. Kartentabs (`editable-card`) folgen
 * schon `controlHeightLG` und bleiben unberührt. Den Collapse-Kopf trägt {@link antdKlappkopf}
 * (LFH-653) am Kontext. Rein und exportiert wie {@link switchMasse}.
 */
export function kopfzeilenMasse(
  stufe: Pick<Dichtestufe, 'zeilenhoehe' | 'schriftgroesse' | 'abstand'>,
) {
  const zeile = stufe.schriftgroesse + 8;
  return { polsterVertikal: Math.max(stufe.abstand.sm, (stufe.zeilenhoehe - zeile) / 2) };
}

/**
 * Komponenten-Tokens, die aus den Rollen und der Dichte-Stufe folgen.
 *
 * `aufBedien` gehört an den KNOPF, nicht an antds globales `colorTextLightSolid`: das färbt auch
 * Tooltip, Avatar, Badge, Layout-Kopf u. v. m., und `#08090b` wäre dort nachts dunkel auf
 * dunkel.
 *
 * Textrollen statt Füll- und Hover-Tönen, wo antd TEXT färbt (LFH-652, Spec
 * `textkontrast-rollen`): die Beschriftung des Standardknopfs unter dem Zeiger (`bedienHover`
 * hält am Tag auf `flaeche2` nur 6,77, auf `grund` 6,13) und die Feldmeldung samt Pflichtmarke
 * (LFH-667; `alarm` lag am Tag bei 5,67 : 1 auf `grund`). Das `Form`-Token färbt nur Feldmeldung,
 * Pflichtsternchen und Rückmeldesymbol; die Felder selbst ziehen ihren Fehlerrand aus dem eigenen
 * Komponententoken.
 * Roter Text außerhalb des Formulars (`Typography` `danger`) läuft dagegen GLOBAL über
 * `colorErrorText` (LFH-874, Begründung am Wert in `antdToken`): das Token ist in antd schon die
 * Textrolle, kein Rand und keine Fläche liest es.
 * `colorError` global umzustellen träfe auch Ränder, Icons und Feldränder, und dort ist die
 * Füllfarbe richtig. Die Kante des Standardknopfs bleibt `bedienHover` (Boden 3 : 1). Ein
 * Linkknopf zeigt den Zeiger als `bedienFlaeche`, denn sein Ton wechselt nicht mehr (`bedienText`
 * darauf 8,04 Tag · 9,65 Nacht).
 *
 * Gefahrrot (LFH-693, Spec `farbrollen-kontrast`) läuft deshalb über die Komponenten-Tokens.
 * Das `Dropdown` färbt den roten Eintrag in Ruhe mit `colorError` und hinterlegt ihn unter dem
 * Zeiger mit `colorError`, die Schrift dort ist `colorTextLightSolid`. Der `Button` nimmt
 * `colorError` als Fläche des gefüllten und als Schrift des umrandeten und des `text`-Knopfs,
 * `colorErrorHover`/`colorErrorActive` ebenso unter dem Zeiger und beim Drücken (warum der
 * Zeiger dunkelt: {@link Farbrollen.alarmHover}).
 * Ebenso schreibt der gewählte Dropdown-Eintrag in `bedienText` statt `colorPrimary` (LFH-984,
 * Fläche: {@link Farbrollen.auswahlFlaeche}).
 *
 * Gedrückt (LFH-897, Spec `farbrollen-kontrast`) hält denselben Textboden und trägt den
 * Zeigerton, nicht antds Ableitung: `colorPrimaryActive` = `bedienHover` (die Fläche des
 * gedrückten Primärknopfs, auch der Rand des gedrückten Standardknopfs), `colorErrorBgActive` =
 * `alarmFlaeche` (die Tönung unter einem gedrückten Gefahrknopf ohne Rahmen; sonst liest sie im
 * Knopf nur die Variante `filled`, die die App nicht nutzt). Beide im `Button`, nicht global:
 * `colorPrimaryActive` färbt auch Eingaben und Auswahlen, die keinen Text auf satter Fläche
 * tragen.
 * GRENZE: antd setzt die Überschreibungen als CSS-Variablen auf die Wurzel des Popups bzw. des
 * Knopfs; sie gelten für den ganzen Teilbaum. Heute liest darin nur der Gefahreintrag
 * (`dropdown/style/status.js`) bzw. der Knopf selbst sie (antd 6.6.5). Wer Badge, gefüllten Tag,
 * Switch oder Spin in ein Dropdown oder einen Gefahrknopf legt, prüft deren Farbe.
 *
 * Hinweisflächen (LFH-739, Spec `farbrollen-kontrast`): der `Alert` liest `colorInfoBg`,
 * `colorWarningBg`, `colorErrorBg` und `colorSuccessBg` (`alert/style/index.js`), und diese vier
 * tragen die Statusflächen. Angepasst wird die FLÄCHE, nicht der Rand: `steuerRahmen` hält auf den
 * Statusflächen ≥ 3 : 1, ein eigener Knopfrand je Hinweistyp wäre eine zweite Randfarbe. Global
 * gesetzt träfen die vier Tokens auch Knopf, Eingabefeld, Menü, Select und Schritte. Der Override
 * landet ohne Komponentenpräfix als `--ant-color-error-bg` usw. auf der Wurzel des Hinweises,
 * reicht aber NICHT in antd-Bausteine darin: jeder trägt selbst die `css-var-…`-Klasse, die die
 * globalen Variablen neu setzt (im Browser belegt, antd 6.6.5). Ein Gefahrknopf ohne Rahmen im
 * Live-Banner steht in Ruhe auf `alarmFlaeche`, unter dem Zeiger auf antds globaler Tönung.
 *
 * Die Dichte ist PFLICHT: eine Vorgabe ließe den Schalter bei einem vergessenen Argument still
 * auf der kompakten Stufe stehen.
 */
export function antdKomponenten(
  farben: Farbrollen,
  dichte: Dichte,
): NonNullable<ThemeConfig['components']> {
  const stufe = dichten[dichte];
  const kopfPolster = kopfzeilenMasse(stufe).polsterVertikal;
  return {
    Button: {
      primaryColor: farben.aufBedien,
      dangerColor: farben.aufBedien,
      defaultHoverColor: farben.bedienText,
      defaultActiveColor: farben.bedienText,
      linkHoverBg: farben.bedienFlaeche,
      colorError: farben.alarmText,
      colorErrorHover: farben.alarmHover,
      colorErrorActive: farben.alarmHover,
      // Gedrückt trägt den Zeigerton (LFH-897), wie beim gefüllten Gefahrknopf. antd leitete
      // beide Töne selbst ab: der Primärknopf dunkelte nachts unter die dunkle Schrift ab
      // (3,35 : 1), der Gefahrknopf ohne Rahmen bekam eine satte Tönung (Tag 4,13, Nacht 4,49).
      colorPrimaryActive: farben.bedienHover,
      colorErrorBgActive: farben.alarmFlaeche,
    },
    Dropdown: {
      colorError: farben.alarmText,
      colorTextLightSolid: farben.aufBedien,
      // Der gewählte Eintrag (Statuswahl, ETB-Typwahl) und der Titel eines gewählten Untermenüs
      // sind Text in `colorPrimary` (LFH-984): `bedien` hielte auf der Auswahlfläche am Tag nur
      // 6,83, `bedienText` hält 7,59 (Nacht 9,53).
      colorPrimary: farben.bedienText,
    },
    // Gewählter Tag und Bereichsenden der Datums- und Zeitraumwahl, auch die Zelle unter dem
    // Zeiger, während das Ende gesucht wird (LFH-1068): Schrift `colorTextLightSolid` auf
    // `colorPrimary` (= `bedien`). antds Weiß lag nachts bei 3,22, `aufBedien` hält 6,19 (Tag
    // unverändert Weiß, 8,55). Die Tage dazwischen stehen auf `cellActiveWithRangeBg` (=
    // Auswahlfläche). `cellHoverWithRangeBg`/`cellRangeBorderColor` leitet antd 6.6.5 noch ab,
    // liest sie aber nirgends; sie bleiben deshalb ungesetzt. Im Picker liest nur die Zellschrift
    // (`date-picker/style/panel.js`) das Token; der OK-Knopf trägt seine eigenen Variablen.
    DatePicker: {
      colorTextLightSolid: farben.aufBedien,
    },
    Form: {
      colorError: farben.alarmText,
      colorWarning: farben.achtungText,
    },
    // Hinweisflächen tragen die Statusflächen ihrer Bedeutung (LFH-739). antd leitete sie aus den
    // Signalfarben ab, bei den dunklen Tagesrollen trübe Grautöne (Info `#b9c1c4`, Erfolg
    // `#9ca69f`), auf denen der Rand eines Knopfs (`steuerRahmen`) bis auf 1,57 : 1 fiel.
    Alert: {
      colorInfoBg: farben.bedienFlaeche,
      colorWarningBg: farben.achtungFlaeche,
      colorErrorBg: farben.alarmFlaeche,
      colorSuccessBg: farben.normalFlaeche,
    },
    Switch: switchMasse(stufe),
    Tabs: { horizontalItemPadding: `${kopfPolster}px 0` },
    // `colorPrimaryBorder` (= `bedien`, Fokusring LFH-737) ist auch die Ruhefarbe von Spur und
    // Griff. antd färbt den Griff unter dem Zeiger in `colorPrimary`, also gleich, und die Spur in
    // der abgeleiteten Hover-Stufe, nachts dunkler als `bedien`. Der Zeiger nimmt deshalb
    // `bedienHover` wie am Primärknopf.
    Slider: {
      trackHoverBg: farben.bedienHover,
      handleActiveColor: farben.bedienHover,
    },
  };
}

/**
 * Der Boden der kurzen Achse für JEDEN Knopf (LFH-381): `minWidth` = kleine Steuerhöhe der
 * Stufe, also 24 / 48 / 72 (A1 Gate 3).
 *
 * antds Polsterung kleiner Knöpfe ist das Literal 7 (`paddingInlineSM`), ohne Dichte: ein „OK“
 * blieb rund 38 px breit. Ein Boden statt mitwachsender Polsterung wirkt nur, wo der Knopf zu
 * schmal wäre, und lässt breite Etiketten unberührt.
 *
 * Am Kontext statt in CSS, damit er auch die Knöpfe erreicht, die antd selbst baut
 * (Bestätigungsblase, Modal-Fuß, Filter-Dropdown). Ein `style` am einzelnen Knopf schlägt den
 * Kontext; das ist der Weg für eine benannte Ausnahme, nicht für Neues.
 *
 * Grenze: als Inline-Stil überstimmt er antds Kreis-`minWidth` und erreicht `FloatButton`. Heute
 * gibt es keinen Aufrufer von beidem; wer einen einführt, prüft das.
 */
export function antdKnopf(dichte: Dichte = 'kompakt'): NonNullable<ConfigProviderProps['button']> {
  return { style: { minWidth: dichten[dichte].kleineZeilenhoehe } };
}

/**
 * Der Boden des Klappkopfs für JEDES `Collapse` (LFH-653): `minHeight` = Steuerhöhe der Stufe,
 * also 30 / 48 / 72 (A1 Gate 3), dazu die Beschriftung senkrecht mittig.
 *
 * antd rechnet den Kopf aus Schrift und Polsterung (`fontSize · lineHeight + 2 · paddingSM`,
 * `collapse/style/index.js`), nicht aus `controlHeight`: gemessen 36 / 45 / 55 px. Das Muster ist
 * das handgebaute Bedienziel aus LFH-365 — Boden plus Polsterung; die Polsterung bleibt antds
 * `paddingSM`/`padding`, die schon der Staffel folgen. In `kompakt` liegt der Inhalt mit 36 px
 * über dem Boden, der Kopf bleibt dort wie er war. Ohne `alignItems` stünde die Beschriftung im
 * 72-px-Kopf oben (antd: `flex-start`).
 *
 * Am Kontext statt je Stelle (rund 25 `Collapse`, kein gemeinsamer Baustein): antd führt
 * Kontext- und Komponenten-`styles` zusammen. Der Wert kommt aus derselben Zeile von
 * {@link dichten}, aus der `antdToken` `controlHeight` setzt — der Kontext sieht kein Token.
 */
export function antdKlappkopf(dichte: Dichte): NonNullable<ConfigProviderProps['collapse']> {
  return { styles: { header: { minHeight: dichten[dichte].zeilenhoehe, alignItems: 'center' } } };
}

/**
 * Der Boden des beschrifteten Kästchens für JEDE `Checkbox` (LFH-907): `minHeight` = kurze Achse
 * der Stufe, also 24 / 48 / 72 (A1 Gate 3), dazu Box und Text senkrecht mittig.
 *
 * antd rendert Box und Text als EIN `label.ant-checkbox-wrapper` (`display: inline-flex`) und
 * kennt für dessen Höhe kein Komponenten-Token: das Label ist so hoch wie seine Schrift, gemessen
 * 21,5 px in `kompakt` und 36 px in `handschuh`. Ohne `alignItems` stünde die Box im 72-px-Label
 * oben (antd: `baseline`). Der Boden ist `kleineZeilenhoehe`, nicht `zeilenhoehe`: in `kompakt`
 * steht das Kästchen oft in einer Text- oder Chipzeile (ETB „Werte behalten“) und wächst dort nur
 * auf den Gate-3-Boden, nicht auf die Steuerhöhe 30.
 *
 * Am Kontext statt als eigener Baustein oder als Regel in `rollen.css`: der Kontext erreicht jede
 * `Checkbox` auf einmal, ohne die rund zehn Stellen umzubauen, und folgt der Stufe aus derselben
 * Zeile von {@link dichten} wie `controlHeightSM` — ein Baustein bräuchte einen Wächter gegen das
 * nackte antd-Kästchen, und `rollen.css` trägt in den `[data-dichte]`-Blöcken nur Dichte-Properties
 * (`rollen.guard.test.ts`). Muster wie {@link antdKlappkopf}. Ein eigener `minHeight` an der Stelle
 * schlägt den Kontext (Stab-Checkliste: `checklistenZeileStil`; Spaltenschalter: `minHeight: 0`,
 * weil dort der Menüeintrag das Ziel ist).
 */
export function antdKaestchen(dichte: Dichte): NonNullable<ConfigProviderProps['checkbox']> {
  const boden = { minHeight: dichten[dichte].kleineZeilenhoehe, alignItems: 'center' } as const;
  // Nur das BESCHRIFTETE Kästchen: ein Kästchen ohne Text baut antd selbst in den Spaltenfilter
  // einer Tabelle (`FilterDropdown`, Text als Geschwister im Menüeintrag). Dort trägt der
  // Menüeintrag die Steuerhöhe schon; ein Boden am Label käme oben drauf (handschuh ≈ 120 px).
  return { styles: ({ props }) => (props.children == null ? {} : { root: boden }) };
}

/**
 * Der Boden des beschrifteten Optionsknopfs für JEDES `Radio` (LFH-1048): dasselbe Maß wie
 * {@link antdKaestchen}, 24 / 48 / 72, dazu Kreis und Text senkrecht mittig.
 *
 * antd baut `label.ant-radio-wrapper` wie das Kästchen-Label (`inline-flex`, Ausrichtung
 * `baseline`) und kennt für dessen Höhe ebenfalls kein Komponenten-Token: ohne Boden gemessen
 * 21,5 / 23 / 23 px (Kartenansicht löschen, `e2e/trefflaeche-optionsknopf.spec.ts`). Ohne Text
 * bleibt der Optionsknopf frei, wie die Auswahlspalte einer Tabelle ihn baut.
 *
 * `Radio.Button` und die Gruppe mit `optionType="button"` laufen durch dieselbe Stilfunktion, und
 * die Props verraten den Knopf nicht (antd liest ihn aus dem Gruppenkontext). Der Boden ist dort
 * wirkungslos: der Knopf ist `inline-block` (kein `alignItems`) und `controlHeight` bzw.
 * `controlHeightSM` hoch, beide nie unter `kleineZeilenhoehe` (`tokens.test.ts`).
 */
export function antdOptionsknopf(dichte: Dichte): NonNullable<ConfigProviderProps['radio']> {
  const boden = { minHeight: dichten[dichte].kleineZeilenhoehe, alignItems: 'center' } as const;
  return { styles: ({ props }) => (props.children == null ? {} : { root: boden }) };
}

/** Eine deckende Rollenfarbe `#rrggbb` als `rgba(…)` mit Deckkraft. */
function mitDeckkraft(hex: string, deckkraft: number): string {
  const [r, g, b] = [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16));
  return `rgba(${r}, ${g}, ${b}, ${deckkraft})`;
}

/**
 * Leitet die antd-Tokens aus den Rollen ab, eine Richtung, keine zweite Liste. Was antd nicht
 * kennt (Marke, Kartenraster, Versal-Sperrung), lebt allein in `rollen.css`.
 */
export function antdToken(farben: Farbrollen, dichte: Dichte = 'kompakt'): ThemeConfig['token'] {
  const stufe = dichten[dichte];
  return {
    colorPrimary: farben.bedien,
    colorPrimaryHover: farben.bedienHover,
    colorError: farben.alarm,
    // Roter TEXT trägt die Textrolle (LFH-874, Spec `textkontrast-rollen`). Anders als `colorError`
    // global gesetzt, weil antd `colorErrorText*` nur als Schriftfarbe liest (antd 6.6.5:
    // `typography/style` für `type="danger"`, `input/style/variants.js` und
    // `select/style/select-input.js` für Schrift und Beiwerk im Fehlerzustand der Variante
    // `filled`). Abgeleitet lag er am Tag bei `alarm` (5,67 auf `grund`), nachts bei `#dc5e5e`
    // (4,97 auf `flaeche2`). Zeiger und Drücken eines Links `danger` wie am Gefahrknopf
    // ({@link Farbrollen.alarmHover}). Nach einem antd-Update `grep -rln colorErrorText
    // node_modules/antd/es` wiederholen: liest ein Rand oder eine Fläche mit, gehört der Wert in
    // die Komponenten-Tokens. Gerechnet in `fehlertextKontrast.test.ts`.
    colorErrorText: farben.alarmText,
    colorErrorTextHover: farben.alarmHover,
    colorErrorTextActive: farben.alarmHover,
    colorWarning: farben.achtung,
    colorSuccess: farben.normal,
    colorInfo: farben.bedien,
    // Der Fokusring (LFH-737): antd zeichnet ihn als `outline` in `colorPrimaryBorder`
    // (`genFocusOutline`) und leitete ihn als helle Stufe aus `bedien` ab, Tag 2,57–3,28 und Nacht
    // 1,51–1,70 gegen die Flächen, unter WCAG 1.4.11. Als `bedien` hält er Tag ≥ 6,71, Nacht
    // ≥ 5,48 auf jeder deckenden Fläche, wie die eigenen Klassen in `sprache.css`. Gerechnet in
    // `bedienKontrast.test.ts`, gemessen in `e2e/fokusring-kontrast.spec.ts`.
    colorPrimaryBorder: farben.bedien,
    // Die Auswahlfläche (LFH-984, Spec `farbrollen-kontrast`): antd leitet `colorPrimaryBg` und
    // daraus `controlItemBgActive` aus `bedien` ab, am Tag ein trübes `#b9c1c4`, nachts `#253a4e`.
    // Darauf lagen Beschreibung (6,01 · 4,53), Tertiärtext (4,92 · 3,39) und der Rand eines
    // Steuerelements (2,16 · 2,13) unter dem Boden. Gesetzt GLOBAL, weil die Fläche überall „das
    // ist gewählt“ heißt: gewählte Option in Select/AutoComplete, gewählter Dropdown-Eintrag und
    // Baumknoten, dazu die eigenen Leser (Gefahrengebiete, Lagekarte, UHS-Grundriss, Slash-Menü).
    // Unter dem Zeiger dieselbe Fläche: zwischen den Böden ist für eine eigene Stufe mit
    // Abstand zu Ruhe, Zeigerspur und Auswahl kein Platz. Gerechnet in `auswahlKontrast.test.ts`,
    // gemessen in `e2e/auswahl-kontrast.spec.ts`.
    colorPrimaryBg: farben.auswahlFlaeche,
    colorPrimaryBgHover: farben.auswahlFlaeche,
    // Den Fokus-Halo von Eingabefeld und Auswahlliste (`activeShadow`, `activeOutlineColor`) leitet
    // antd aus `colorPrimaryBg` ab; aus der Auswahlfläche wäre er nachts dunkler als die Fläche und
    // verschwände. Er ist keine Auswahl, sondern eine Tönung der Bedienfarbe. Dekorativ: den Fokus
    // trägt der Ring (`colorPrimaryBorder`), der Halo trägt keinen Boden.
    controlOutline: mitDeckkraft(farben.bedien, 0.25),
    // Ein Link ist blauer TEXT und trägt die Textrolle, in Ruhe wie unter dem Zeiger (LFH-652):
    // aus `bedien` abgeleitet verdunkelte ihn die Nachtpalette auf 4,55, und antds Hover-Ableitung
    // hellt ihn am Tag auf rund 4,3 auf. Die Rückmeldung unter dem Zeiger ist die Unterstreichung,
    // kein hellerer Ton (`bedienHover` auf `grund` 6,13).
    colorLink: farben.bedienText,
    colorLinkHover: farben.bedienText,
    colorLinkActive: farben.bedienText,
    linkHoverDecoration: 'underline',

    colorBgLayout: farben.grund,
    colorBgContainer: farben.flaeche,
    colorBgElevated: farben.flaeche2,

    colorText: farben.text,
    colorTextSecondary: farben.gedaempft,
    // Beschreibung ist Text (`Typography` `secondary`, Seitenbeschreibung, „—“). antd leitete sie
    // aus der tertiären Stufe ab, `schwach` läge am Tag bei 5,33–6,37 (LFH-652). `schwach` bleibt
    // Augenbraue, Platzhalter und Icons (LFH-643).
    colorTextDescription: farben.gedaempft,
    colorTextTertiary: farben.schwach,
    // Der Platzhalter ist bei mehreren Filtern die EINZIGE Beschriftung. antds Ableitung aus
    // `colorTextQuaternary` lag unter 2,5 : 1; `schwach` hält den Textboden (Tag ≥ 7, Nacht ≥ 5).
    colorTextPlaceholder: farben.schwach,

    // Der Rahmen eines Steuerelements, nicht die dekorative Linie (Messwerte bei `farbenDunkel`).
    colorBorder: farben.steuerRahmen,
    colorBorderSecondary: farben.linie,

    borderRadius: form.radiusFlaeche,
    borderRadiusSM: form.radiusSteuer,
    borderRadiusLG: form.radiusFlaeche,

    fontFamily: schrift.text,
    fontFamilyCode: schrift.zahl,
    // Archivo 600 wird lokal ausgeliefert.
    fontWeightStrong: 600,
    fontSize: stufe.schriftgroesse,

    // `controlHeight` trägt die Dichte für alle Steuerelemente auf einmal.
    controlHeight: stufe.zeilenhoehe,
    // Ohne diese Zeile leitet antd mit Faktor 0,75 ab und unterschreitet den Boden aus A1 Gate 3
    // (siehe `dichten`); so hebelt auch antds erzwungene Kleingröße (z. B. in einer
    // Bestätigungsblase) die Dichteachse nicht aus.
    controlHeightSM: stufe.kleineZeilenhoehe,
    padding: stufe.abstand.md,
    paddingSM: stufe.abstand.sm,
    paddingXS: stufe.abstand.xs,
    paddingLG: stufe.abstand.lg,
    margin: stufe.abstand.md,
    marginSM: stufe.abstand.sm,
    marginXS: stufe.abstand.xs,
    marginLG: stufe.abstand.lg,

    motionDurationMid: '120ms',
    motionDurationSlow: '150ms',
  };
}
