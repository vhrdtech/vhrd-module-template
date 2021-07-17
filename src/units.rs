use core::fmt;
use core::fmt::Formatter;
// use core::ops::{Sub, Add, Div};

#[derive(Eq, PartialEq, PartialOrd, Clone, Copy, Default)]
pub struct MilliVolts(pub i32);
impl fmt::Display for MilliVolts {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { write!(f, "{}mV", self.0) }
}
impl fmt::Debug for MilliVolts {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { write!(f, "{}", self) }
}
