use std::env;

use futures::{Future, Stream};
use futures::future::{self, FutureResult};

use hyper::{self, Body, HttpVersion, Method, Request, Response};

use serde_json::{self, Value};

use tokio_core::reactor::Core;

use self::transport::Transport;
use super::errors::{Error, Result};

mod transport;

// TODO: Consider `impl trait` instead of boxes
type DockerResponse<'a, T> = Box<Future<Item = T, Error = Error> + 'a>;

fn future_try<F, T>(f: F) -> FutureResult<T, Error>
where
	F: Fn() -> Result<T>,
{
	future::result(f())
}

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

			Err(hyper::Error::Status.into())
		});

		Box::new(work)
	}

	pub fn new(core: &Core) -> Result<Docker> {
		let host = env::var("DOCKER_HOST")
			.unwrap_or_else(|_| "unix:///var/run/docker.sock".into());

		let transport = if host.starts_with("unix://") {
			Transport::new_unix(core, &host)
		} else if let Ok(ref certs) = env::var("DOCKER_CERT_PATH") {
			Transport::new_tls(core, &host, certs)
		} else {
			Transport::new_tcp(core, &host)
		}?;

		Ok(Docker { transport })
	}

	// TODO: Inspect container state before attempting to attach
	// TODO: Listen for SIGWINCH with `chan_signal`
	pub fn attach<'a>(
		&'a self,
		id: &'a str,
		body: Body,
	) -> DockerResponse<Response> {
		// TODO: Take query params as arguments

		let work = future_try(|| {
			let uri = self.transport.uri(&path_format!(
				"/containers/{}/attach?stream=1&stdin=1&stdout=1&stderr=1",
				id
			))?;

			Ok(uri)
		}).and_then(move |uri| {
			let mut req = Request::new(Method::Post, uri);

			// HACK: Horrible hack to force hyper into `eof` encoding mode
			req.set_version(HttpVersion::Http10);
			req.set_body(body);

			let attach = self.transport.request(req).map_err(Error::from);

			self.ensure_running(id).and_then(|_| attach)
		});

		Box::new(work)
	}

	pub fn info(&self) -> DockerResponse<serde_json::Value> {
		// TODO: This code is repeated, make a method for it
		let work = future_try(|| {
			let uri = self.transport.uri("/info")?;

			Ok(uri)
		}).and_then(move |uri| {
			self.transport
				.get(uri)
				.map_err(Error::from)
				.and_then(|res| {
					res.body().concat2().map_err(Error::from).and_then(
						move |body| {
							future::result(serde_json::from_slice(&body))
								.map_err(Error::from)
						},
					)
				})
		});

		Box::new(work)
	}

	pub fn inspect(&self, id: &str) -> DockerResponse<serde_json::Value> {
		let work = future_try(|| {
			let uri = self.transport
				.uri(&path_format!("/containers/{}/json", id))?;

			Ok(uri)
		}).and_then(move |uri| {
			self.transport
				.get(uri)
				.map_err(Error::from)
				.and_then(|res| {
					res.body().concat2().map_err(Error::from).and_then(
						move |body| {
							future::result(serde_json::from_slice(&body))
								.map_err(Error::from)
						},
					)
				})
		});

		Box::new(work)
	}

	// TODO: Create unit types for Height / Width to make this less error prone
	pub fn resize<'a>(
		&'a self,
		id: &'a str,
		width: usize,
		height: usize,
	) -> DockerResponse<()> {
		let work = future_try(|| {
			let uri = self.transport.uri(&format!(
				"{}?w={}&h={}",
				path_format!("/containers/{}/resize", id),
				width,
				height,
			))?;

			Ok(uri)
		}).and_then(move |uri| {
			let req = Request::new(Method::Post, uri);

			self.transport
				.request(req)
				.and_then(|_| Ok(()))
				.map_err(Error::from)
		});

		Box::new(work)
	}
}
