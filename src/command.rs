use clap::CommandFactory;
use clap::{Parser, Subcommand};
use std::fmt::Display;
use std::process;

use crate::tables::t_book;
use crate::tables::t_item2;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Decode t_bookXX._dt to json
    TBookToJson {
        /// Input file path for the t_bookXX._dt file
        input_path: String,
    },
    /// Decode t_items2._dt to json
    TItem2ToJson {
        /// Input file path for the t_items2._dt file
        input_path: String,
    },
    JsonToTBook {
        input_path: String,
    },
    /// Encode t_items2.json to _dt
    JsonToTItem2 {
        /// Input file path for the json representation of a t_items2._dt file
        input_path: String,
    },
}

pub fn run() {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::TBookToJson { input_path } => {
                run_function(t_book::convert_t_book_to_json_file, input_path);
            }
            Commands::TItem2ToJson { input_path } => {
                run_function(t_item2::convert_t_items2_to_json_file, input_path);
            }
            Commands::JsonToTBook { input_path } => {
                run_function(t_book::convert_json_to_t_book, input_path);
            }
            Commands::JsonToTItem2 { input_path } => {
                run_function(t_item2::convert_json_to_t_items2, input_path);
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
