#[cfg(feature = "agones")]
mod agones;
#[cfg(feature = "agones")]
pub use agones::AgonesPlugin;

#[cfg(not(feature = "agones"))]
mod agones_mock;
#[cfg(not(feature = "agones"))]
pub use agones_mock::AgonesPlugin;
