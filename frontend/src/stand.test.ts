import {
  SPERRE_MS,
  aufStandWechseln,
  bundles,
  eigenerStand,
  gesperrt,
  istVeraltet,
  type Umgebung,
} from './stand';

const NEU = 'b2b2b2b2';
const SEITE = 'https://lagemonitor.example/monitor/';
const HTML = `<!doctype html><html><head><meta name="lagemonitor-stand" content="${NEU}" />
<script type="module" crossorigin src="./assets/index-Ab1.js"></script>
<link rel="stylesheet" crossorigin href="./assets/index-Cd2.css"></head><body></body></html>`;

function umgebung(antwort: Response | Error, jetzt = 1_000_000): Umgebung & { geholt: string[]; neu: number } {
  const u = {
    geholt: [] as string[],
    neu: 0,
    seite: SEITE,
    jetzt: () => jetzt,
    neuLaden: () => {
      u.neu++;
    },
    holen: (async (eingabe: RequestInfo | URL) => {
      u.geholt.push(String(eingabe));
      if (String(eingabe) !== SEITE) return new Response('');
      if (antwort instanceof Error) throw antwort;
      return antwort.clone();
    }) as typeof fetch,
  };
  return u;
}

beforeEach(() => localStorage.clear());

test('der eigene Stand kommt aus der Seite, im Dev-Server gibt es keinen', () => {
  expect(eigenerStand()).toBeNull();
  const meta = document.createElement('meta');
  meta.name = 'lagemonitor-stand';
  meta.content = 'a1a1a1a1';
  document.head.append(meta);
  expect(eigenerStand()).toBe('a1a1a1a1');
  meta.remove();
});

test('nur ein bekannter, abweichender Stand ist veraltet', () => {
  expect(istVeraltet('a1', 'b2')).toBe(true);
  expect(istVeraltet('a1', 'a1')).toBe(false);
  expect(istVeraltet(null, 'b2')).toBe(false);
  expect(istVeraltet('a1', null)).toBe(false);
});

test('für denselben Stand höchstens alle zehn Minuten neu laden', () => {
  expect(gesperrt(NEU, 5_000, null)).toBe(false);
  expect(gesperrt(NEU, 5_000, `${NEU}|1000`)).toBe(true);
  expect(gesperrt(NEU, 1_000 + SPERRE_MS, `${NEU}|1000`)).toBe(false);
  expect(gesperrt(NEU, 5_000, `anderer|1000`)).toBe(false);
});

test('die Bundles der neuen Seite werden erkannt', () => {
  expect(bundles(HTML)).toEqual(['./assets/index-Ab1.js', './assets/index-Cd2.css']);
});

test('lädt die neue Seite samt Bundles vor und dann neu', async () => {
  const u = umgebung(new Response(HTML));
  expect(await aufStandWechseln(NEU, u)).toBe(true);
  expect(u.neu).toBe(1);
  expect(u.geholt).toEqual([
    SEITE,
    'https://lagemonitor.example/monitor/assets/index-Ab1.js',
    'https://lagemonitor.example/monitor/assets/index-Cd2.css',
  ]);
  // Zeigt der Server danach noch einmal den alten Stand, nicht im Kreis laden.
  expect(await aufStandWechseln(NEU, umgebung(new Response(HTML)))).toBe(false);
});

test('liefert der Server die neue Seite (noch) nicht, bleibt die alte stehen', async () => {
  for (const antwort of [
    new Response('alt', { status: 200 }),
    new Response('', { status: 502 }),
    new Error('offline'),
  ]) {
    const u = umgebung(antwort);
    expect(await aufStandWechseln(NEU, u)).toBe(false);
    expect(u.neu).toBe(0);
  }
});
