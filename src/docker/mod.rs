use std::env;

use futures::{Future, Stream};

use hyper::{self, Body, HttpVersion, Method, Request, Response};

use serde_json::{self, Value};

use tokio_core::reactor::Core;

use self::transport::Transport;

mod transport;

// FIXME: Use percent encoding on all ID values
pub struct Docker<'a> {
	transport: Transport,
	core: &'a mut Core,
}

pub mod tokio_stdin {
	use std;
	use std::io::{self, Read};
	use std::thread;

	use hyper::{self, Body, Chunk};

	use futures::stream::iter_result;
	use futures::{Future, Sink, Stream};
	use futures::sync::mpsc::{unbounded, SendError, UnboundedReceiver};

	use tokio_core::reactor::Core;

	#[derive(Debug)]
	enum Error {
		Stdin(std::io::Error),
		Channel(SendError<u8>),
	}

	/// Spawn a new thread that reads from stdin and passes messages back using an unbounded channel.
	pub fn spawn_stdin_stream_unbounded() -> UnboundedReceiver<u8> {
		let (channel_sink, channel_stream) = unbounded();
		let stdin_sink = channel_sink.sink_map_err(Error::Channel);

		thread::spawn(move || {
			let stdin = io::stdin();
			let stdin_lock = stdin.lock();

			// Push a couple of empty bytes onto the stream that will be
			// dropped by body encoder
			let bytes = b"\0\0"
				.into_iter()
				.map(|byte| Ok(*byte))
				.chain(stdin_lock.bytes());

			iter_result(bytes)
				.map_err(Error::Stdin)
				.forward(stdin_sink)
				.wait()
				.unwrap();
		});

		channel_stream
	}

	pub fn stdin_body(core: &Core) -> Body {
		let stdin = spawn_stdin_stream_unbounded()
			.map(|byte| Ok(Chunk::from(vec![byte])))
			.map_err(|_| unreachable!());

		let (tx, body) = hyper::Body::pair();

		core.handle()
			.spawn(tx.send_all(stdin).map(|_| ()).map_err(|_| ()));

		body
	}
}

impl<'a> Docker<'a> {
	fn ensure_running(&mut self, id: &str) -> Result<(), hyper::Error> {
		let res_value = self.inspect(id)?;

		// Need to get the type definitions for deserialization, this is
		// terrible.
		if let Value::Object(res) = res_value {
			if let Some(state_value) = res.get("State") {
				if let Value::Object(ref state) = *state_value {
					if let Some(status_value) = state.get("Status") {
						if let Value::String(ref status) = *status_value {
							if status == "running" {
								return Ok(());
							}
						}
					}
				}
			}
		}

		Err(hyper::Error::Status)
	}

	pub fn new(core: &'a mut Core) -> Docker<'a> {
		let host = env::var("DOCKER_HOST")
			.unwrap_or_else(|_| "unix:///var/run/docker.sock".into());

		let transport = if host.starts_with("unix://") {
			Transport::new_unix(core, &host)
		} else if let Ok(ref certs) = env::var("DOCKER_CERT_PATH") {
			Transport::new_tls(core, &host, certs)
		} else {
			Transport::new_tcp(core, &host)
		};

		Docker { core, transport }
	}

	// TODO: Inspect container state before attempting to attach
	// TODO: Listen for SIGWINCH with `chan_signal`
	pub fn attach<F, T, I>(
		&mut self,
		id: &str,
		body: Body,
		cb: F,
	) -> Result<I, hyper::Error>
	where
		F: Fn(Response) -> T,
		T: Future<Item = I, Error = hyper::Error>,
	{
		// FIXME: Losing the error message
		if let Err(err) = self.ensure_running(id) {
			return Err(err);
		}

		// TODO: Take query params as arguments
		let uri = self.transport.uri(&format!(
			"/containers/{}/attach?stream=1&stdin=1&stdout=1&stderr=1",
			id
		))?;

		let mut req = Request::new(Method::Post, uri);

		req.set_version(HttpVersion::Http10);
		req.set_body(body);

		let work = self.transport.request(req).and_then(cb);

		self.core.run(work)
	}

	pub fn info(&mut self) -> Result<Value, hyper::Error> {
		let uri = self.transport.uri("/info").unwrap();
		let work = self.transport.get(uri).and_then(|res| {
			res.body().concat2().and_then(move |body| {
				let v = serde_json::from_slice(&body).unwrap();

				Ok(v)
			})
		});

		self.core.run(work)
	}

	pub fn inspect(&mut self, id: &str) -> Result<Value, hyper::Error> {
		let uri = self.transport
			.uri(&format!("/containers/{}/json", id))
			.unwrap();

		let work = self.transport.get(uri).and_then(|res| {
			res.body().concat2().and_then(move |body| {
				let v = serde_json::from_slice(&body).unwrap();

				Ok(v)
			})
		});

		self.core.run(work)
	}

	// TODO: Create unit types for Height / Width to make this less error prone
	pub fn resize(
		&mut self,
		id: &str,
		width: usize,
		height: usize,
	) -> Result<(), hyper::Error> {
		let uri = self.transport.uri(&format!(
			"/containers/{}/resize?w={}&h={}",
			id, width, height,
		))?;

		let req = Request::new(Method::Post, uri);
		let work = self.transport.request(req).and_then(|_| Ok(()));

		self.core.run(work)
	}
}
