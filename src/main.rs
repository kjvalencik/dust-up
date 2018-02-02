extern crate clap;
extern crate futures;
extern crate hyper;
extern crate hyper_openssl;
extern crate hyperlocal;
extern crate openssl;
extern crate serde_json;
extern crate serde_yaml;
extern crate term_size;
extern crate termios;
extern crate tokio_core;
extern crate url;

use std::io::{self, Write};

use clap::{App, Arg, SubCommand};

use futures::{Future, Stream};

use tokio_core::reactor::Core;

use self::docker::Docker;
use self::lib::stdio::stdin_body;
use self::lib::terminal::RawTerminal;

#[macro_use]
mod lib;
mod docker;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

fn main() {
	let matches = App::new(NAME)
		.version(VERSION)
		.about(DESCRIPTION)
		.author(AUTHORS)
		.subcommand(
			SubCommand::with_name("info")
				.about("Display system-wide information")
		)
		.subcommand(
			SubCommand::with_name("inspect")
				.about("Return low-level information on Docker objects")
				.arg(Arg::with_name("CONTAINER").required(true))
		)
		.subcommand(
			SubCommand::with_name("attach")
				.about("Attach local standard input, output, and error streams to a running container")
				.arg(Arg::with_name("CONTAINER").required(true))
		)
		.get_matches();

	let mut core = Core::new().unwrap();
	let docker = Docker::new(&core);

	if matches.subcommand_matches("info").is_some() {
		let work = docker.info();

		core.run(work)
			.map(|v| println!("{}", v))
			.expect("Failed to connect to docker");
	} else if let Some(matches) = matches.subcommand_matches("inspect") {
		let container_id = matches
			.value_of("CONTAINER")
			.expect("CONTAINER is required.");

		let work = docker.inspect(container_id);

		core.run(work)
			.map(|v| println!("{}", v))
			.expect("Failed to connect to docker");
	} else if let Some(matches) = matches.subcommand_matches("attach") {
		let container_id = matches
			.value_of("CONTAINER")
			.expect("CONTAINER is required.");

		let _term = RawTerminal::new();
		let body = stdin_body(&core);

		let work = docker.attach(container_id, body).and_then(|res| {
			let (width, height) = term_size::dimensions().unwrap();
			let work = docker.resize(container_id, width, height);

			work.and_then(|_| {
				res.body().for_each(|chunk| {
					let mut stdout = io::stdout();

					stdout
						.write_all(&chunk)
						.map(|_| stdout.flush())
						.map(|_| ())
						.map_err(From::from)
				})
			})
		});

		core.run(work).expect("not to fail");
	}
}