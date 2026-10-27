import { useQuery } from '@tanstack/react-query';
import type { MonitorView } from './contract/MonitorView';

export async function fetchMonitor(): Promise<MonitorView> {
  const res = await fetch('/api/monitor');
  if (!res.ok) throw new Error(`Monitor-Abfrage fehlgeschlagen: HTTP ${res.status}`);
  return res.json() as Promise<MonitorView>;
}

// Bei einem Fehler behält react-query die letzten Daten (`data` bleibt gesetzt, `isError` wird wahr).
export function useMonitor() {
  return useQuery({
    queryKey: ['monitor'],
    queryFn: fetchMonitor,
    refetchInterval: 30_000,
    refetchIntervalInBackground: true,
  });
}
