#[cfg(feature = "agones")]
mod agones;
#[cfg(feature = "agones")]
pub use agones::AgonesPlugin;

#[cfg(not(feature = "agones"))]
mod agones_mock;
#[cfg(not(feature = "agones"))]
pub use agones_mock::AgonesPlugin;

#[cfg(feature = "debug")]
mod debug;
#[cfg(feature = "debug")]
pub use debug::AppPlugin;

#[cfg(not(feature = "debug"))]
mod headless;
#[cfg(not(feature = "debug"))]
pub use headless::AppPlugin;
