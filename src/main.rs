extern crate diamond_drops;
use diamond_drops::cli;

use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();

    let config = cli::args::parse_cli_args(args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    let result: u32 = diamond_drops::run(config);
}
