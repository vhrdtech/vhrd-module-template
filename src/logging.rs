pub fn init() {
    #[cfg(feature = "log-text-rtt")] {
        rtt_target::rtt_init_print!();
    }
}

#[macro_export]
macro_rules! _level_to_color {
    (trace) => { crate::vt100::CYAN };
    (debug) => { crate::vt100::DEFAULT };
    (info) => { crate::vt100::GREEN };
    (warn) => { crate::vt100::YELLOW };
    (error) => { crate::vt100::RED };
}

#[macro_export]
macro_rules! _log_internal {
    ($level: ident, => $terminal:expr) => {
        #[cfg(feature = "log-text-rtt")]
        rtt_target::rprintln!(=> $terminal);
    };
    ($level: ident, => $terminal:expr, $fmt:expr) => {
        #[cfg(feature = "log-text-rtt")] {
            rtt_target::rprint!(=> $terminal, "{}", crate::_level_to_color!($level));
            rtt_target::rprintln!($fmt);
            rtt_target::rprint!(crate::vt100::DEFAULT);
        }
    };
    ($level: ident, => $terminal:expr, $fmt:expr, $($arg:tt)*) => {
        #[cfg(feature = "log-text-rtt")] {
            rtt_target::rprint!(=> $terminal, "{}", crate::_level_to_color!($level));
            rtt_target::rprintln!($fmt, $($arg)*);
            rtt_target::rprint!(crate::vt100::DEFAULT);
        }
    };
    ($level: ident) => {
        #[cfg(feature = "log-text-rtt")]
        rtt_target::rprintln!();
    };
    ($level: ident, $fmt:expr) => {
        #[cfg(feature = "log-text-rtt")] {
            rtt_target::rprint!("{}", crate::_level_to_color!($level));
            rtt_target::rprintln!($fmt);
            rtt_target::rprint!(crate::vt100::DEFAULT);
        }
    };
    ($level: ident, $fmt:expr, $($arg:tt)*) => {
        #[cfg(feature = "log-text-rtt")] {
            rtt_target::rprint!("{}", crate::_level_to_color!($level));
            rtt_target::rprintln!($fmt, $($arg)*);
            rtt_target::rprint!(crate::vt100::DEFAULT);
        }
    };
}

#[cfg(any(feature = "log-level-trace", feature = "log-level-debug", feature = "log-level-info", feature = "log-level-warn", feature = "log-level-error"))]
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        crate::_log_internal!(trace, $($arg)*);
    };
}
#[cfg(not(any(feature = "log-level-trace", feature = "log-level-debug", feature = "log-level-info", feature = "log-level-warn", feature = "log-level-error")))]
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {}
}

#[cfg(any(feature = "log-level-debug", feature = "log-level-info", feature = "log-level-warn", feature = "log-level-error"))]
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        crate::_log_internal!(debug, $($arg)*);
    };
}
#[cfg(not(any(feature = "log-level-debug", feature = "log-level-info", feature = "log-level-warn", feature = "log-level-error")))]
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {}
}

#[cfg(any(feature = "log-level-info", feature = "log-level-warn", feature = "log-level-error"))]
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        crate::_log_internal!(info, $($arg)*);
    };
}
#[cfg(not(any(feature = "log-level-info", feature = "log-level-warn", feature = "log-level-error")))]
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {}
}

#[cfg(any(feature = "log-level-warn", feature = "log-level-error"))]
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        crate::_log_internal!(warn, $($arg)*);
    };
}
#[cfg(not(any(feature = "log-level-warn", feature = "log-level-error")))]
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {}
}

#[cfg(feature = "log-level-error")]
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        crate::_log_internal!(error, $($arg)*);
    };
}
#[cfg(not(feature = "log-level-error"))]
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {}
}