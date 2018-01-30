extern crate clap;
extern crate futures;
extern crate hyper;
extern crate hyper_openssl;
extern crate hyperlocal;
extern crate openssl;
extern crate serde_json;
extern crate serde_yaml;
extern crate tokio_core;

use clap::{App, SubCommand};

use docker::Docker;

mod docker;

const NAME: &'static str = env!("CARGO_PKG_NAME");
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &'static str = env!("CARGO_PKG_DESCRIPTION");
const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");

fn main() {
	let matches = App::new(NAME)
		.version(VERSION)
		.about(DESCRIPTION)
		.author(AUTHORS)
		.subcommand(
			SubCommand::with_name("info")
				.about("Display system-wide information")
		)
		.get_matches();

	if let Some(_) = matches.subcommand_matches("info") {
		Docker::new()
			.info()
			.map(|v| println!("{}", v))
			.expect("Failed to connect to docker");
	}
}
