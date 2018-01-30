use std::env;

use futures::Future;
use futures::Stream;

use hyper;

use serde_json::{self, Value};

use tokio_core::reactor::Core;

use self::transport::Transport;

mod transport;

pub struct Docker {
	transport: Transport,
	core: Core,
}

impl Docker {
	pub fn new() -> Docker {
		let core = Core::new().unwrap();
		let host = env::var("DOCKER_HOST")
			.unwrap_or_else(|_| "unix:///var/run/docker.sock".into());

		let transport = if host.starts_with("unix://") {
			Transport::new_unix(&core, &host)
		} else if let Ok(ref certs) = env::var("DOCKER_CERT_PATH") {
			Transport::new_tls(&core, &host, certs)
		} else {
			Transport::new_tcp(&core, &host)
		};

		Docker { core, transport }
	}

	pub fn info(&mut self) -> Result<Value, hyper::Error> {
		let uri = self.transport.uri("/info").unwrap();
		let work = self.transport.get(uri).and_then(|res| {
			res.body().concat2().and_then(move |body| {
				let v: Value = serde_json::from_slice(&body).unwrap();

				Ok(v)
			})
		});

		self.core.run(work)
	}
}
