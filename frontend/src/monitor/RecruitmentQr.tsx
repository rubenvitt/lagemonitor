import { useMemo } from 'react';
import { encode } from 'uqr';
import type { Recruitment } from '../contract/Recruitment';
import { qrFarben } from '../theme/tokens';

const GROESSE = 168;
/** Ruhezone in Modulen, wie die Norm sie verlangt. */
const RUHEZONE = 4;

/** Alle dunklen Module als ein Pfad: ein Element statt einiger Hundert. */
export function qrPfad(wert: string): { pfad: string; module: number } {
  const { data, size } = encode(wert, { ecc: 'M', border: RUHEZONE });
  let pfad = '';
  data.forEach((zeile, y) =>
    zeile.forEach((dunkel, x) => {
      if (dunkel) pfad += `M${x} ${y}h1v1h-1z`;
    }),
  );
  return { pfad, module: size };
}

/**
 * QR-Code fürs Ehrenamt, unten rechts auf der Karte. Schwarz auf Weiß auch nachts: ein invertierter
 * Code liest nicht jedes Telefon; dazu vier Module Ruhezone, wie die Norm sie verlangt. Ob er zu sehen ist (nur in ruhiger Lage), entscheidet `MonitorPage`.
 */
export default function RecruitmentQr({ recruitment }: { recruitment: Recruitment }) {
  const { pfad, module } = useMemo(() => qrPfad(recruitment.url), [recruitment.url]);
  return (
    <aside className="lm-mitmachen" aria-label="Mitmachen im Ehrenamt">
      <div className="lm-mitmachen-code" style={{ background: qrFarben.grund }}>
        <svg
          width={GROESSE}
          height={GROESSE}
          viewBox={`0 0 ${module} ${module}`}
          shapeRendering="crispEdges"
          role="img"
          aria-label="QR-Code"
        >
          <rect width={module} height={module} fill={qrFarben.grund} />
          <path d={pfad} fill={qrFarben.code} />
        </svg>
      </div>
      <span className="lm-mitmachen-text">{recruitment.text}</span>
    </aside>
  );
}
