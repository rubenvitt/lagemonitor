import { useRef, useState } from 'react';
import type { SuchlageAircraft } from '../contract/SuchlageAircraft';
import type { SuchlageVideo } from '../contract/SuchlageVideo';
import { usePlayer } from '../video/usePlayer';
import type { PlayerState } from '../video/playerMachine';

/** Pfad des Video-Proxys auf diesem Server (`web::video`). */
/** Relativ wie alle Abfragen, damit der Monitor auch unter einem Pfad läuft. */
export const VIDEO_BASE = 'api/suchlage/video';

function stateText(p: PlayerState): string {
  switch (p.kind) {
    case 'idle':
      return 'kein Livebild';
    case 'connecting':
      return p.hadPicture ? 'Bild hängt · verbindet neu' : 'verbindet …';
    case 'retrying':
      return p.hadPicture ? 'Bild hängt · neuer Versuch' : 'neuer Versuch …';
    case 'playing':
      return p.transport === 'hls' ? 'live · HLS' : 'live';
  }
}

interface Props {
  /** `null`: kein Livebild, das Paneel fährt hinaus. */
  video: SuchlageVideo | null;
  aircraft: SuchlageAircraft[];
}

/**
 * Das Drohnenbild fährt von rechts über die Wetterspalte ein, sobald Suchlage ein Livebild meldet,
 * und wieder hinaus, wenn es endet. Kein Umblättern: Es kommt nur, wenn es etwas zu zeigen gibt.
 */
export default function DronePanel({ video, aircraft }: Props) {
  const el = useRef<HTMLVideoElement>(null);
  // Beim Hinausfahren steht die Bezeichnung noch da; sonst stünde dort kurz ein leeres Paneel.
  const [last, setLast] = useState(video);
  if (video && (video.gatewaySn !== last?.gatewaySn || video.label !== last.label)) setLast(video);
  const sn = (video ?? last)?.gatewaySn ?? '';
  const player = usePlayer(el, sn, video !== null, VIDEO_BASE);
  const a = aircraft.find((x) => x.gatewaySn === sn);
  const label = (video ?? last)?.label ?? a?.label ?? sn;
  return (
    <section className="lm-drohne" data-offen={video !== null} aria-hidden={video === null} aria-label="Drohnenbild">
      <div className="lm-drohne-kopf">
        <span className="lm-drohne-live" data-live={player.kind === 'playing'}>
          <span className="lm-punkt" aria-hidden />
          {stateText(player)}
        </span>
        <span className="lm-augenbraue">Drohnenbild · {label}</span>
      </div>
      <div className="lm-drohne-bild">
        <video ref={el} muted playsInline autoPlay data-testid="drone-video" />
      </div>
      {a && (
        <div className="lm-drohne-werte lm-zahl">
          <span>Höhe {a.elevationM === null ? '–' : `${Math.round(a.elevationM)} m`}</span>
          <span>Akku {a.batteryPercent === null ? '–' : `${a.batteryPercent} %`}</span>
          <span>{a.airborne ? 'in der Luft' : 'am Boden'}</span>
        </div>
      )}
    </section>
  );
}
