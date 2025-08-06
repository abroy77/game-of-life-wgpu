import init from "./pkg/game_of_life.js"

async function run() {
  try {
    console.log("Loading WASM...");
    
    // Debug canvas before WASM loads
    const canvas = document.getElementById('canvas');
    console.log("Canvas found:", canvas);
    console.log("Canvas dimensions:", canvas.width, "x", canvas.height);
    console.log("Canvas client size:", canvas.clientWidth, "x", canvas.clientHeight);
    console.log("Total canvases before WASM:", document.querySelectorAll('canvas').length);
    
    await init();
    console.log("WASM Loaded successfully!");
    
    // Debug canvas after WASM loads
    console.log("After WASM - Canvas dimensions:", canvas.width, "x", canvas.height);
    console.log("After WASM - Total canvases:", document.querySelectorAll('canvas').length);
    
    // Check if there are multiple canvases now
    const allCanvases = document.querySelectorAll('canvas');
    allCanvases.forEach((c, index) => {
      console.log(`Canvas ${index}:`, c.id, c.width + "x" + c.height, c.style.display);
    });
    
    // Try to get the WebGL/WebGPU context to see if it's being used
    console.log("WebGL context:", canvas.getContext('webgl'));
    console.log("WebGL2 context:", canvas.getContext('webgl2'));
    
  } catch (error) {
    console.error("Failed to load WASM:", error);
  }
}

run();
