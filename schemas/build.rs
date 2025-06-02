use std::path::Path;

fn main() {
    let schema_files = [
        "schemas/character.fbs",
        "schemas/common.fbs",
        "schemas/enter_game_request.fbs",
        "schemas/enter_game_response.fbs",
        "schemas/entity.fbs",
        "schemas/event.fbs",
    ];

    for schema in &schema_files {
        println!("cargo:rerun-if-changed={}", schema);
    }

    for schema in &schema_files {
        flatc_rust::run(flatc_rust::Args {
            inputs: &[Path::new(schema)],
            out_dir: Path::new("src/"),
            ..Default::default()
        })
        .expect(&format!("Failed to compile {}", schema));
    }
}
