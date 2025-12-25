#![allow(warnings, unused)]
pub mod game {
    use super::*;
    mod vec_2_generated;
    pub use self::vec_2_generated::*;
    mod vec_3_generated;
    pub use self::vec_3_generated::*;
    mod transform_generated;
    pub use self::transform_generated::*;
    mod vitals_generated;
    pub use self::vitals_generated::*;
    mod entity_generated;
    pub use self::entity_generated::*;
    mod entity_attributes_generated;
    pub use self::entity_attributes_generated::*;
    mod player_attributes_generated;
    pub use self::player_attributes_generated::*;
    mod npc_attributes_generated;
    pub use self::npc_attributes_generated::*;
    mod enter_game_response_generated;
    pub use self::enter_game_response_generated::*;
    mod entity_move_event_generated;
    pub use entity_move_event_generated::*;
    mod entity_spawn_event_generated;
    pub use entity_spawn_event_generated::*;
    mod entity_despawn_event_generated;
    pub use entity_despawn_event_generated::*;
    mod targetting_event_generated;
    pub use targetting_event_generated::*;
    mod event_data_generated;
    pub use event_data_generated::*;
    mod event_generated;
    pub use event_generated::*;
    mod batched_events_generated;
    pub use batched_events_generated::*;

    mod player_move_action_generated;
    pub use player_move_action_generated::*;
    mod player_jump_action_generated;
    pub use player_jump_action_generated::*;
    mod cast_spell_action_generated;
    pub use cast_spell_action_generated::*;
    mod targetting_action_generated;
    pub use targetting_action_generated::*;
    mod action_data_generated;
    pub use action_data_generated::*;
    mod action_generated;
    pub use action_generated::*;
    mod batched_actions_generated;
    pub use batched_actions_generated::*;

    mod channel_type_generated;
    pub use self::channel_type_generated::*;
    mod client_chat_message_generated;
    pub use self::client_chat_message_generated::*;
    mod server_chat_message_generated;
    pub use self::server_chat_message_generated::*;
} // mmo

pub mod social {
    use super::*;
    mod channel_type_generated;
    pub use channel_type_generated::*;

    mod client_chat_message_generated;
    pub use client_chat_message_generated::*;
    mod client_whisper_by_id_generated;
    pub use client_whisper_by_id_generated::*;
    mod client_whisper_by_name_generated;
    pub use client_whisper_by_name_generated::*;
    mod action_data_generated;
    pub use action_data_generated::*;
    mod action_generated;
    pub use action_generated::*;

    mod server_chat_message_generated;
    pub use server_chat_message_generated::*;
    mod server_system_message_generated;
    pub use server_system_message_generated::*;
    mod server_whisper_generated;
    pub use server_whisper_generated::*;
    mod server_whisper_receipt_generated;
    pub use server_whisper_receipt_generated::*;
    mod event_data_generated;
    pub use event_data_generated::*;
    mod event_generated;
    pub use event_generated::*;
}

pub mod protocol {
    use super::*;
    mod token_user_data_generated;
    pub use token_user_data_generated::*;
}
