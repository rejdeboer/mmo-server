#[cfg(feature = "debug")]
mod debug;
#[cfg(feature = "debug")]
pub use debug::AppPlugin;

#[cfg(not(feature = "debug"))]
mod headless;
#[cfg(not(feature = "debug"))]
pub use headless::AppPlugin;
