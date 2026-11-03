import { render } from '@testing-library/react';
import { encode } from 'uqr';
import RecruitmentQr, { qrPfad } from './RecruitmentQr';

describe('RecruitmentQr', () => {
  it('zeichnet jedes dunkle Modul samt vier Modulen Ruhezone', () => {
    const url = 'https://example.org/mitmachen';
    const { pfad, module } = qrPfad(url);
    const dunkle = encode(url, { ecc: 'M', border: 4 }).data.flat().filter(Boolean).length;
    expect(pfad.match(/M/g)).toHaveLength(dunkle);
    // Version 2 hat 25 Module, dazu je Seite vier Ruhezone.
    expect(module).toBe(encode(url, { ecc: 'M' }).size - 2 + 8);
  });

  it('zeigt den Code als SVG mit dem Text daneben', () => {
    const { container, getByText } = render(
      <RecruitmentQr recruitment={{ url: 'https://example.org/x', text: 'Mach mit' }} />,
    );
    expect(container.querySelector('svg path')?.getAttribute('d')).toMatch(/^M\d+ \d+h1v1h-1z/);
    expect(getByText('Mach mit')).toBeTruthy();
  });
});
