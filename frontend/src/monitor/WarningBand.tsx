import type { Provider } from '../contract/Provider';
import type { Warning } from '../contract/Warning';
import { taktischText } from './format';

export const PROVIDER_LABEL: Record<Provider, string> = {
  dwd: 'DWD',
  mowas: 'MoWaS',
  katwarn: 'KATWARN',
  biwapp: 'BIWAPP',
  lhp: 'Hochwasserportal',
  police: 'Polizei',
};

/** Die schwerste Warnung über der Schwelle; Stufe immer auch als Wort, nicht nur als Farbe. */
export default function WarningBand({ band }: { band: Warning[] }) {
  const [top] = band;
  if (!top) return null;
  const source = PROVIDER_LABEL[top.provider];
  return (
    <section className="lm-band" aria-label="Warnung">
      <span className="lm-band-kuerzel">
        {top.event} · {top.levelLabel}
      </span>
      <span className="lm-band-titel">{top.headline}</span>
      <span className="lm-band-meta">
        {top.expires ? `bis ${taktischText(top.expires)} · ${source}` : source}
        {band.length > 1 && <span className="lm-band-weitere">+{band.length - 1} weitere</span>}
      </span>
    </section>
  );
}
