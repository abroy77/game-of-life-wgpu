pub const ROWS: usize = 100;
pub const COLS: usize = 100;
pub const GAP: f32 = 0.2 / ROWS as f32;
pub const CELL_SIZE: f32 = (2.0 - (COLS as f32 + 1.0) * GAP) / COLS as f32;
pub const BACKGROUND_COLOR: wgpu::Color = wgpu::Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 };

// this should be a glider
// pub const INITIAL_STATE: [[bool; COLS]; ROWS] = [
//     [
//         false, true, false, false, false, false, false, false, false, false,
//     ],
//     [
//         false, false, true, false, false, false, false, false, false, false,
//     ],
//     [
//         true, true, true, false, false, false, false, false, false, false,
//     ],
//     [
//         false, false, false, false, false, false, false, false, false, false,
//     ],
//     [
//         false, false, false, false, false, false, false, false, false, false,
//     ],
//     [
//         false, false, false, false, false, false, false, false, false, false,
//     ],
//     [
//         false, false, false, false, false, false, false, false, false, false,
//     ],
//     [
//         false, false, false, false, false, false, false, false, false, false,
//     ],
//     [
//         false, false, false, false, false, false, false, false, false, false,
//     ],
//     [
//         false, false, false, false, false, false, false, false, false, false,
//     ],
// ];
