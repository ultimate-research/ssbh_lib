use super::*;
use thiserror::Error;

/// Errors while creating an [Anim] from [AnimData].
#[derive(Debug, Error)]
pub enum Error {
    /// Creating an [Anim] file for the given version is not supported.
    #[error(
        "creating a version {}.{} anim is not supported",
        major_version,
        minor_version
    )]
    UnsupportedVersion {
        major_version: u16,
        minor_version: u16,
    },

    /// The final frame index is negative or smaller than the
    // index of the final frame in the longest track.
    #[error(
        "final frame index {} must be non negative and at least as 
             large as the index of the final frame in the longest track",
        final_frame_index
    )]
    InvalidFinalFrameIndex { final_frame_index: f32 },

    /// An error occurred while writing data to a buffer.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// An error occurred while reading data from a buffer.
    #[error(transparent)]
    BinRead(#[from] binrw::error::Error),

    /// An error occurred while reading compressed data from a buffer.
    #[error(transparent)]
    BitError(#[from] bitutils::BitReadError),

    #[error(
        "compressed header bits per entry of {} does not match expected value of {}",
        actual,
        expected
    )]
    UnexpectedBitCount { expected: usize, actual: usize },

    #[error(
        "track data range {0}..{0}+{1} is out of range for a buffer of size {2}",
        start,
        size,
        buffer_size
    )]
    InvalidTrackDataRange {
        start: usize,
        size: usize,
        buffer_size: usize,
    },

    /// The buffer index is not valid for a version 1.2 anim file.
    #[error(
        "buffer index {} is out of range for a buffer collection of size {}",
        buffer_index,
        buffer_count
    )]
    BufferIndexOutOfRange {
        buffer_index: usize,
        buffer_count: usize,
    },

    #[error("the provided animation data is malformed or incomplete")]
    InvalidData,

    /// An error occurred while reading the compressed header for version 2.0 or later.
    #[error("the track data compression header is malformed and cannot be read")]
    MalformedCompressionHeader,
}
