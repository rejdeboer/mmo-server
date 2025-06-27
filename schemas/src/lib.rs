pub mod mmo {
    use super::*;
    mod vec_2_generated;
    pub use self::vec_2_generated::*;
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
    mod netcode_token_user_data_generated;
    pub use self::netcode_token_user_data_generated::*;
    mod entity_move_event_generated;
    pub use entity_move_event_generated::*;
    mod entity_spawn_event_generated;
    pub use entity_spawn_event_generated::*;
    mod entity_despawn_event_generated;
    pub use entity_despawn_event_generated::*;
    mod event_data_generated;
    pub use event_data_generated::*;
    mod event_generated;
    pub use event_generated::*;
    mod batched_events_generated;
    pub use batched_events_generated::*;
} // mmo
