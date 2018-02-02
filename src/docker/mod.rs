use std::env;

use futures::{Future, Stream};

use hyper::{self, Body, HttpVersion, Method, Request, Response};

use serde_json::{self, Value};

use tokio_core::reactor::Core;

use self::transport::Transport;

mod transport;

type DockerResponse<T> = Box<Future<Item = T, Error = hyper::Error>>;

// TODO: Consider `impl trait` instead of boxes
pub struct Docker {
	transport: Transport,
}

impl Docker {
	fn ensure_running(&self, id: &str) -> DockerResponse<()> {
		let work = self.inspect(id).and_then(|res_value| {
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
		});

		Box::new(work)
	}

	pub fn new(core: &Core) -> Docker {
		let host = env::var("DOCKER_HOST")
			.unwrap_or_else(|_| "unix:///var/run/docker.sock".into());

		let transport = if host.starts_with("unix://") {
			Transport::new_unix(core, &host)
		} else if let Ok(ref certs) = env::var("DOCKER_CERT_PATH") {
			Transport::new_tls(core, &host, certs)
		} else {
			Transport::new_tcp(core, &host)
		};

		Docker { transport }
	}

	// TODO: Inspect container state before attempting to attach
	// TODO: Listen for SIGWINCH with `chan_signal`
	pub fn attach(&self, id: &str, body: Body) -> DockerResponse<Response> {
		// TODO: Take query params as arguments
		let uri = self.transport
			.uri(&path_format!(
				"/containers/{}/attach?stream=1&stdin=1&stdout=1&stderr=1",
				id
			))
			.unwrap();

		let mut req = Request::new(Method::Post, uri);

		// HACK: Horrible hack to force hyper into `eof` encoding mode
		req.set_version(HttpVersion::Http10);
		req.set_body(body);

		let attach = self.transport.request(req);
		let work = self.ensure_running(id).then(|_| attach);

		Box::new(work)
	}

	pub fn info(&self) -> DockerResponse<serde_json::Value> {
		let uri = self.transport.uri("/info").unwrap();

		Box::new(self.transport.get(uri).and_then(|res| {
			res.body().concat2().and_then(move |body| {
				let v = serde_json::from_slice(&body).unwrap();

				Ok(v)
			})
		}))
	}

	pub fn inspect(&self, id: &str) -> DockerResponse<serde_json::Value> {
		let uri = self.transport
			.uri(&path_format!("/containers/{}/json", id))
			.unwrap();

		Box::new(self.transport.get(uri).and_then(|res| {
			res.body().concat2().and_then(move |body| {
				let v = serde_json::from_slice(&body).unwrap();

				Ok(v)
			})
		}))
	}

	// TODO: Create unit types for Height / Width to make this less error prone
	pub fn resize(
		&self,
		id: &str,
		width: usize,
		height: usize,
	) -> DockerResponse<()> {
		let uri = self.transport
			.uri(&format!(
				"{}?w={}&h={}",
				path_format!("/containers/{}/resize", id),
				width,
				height,
			))
			.unwrap();

		let req = Request::new(Method::Post, uri);

		Box::new(self.transport.request(req).and_then(|_| Ok(())))
	}
}
