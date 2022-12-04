pub trait SearchItem {
    fn as_str(&self) -> &str;
}

impl SearchItem for str {
    fn as_str(&self) -> &str {
        self
    }
}

impl SearchItem for String {
    fn as_str(&self) -> &str {
        self
    }
}

impl<'a> SearchItem for std::borrow::Cow<'a, str> {
    fn as_str(&self) -> &str {
        self
    }
}

impl<'a, S: SearchItem> SearchItem for &'a S {
    fn as_str(&self) -> &str {
        <_ as SearchItem>::as_str(*self)
    }
}
