use std::time::{ SystemTime, UNIX_EPOCH, Duration };

pub struct Keyboard {
    pub is_w_down: bool,
    pub is_a_down: bool,
    pub is_s_down: bool,
    pub is_d_down: bool,
    pub should_quit: bool,
}

impl Keyboard {
    pub fn new() -> Self {
        Keyboard {
            is_w_down: false,
            is_a_down: false,
            is_s_down: false,
            is_d_down: false,
            should_quit: false,
        }
    }
}

pub fn get_current_time() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards")
}
