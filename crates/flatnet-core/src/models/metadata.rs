use bytemuck::{Pod, Zeroable};

use crate::models::{Date, Optional, Sentinel, Settings, StringSlice};

/// What the file says about itself: where it came from, when it was made and when it is good for.
///
/// The text fields are slices of the `strings` table. There is exactly one `Metadata` in a file.
/// Anything beyond these fields is in the JSON section, which the consumer hands back as text.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Metadata {
    /// When the file was compiled, in seconds since the unix epoch, or 0 if the compiler was not
    /// told. The compiler never reads the clock itself, so that one feed gives one file.
    pub compiled_at: u64,

    /// The first day any service runs, or `Date::NONE` if the feed has no services.
    pub active_from: Date,

    /// The last day any service runs, or `Date::NONE` if the feed has no services.
    pub active_until: Date,

    /// The version of the compiler that wrote the file.
    pub compiler_version: Optional<StringSlice>,

    /// The name of the feed the file was compiled from, such as `sl.zip`.
    pub source: Optional<StringSlice>,

    /// `feed_publisher_name` from `feed_info.txt`.
    pub publisher_name: Optional<StringSlice>,

    /// `feed_publisher_url` from `feed_info.txt`.
    pub publisher_url: Optional<StringSlice>,

    /// `feed_lang` from `feed_info.txt`.
    pub language: Optional<StringSlice>,

    /// `feed_version` from `feed_info.txt`.
    pub feed_version: Optional<StringSlice>,

    /// `feed_contact_email` from `feed_info.txt`.
    pub contact_email: Optional<StringSlice>,

    /// `agency_name` of the first agency in `agency.txt`.
    pub agency_name: Optional<StringSlice>,

    /// `agency_url` of the first agency in `agency.txt`.
    pub agency_url: Optional<StringSlice>,

    /// The choices the compiler was given, such as how far a footpath reaches.
    pub settings: Settings,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            compiled_at: 0,
            active_from: Date::NONE,
            active_until: Date::NONE,
            compiler_version: Optional::default(),
            source: Optional::default(),
            publisher_name: Optional::default(),
            publisher_url: Optional::default(),
            language: Optional::default(),
            feed_version: Optional::default(),
            contact_email: Optional::default(),
            agency_name: Optional::default(),
            agency_url: Optional::default(),
            settings: Settings::default(),
        }
    }
}

impl Metadata {
    /// Every text field, to check or list them without naming each one.
    #[must_use]
    pub const fn texts(&self) -> [(&'static str, Optional<StringSlice>); 9] {
        [
            ("compiler_version", self.compiler_version),
            ("source", self.source),
            ("publisher_name", self.publisher_name),
            ("publisher_url", self.publisher_url),
            ("language", self.language),
            ("feed_version", self.feed_version),
            ("contact_email", self.contact_email),
            ("agency_name", self.agency_name),
            ("agency_url", self.agency_url),
        ]
    }
}
