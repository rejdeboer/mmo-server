use bevy::prelude::*;

use crate::messages::{CastSpellActionMessage, OutgoingMessageData};

pub fn process_spell_casts(
    mut reader: MessageReader<CastSpellActionMessage>,
    mut writer: MessageWriter<OutgoingMessageData>,
) {
    for msg in reader.read() {}
}
