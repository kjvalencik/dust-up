error_chain! {
	foreign_links {
		Hyper(::hyper::Error);
		Io(::std::io::Error);
		Json(::serde_json::Error);
		OpenSsl(::openssl::error::ErrorStack);
		Uri(::hyper::error::UriError);
	}
}
