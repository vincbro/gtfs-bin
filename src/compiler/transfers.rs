use std::collections::HashMap;

use rayon::slice::ParallelSliceMut;

use crate::models::{Duration, Sentinel, Slice, StopIdx, Transfer, TransferIdx, TransferSlice};

pub fn build_transfers(
    raw_transfers: &[gtfs_structures::RawTransfer],
    stop_map: &HashMap<String, StopIdx>,
) -> (
    Vec<Transfer>,
    Vec<TransferSlice>,
    Vec<TransferIdx>,
    Vec<TransferSlice>,
) {
    let mut transfers: Vec<_> = raw_transfers
        .iter()
        .filter_map(|transfer| {
            stop_map
                .get(&transfer.from_stop_id)
                .copied()
                .and_then(|from_stop_idx| {
                    stop_map
                        .get(&transfer.to_stop_id)
                        .copied()
                        .map(|to_stop_idx| (transfer, from_stop_idx, to_stop_idx))
                })
        })
        .map(|(transfer, from_stop_idx, to_stop_idx)| {
            Transfer::new(
                from_stop_idx,
                to_stop_idx,
                transfer.min_transfer_time.map(Duration).into(),
                transfer.transfer_type as u8,
            )
        })
        .collect();

    // Outbound
    transfers.par_sort_unstable_by_key(|t| t.from_stop_idx);
    let mut stop_to_transfers_out = vec![TransferSlice::NONE; stop_map.len()];
    let mut current_stop = StopIdx::NONE;
    let mut start = 0;
    let mut count = 0;
    for (i, transfer) in transfers.iter().enumerate() {
        if transfer.from_stop_idx != current_stop {
            if current_stop != StopIdx::NONE {
                stop_to_transfers_out[current_stop.as_usize()] =
                    TransferSlice::from_usize(start, count);
            }
            start = i;
            count = 0;
            current_stop = transfer.from_stop_idx;
        }
        count += 1;
    }
    if current_stop != StopIdx::NONE {
        stop_to_transfers_out[current_stop.as_usize()] = TransferSlice::from_usize(start, count);
    }

    // Inbound
    let mut transfers_in_indencies: Vec<_> = (0..transfers.len()).map(TransferIdx::from).collect();
    transfers_in_indencies.par_sort_unstable_by_key(|idx| transfers[idx.as_usize()].to_stop_idx);
    let mut stop_to_transfers_in = vec![TransferSlice::NONE; stop_map.len()];
    let mut current_stop = StopIdx::NONE;
    let mut start = 0;
    let mut count = 0;
    for (i, transfer_idx) in transfers_in_indencies.iter().enumerate() {
        let transfer = &transfers[transfer_idx.as_usize()];
        if transfer.to_stop_idx != current_stop {
            if current_stop != StopIdx::NONE {
                stop_to_transfers_in[current_stop.as_usize()] =
                    TransferSlice::from_usize(start, count);
            }
            start = i;
            count = 0;
            current_stop = transfer.to_stop_idx;
        }
        count += 1;
    }
    if current_stop != StopIdx::NONE {
        stop_to_transfers_in[current_stop.as_usize()] = TransferSlice::from_usize(start, count);
    }

    (
        transfers,
        stop_to_transfers_out,
        transfers_in_indencies,
        stop_to_transfers_in,
    )
}
