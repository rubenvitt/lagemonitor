import { pollInterval } from './api';
import { makeView } from './monitor/testViews';

test('mit Suchlage fragt der Monitor alle 5 s, sonst alle 30 s', () => {
  expect(pollInterval(undefined)).toBe(30_000);
  expect(pollInterval(makeView())).toBe(30_000);
  expect(pollInterval(makeView({ modules: { ...makeView().modules, suchlage: true } }))).toBe(5_000);
});
