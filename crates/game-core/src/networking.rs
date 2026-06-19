use bevy::{platform::collections::HashMap, prelude::*};

#[derive(Debug, Component, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NetworkId(pub u32);

#[derive(Resource, Debug, Default)]
pub struct NetworkIdMapping(pub HashMap<NetworkId, Entity>);
