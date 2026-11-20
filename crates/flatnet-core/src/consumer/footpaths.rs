use crate::{
    consumer::Consumer,
    models::{Footpath, Place, Settings, Slice, StopIndex},
};

impl<'a> Consumer<'a> {
    /// Returns the place a stop belongs to, as the index of its root stop: the stop itself if it has
    /// no parent, otherwise the top of its parent chain. Footpaths run between places.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `StopIndex::NONE`.
    #[must_use]
    pub fn place_of(&self, index: StopIndex) -> StopIndex {
        Place::root_of(self.stops, index)
    }

    /// Returns the stops a passenger can board at in the place, in index order. It is empty for a
    /// stop that is not a place, and for every stop in a file without the footpaths part.
    ///
    /// # Panics
    ///
    /// Panics if `place` is out of range, which includes `StopIndex::NONE`.
    #[must_use]
    pub fn stops_by_place(&self, place: StopIndex) -> &'a [StopIndex] {
        assert!(
            place.as_usize() < self.stops.len(),
            "stop {place:?} is out of range"
        );
        // A file without the footpaths part has no places at all.
        let Some(place) = self.places.get(place.as_usize()) else {
            return &[];
        };
        &self.place_stops[place.stops.range()]
    }

    /// Returns the walks that start at the place, nearest first. Walking between the stops of one
    /// place is a given and is not listed. Empty for a stop that is not a place, and in a file
    /// without the footpaths part.
    ///
    /// # Panics
    ///
    /// Panics if `place` is out of range, which includes `StopIndex::NONE`.
    #[must_use]
    pub fn footpaths_by_place(&self, place: StopIndex) -> &'a [Footpath] {
        assert!(
            place.as_usize() < self.stops.len(),
            "stop {place:?} is out of range"
        );
        let Some(place) = self.places.get(place.as_usize()) else {
            return &[];
        };
        &self.footpaths[place.footpaths.range()]
    }

    /// Returns the walks from the place a stop belongs to, see `footpaths_by_place`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `StopIndex::NONE`.
    #[must_use]
    pub fn footpaths_by_stop(&self, index: StopIndex) -> &'a [Footpath] {
        self.footpaths_by_place(self.place_of(index))
    }

    /// Returns the choices the compiler was given: how far a footpath reaches and how they were
    /// picked. Read it to know what the tables mean, because the file is only as good as its
    /// settings.
    #[must_use]
    pub fn settings(&self) -> &'a Settings {
        &self.metadata().settings
    }
}
