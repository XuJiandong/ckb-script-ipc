use crate::error::IpcError;
use serde::{Deserialize, Serialize};

pub trait Serve {
    /// Type of request.
    type Req: Serialize + for<'de> Deserialize<'de>;

    /// Type of response.
    type Resp: Serialize + for<'de> Deserialize<'de>;

    /// Responds to a single request.
    fn serve(&mut self, req: Self::Req) -> Result<Self::Resp, IpcError>;

    /// Extracts a method name from the request.
    fn method(&self, _request: &Self::Req) -> Option<&'static str> {
        None
    }
}
