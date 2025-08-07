#!/bin/bash
# For development on linux this is required to run webgpu properly
# otherwise it will run on the CPU. not ideal.
brave-browser http://127.0.0.1:8080/ \
  --new-window \
  --enable-unsafe-webgpu \
  --enable-features=Vulkan,VulkanFromANGLE,DefaultANGLEVulkan \
  --use-vulkan
