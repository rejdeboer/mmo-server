mod client;
mod reader;
mod websocket;
mod writer;

pub use client::{WebClient, WebClientError};
pub use protocol::social::{ChannelType, SocialAction, SocialEvent};
pub use web_types::*;
pub use websocket::{ConnectionError, ConnectionResult, connect};
