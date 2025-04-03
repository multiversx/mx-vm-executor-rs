use std::fmt;

#[derive(Clone, Copy, Debug)]
pub enum GasSchedule {
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
}

impl fmt::Display for GasSchedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gasScheduleV{}.toml", *self as u8 + 1)
    }
}
