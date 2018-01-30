extern crate futures;
extern crate hyper;
extern crate hyper_openssl;
extern crate hyperlocal;
extern crate openssl;
extern crate serde_json;
extern crate serde_yaml;
extern crate tokio_core;

mod docker;

use docker::Docker;

fn main() {
	let mut docker = Docker::new();

	docker.info().expect("Expected info");
}
