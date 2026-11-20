use crate::{
    consumer::Consumer,
    models::{Date, Metadata, Sentinel},
};

impl<'a> Consumer<'a> {
    /// Returns what the file says about itself.
    ///
    /// # Panics
    ///
    /// Panics if the file has no metadata, which `Consumer::new` rules out and a file opened with
    /// `new_unvalidated` may have if it is damaged.
    #[must_use]
    pub fn metadata(&self) -> &'a Metadata {
        self.metadata
            .first()
            .expect("a file's metadata section is checked to hold one item when it is opened")
    }

    /// Returns the JSON object the compiler was given beyond the fixed fields, as text for the
    /// caller to parse, or `None` if it was given none. It is valid UTF-8, and nothing more is
    /// promised about it: the file was written by a compiler that checked it, but a reader still
    /// parses it as it would any input.
    #[must_use]
    pub fn metadata_json(&self) -> Option<&'a str> {
        let json = super::text_of(self.metadata_json);
        (!json.is_empty()).then_some(json)
    }

    /// Returns the first and last day any service runs, or `None` if there are no services. Unlike
    /// the feed's own dates, it can't be left out of the feed.
    #[must_use]
    pub fn service_range(&self) -> Option<(Date, Date)> {
        let metadata = self.metadata();
        Some((
            metadata.active_from.as_option()?,
            metadata.active_until.as_option()?,
        ))
    }

    /// Returns whether no service runs on `today` or after it, so the file is out of date.
    ///
    /// It compares two days stored in the header, so it can be asked on every launch. `today` is a
    /// day in the agency's time zone, see `timezone`. A file with no services is always stale.
    #[must_use]
    pub fn is_stale(&self, today: Date) -> bool {
        self.service_range().is_none_or(|(_, last)| today > last)
    }
}
