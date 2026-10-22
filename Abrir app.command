#!/bin/sh
set -eu
cd "$(dirname "$0")"
if [ ! -d "dist/Prospect Intelligence.app" ]; then
  ./scripts/bundle-macos.sh
fi
open "dist/Prospect Intelligence.app"
