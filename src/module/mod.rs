#[cfg(feature = "module-pi")]
pub mod pi;
#[cfg(feature = "module-pi")]
pub use pi::can_rx_router;

#[cfg(feature = "module-led")]
pub mod led;
#[cfg(feature = "module-led")]
pub use led::can_rx_router;

#[cfg(feature = "module-button")]
pub mod button;
#[cfg(feature = "module-button")]
pub use button::can_rx_router;
#[cfg(not(feature = "module-button"))]
pub mod button {
    pub type Resources = ();
}

#[cfg(feature = "module-afe")]
pub mod afe;
#[cfg(feature = "module-afe")]
pub use afe::can_rx_router;
