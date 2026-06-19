pub mod messages;
mod receive;
mod tick_sync;

pub use messages::*;
pub use receive::{poll_connection, receive_server_events};
pub use tick_sync::TickSync;

use bevy::prelude::*;

use crate::application::{AppState, EnterGame};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum NetworkingSet {
    Receive,
}

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ActorSpawnMessage>();
        app.add_message::<ActorDespawnMessage>();
        app.add_message::<CombatHitMessage>();
        app.add_message::<SpellImpactMessage>();
        app.add_message::<ActorDeathMessage>();
        app.add_message::<StartCastingMessage>();
        app.add_message::<KillRewardMessage>();
        app.add_message::<ServerChatMessage>();

        app.add_observer(on_enter_game);

        app.add_systems(
            Update,
            poll_connection.run_if(in_state(AppState::Connecting)),
        );

        app.add_systems(
            FixedPreUpdate,
            (tick_sync::increment_tick, tick_sync::send_ping).run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            Update,
            (
                receive_server_events,
                tick_sync::adjust_tick_rate,
                receive::handle_spell_impacts,
                receive::handle_actor_deaths,
                receive::handle_start_casting,
                receive::handle_kill_rewards,
                receive::handle_server_chat,
            )
                .in_set(NetworkingSet::Receive)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

fn on_enter_game(event: On<EnterGame>, mut commands: Commands) {
    let response = &event.0;
    commands.insert_resource(TickSync::new(response.server_tick));
    tracing::info!(server_tick = ?response.server_tick, "initial tick sync established");
}
