use std::time::Duration;

#[derive(Debug)]
pub enum TcpConnectMode {
    OneShot,
    Poll(TcpPollOptions),
}

#[derive(Debug)]
pub struct TcpPollOptions {
    pub interval: Duration,
    pub abort_condition: PollAbortCondition,
}

impl TcpPollOptions {
    pub fn new(poll_interval: Duration, abort_condition: PollAbortCondition) -> Self {
        Self {
            interval: poll_interval,
            abort_condition,
        }
    }
}

#[derive(Debug)]
pub enum PollAbortCondition {
    Attempts(u32),
    Timeout(Duration),
}

impl TcpConnectMode {
    pub fn poll_from_ms<M>(ms: M, abort_condition: PollAbortCondition) -> Self
    where
        M: Into<u64>,
    {
        let opts = TcpPollOptions::new(Duration::from_millis(ms.into()), abort_condition);
        Self::Poll(opts)
    }
}
