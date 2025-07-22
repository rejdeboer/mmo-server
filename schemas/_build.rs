use std::path::Path;

#[allow(clippy::all)]
fn main() {
    let schema_files = [
        "schemas/game/action.fbs",
        "schemas/game/chat.fbs",
        "schemas/game/common.fbs",
        "schemas/game/entity.fbs",
        "schemas/game/event.fbs",
        "schemas/game/character.fbs",
        "schemas/game/enter_game_response.fbs",
        "schemas/social/common.fbs",
        "schemas/social/action.fbs",
        "schemas/social/event.fbs",
        "schemas/protocol/token_user_data.fbs",
    ];

    for schema in &schema_files {
        println!("cargo:rerun-if-changed={}", schema);
    }

    let paths: Vec<&Path> = schema_files
        .into_iter()
        .map(|schema| Path::new(schema))
        .collect();

    flatc_rust::run(flatc_rust::Args {
        inputs: &paths,
        out_dir: Path::new("src/"),
        // TODO: Do we need this automation? It only generates mod file for last schema
        extra: &["--gen-all", "--rust-module-root-file"],
        ..Default::default()
    })
    .expect(&format!("Failed to compile"));
}
