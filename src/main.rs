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

use std::io::{self, Write};

use clap::{App, Arg, SubCommand};

use futures::Stream;

use tokio_core::reactor::Core;

use docker::{tokio_stdin, Docker};
use terminal::RawTerminal;

mod docker;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

mod terminal {
	use std::io;
	use std::os::unix::io::AsRawFd;

	use termios::{self, cfmakeraw, tcsetattr, Termios};

	struct PrevTerminal {
		fd: i32,
		prev_ios: Termios,
	}

	impl Drop for PrevTerminal {
		fn drop(&mut self) {
			tcsetattr(self.fd, termios::TCSADRAIN, &self.prev_ios)
				.expect("failed to restore term");
		}
	}

	pub struct RawTerminal {
		_prev_ios: PrevTerminal,
	}

	impl RawTerminal {
		pub fn new() -> RawTerminal {
			let stdin = io::stdin();
			let fd = stdin.as_raw_fd();
			let mut ios = Termios::from_fd(fd).expect("valid stdin handle");
			let prev_ios = PrevTerminal {
				fd,
				prev_ios: ios
			};

			cfmakeraw(&mut ios);
			tcsetattr(fd, termios::TCSADRAIN, &ios).expect("set attr on ios");

			RawTerminal {
				_prev_ios: prev_ios,
			}
		}
	}
}

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

	if matches.subcommand_matches("info").is_some() {
		Docker::new(&mut core)
			.info()
			.map(|v| println!("{}", v))
			.expect("Failed to connect to docker");
	} else if let Some(matches) = matches.subcommand_matches("inspect") {
		let container_id = matches
			.value_of("CONTAINER")
			.expect("CONTAINER is required.");

		Docker::new(&mut core)
			.inspect(container_id)
			.map(|v| println!("{}", v))
			.expect("Failed to connect to docker");
	} else if let Some(matches) = matches.subcommand_matches("attach") {
		let container_id = matches
			.value_of("CONTAINER")
			.expect("CONTAINER is required.");

		// TODO: Use the `stdin` from here to create the tokio so this
		// isn't unused.
		let _term = RawTerminal::new();
		let body = tokio_stdin::stdin_body(&core);

		Docker::new(&mut core)
			.attach(container_id, body, |res| {
				let (width, height) = term_size::dimensions().unwrap();

				// FIXME: Make docker own `core` so it can be reused
				Docker::new(&mut Core::new().unwrap())
					.resize(container_id, width, height)
					.unwrap();

				res.body().for_each(|chunk| {
					let mut stdout = io::stdout();

					stdout
						.write_all(&chunk)
						.map(|_| stdout.flush())
						.map(|_| ())
						.map_err(From::from)
				})
			})
			.expect("not to fail");
	}
}
