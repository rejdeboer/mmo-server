use std::path::Path;

fn main() {
    let schema_files = [
        "schemas/common.fbs",
        "schemas/entity.fbs",
        "schemas/event.fbs",
        "schemas/character.fbs",
        "schemas/enter_game_request.fbs",
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
        extra: &["--rust-module-root-file"],
        ..Default::default()
    })
    .expect(&format!("Failed to compile"));
}
