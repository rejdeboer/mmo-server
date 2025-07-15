use std::path::Path;

#[allow(clippy::all)]
fn main() {
    let schema_files = [
        "schemas/action.fbs",
        "schemas/chat.fbs",
        "schemas/common.fbs",
        "schemas/entity.fbs",
        "schemas/event.fbs",
        "schemas/character.fbs",
        "schemas/netcode_token_user_data.fbs",
        "schemas/enter_game_response.fbs",
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
