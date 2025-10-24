use crate::domain::Transform;
use schemas::game as schema;

const SPEED_PRECISION_MULTIPLIER: f32 = 100.0;

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: u64,
    pub name: String,
    pub attributes: EntityAttributes,
    pub vitals: Vitals,
    pub level: i32,
    pub transform: Transform,
    pub movement_speed: f32,
}

#[derive(Debug, Clone)]
pub enum EntityAttributes {
    Player {
        character_id: i32,
        guild_name: Option<String>,
    },
    Npc,
}

#[derive(Debug, Clone)]
pub struct Vitals {
    pub hp: i32,
    pub max_hp: i32,
}

impl TryInto<Entity> for schema::Entity<'_> {
    type Error = &'static str;

    fn try_into(self) -> Result<Entity, Self::Error> {
        let Some(attributes) = self.attributes_as_player_attributes() else {
            return Err("player entity should have player attributes");
        };

        Ok(Entity {
            id: self.id(),
            attributes: EntityAttributes::Player {
                character_id: attributes.character_id(),
                guild_name: attributes.guild_name().map(&str::to_string),
            },
            name: self.name().to_string(),
            vitals: self.vitals().into(),
            level: self.level(),
            transform: self.transform().into(),
            movement_speed: self.movement_speed() as f32 / SPEED_PRECISION_MULTIPLIER,
        })
    }
}

impl TryInto<Entity> for schema::EnterGameResponse<'_> {
    type Error = &'static str;

    fn try_into(self) -> Result<Entity, Self::Error> {
        self.player_entity()
            .ok_or("player entity is empty")?
            .try_into()
    }
}

impl From<&schema::Vitals> for Vitals {
    fn from(val: &schema::Vitals) -> Self {
        Vitals {
            hp: val.hp(),
            max_hp: val.max_hp(),
        }
    }
}
