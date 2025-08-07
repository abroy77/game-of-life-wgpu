import init from "./pkg/game_of_life.js";

async function run() {
  try {
    console.log("Loading WASM...");
    console.log("WebGPU supported:", !!navigator.gpu);

    await init();
    console.log("WASM Loaded successfully!");
  } catch (error) {
    console.error("Failed to load WASM:", error);
  }
}

run();
