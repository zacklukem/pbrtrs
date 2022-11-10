#!/bin/bash

cargo build --release --features enable_oidn

export TEV_PATH=""
for f in `find examples -name "*.toml"`; do
  echo "Rendering $f"
  target/release/pbrtrs_main $f
  echo "Converting $f"
  magick out.exr "${f%.*}.png"
done
