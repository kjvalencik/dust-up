error_chain! {
	foreign_links {
		Hyper(::hyper::Error);
		Io(::std::io::Error);
		Json(::serde_json::Error);
		SendErrorU8(::futures::sync::mpsc::SendError<u8>);
		SendErrorChunk(::futures::sync::mpsc::SendError<
			::std::result::Result<::hyper::Chunk, ::hyper::Error>
		>);
		OpenSsl(::openssl::error::ErrorStack);
		Uri(::hyper::error::UriError);
	}
}
