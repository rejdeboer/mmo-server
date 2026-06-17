mod loot;

pub use loot::LootEntry;

use bevy::prelude::*;

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(loot::reward_kill);
    }
}
