use serde::{Deserialize, Serialize};
use crate::error::IpcError;

pub trait Serve {
    /// Type of request.
    type Req: Serialize;

    /// Type of response.
    type Resp: Deserialize;

    /// Responds to a single request.
    fn serve(self, req: Self::Req) -> Result<Self::Resp, IpcError>;

    /// Extracts a method name from the request.
    fn method(&self, _request: &Self::Req) -> Option<&' str> {
        None
    }
}

