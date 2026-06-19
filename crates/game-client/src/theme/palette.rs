use bevy::prelude::Color;

// Unit frames
pub const HP_GREEN: Color = Color::srgb(0.1, 0.7, 0.1);
pub const HP_BG: Color = Color::srgb(0.2, 0.0, 0.0);
pub const FRAME_BG: Color = Color::srgba(0.1, 0.1, 0.1, 0.85);
pub const LEVEL_COLOR: Color = Color::srgb(0.8, 0.8, 0.2);

// Panels and overlays
pub const PANEL_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.4);
pub const PANEL_BG_DARK: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
pub const CONTEXT_MENU_BG: Color = Color::srgba(0.15, 0.15, 0.15, 0.95);
pub const CONTEXT_MENU_HOVER: Color = Color::srgba(0.3, 0.3, 0.5, 0.5);

// Action bar
pub const SLOT_BG: Color = Color::srgba(0.2, 0.2, 0.2, 0.9);
pub const SLOT_HOVER: Color = Color::srgba(0.3, 0.3, 0.3, 0.9);
pub const SLOT_COOLDOWN: Color = Color::srgba(0.1, 0.1, 0.1, 0.9);

// Cast bar
pub const CAST_BAR_FILL: Color = Color::srgba(0.8, 0.6, 0.1, 0.8);

// Combat feedback
pub const DAMAGE_TEXT: Color = Color::srgba(1.0, 0.9, 0.1, 1.0);
