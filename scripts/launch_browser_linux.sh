#!/bin/bash
# For development on linux this is required to run webgpu properly
# otherwise it will run on the CPU. not ideal.
google-chrome https://abroy77.github.io/game-of-life-wgpu/ --new-window --enable-features=Vulkan --enable-unsafe-webgpu
