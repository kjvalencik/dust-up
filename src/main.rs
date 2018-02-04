extern crate clap;

#[macro_use]
extern crate error_chain;

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
use self::errors::{Error, Result, ResultExt};
use self::lib::stdio::stdin_body;
use self::lib::terminal::RawTerminal;

#[macro_use]
mod lib;
mod errors;
mod docker;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

enum Command {
	Info,
	Inspect(String),
	Attach(String),
}

fn run_async(core: &mut Core, cmd: Command, docker: &Docker) -> Result<()> {
	match cmd {
		Command::Info => {
			let work = docker.info();

			core.run(work).and_then(|v| {
				println!("{}", v);

				Ok(())
			})
		}
		Command::Inspect(id) => {
			let work = docker.inspect(&id);

			core.run(work).and_then(|v| {
				println!("{}", v);

				Ok(())
			})
		}
		Command::Attach(id) => {
			let _term = RawTerminal::new();
			let (body, body_work) = stdin_body();
			let work = docker
				.attach(&id, body)
				.and_then(|res| {
					term_size::dimensions()
						.chain_err(|| "Could not get terminal dimensions")
						.map(|(width, height)| {
							docker.resize(&id, width, height)
						})
						.map(|_| res)
				})
				.and_then(|res| {
					let work =
						res.body().map_err(Error::from).for_each(|chunk| {
							let mut stdout = io::stdout();

							stdout
								.write_all(&chunk)
								.map(|_| stdout.flush())
								.map(|_| ())
								.map_err(Error::from)
						});

					body_work
						.map_err(Error::from)
						.select(work)
						.map(|_| ())
						.map_err(|(err, _)| err)
				});

			core.run(work)
		}
	}
}

fn run() -> Result<()> {
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

	let cmd = if matches.subcommand_matches("info").is_some() {
		Command::Info
	} else if let Some(matches) = matches.subcommand_matches("inspect") {
		let container_id = matches
			.value_of("CONTAINER")
			.chain_err(|| "CONTAINER is required")?;

		Command::Inspect(container_id.to_owned())
	} else if let Some(matches) = matches.subcommand_matches("attach") {
		let container_id = matches
			.value_of("CONTAINER")
			.chain_err(|| "CONTAINER is required")?;

		Command::Attach(container_id.to_owned())
	} else {
		return Ok(());
	};

	let mut core = Core::new()?;
	let docker = Docker::new(&core)?;

	run_async(&mut core, cmd, &docker)
}

quick_main!(run);
