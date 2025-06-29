pub const ROWS: usize = 10;
pub const COLS: usize = 10;
pub const GAP: f32 = 0.02;
pub const CELL_SIZE: f32 = (2.0 - (COLS as f32 + 1.0) * GAP) / COLS as f32;

// this should be a glider
pub const INITIAL_STATE: [[bool; COLS]; ROWS] = [
    [
        false, true, false, false, false, false, false, false, false, false,
    ],
    [
        false, false, true, false, false, false, false, false, false, false,
    ],
    [
        true, true, true, false, false, false, false, false, false, false,
    ],
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
];
