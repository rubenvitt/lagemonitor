import { useQuery } from '@tanstack/react-query';
import type { MonitorView } from './contract/MonitorView';

export async function fetchMonitor(): Promise<MonitorView> {
  const res = await fetch('/api/monitor');
  if (!res.ok) throw new Error(`Monitor-Abfrage fehlgeschlagen: HTTP ${res.status}`);
  return res.json() as Promise<MonitorView>;
}

/** Takt der Abfrage. Mit Suchlage alle 5 s wie der Server dort: sonst käme das Drohnenbild bis zu einer halben Minute zu spät. */
export function pollInterval(view: MonitorView | undefined): number {
  return view?.modules.suchlage ? 5_000 : 30_000;
}

// Bei einem Fehler behält react-query die letzten Daten (`data` bleibt gesetzt, `isError` wird wahr).
// `networkMode: 'always'`: Meldet der Browser „offline“, pausierte react-query sonst still, ohne
// Fehler – der alte Stand sähe dann frisch aus. Der Server läuft lokal, abfragen lohnt immer.
export function useMonitor() {
  return useQuery({
    queryKey: ['monitor'],
    queryFn: fetchMonitor,
    networkMode: 'always',
    refetchInterval: (query) => pollInterval(query.state.data),
    refetchIntervalInBackground: true,
  });
}
