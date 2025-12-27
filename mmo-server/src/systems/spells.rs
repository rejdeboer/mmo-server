use bevy::prelude::*;

use crate::messages::CastSpellActionMessage;

pub fn process_spell_casts(reader: MessageReader<CastSpellActionMessage>) {}
