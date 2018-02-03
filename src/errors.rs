error_chain! {
	foreign_links {
		Hyper(::hyper::Error);
		Io(::std::io::Error);
	}
}
