extern crate flatbuffers;

mod generated;

// #[allow(dead_code, unused_imports)]
// #[path = "./entity_generated.rs"]
// mod entity_generated;
// pub use entity_generated::mmo::Entity;
//
// #[allow(dead_code, unused_imports)]
// #[path = "./player_generated.rs"]
// mod player_generated;
// pub use player_generated::mmo::{Character, PlayerData};
//
// #[allow(dead_code, unused_imports)]
// #[path = "./common_generated.rs"]
// mod common_generated;
// pub use common_generated::mmo::Vec3;

#[allow(dead_code, unused_imports)]
pub use generated::enter_game_request_generated::mmo::{EnterGameRequest, EnterGameRequestArgs};
