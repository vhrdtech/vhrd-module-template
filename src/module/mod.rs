#[cfg(feature = "module-pi")]
pub mod pi;
#[cfg(feature = "module-pi")]
pub use pi::handle_message;
#[cfg(feature = "module-pi")]
pub use pi::handle_service_request;

#[cfg(feature = "module-led")]
pub mod led;
#[cfg(feature = "module-led")]
pub use pi::handle_message;
#[cfg(feature = "module-led")]
pub use pi::handle_service_request;
#[cfg(not(feature = "module-led"))]
pub mod led {
    pub type Drv8323Instance = ();
}

#[cfg(feature = "module-button")]
pub mod button;
#[cfg(not(feature = "module-button"))]
pub mod button {
    pub type Resources = ();
}
#[cfg(feature = "module-button")]
pub use pi::handle_message;
#[cfg(feature = "module-button")]
pub use pi::handle_service_request;

#[cfg(feature = "module-afe")]
pub mod afe;
#[cfg(feature = "module-afe")]
pub use afe::can_rx_router;
#[cfg(feature = "module-afe")]
pub use pi::handle_message;
#[cfg(feature = "module-afe")]
pub use pi::handle_service_request;
