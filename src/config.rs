use embedded_time::duration::{Seconds, Milliseconds};

/// How often to update LED brightness during transitions (Breath, etc)
pub const BLINKER_UPDATE_PERIOD: Milliseconds = Milliseconds(20);
pub const BLINKER_BREATH_PERIOD: Seconds = Seconds(5);