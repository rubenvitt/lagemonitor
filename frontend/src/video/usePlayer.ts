// Übernommen aus Suchlage 98f9265 (27.10.2026): frontend/src/video/usePlayer.ts (Adresse als Parameter)
import { useEffect, useReducer, useState, type RefObject } from 'react';
import { startHls } from './hls';
import { ICE_TIMEOUT_MS, RETRY_MS, STALL_MS, nextState, type PlayerState } from './playerMachine';
import { WhepNotReady, startWhep, type MediaSession } from './whep';

const initial = (live: boolean): PlayerState =>
  nextState({ kind: 'idle' }, { type: live ? 'stream_live' : 'stream_gone' });

/** Führt den Zustandsautomaten am Video-Element aus: Sitzung, ICE-Frist, Neuversuch und Hänger. */
export function usePlayer(
  video: RefObject<HTMLVideoElement | null>,
  gatewaySn: string,
  isLive: boolean,
  /** Ohne abschließenden Schrägstrich; daran hängen `/{sn}/whep` und `/{sn}/hls/index.m3u8`. */
  basePath: string,
) {
  const [player, dispatch] = useReducer(nextState, isLive, initial);
  // Beginn, Ende und Wechsel der Fernsteuerung beim Rendern übernehmen statt im Effekt (React: Zustand
  // an Props anpassen). Ein neuer Stream beginnt immer von vorn, also mit WebRTC.
  const stream = `${gatewaySn}:${isLive}`;
  const [seen, setSeen] = useState(stream);
  if (seen !== stream) {
    setSeen(stream);
    dispatch({ type: 'stream_gone' });
    if (isLive) dispatch({ type: 'stream_live' });
  }

  // Die Sitzung hängt an Transport und Versuch, nicht am ganzen Zustand: der Wechsel
  // `connecting → playing` darf die laufende Verbindung nicht schließen.
  const active = player.kind === 'connecting' || player.kind === 'playing';
  const transport = active ? player.transport : null;
  const attempt = active ? player.attempt : null;

  useEffect(() => {
    const el = video.current;
    if (!transport || !el) return;
    let session: MediaSession | undefined;
    // Bricht auch eine noch laufende WHEP-Aushandlung ab, nicht nur die fertige Sitzung.
    const abort = new AbortController();
    const cancelled = () => abort.signal.aborted;
    const cb = {
      onConnected: () => !cancelled() && dispatch({ type: 'connected' }),
      onFailed: () => !cancelled() && dispatch({ type: 'failed' }),
    };
    const base = `${basePath}/${encodeURIComponent(gatewaySn)}`;
    const start =
      transport === 'webrtc'
        ? startWhep(`${base}/whep`, el, cb, abort.signal)
        : startHls(`${base}/hls/index.m3u8`, el, cb);
    start
      .then((s) => (cancelled() ? s.close() : (session = s)))
      .catch(
        (e: unknown) => !cancelled() && dispatch({ type: e instanceof WhepNotReady ? 'not_ready' : 'failed' }),
      );
    return () => {
      abort.abort();
      session?.close();
    };
  }, [video, gatewaySn, transport, attempt, basePath]);

  useEffect(() => {
    if (player.kind !== 'connecting' || player.transport !== 'webrtc') return;
    const id = window.setTimeout(() => dispatch({ type: 'ice_timeout' }), ICE_TIMEOUT_MS);
    return () => window.clearTimeout(id);
  }, [player]);

  useEffect(() => {
    if (player.kind !== 'retrying') return;
    const id = window.setTimeout(() => dispatch({ type: 'retry_due' }), RETRY_MS);
    return () => window.clearTimeout(id);
  }, [player]);

  // Hänger: kommt `STALL_MS` lang kein neues Bild, gilt das Bild als eingefroren. Ein WebRTC-Strom
  // meldet dabei kein `waiting`, darum zählt der Wächter Frames; `waiting` nur ohne Frame-Rückruf.
  useEffect(() => {
    const el = video.current;
    if (!el || player.kind !== 'playing') return;
    let timer: number | undefined;
    const arm = () => {
      window.clearTimeout(timer);
      // Ein verdeckter Tab malt keine Frames; das ist kein Hänger.
      timer = window.setTimeout(() => (document.hidden ? arm() : dispatch({ type: 'stalled' })), STALL_MS);
    };
    if (typeof el.requestVideoFrameCallback === 'function') {
      const onFrame = () => {
        arm();
        frame = el.requestVideoFrameCallback(onFrame);
      };
      let frame = el.requestVideoFrameCallback(onFrame);
      arm();
      return () => {
        window.clearTimeout(timer);
        el.cancelVideoFrameCallback(frame);
      };
    }
    const playing = () => window.clearTimeout(timer);
    el.addEventListener('waiting', arm);
    el.addEventListener('playing', playing);
    return () => {
      window.clearTimeout(timer);
      el.removeEventListener('waiting', arm);
      el.removeEventListener('playing', playing);
    };
  }, [video, player]);

  return player;
}
