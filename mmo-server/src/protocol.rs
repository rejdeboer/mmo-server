use flatbuffers::{FlatBufferBuilder, WIPOffset};

pub trait Encode {
    fn encode<'a>(
        self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> WIPOffset<schemas::mmo::Character<'a>>;
}
