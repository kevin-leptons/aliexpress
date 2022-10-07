use clap::{Args, Parser, Subcommand};
use dropshipping::aliexpress_provider::AliexpressProvider;
use dropshipping::analyst::Analyst;
use dropshipping::config::Config;
use dropshipping::data_puller::DataPuller;
use dropshipping::database::Database;
use dropshipping::log::{Log, LogFileSystem};
use dropshipping::provider::{Product, Provider, Store};
use dropshipping::result::BoxResult;
use dropshipping::{error, info};
use serde::de::Error;
use std;
use std::collections::VecDeque;
use std::io::{stdin, BufRead, Write};
use std::io::{stdout, Read};
use std::path::{Path, PathBuf};
use std::{env, io, process};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(
        short,
        long,
        default_value = "config.json",
        help = "Path to configuration file"
    )]
    file: String,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Scrape products from online platforms")]
    Pull(PullArguments),

    #[command(about = "Analyze data and make reports")]
    Analyze(AnalyzeArguments),

    Clear,
}

#[derive(Args)]
struct PullArguments {}

#[derive(Args)]
struct AnalyzeArguments {}

fn main() {
    let mut out = stdout();
    let mut log = Log::new("main", &mut out);
    let cli = Cli::parse();
    let config_file = PathBuf::from(&cli.file);
    let config = match Config::from_json_file(Some(config_file.clone())) {
        Err(e) => {
            error!(
                log,
                "unable to read configuration file: {}",
                e.to_string().as_str()
            );
            process::exit(1);
        }
        Ok(v) => v,
    };
    info!(
        log,
        "use configuration file: {}",
        config_file.to_string_lossy()
    );
    match &cli.command {
        Commands::Pull(v) => command_pull(&mut log, config, v),
        Commands::Analyze(v) => command_analyze(&mut log, config),
        Commands::Clear => command_clear(&mut log, config),
    }
}

fn command_pull<'a>(log: &'a mut Log<'a>, config: Config, args: &PullArguments) {
    let log_fs = LogFileSystem::new(config.log_directory.clone());
    let mut puller = DataPuller::new(config, log, log_fs).unwrap();
    match puller.run() {
        Err(e) => {
            println!("error: {}", e.to_string());
            std::process::exit(1);
        }
        Ok(_) => {
            println!("succeed");
            std::process::exit(0);
        }
    }
}

fn command_analyze(log: &mut Log, config: Config) {
    let mut analyst = Analyst::new(log, &config).unwrap();
    analyst.start().unwrap();
}

fn command_clear<'a>(log: &'a mut Log<'a>, config: Config) {
    let log_fs = LogFileSystem::new(config.log_directory.clone());
    validate_user_confirmation(log);
    let mut puller = DataPuller::new(config, log, log_fs).unwrap();
    puller.clear().unwrap();
}

fn validate_user_confirmation(log: &mut Log) {
    let expectation = "destroy all data".to_string();
    info!(log, "type exact words for confirmation : {}", expectation);
    let input_result = match stdin().lock().lines().next() {
        None => return error!(log, "no confirmation, no destroying data!"),
        Some(v) => v,
    };
    let input = match input_result {
        Err(e) => return error!(log, "{}", e.to_string()),
        Ok(v) => v,
    };
    if input != expectation {
        error!(log, "wrong confirmation, no destroying data!");
        std::process::exit(1);
    }
}
