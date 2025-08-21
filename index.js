import init, {
  playPause,
  stepForward,
  randomiseState,
  updateFps,
  resetState,
} from "./pkg/game_of_life.js";

async function run() {
  try {
    console.log("Loading WASM...");
    console.log("WebGPU supported:", !!navigator.gpu);

    await init();
    console.log("WASM Loaded successfully!");
  } catch (error) {
    console.error("Failed to load WASM:", error);
  }

  // now we setup our helper functions
  // give them the window scope:
  window.playPause = playPause;
  window.stepForward = stepForward;
  window.randomiseState = randomiseState;
  window.updateFps = updateFps;
  window.resetState = resetState;
}

run();
