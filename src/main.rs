use std::path::Path;

use clap::{Args, Parser, Subcommand};
use log::{Level, LevelFilter};
use simplelog::{Color, ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use what::{convert_cubemap, convert_texture};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Convert(ConvertArgs),
}

#[derive(Args)]
struct ConvertArgs {
    input: Vec<String>,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(long, default_value_t = false)]
    overwrite: bool,
}

fn main() {
    let config = ConfigBuilder::new()
        .set_level_color(Level::Trace, Some(Color::White))
        .set_level_color(Level::Info, Some(Color::Green))
        .set_level_color(Level::Warn, Some(Color::Yellow))
        .set_level_color(Level::Error, Some(Color::Red))
        .build();

    let _ = TermLogger::init(
        LevelFilter::Info,
        config,
        TerminalMode::Mixed,
        ColorChoice::Auto,
    );

    let cli = Cli::parse();

    match &cli.command {
        Commands::Convert(args) => {
            let inputs = args.input.iter().map(Path::new).collect::<Vec<&Path>>();

            match 1.cmp(&args.input.len()) {
                std::cmp::Ordering::Greater => {
                    let output = args
                        .output
                        .clone()
                        .expect("Cannot infer output file name of a cube map. Please provide one.");

                    if let Err(e) = convert_cubemap(Path::new(&output), inputs, args.overwrite) {
                        log::error!("{}", e);
                    }
                }
                std::cmp::Ordering::Equal => {
                    let output = if args.output.is_none() {
                        let file_name_os = inputs[0]
                            .file_name()
                            .expect("Failed to infer output file name. Please provide one.");
                        let file_name = file_name_os
                            .to_os_string()
                            .into_string()
                            .expect("Failed to infer output file name. Please provide one.");

                        let index = file_name.find('.').unwrap_or(usize::MAX);
                        let mut new_file = file_name[0..index].to_string();
                        new_file.push_str(".fur");
                        new_file
                    } else {
                        args.output.clone().unwrap()
                    };

                    if let Err(e) = convert_texture(Path::new(&output), inputs[0], args.overwrite) {
                        log::error!("{}", e);
                    }
                }
                std::cmp::Ordering::Less => {
                    panic!("Please provide an input file path.");
                }
            }
        }
    }
}