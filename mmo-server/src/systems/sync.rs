use crate::{
    messages::{OutgoingMessage, OutgoingMessageData},
    telemetry::Metrics,
};
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer};
use protocol::server::ServerEvent;

// TODO: Maybe use change detection for transform changes
pub fn sync_players(
    mut server: ResMut<RenetServer>,
    mut ev_msg: MessageReader<OutgoingMessage>,
    metrics: Res<Metrics>,
) {
    let mut client_messages: HashMap<ClientId, Vec<&OutgoingMessageData>> = HashMap::new();
    for event in ev_msg.read() {
        client_messages
            .entry(event.client_id)
            .or_default()
            .push(&event.data);
    }

    if client_messages.is_empty() {
        return;
    }

    for (client_id, messages) in client_messages {
        let mut player_messages = Vec::<ServerEvent>::with_capacity(messages.len());
        let mut channel = DefaultChannel::Unreliable;

        for msg in messages {
            if !matches!(msg, OutgoingMessageData::Movement(_, _)) {
                channel = DefaultChannel::ReliableOrdered;
            }
            player_messages.push(bitcode::encode(msg));
        }

        if player_messages.is_empty() {
            return;
        }

        let channel_label = match channel {
            DefaultChannel::Unreliable => "unreliable",
            _ => "reliable",
        };
        let metric_labels = &["outgoing", channel_label];
        metrics
            .network_packets_total
            .with_label_values(metric_labels)
            .inc();
        metrics
            .network_bytes_total
            .with_label_values(metric_labels)
            .inc_by(data.len() as u64);

        server.send_message(client_id, channel, data);
        builder.reset();
    }
}
