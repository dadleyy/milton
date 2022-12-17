//! This cli is very much a work-in-progress. The immediate goal was to provide some tooling for
//! provisioning admin tokens using the same `config.toml` file that the main `milton-web`
//! application uses.

use clap::Parser;
use serde::Deserialize;
use std::io;

#[derive(Deserialize, Debug)]
struct CliConfiguration {
  redis_addr: String,
  token_store: String,
}

#[derive(Deserialize, Debug)]
struct RuntimeConfiguration {
  cli: CliConfiguration,
}

#[derive(clap::Subcommand, Deserialize)]
enum CliCommand {
  CreateAdminToken,
}

#[derive(Deserialize, clap::Parser)]
#[command(author, version = option_env!("MILTON_VERSION").unwrap_or_else(|| "dev"), about, long_about = None)]
struct CommandLineOptions {
  #[arg(short = 'c', long)]
  config: String,

  #[command(subcommand)]
  command: CliCommand,
}

async fn run(args: CommandLineOptions, config: RuntimeConfiguration) -> io::Result<()> {
  let mut client = async_std::net::TcpStream::connect(&config.cli.redis_addr)
    .await
    .map_err(|error| io::Error::new(io::ErrorKind::Other, format!("unable to connect to redis - {error}")))?;

  match args.command {
    CliCommand::CreateAdminToken => {
      // Start by getting the list of our current tokens
      let get_command = kramer::Command::Hashes::<&str, &str>(kramer::HashCommand::Get(
        &config.cli.token_store,
        Some(kramer::Arity::One("_admin")),
      ));
      let result = kramer::execute(&mut client, get_command).await.map_err(|error| {
        io::Error::new(
          io::ErrorKind::Other,
          format!("unable to get current admin tokens - {error}"),
        )
      })?;

      let mut current_tokens = match &result {
        kramer::Response::Item(kramer::ResponseValue::String(content)) => serde_json::from_str::<Vec<String>>(content)?,
        kramer::Response::Item(kramer::ResponseValue::Empty) => vec![],
        response => {
          log::warn!("unrecognized response from admin token lookup");
          return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("bad lookup - {response:?}"),
          ));
        }
      };
      log::info!("{result:?}");

      let new_token = uuid::Uuid::new_v4().to_string();
      current_tokens.push(new_token.clone());
      let new_contents = serde_json::to_string(&current_tokens)?;

      let command = kramer::Command::Hashes::<&str, &str>(kramer::HashCommand::Set(
        &config.cli.token_store,
        kramer::Arity::One(("_admin", &new_contents)),
        kramer::Insertion::Always,
      ));

      let result = kramer::execute(&mut client, command).await?;
      log::info!("created admin token '{new_token}' -> {result:?}");
    }
  }
  Ok(())
}

fn main() -> io::Result<()> {
  if dotenv::dotenv().is_err() {
    eprintln!("warning: no '.env' file detected'");
  }

  env_logger::init();
  let args = CommandLineOptions::parse();
  log::info!("loading config from '{}'", args.config);
  let contents = std::fs::read_to_string(&args.config)?;
  let parsed = toml::from_str::<RuntimeConfiguration>(&contents)?;
  async_std::task::block_on(run(args, parsed))
}
