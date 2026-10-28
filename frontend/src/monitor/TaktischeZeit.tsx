import { taktisch } from './format';

/** Taktische Zeit mit hervorgehobener Uhrzeit; Tag und Monat treten zurück. */
export default function TaktischeZeit({ at, className }: { at: number | string; className?: string }) {
  const t = taktisch(at);
  return (
    <span className={className ? `lm-dtg ${className}` : 'lm-dtg'}>
      <span className="lm-dtg-datum">{t.tag}</span>
      <span className="lm-dtg-zeit">{t.zeit}</span>
      <span className="lm-dtg-datum">{t.monatJahr}</span>
    </span>
  );
}
