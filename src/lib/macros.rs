macro_rules! path_format {
	($fmt: expr $(, $arg: expr)+,) => (path_format!($fmt $(, $arg)+));
	($fmt: expr $(, $arg: expr)+) => (format!(
		$fmt
		$(, ::url::percent_encoding::percent_encode(
			$arg.as_bytes(),
			::url::percent_encoding::DEFAULT_ENCODE_SET,
		))+
	))
}
