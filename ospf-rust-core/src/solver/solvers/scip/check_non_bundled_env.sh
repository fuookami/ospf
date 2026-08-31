#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${SCIPOPTDIR:-}" ]]; then
  echo "SCIPOPTDIR is not set. Please set SCIPOPTDIR to your SCIP installation root." >&2
  exit 1
fi

if [[ ! -d "$SCIPOPTDIR" ]]; then
  echo "SCIPOPTDIR does not exist: $SCIPOPTDIR" >&2
  exit 1
fi

if [[ -z "${LIBCLANG_PATH:-}" ]]; then
  echo "LIBCLANG_PATH is not set. bindgen may fail if libclang is not in default loader path." >&2
else
  if [[ ! -d "$LIBCLANG_PATH" ]]; then
    echo "LIBCLANG_PATH does not exist: $LIBCLANG_PATH" >&2
    exit 1
  fi
fi

if ! find "$SCIPOPTDIR" -type f \( -name 'libscip.so' -o -name 'libscip.dylib' -o -name 'libscip.lib' -o -name 'scip.lib' \) | head -n1 | grep -q .; then
  echo "No SCIP library file found under SCIPOPTDIR: $SCIPOPTDIR" >&2
  exit 1
fi

if ! find "$SCIPOPTDIR" -type f \( -name 'scip.h' -o -name 'scip/scip.h' \) | head -n1 | grep -q .; then
  echo "No SCIP header file found under SCIPOPTDIR: $SCIPOPTDIR" >&2
  exit 1
fi

echo "[scip] environment check passed."
echo "[scip] SCIPOPTDIR=$SCIPOPTDIR"
if [[ -n "${LIBCLANG_PATH:-}" ]]; then
  echo "[scip] LIBCLANG_PATH=$LIBCLANG_PATH"
fi
