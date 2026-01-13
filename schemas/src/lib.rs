#![allow(warnings, unused)]

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
