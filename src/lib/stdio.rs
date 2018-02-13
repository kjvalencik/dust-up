use std::io::{self, Read};
use std::thread;

use hyper::{self, Body, Chunk};

use futures::stream::{iter_result};
use futures::{Future, Sink, Stream};
use futures::sync::mpsc::{self, UnboundedReceiver};

use tokio_core::reactor::Handle;

use tokio_signal::unix::Signal;
use tokio_signal::unix::libc::SIGWINCH;


use super::super::errors::Error;

pub fn stdin_stream() -> UnboundedReceiver<u8> {
	let (channel_sink, channel_stream) = mpsc::unbounded();
	let stdin_sink = channel_sink.sink_map_err(Error::from);

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
			.map_err(Error::from)
			.forward(stdin_sink)
			.wait()
			.expect("Stdin stream failed");
	});

	channel_stream
}

pub fn stdin_body() -> (Body, Box<Future<Item = (), Error = Error>>) {
	let stdin = stdin_stream()
		.map(|byte| Ok(Chunk::from(vec![byte])))
		.map_err(|_| unreachable!());

	let (tx, body) = hyper::Body::pair();

	let work = tx.send_all(stdin).map(|_| ()).map_err(Error::from);

	(body, Box::new(work))
}

pub fn sigwinch_stream(handle: &Handle) -> Box<Stream<Item = i32, Error = Error>> {
	let stream = Signal::new(SIGWINCH, handle)
		.flatten_stream()
		.map_err(Error::from);

	Box::new(stream)
}
