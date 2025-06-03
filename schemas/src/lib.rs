pub mod mmo {
    use super::*;
    mod vec_3_generated;
    pub use self::vec_3_generated::*;
    mod transform_generated;
    pub use self::transform_generated::*;
    mod entity_generated;
    pub use self::entity_generated::*;
    mod character_generated;
    pub use self::character_generated::*;
    mod enter_game_response_generated;
    pub use self::enter_game_response_generated::*;
    mod enter_game_request_generated;
    pub use self::enter_game_request_generated::*;
} // mmo
