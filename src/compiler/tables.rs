use std::collections::HashMap;

use rayon::slice::ParallelSliceMut;

use crate::models::{Index, Slice, SliceBuilder};

/// The id strings of one table, in the three forms the binary file stores them.
pub(super) struct IdTable<S, I> {
    /// Where each item's id sits in `buffer`, by item index.
    pub slices: Vec<S>,
    /// Item indices sorted by id, so the consumer can binary search.
    pub lookup: Vec<I>,
    pub buffer: String,
}

/// Builds the id table for `count` items from a map of id to index.
///
/// Ids are laid out in index order, so the same input always gives the same bytes. An item with no
/// entry in the map gets an empty id.
pub(super) fn build_id_table<S: Slice, I: Index + Send>(
    id_map: &HashMap<String, I>,
    count: usize,
) -> IdTable<S, I> {
    let mut ids = vec![""; count];
    for (id, index) in id_map {
        ids[index.as_usize()] = id;
    }

    let mut builder = SliceBuilder::with_capacity(ids.iter().map(|id| id.len()).sum());
    let slices = ids.iter().map(|id| builder.add(id)).collect();

    let mut lookup: Vec<I> = (0..count).map(I::from).collect();
    lookup.par_sort_unstable_by(|first, second| ids[first.as_usize()].cmp(ids[second.as_usize()]));

    IdTable {
        slices,
        lookup,
        buffer: builder.into_buffer(),
    }
}

/// Finds where each key's run sits in a sorted sequence of keys.
///
/// Returns one slice per key in `0..key_count`. A key that never appears gets `S::NONE`.
pub(super) fn slices_by_key<K: Index, S: Slice>(
    sorted_keys: impl Iterator<Item = K>,
    key_count: usize,
) -> Vec<S> {
    let mut slices = vec![S::NONE; key_count];
    let mut run_key: Option<K> = None;
    let mut run_start = 0;
    let mut total = 0;

    for (position, key) in sorted_keys.enumerate() {
        total = position + 1;
        if run_key == Some(key) {
            continue;
        }
        if let Some(finished) = run_key {
            slices[finished.as_usize()] = S::from_usize(run_start, position - run_start);
        }
        run_key = Some(key);
        run_start = position;
    }
    if let Some(finished) = run_key {
        slices[finished.as_usize()] = S::from_usize(run_start, total - run_start);
    }

    slices
}

#[cfg(test)]
mod tests {
    use std::iter;

    use super::*;
    use crate::models::{Sentinel, StopIdSlice, StopIndex, TripSlice};

    #[test]
    fn id_table_is_in_index_order_and_lookup_is_sorted() {
        let id_map = HashMap::from([
            ("b".to_owned(), StopIndex(0)),
            ("c".to_owned(), StopIndex(1)),
            ("a".to_owned(), StopIndex(2)),
        ]);

        let table = build_id_table::<StopIdSlice, StopIndex>(&id_map, 3);

        assert_eq!(table.buffer, "bca", "ids are laid out by item index");
        assert_eq!(
            table.lookup,
            vec![StopIndex(2), StopIndex(0), StopIndex(1)],
            "lookup is sorted by id"
        );
    }

    #[test]
    fn runs_become_slices_and_missing_keys_stay_none() {
        let keys = [StopIndex(0), StopIndex(0), StopIndex(2)];

        let slices: Vec<TripSlice> = slices_by_key(keys.into_iter(), 4);

        assert_eq!(
            slices[0],
            TripSlice::from_usize(0, 2),
            "key 0 has two items"
        );
        assert_eq!(slices[1], TripSlice::NONE, "key 1 never appears");
        assert_eq!(slices[2], TripSlice::from_usize(2, 1), "key 2 has one item");
        assert_eq!(slices[3], TripSlice::NONE, "key 3 never appears");
    }

    #[test]
    fn no_keys_gives_no_slices() {
        let slices: Vec<TripSlice> = slices_by_key(iter::empty::<StopIndex>(), 2);

        assert_eq!(slices, vec![TripSlice::NONE; 2], "every key is absent");
    }
}
