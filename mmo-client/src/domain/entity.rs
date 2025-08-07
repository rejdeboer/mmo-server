use crate::domain::Transform;
use schemas::game as schema;

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: u64,
    pub name: String,
    pub attributes: EntityAttributes,
    pub vitals: Vitals,
    pub level: i32,
    pub transform: Transform,
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

        let guild_name = attributes.guild_name().map(|n| n.to_string());

        Ok(Entity {
            id: self.id(),
            attributes: EntityAttributes::Player {
                character_id: attributes.character_id(),
                guild_name,
            },
            name: self.name().to_string(),
            vitals: self.vitals().into(),
            level: self.level(),
            transform: self.transform().into(),
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

impl Into<Vitals> for &schema::Vitals {
    fn into(self) -> Vitals {
        Vitals {
            hp: self.hp(),
            max_hp: self.max_hp(),
        }
    }
}
