use crate::error::IpcError;
use serde::{Deserialize, Serialize};

/// The `Serve` trait defines the interface for handling requests and generating responses in an IPC context.
/// Types implementing this trait can be used to process incoming requests and produce appropriate responses.
///
/// # Associated Types
///
/// * `Req` - The type of the request messages. It must implement `Serialize` and `Deserialize`.
/// * `Resp` - The type of the response messages. It must implement `Serialize` and `Deserialize`.
///
/// # Required Methods
///
/// * `serve` - This method is responsible for processing a single request and generating a response.
/// * `method` - This method extracts a method name from the request, if applicable. It returns an `Option` containing a static string slice representing the method name.
///
/// # Example
///
/// ```rust,ignore
/// use ckb_script_ipc_common::ipc::Serve;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct MyRequest {
///     // request fields
/// }
///
/// #[derive(Serialize, Deserialize)]
/// struct MyResponse {
///     // response fields
/// }
///
/// struct MyService;
///
/// impl Serve for MyService {
///     type Req = MyRequest;
///     type Resp = MyResponse;
///
///     fn serve(&mut self, req: Self::Req) -> Result<Self::Resp, IpcError> {
///         // process the request and generate a response
///         Ok(MyResponse { /* fields */ })
///     }
///
///     fn method(&self, _request: &Self::Req) -> Option<&'static str> {
///         Some("my_method")
///     }
/// }
/// ```
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
