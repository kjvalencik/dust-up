use hyper::{Client, Request, Uri};
use hyper::client::{FutureResponse, HttpConnector};

use hyper_openssl::HttpsConnector;

use hyperlocal::{self, UnixConnector};

use openssl::ssl::{SslConnector, SslFiletype, SslMethod};

use tokio_core::reactor::Core;

use super::super::errors::Result;

pub struct Tcp {
	host: String,
	client: Client<HttpConnector>,
}

pub struct Tls {
	host: String,
	client: Client<HttpsConnector<HttpConnector>>,
}

pub struct Unix {
	file: String,
	client: Client<UnixConnector>,
}

pub enum Transport {
	Tcp(Tcp),
	Tls(Tls),
	Unix(Unix),
}

impl From<Tcp> for Transport {
	fn from(tcp: Tcp) -> Self {
		Transport::Tcp(tcp)
	}
}

impl From<Tls> for Transport {
	fn from(tls: Tls) -> Self {
		Transport::Tls(tls)
	}
}

impl From<Unix> for Transport {
	fn from(unix: Unix) -> Self {
		Transport::Unix(unix)
	}
}

impl Transport {
	pub fn get(&self, uri: Uri) -> FutureResponse {
		match *self {
			Transport::Tcp(ref t) => t.client.get(uri),
			Transport::Tls(ref t) => t.client.get(uri),
			Transport::Unix(ref t) => t.client.get(uri),
		}
	}

	pub fn request(&self, req: Request) -> FutureResponse {
		match *self {
			Transport::Tcp(ref t) => t.client.request(req),
			Transport::Tls(ref t) => t.client.request(req),
			Transport::Unix(ref t) => t.client.request(req),
		}
	}

	pub fn uri(&self, path: &str) -> Result<Uri> {
		let uri = match *self {
			Transport::Tcp(ref t) => format!("{}{}", &t.host, path).parse(),
			Transport::Tls(ref t) => format!("{}{}", &t.host, path).parse(),
			Transport::Unix(ref t) => {
				Ok(hyperlocal::Uri::new(&t.file, path).into())
			}
		};

		Ok(uri?)
	}

	pub fn new_unix(core: &Core, host: &str) -> Result<Transport> {
		let client = Client::configure()
			.connector(UnixConnector::new(core.handle()))
			.build(&core.handle());

		Ok(Unix {
			client,
			file: host.chars().skip(7).collect(),
		}.into())
	}

	pub fn new_tls(core: &Core, host: &str, certs: &str) -> Result<Transport> {
		let mut builder = SslConnector::builder(SslMethod::tls())?;

		builder.set_certificate_file(
			format!("{}/cert.pem", certs),
			SslFiletype::PEM,
		)?;

		builder.set_private_key_file(
			format!("{}/key.pem", certs),
			SslFiletype::PEM,
		)?;

		builder.set_ca_file(format!("{}/ca.pem", certs))?;

		let mut http = HttpConnector::new(4, &core.handle());

		http.enforce_http(false);

		let connector = HttpsConnector::with_connector(http, builder.build());

		let client = Client::configure()
			.connector(connector)
			.build(&core.handle());

		Ok(Tls {
			client,
			host: host.into(),
		}.into())
	}

	pub fn new_tcp(core: &Core, host: &str) -> Result<Transport> {
		let client = Client::new(&core.handle());

		Ok(Tcp {
			client,
			host: host.into(),
		}.into())
	}
}
