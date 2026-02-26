use std::sync::{Arc, RwLock};

use bonfire::{
    http,
    server::{self, auth},
};
use clap::{Parser, Subcommand, builder::Styles, crate_description, crate_version};

/// Clap v3 style (approximate)
/// See https://stackoverflow.com/a/75343828
fn style() -> clap::builder::Styles {
    Styles::styled()
        .usage(
            anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)))
                .bold(),
        )
        .header(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .literal(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
}

/// The root object for parsing CLI arguments.
///
/// Subcommands and flags and hierarchically defined below this.
#[derive(Parser)]
#[command(
	version,
	about = format!("{} v{}", crate_description!(), crate_version!()),
	styles(style()),
	disable_colored_help(false),
	arg_required_else_help(true)
)]
struct CliArguments {
    #[command(subcommand)]
    pub subcommand: ToplevelCommmands,
}

/// The top-level commands available to the CLI.
#[derive(Subcommand)]
enum ToplevelCommmands {
    Server,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let cli_args = CliArguments::parse();

    match &cli_args.subcommand {
        ToplevelCommmands::Server => {
            let config = server::Config {
                data_dir: "data/".into(),
                auth: auth::AuthConfig {
                    oauth2_clients: vec![],
                },
            };

            let srv = Arc::new(RwLock::new(server::Server::new(config).unwrap()));

            let app = http::make_app_router(srv);

            // run our app with hyper, listening globally on port 3000
            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

            axum::serve(listener, app).await.unwrap();
        }
    }
}
