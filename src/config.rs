/// Terrain generation and recreation constants
pub mod terrain {
    pub const RADIUS: usize = 200;
    pub const RECREATION_THRESHOLD_DIVISOR: usize = 4;
    pub const RECREATION_COOLDOWN_SECS: f32 = 1.0;
    pub const LANDSCAPE_RADIUS: usize = 3;
    pub const SPAWN_PROBABILITY: f64 = 0.003;
}

/// Player movement constants
pub mod player {
    pub const MOVE_SPEED: f32 = 15.0;
    pub const MOUSE_SENSITIVITY: f32 = 0.002;
    pub const JUMP_FORCE: f32 = 8.0;
    pub const JUMP_COOLDOWN_SECS: f32 = 0.5;
}

/// Third-person camera constants
pub mod camera {
    pub const DISTANCE: f32 = 20.0;
    pub const HEIGHT: f32 = 14.0;
    pub const FOLLOW_SPEED: f32 = 5.0;
    pub const ZOOM_SPEED: f32 = 2.0;
    pub const MIN_DISTANCE: f32 = 5.0;
    pub const MAX_DISTANCE: f32 = 50.0;
}

/// Texture atlas constants
pub mod atlas {
    pub const SIZE: usize = 16;
}
