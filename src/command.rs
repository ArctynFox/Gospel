use clap::CommandFactory;
use clap::{Parser, Subcommand};
use std::fmt::Display;
use std::process;

use crate::t_items2::{convert_json_to_t_items2, convert_t_items2_to_json_file};

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Decode t_items2._dt to json
    TItems2ToJson {
        /// Input file path for the t_items2._dt file
        input_path: String,
    },
    /// Encode t_items2.json to _dt
    TItems2FromJson {
        /// Input file path for the json representation of a t_items2._dt file
        input_path: String,
    },
}

pub fn run() {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::TItems2ToJson { input_path } => {
                run_function(convert_t_items2_to_json_file, input_path);
            }
            Commands::TItems2FromJson { input_path } => {
                run_function(convert_json_to_t_items2, input_path);
            }
        }
    } else {
        Cli::command().print_help().unwrap();
        println!();
        process::exit(0);
    }
}

fn run_function<T, E, F>(func: F, path: String)
where
    F: FnOnce(String) -> Result<T, E>,
    E: std::error::Error + Display,
{
    if let Err(e) = func(path) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
