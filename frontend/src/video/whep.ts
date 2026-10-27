// Übernommen aus Suchlage 98f9265 (27.10.2026): frontend/src/video/whep.ts
export interface MediaSession {
  close(): void;
}

export interface MediaCallbacks {
  onConnected: () => void;
  onFailed: () => void;
}

/** MediaMTX kennt den Pfad noch nicht (WHEP 404): Pilot 2 hat noch nicht zu senden begonnen. */
export class WhepNotReady extends Error {}

const GATHERING_MAX_MS = 2_000;

function gathered(pc: RTCPeerConnection): Promise<void> {
  if (pc.iceGatheringState === 'complete') return Promise.resolve();
  return new Promise((resolve) => {
    const done = () => {
      window.clearTimeout(timer);
      pc.removeEventListener('icegatheringstatechange', check);
      resolve();
    };
    const check = () => pc.iceGatheringState === 'complete' && done();
    const timer = window.setTimeout(done, GATHERING_MAX_MS);
    pc.addEventListener('icegatheringstatechange', check);
  });
}

/**
 * WHEP ohne Trickle-ICE: alle Kandidaten im Offer, die Signalisierung läuft über den Server-Proxy.
 * `signal` bricht ab, wenn der Player inzwischen weiter ist (ICE-Frist, HLS, andere Fernsteuerung):
 * Eine späte Antwort darf dann kein Bild mehr ans Element hängen, `srcObject` ginge vor HLS.
 */
export async function startWhep(
  url: string,
  video: HTMLVideoElement,
  cb: MediaCallbacks,
  signal: AbortSignal,
): Promise<MediaSession> {
  const pc = new RTCPeerConnection();
  // Eine geschlossene Verbindung lässt auch eine laufende Aushandlung scheitern.
  signal.addEventListener('abort', () => pc.close(), { once: true });
  try {
    pc.addTransceiver('video', { direction: 'recvonly' });
    pc.addTransceiver('audio', { direction: 'recvonly' });
    pc.ontrack = (e) => {
      if (signal.aborted) return;
      video.srcObject = e.streams[0] ?? new MediaStream([e.track]);
      void video.play().catch(() => {});
    };
    pc.onconnectionstatechange = () => {
      if (pc.connectionState === 'connected') cb.onConnected();
      if (pc.connectionState === 'failed') cb.onFailed();
    };
    await pc.setLocalDescription(await pc.createOffer());
    await gathered(pc);
    const res = await fetch(url, {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'content-type': 'application/sdp' },
      body: pc.localDescription!.sdp,
      signal,
    });
    if (!res.ok) throw res.status === 404 ? new WhepNotReady('WHEP 404') : new Error(`WHEP ${res.status}`);
    await pc.setRemoteDescription({ type: 'answer', sdp: await res.text() });
  } catch (e) {
    // Jeder Neuversuch baut eine eigene Verbindung auf; eine gescheiterte darf nicht offen bleiben.
    pc.close();
    throw e;
  }
  return { close: () => pc.close() };
}
