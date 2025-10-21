use db_seeder::{seed::*, server::Application};

use clap::{Command, arg};

fn cli() -> Command {
    let url_arg = arg!(--url <URL> "The DB to seed");
    Command::new("db-seeder")
        .about("A CLI to seed the MMO database")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(false)
        .subcommand(
            Command::new("serve")
                .about("Starts HTTP seeding server")
                .arg(arg!(--port <PORT> "The port to listen on").default_value("8032"))
                .arg(arg!(--host <HOST> "The host to listen on").default_value("127.0.0.1"))
                .arg(url_arg.clone()),
        )
        .subcommand(
            Command::new("seed")
                .about("Seeds a given MMO DB")
                .arg(arg!(--count <COUNT> "Amount of users to create").default_value("2"))
                .arg(url_arg),
        )
}

#[tokio::main]
async fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("seed", sub_matches)) => {
            tracing::info!("Seeding database");

            let count = sub_matches
                .get_one::<u32>("count")
                .expect("should be set by default");

            let url = sub_matches.get_one::<String>("url").expect("required");

            seed(url, *count).await.expect("seed is successful");
        }
        Some(("serve", sub_matches)) => {
            let host = sub_matches
                .get_one::<String>("host")
                .expect("host should be set by default");
            let port = sub_matches
                .get_one::<u32>("port")
                .expect("port should be set by default");
            let url = sub_matches
                .get_one::<String>("url")
                .expect("db url is required");

            let mut app = Application::build(&host, port, &url)
                .await
                .expect("server should build");
            app.run_until_stopped().await;
        }
        _ => unreachable!(),
    }
}
