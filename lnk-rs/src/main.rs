use std::{fs::File, path::PathBuf};

use clap::Parser;
use lnk_rs::Lnk;

#[derive(Debug, clap::Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    LinkRead {
        #[arg()]
        target: PathBuf,
    },
}

fn main() {
    let args = Args::parse();
    match args.command {
        Command::LinkRead { target } => {
            let mut file = File::open(target.as_path()).unwrap();
            match Lnk::parse(&mut file) {
                Ok(lnk) => println!("{:#?}", lnk),
                Err(err) => eprintln!("Failed to parse link file:\n{}", err),
            }
        }
    }
}
