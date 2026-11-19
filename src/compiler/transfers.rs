use std::collections::HashMap;

use rayon::slice::ParallelSliceMut;

use crate::{
    compiler::{records::RawTransfer, tables},
    models::{Duration, Optional, StopIndex, Transfer, TransferIndex, TransferSlice, TripIndex},
};

/// The transfers and the two lookups from a stop to its transfers.
pub(super) struct BuiltTransfers {
    /// Sorted by the stop they start at.
    pub transfers: Vec<Transfer>,
    /// By stop: the run of `transfers` that starts there.
    pub outbound: Vec<TransferSlice>,
    /// Indices into `transfers`, sorted by the stop they end at.
    pub inbound_indices: Vec<TransferIndex>,
    /// By stop: the run of `inbound_indices` that ends there.
    pub inbound: Vec<TransferSlice>,
}

pub(super) fn build_transfers(
    raw_transfers: &[RawTransfer],
    stop_map: &HashMap<String, StopIndex>,
    trip_map: &HashMap<String, TripIndex>,
    stop_count: usize,
) -> BuiltTransfers {
    let mut transfers: Vec<_> = raw_transfers
        .iter()
        .filter_map(|transfer| {
            let from_stop_index = stop_map.get(&transfer.from_stop_id).copied()?;
            let to_stop_index = stop_map.get(&transfer.to_stop_id).copied()?;
            // A transfer that names a trip the feed doesn't have can never apply.
            let trip = |id: &Option<String>| match id {
                Some(id) => trip_map.get(id).map(|&index| Optional::new(index)),
                None => Some(Optional::default()),
            };
            Some(Transfer::new(
                from_stop_index,
                to_stop_index,
                trip(&transfer.from_trip_id)?,
                trip(&transfer.to_trip_id)?,
                transfer.min_transfer_time.map(Duration).into(),
                transfer.transfer_type,
            ))
        })
        .collect();

    transfers.par_sort_unstable_by_key(|transfer| transfer.from_stop_index);
    let outbound = tables::slices_by_key(
        transfers.iter().map(|transfer| transfer.from_stop_index),
        stop_count,
    );

    let mut inbound_indices: Vec<_> = (0..transfers.len()).map(TransferIndex::from).collect();
    inbound_indices.par_sort_unstable_by_key(|index| transfers[index.as_usize()].to_stop_index);
    let inbound = tables::slices_by_key(
        inbound_indices
            .iter()
            .map(|index| transfers[index.as_usize()].to_stop_index),
        stop_count,
    );

    BuiltTransfers {
        transfers,
        outbound,
        inbound_indices,
        inbound,
    }
}
