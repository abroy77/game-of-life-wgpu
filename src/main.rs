//! Native (desktop) entry point. All the real work lives in the library; see [`game_of_life`].
//! On the web there is no `main`. `app::run_web` is called automatically when the wasm loads.

use game_of_life::app;

fn main() {
    _ = app::run();
}
