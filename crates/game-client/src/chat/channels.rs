use bevy::prelude::*;
use std::collections::VecDeque;

/// A unified chat message from any source (game-server or social hub).
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub channel: ChatMessageChannel,
    pub sender: String,
    pub text: String,
}

/// All possible chat channels, unifying game-server and social hub channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatMessageChannel {
    Say,
    Yell,
    Zone,
    Guild,
    Party,
    Trade,
    Whisper,
    WhisperSent,
    System,
    Combat,
}

impl ChatMessageChannel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Say => "[Say]",
            Self::Yell => "[Yell]",
            Self::Zone => "[Zone]",
            Self::Guild => "[Guild]",
            Self::Party => "[Party]",
            Self::Trade => "[Trade]",
            Self::Whisper => "[Whisper]",
            Self::WhisperSent => "[To]",
            Self::System => "[System]",
            Self::Combat => "[Combat]",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Self::Say => Color::WHITE,
            Self::Yell => Color::srgb(1.0, 0.2, 0.2),
            Self::Zone => Color::srgb(1.0, 0.75, 0.8),
            Self::Guild => Color::srgb(0.2, 1.0, 0.2),
            Self::Party => Color::srgb(0.4, 0.6, 1.0),
            Self::Trade => Color::srgb(1.0, 0.8, 0.2),
            Self::Whisper | Self::WhisperSent => Color::srgb(0.8, 0.4, 1.0),
            Self::System => Color::srgb(1.0, 1.0, 0.4),
            Self::Combat => Color::srgb(1.0, 0.5, 0.2),
        }
    }
}

const MAX_CHAT_HISTORY: usize = 200;

#[derive(Resource)]
pub struct ChatLog {
    pub messages: VecDeque<ChatMessage>,
}

impl Default for ChatLog {
    fn default() -> Self {
        Self {
            messages: VecDeque::with_capacity(MAX_CHAT_HISTORY),
        }
    }
}

impl ChatLog {
    pub fn push(&mut self, message: ChatMessage) {
        if self.messages.len() >= MAX_CHAT_HISTORY {
            self.messages.pop_front();
        }
        self.messages.push_back(message);
    }
}
