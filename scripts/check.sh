#!/bin/sh
# Qualitäts-Gate: vor jedem Commit, unverändert auch in einer späteren CI (dort mit CI=1).
#   scripts/check.sh            alles
#   scripts/check.sh --rust     nur Rust und Vertrag
#   scripts/check.sh --frontend nur das Frontend (tut nichts, solange frontend/package.json fehlt)
set -eu
cd "$(dirname "$0")/.."
part=${1:-all}
case "$part" in all|--rust|--frontend) ;; *) echo "Aufruf: scripts/check.sh [--rust|--frontend]"; exit 2 ;; esac

if [ "$part" != --frontend ]; then
  echo "→ Rust"
  # Lokal formatiert das Gate selbst; in CI prüft es nur.
  if [ "${CI:-}" = 1 ]; then cargo fmt --check; else cargo fmt; fi
  cargo clippy --all-targets -- -D warnings

  # Der Vertrag wird bei cargo test neu erzeugt. Ändert sich dabei etwas, war er veraltet.
  contract_hash() { [ -d frontend/src/contract ] && find frontend/src/contract -type f -exec shasum {} + | sort | shasum || echo none; }
  before=$(contract_hash)
  cargo test
  if [ "$before" != "$(contract_hash)" ]; then
    echo "FAIL Vertrag war veraltet und wurde neu erzeugt – Änderungen in frontend/src/contract prüfen und mitcommitten"
    exit 1
  fi
fi

if [ "$part" != --rust ] && [ -f frontend/package.json ]; then
  echo "→ Frontend"
  pnpm -C frontend install --frozen-lockfile
  pnpm -C frontend lint
  pnpm -C frontend typecheck
  pnpm -C frontend test
  pnpm -C frontend build
fi
echo "ok"
