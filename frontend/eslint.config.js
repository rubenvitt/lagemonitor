import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';

export default tseslint.config(
  { ignores: ['dist', 'node_modules', 'public', 'src/contract'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    plugins: { 'react-hooks': reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      'no-restricted-syntax': [
        'error',
        {
          selector: 'Literal[value=/^#[0-9a-fA-F]{3,8}$/]',
          message: 'Farben nur aus src/theme (Farbrollen), keine Hex-Literale.',
        },
      ],
    },
  },
  // Kartenfarben sind eine eigene Palette (wie in Lifeline), keine Farbrollen der Oberfläche.
  {
    files: ['src/theme/**', 'src/components/instrument/**', 'src/map/basemapStyle.ts'],
    rules: { 'no-restricted-syntax': 'off' },
  },
);
