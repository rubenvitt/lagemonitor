import { QRCode } from 'antd';
import type { Recruitment } from '../contract/Recruitment';
import { qrFarben } from '../theme/tokens';

/**
 * QR-Code fürs Ehrenamt, unten rechts auf der Karte. Schwarz auf Weiß auch nachts: ein invertierter
 * Code liest nicht jedes Telefon; dazu vier Module Ruhezone, wie die Norm sie verlangt. Ob er zu sehen ist (nur in ruhiger Lage), entscheidet `MonitorPage`.
 */
export default function RecruitmentQr({ recruitment }: { recruitment: Recruitment }) {
  return (
    <aside className="lm-mitmachen" aria-label="Mitmachen im Ehrenamt">
      <div className="lm-mitmachen-code" style={{ background: qrFarben.grund }}>
        <QRCode value={recruitment.url} type="svg" size={168} color={qrFarben.code} bgColor={qrFarben.grund} bordered={false} errorLevel="M" marginSize={4} />
      </div>
      <span className="lm-mitmachen-text">{recruitment.text}</span>
    </aside>
  );
}
