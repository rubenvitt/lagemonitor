// Übernommen aus Suchlage 98f9265 (27.10.2026): frontend/src/video/playerMachine.ts
export type Transport = 'webrtc' | 'hls';

export type PlayerState =
  | { kind: 'idle' }
  /** `hadPicture`: das Bild lief schon und hängt jetzt (Schraffur über dem letzten Bild). Bleibt
   *  über alle Neuversuche, bis wieder ein Bild kommt oder der Stream endet. */
  | { kind: 'connecting'; transport: Transport; attempt: number; hadPicture?: true }
  | { kind: 'playing'; transport: Transport; attempt: number }
  | { kind: 'retrying'; transport: Transport; attempt: number; hadPicture?: true };

export type PlayerEvent =
  | { type: 'stream_live' }
  | { type: 'stream_gone' }
  | { type: 'connected' }
  | { type: 'ice_timeout' }
  | { type: 'not_ready' }
  | { type: 'failed' }
  | { type: 'stalled' }
  | { type: 'retry_due' };

/** Ohne VPN kommt kein UDP an; nach dieser Frist geht es per HLS weiter (Review Focus 2). */
export const ICE_TIMEOUT_MS = 8_000;
export const RETRY_MS = 3_000;
/** So lange darf das Video puffern, bevor es als hängend gilt. */
export const STALL_MS = 5_000;
const WEBRTC_ATTEMPTS = 2;

/**
 * Rückfall auf HLS nur nach `ice_timeout` oder zwei `failed` in Folge, Letzteres nur, solange WebRTC in
 * diesem Stream noch kein Bild hatte: Ein kurzer Ausfall von MediaMTX soll den Rest des Flugs nicht auf
 * HLS mit 5–10 s Verzögerung festlegen. `not_ready` (WHEP 404 direkt
 * nach dem Streamstart, Pilot 2 sendet noch nicht) wartet mit demselben Transport und Versuch. Jeder
 * neue Stream beginnt wieder mit WebRTC; zurück aus laufendem HLS geht es bewusst nicht, außerhalb
 * des VPN unterbräche jeder Versuch das laufende Bild.
 */
export function nextState(s: PlayerState, e: PlayerEvent): PlayerState {
  if (e.type === 'stream_gone') return { kind: 'idle' };
  switch (s.kind) {
    case 'idle':
      return e.type === 'stream_live' ? { kind: 'connecting', transport: 'webrtc', attempt: 1 } : s;
    case 'connecting': {
      // Bedingt gespreizt: Ein `hadPicture: undefined` wäre eine vorhandene Eigenschaft.
      const pic = s.hadPicture && { hadPicture: s.hadPicture };
      if (e.type === 'connected') return { kind: 'playing', transport: s.transport, attempt: s.attempt };
      if (e.type === 'ice_timeout' && s.transport === 'webrtc') {
        return { kind: 'connecting', transport: 'hls', attempt: 1, ...pic };
      }
      if (e.type === 'not_ready') return { kind: 'retrying', transport: s.transport, attempt: s.attempt, ...pic };
      if (e.type === 'failed') {
        if (s.transport === 'webrtc' && s.attempt >= WEBRTC_ATTEMPTS && !s.hadPicture) {
          return { kind: 'retrying', transport: 'hls', attempt: 1, ...pic };
        }
        return { kind: 'retrying', transport: s.transport, attempt: s.attempt + 1, ...pic };
      }
      return s;
    }
    case 'playing':
      return e.type === 'stalled' || e.type === 'failed'
        ? { kind: 'retrying', transport: s.transport, attempt: 1, hadPicture: true }
        : s;
    case 'retrying':
      return e.type === 'retry_due'
        ? { kind: 'connecting', transport: s.transport, attempt: s.attempt, ...(s.hadPicture && { hadPicture: s.hadPicture }) }
        : s;
  }
}
