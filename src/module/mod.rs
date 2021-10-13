#[cfg(feature = "module-pi")]
pub mod pi;
#[cfg(feature = "module-pi")]
pub use pi::handle_message;
#[cfg(feature = "module-pi")]
pub use pi::handle_service_request;
#[cfg(not(feature = "module-pi"))]
pub mod pi {
    pub type Event = ();
    pub type PiEn = ();
}

#[cfg(feature = "module-led")]
pub mod led;
#[cfg(feature = "module-led")]
pub use led::handle_message;
#[cfg(feature = "module-led")]
pub use led::handle_service_request;
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
pub use button::handle_message;
#[cfg(feature = "module-button")]
pub use button::handle_service_request;

#[cfg(feature = "module-afe")]
pub mod afe;
#[cfg(feature = "module-afe")]
pub use afe::handle_message;
#[cfg(feature = "module-afe")]
pub use afe::handle_service_request;
