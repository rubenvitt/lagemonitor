// Übernommen aus Suchlage 98f9265 (27.10.2026): frontend/src/video/playerMachine.test.ts
import { nextState, type PlayerState } from './playerMachine';

const live = (): PlayerState => nextState({ kind: 'idle' }, { type: 'stream_live' });

test('Livebild beginnt mit WebRTC', () => {
  expect(live()).toEqual({ kind: 'connecting', transport: 'webrtc', attempt: 1 });
});

// Review Focus 2: außerhalb des VPN kommt kein UDP an – der Player darf nicht hängen bleiben.
test('falls back to HLS when ICE does not connect', () => {
  expect(nextState(live(), { type: 'ice_timeout' })).toEqual({
    kind: 'connecting',
    transport: 'hls',
    attempt: 1,
  });
});

test('zwei WebRTC-Fehler in Folge wechseln ebenfalls auf HLS', () => {
  let s = nextState(live(), { type: 'failed' });
  expect(s).toEqual({ kind: 'retrying', transport: 'webrtc', attempt: 2 });
  s = nextState(s, { type: 'retry_due' });
  s = nextState(s, { type: 'failed' });
  expect(s).toEqual({ kind: 'retrying', transport: 'hls', attempt: 1 });
});

test('lief WebRTC schon, bleiben gescheiterte Neuversuche bei WebRTC', () => {
  let s = nextState(nextState(live(), { type: 'connected' }), { type: 'failed' });
  for (let i = 0; i < 3; i++) {
    s = nextState(nextState(s, { type: 'retry_due' }), { type: 'failed' });
    expect(s).toMatchObject({ kind: 'retrying', transport: 'webrtc', hadPicture: true });
  }
  // Ohne UDP (außerhalb des VPN) gilt weiter die ICE-Frist.
  expect(nextState(nextState(s, { type: 'retry_due' }), { type: 'ice_timeout' })).toMatchObject({
    kind: 'connecting',
    transport: 'hls',
  });
});

test('Hänger führt über Neuversuch zurück ins Verbinden, Stream-Ende in Ruhe', () => {
  const playing = nextState(live(), { type: 'connected' });
  expect(playing).toEqual({ kind: 'playing', transport: 'webrtc', attempt: 1 });
  const stalled = nextState(playing, { type: 'stalled' });
  expect(stalled.kind).toBe('retrying');
  expect(nextState(stalled, { type: 'retry_due' }).kind).toBe('connecting');
  expect(nextState(playing, { type: 'stream_gone' })).toEqual({ kind: 'idle' });
});

test('nur ein Neuversuch nach laufendem Bild weiß, dass es ein Bild gab', () => {
  const playing = nextState(live(), { type: 'connected' });
  expect(nextState(playing, { type: 'stalled' })).toMatchObject({ kind: 'retrying', hadPicture: true });
  expect(nextState(playing, { type: 'failed' })).toMatchObject({ kind: 'retrying', hadPicture: true });
  expect(nextState(live(), { type: 'failed' })).not.toHaveProperty('hadPicture');
  expect(nextState(live(), { type: 'not_ready' })).not.toHaveProperty('hadPicture');
});

test('ein hängendes Bild bleibt über gescheiterte Neuversuche als hängend markiert', () => {
  const playing = nextState(live(), { type: 'connected' });
  let s = nextState(playing, { type: 'stalled' });
  s = nextState(s, { type: 'retry_due' });
  expect(s).toMatchObject({ kind: 'connecting', hadPicture: true });
  s = nextState(s, { type: 'failed' });
  expect(s).toMatchObject({ kind: 'retrying', hadPicture: true });
  s = nextState(nextState(s, { type: 'retry_due' }), { type: 'not_ready' });
  expect(s).toMatchObject({ kind: 'retrying', hadPicture: true });
  s = nextState(nextState(s, { type: 'retry_due' }), { type: 'ice_timeout' });
  expect(s).toMatchObject({ kind: 'connecting', transport: 'hls', hadPicture: true });
  expect(nextState(s, { type: 'connected' })).not.toHaveProperty('hadPicture');
  expect(nextState(s, { type: 'stream_gone' })).toEqual({ kind: 'idle' });
});

test('ICE-Frist nach erfolgreicher Verbindung ändert nichts', () => {
  const playing = nextState(live(), { type: 'connected' });
  expect(nextState(playing, { type: 'ice_timeout' })).toBe(playing);
});

test('ICE-Frist gilt nur für WebRTC', () => {
  const hls = nextState(live(), { type: 'ice_timeout' });
  expect(nextState(hls, { type: 'ice_timeout' })).toBe(hls);
});

test('not_ready keeps WebRTC: WHEP 404 beim Streamstart zählt nicht als Fehlversuch', () => {
  let s = live();
  for (let i = 0; i < 5; i++) {
    s = nextState(s, { type: 'not_ready' });
    expect(s).toEqual({ kind: 'retrying', transport: 'webrtc', attempt: 1 });
    s = nextState(s, { type: 'retry_due' });
  }
  expect(s).toEqual({ kind: 'connecting', transport: 'webrtc', attempt: 1 });
});

test('neuer Stream beginnt nach HLS wieder mit WebRTC', () => {
  const hls = nextState(live(), { type: 'ice_timeout' });
  expect(nextState(nextState(hls, { type: 'stream_gone' }), { type: 'stream_live' })).toEqual({
    kind: 'connecting',
    transport: 'webrtc',
    attempt: 1,
  });
});
