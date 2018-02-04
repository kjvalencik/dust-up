error_chain! {
	foreign_links {
		Hyper(::hyper::Error);
		Io(::std::io::Error);
		Json(::serde_json::Error);
		Uri(::hyper::error::UriError);
	}
}
