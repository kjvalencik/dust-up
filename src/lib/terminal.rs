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
		let prev_ios = PrevTerminal { fd, prev_ios: ios };

		cfmakeraw(&mut ios);
		tcsetattr(fd, termios::TCSADRAIN, &ios).expect("set attr on ios");

		RawTerminal {
			_prev_ios: prev_ios,
		}
	}
}
