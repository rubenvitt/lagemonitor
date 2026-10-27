// Übernommen aus Suchlage 98f9265 (27.10.2026): frontend/src/video/hls.ts
import type { MediaCallbacks, MediaSession } from './whep';

/** Safari und iOS spielen HLS selbst. Fehler kommen dort nur als `error` am Element an. */
function startNative(url: string, video: HTMLVideoElement, cb: MediaCallbacks): MediaSession {
  video.addEventListener('playing', cb.onConnected, { once: true });
  // Ohne diesen Listener bliebe iOS bei 404 oder abgelaufener Playlist für immer auf „Verbindet …“.
  video.addEventListener('error', cb.onFailed, { once: true });
  video.src = url;
  void video.play().catch(() => {});
  return {
    close: () => {
      video.removeEventListener('playing', cb.onConnected);
      video.removeEventListener('error', cb.onFailed);
      video.removeAttribute('src');
      video.load();
    },
  };
}

export async function startHls(
  url: string,
  video: HTMLVideoElement,
  cb: MediaCallbacks,
): Promise<MediaSession> {
  // Ein WebRTC-Strom am Element hat Vorrang vor `src`; ohne Freigabe bliebe der Rückfall schwarz.
  video.srcObject = null;
  if (video.canPlayType('application/vnd.apple.mpegurl')) return startNative(url, video, cb);
  const { default: Hls } = await import('hls.js');
  if (!Hls.isSupported()) {
    cb.onFailed();
    return { close: () => {} };
  }
  const hls = new Hls({ lowLatencyMode: true });
  hls.on(Hls.Events.ERROR, (_e, data) => data.fatal && cb.onFailed());
  hls.loadSource(url);
  hls.attachMedia(video);
  video.addEventListener('playing', cb.onConnected, { once: true });
  void video.play().catch(() => {});
  return {
    close: () => {
      video.removeEventListener('playing', cb.onConnected);
      hls.destroy();
    },
  };
}
