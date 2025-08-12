#!/bin/bash
# For development on linux this is required to run webgpu properly
# otherwise it will run on the CPU. not ideal.

if [[ "$1" == "--dev" ]]; then
  URL="http://127.0.0.1:8080"
else
  URL="https://abroy77.github.io/game-of-life-wgpu/"
fi

google-chrome "$URL" --new-window --enable-features=Vulkan --enable-unsafe-webgpu
