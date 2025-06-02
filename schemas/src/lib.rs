extern crate flatbuffers;

#[allow(dead_code, unused_imports)]
#[path = "./common_generated.rs"]
mod common_generated;
pub use common_generated::mmo::{Transform, Vec3};

#[allow(dead_code, unused_imports)]
#[path = "./enter_game_request_generated.rs"]
mod enter_game_request_generated;
pub use enter_game_request_generated::mmo::{EnterGameRequest, EnterGameRequestArgs};

#[allow(dead_code, unused_imports)]
#[path = "./enter_game_response_generated.rs"]
mod enter_game_response_generated;
pub use enter_game_response_generated::mmo::{EnterGameResponse, EnterGameResponseArgs};

#[allow(dead_code, unused_imports)]
#[path = "./entity_generated.rs"]
mod entity_generated;
pub use entity_generated::mmo::{Entity, EntityArgs};

#[allow(dead_code, unused_imports)]
#[path = "./character_generated.rs"]
mod character_generated;
pub use character_generated::mmo::{Character, CharacterArgs};
