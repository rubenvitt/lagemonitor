import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import 'maplibre-gl/dist/maplibre-gl.css';
import './theme/rollen.css';
import './theme/sprache.css';
import { meldeSchriftenAn } from './theme/schriften';
import { ThemeModeProvider } from './theme/ThemeModeProvider';
import MonitorPage from './monitor/MonitorPage';

meldeSchriftenAn();

const queryClient = new QueryClient();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeModeProvider>
      <QueryClientProvider client={queryClient}>
        <MonitorPage />
      </QueryClientProvider>
    </ThemeModeProvider>
  </StrictMode>,
);
