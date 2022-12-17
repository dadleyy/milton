use async_std::prelude::FutureExt;
use clap::Parser;
use std::io;

#[derive(Parser)]
#[clap(version = option_env!("MILTON_VERSION").unwrap_or("dev"))]
struct CommandLineArguments {
  #[clap(long = "config", short)]
  config: String,
}

async fn run(args: CommandLineArguments) -> io::Result<()> {
  let config_contents = async_std::fs::read_to_string(&args.config).await?;
  let config = toml::from_str::<milton_alexa::config::Config>(&config_contents)
    .map_err(|error| io::Error::new(io::ErrorKind::Other, format!("bad config - {error}")))?;

  log::info!(
    "milton alexa, version {}",
    option_env!("MILTON_VERSION").unwrap_or("dev")
  );

  milton_alexa::discovery::discovery(&config)
    .race(milton_alexa::runtime::application(&config))
    .await
}

fn main() -> io::Result<()> {
  if let Err(error) = dotenv::dotenv() {
    eprintln!("unable to initialize dotenv - {error}");
  }
  env_logger::init();
  let args = CommandLineArguments::parse();

  async_std::task::block_on(run(args))
}
