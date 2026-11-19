use gtfs_rt::FeedMessage;
use prost::Message;

use crate::rt::Result;

/// Decodes the bytes of a GTFS-realtime protobuf feed, as publishers serve it.
///
/// # Errors
///
/// Returns an error if the bytes aren't a feed. The bytes must already be decompressed.
pub fn decode_feed(bytes: &[u8]) -> Result<FeedMessage> {
    Ok(FeedMessage::decode(bytes)?)
}

#[cfg(test)]
mod tests {
    use gtfs_rt::FeedHeader;

    use super::*;

    #[test]
    fn a_feed_round_trips() {
        let feed = FeedMessage {
            header: FeedHeader {
                gtfs_realtime_version: "2.0".to_string(),
                timestamp: Some(42),
                ..Default::default()
            },
            entity: vec![],
        };

        let decoded = decode_feed(&feed.encode_to_vec()).expect("a valid feed");

        assert_eq!(decoded.header.timestamp, Some(42), "the header survives");
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(
            decode_feed(&[0xff, 0xff, 0xff, 0xff]).is_err(),
            "not protobuf"
        );
    }
}
