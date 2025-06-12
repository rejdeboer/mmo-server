use uuid::Uuid;

pub struct Document {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub state_vector: Option<Vec<u8>>,
}
