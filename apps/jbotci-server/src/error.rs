//! Stable error codes and messages shared by public adapters.
use bityzba::{invariant, requires};

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServerErrorCode {
    Busy,
    Deadline,
    Failed,
    OutputLimit,
    OperationLimit,
}
impl ServerErrorCode {
    pub(crate) const ALL: [Self; 5] = [
        Self::Busy,
        Self::Deadline,
        Self::Failed,
        Self::OutputLimit,
        Self::OperationLimit,
    ];
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Busy => "server_busy",
            Self::Deadline => "work_deadline_exceeded",
            Self::Failed => "work_failed",
            Self::OutputLimit => "output_limit_exceeded",
            Self::OperationLimit => "operation_limit_exceeded",
        }
    }
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::Busy => "The server is busy. Try again later.",
            Self::Deadline => "The operation did not finish before its deadline.",
            Self::Failed => "The operation failed.",
            Self::OutputLimit => "The generated output exceeds the server limit.",
            Self::OperationLimit => "The requested operation exceeds the server limit.",
        }
    }
    #[requires(true)]
    #[ensures(ret.is_client_error() || ret.is_server_error())]
    pub(crate) fn status(self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            Self::Busy => StatusCode::SERVICE_UNAVAILABLE,
            Self::Deadline => StatusCode::GATEWAY_TIMEOUT,
            Self::Failed => StatusCode::INTERNAL_SERVER_ERROR,
            Self::OutputLimit | Self::OperationLimit => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}
impl From<&crate::work::WorkError> for ServerErrorCode {
    #[requires(true)]
    #[ensures(true)]
    fn from(error: &crate::work::WorkError) -> Self {
        use crate::work::WorkError;
        match error {
            WorkError::OutputLimit => Self::OutputLimit,
            WorkError::Overloaded { .. } | WorkError::WaitTimedOut { .. } => Self::Busy,
            WorkError::TimedOut { .. } => Self::Deadline,
            WorkError::Panicked { .. } => Self::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn work_failures_use_one_public_code_and_message_across_lanes() {
        use crate::work::{WorkError, WorkLane};
        for lane in [WorkLane::Compute, WorkLane::Fetch, WorkLane::Embedding] {
            for error in [
                WorkError::Overloaded { lane },
                WorkError::WaitTimedOut { lane },
            ] {
                let code = ServerErrorCode::from(&error);
                assert_eq!(code, ServerErrorCode::Busy);
                assert_eq!(code.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
                assert_eq!(error.to_string(), "The server is busy. Try again later.");
            }
            assert_eq!(
                ServerErrorCode::from(&WorkError::TimedOut { lane }),
                ServerErrorCode::Deadline
            );
            assert_eq!(
                ServerErrorCode::from(&WorkError::Panicked { lane }),
                ServerErrorCode::Failed
            );
        }
    }
}
