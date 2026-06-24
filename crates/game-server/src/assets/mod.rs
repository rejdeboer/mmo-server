mod content_id;
mod items;
mod loot;
mod monsters;
mod zone;

pub use content_id::ContentId;
pub use items::*;
pub use loot::*;
pub use monsters::*;

use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;
use game_core::spells::{SpellLibrary, SpellLibraryHandle};
use game_core::zone::ZoneDef;

pub struct ContentPlugin;

impl Plugin for ContentPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RonAssetPlugin::<ItemLibrary>::new(&["items.ron"]),
            RonAssetPlugin::<LootTableLibrary>::new(&["loot_tables.ron"]),
            RonAssetPlugin::<MonsterLibrary>::new(&["monsters.ron"]),
            RonAssetPlugin::<SpellLibrary>::new(&["spells.ron"]),
            RonAssetPlugin::<ZoneDef>::new(&["zone.ron"]),
        ));

        app.add_systems(PreStartup, setup_assets);
        app.add_systems(Update, (zone::spawn_zone_when_ready, zone::despawn_non_lod0));
    }
}

fn setup_assets(mut commands: Commands, assets: Res<AssetServer>) {
    zone::load_zone(&mut commands, &assets);

    let items_handle = assets.load::<ItemLibrary>("items.ron");
    commands.insert_resource(ItemLibraryHandle(items_handle));
    let loot_tables_handle = assets.load::<LootTableLibrary>("loot_tables.ron");
    commands.insert_resource(LootTableLibraryHandle(loot_tables_handle));
    let monsters_handle = assets.load::<MonsterLibrary>("monsters.ron");
    commands.insert_resource(MonsterLibraryHandle(monsters_handle));
    let spells_handle = assets.load::<SpellLibrary>("spells.ron");
    commands.insert_resource(SpellLibraryHandle(spells_handle));
}
