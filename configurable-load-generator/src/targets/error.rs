#[derive(Debug, thiserror::Error)]
pub(crate) enum ClientTargetsError {
    #[error("Client thread failure threshold exceeded with {0} failures")]
    FailureThresholdExceeded(usize),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ClientTargetError {
    #[error(
        "Request failed. status: {status}, timeout: {timeout}, request: {request}, connection: {connection}"
    )]
    RequestFailure {
        status: String,
        timeout: bool,
        request: bool,
        connection: bool,
    },
}
