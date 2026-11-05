//! Construction of the real-time core and its control-side ends. All allocation of
//! buffers, rings and shared state happens here, before the callback runs (the `rt`
//! directory only uses what is passed in).

use crate::rt::{MAX_BLOCK, Retired, Rt, RtCommand, RtParts};
use crate::status::Status;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

/// Ring capacities. Commands are rare (a few per second at most); the return ring holds
/// objects waiting to be freed.
const COMMAND_RING: usize = 64;
const RETURN_RING: usize = 64;

/// What the control thread keeps of an engine: where to send commands and where retired
/// objects come back.
#[derive(Debug)]
pub struct ControlEnds {
    pub cmd: Producer<RtCommand>,
    pub ret: Consumer<Retired>,
    pub status: Arc<Status>,
}

/// A new callback core and its control ends, for `sample_rate`. `status` outlives the
/// core: when the control thread rebuilds the stream after a device change, the counters
/// and the position continue.
pub fn build_rt(sample_rate: u32, status: Arc<Status>) -> (Rt, ControlEnds) {
    let (cmd_tx, cmd_rx) = RingBuffer::new(COMMAND_RING);
    let (ret_tx, ret_rx) = RingBuffer::new(RETURN_RING);
    status.sample_rate.store(sample_rate as u64, Relaxed);
    let rt = Rt::new(RtParts {
        sample_rate,
        left: vec![0.0; MAX_BLOCK].into_boxed_slice(),
        right: vec![0.0; MAX_BLOCK].into_boxed_slice(),
        cmd: cmd_rx,
        ret: ret_tx,
        status: status.clone(),
    });
    (
        rt,
        ControlEnds {
            cmd: cmd_tx,
            ret: ret_rx,
            status,
        },
    )
}
