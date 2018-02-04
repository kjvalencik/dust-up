use std;
use std::io::{self, Read};
use std::thread;

use hyper::{self, Body, Chunk};

use futures::stream::iter_result;
use futures::{Future, Sink, Stream};
use futures::sync::mpsc::{self, SendError, UnboundedReceiver};

#[derive(Debug)]
enum Error {
	Stdin(std::io::Error),
	Channel(SendError<u8>),
}

pub fn stdin_stream() -> UnboundedReceiver<u8> {
	let (channel_sink, channel_stream) = mpsc::unbounded();
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
			.expect("Stdin stream failed");
	});

	channel_stream
}

pub fn stdin_body() -> (Body, Box<Future<Item = (), Error = hyper::Error>>) {
	let stdin = stdin_stream()
		.map(|byte| Ok(Chunk::from(vec![byte])))
		.map_err(|_| unreachable!());

	let (tx, body) = hyper::Body::pair();

	// FIXME: Error is not actually unreachable
	let work = tx.send_all(stdin).map(|_| ()).map_err(|_| unreachable!());

	(body, Box::new(work))
}
