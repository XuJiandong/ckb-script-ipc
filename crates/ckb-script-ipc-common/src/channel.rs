use crate::io::{BufReader, BufWriter, Read, Write};
use crate::{
    error::{IpcError, ProtocolErrorCode},
    ipc::Serve,
    packet::{Packet, RequestPacket, ResponsePacket},
};
use alloc::string::String;
use alloc::vec;
use serde::{Deserialize, Serialize};
use serde_json::{from_slice, to_vec};

/// The `Channel` struct facilitates communication between a client and a server.
/// It handles the transmission of requests from the client to the server and the reception
/// of responses from the server to the client. Communication is achieved through the use of pipes.
///
/// # Fields
///
/// * `reader` - Responsible for reading data from the channel.
/// * `writer` - Responsible for writing data to the channel.
pub struct Channel<R: Read, W: Write> {
    reader: BufReader<R>,
    writer: BufWriter<W>,
}

impl<R: Read, W: Write> Channel<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer: BufWriter::new(writer),
        }
    }
}

impl<R: Read, W: Write> Channel<R, W> {
    /// Executes the server loop, processing incoming requests and sending responses.
    ///
    /// This function runs an infinite loop, continuously receiving requests from the client,
    /// processing them using the provided service implementation, and sending back the responses.
    /// If an error occurs during the processing of a request or the sending of a response, the
    /// error is logged (if logging is enabled) and the error code is sent back to the client.
    ///
    /// # Arguments
    ///
    /// * `serve` - A mutable reference to the service implementation that handles the requests and
    ///   generates the responses. The service must implement the `Serve` trait with the appropriate
    ///   request and response types.
    ///
    /// # Type Parameters
    ///
    /// * `Req` - The type of the request messages. It must implement `Serialize` and `Deserialize`.
    /// * `Resp` - The type of the response messages. It must implement `Serialize` and `Deserialize`.
    /// * `S` - The type of the service implementation. It must implement the `Serve` trait with
    ///   `Req` as the request type and `Resp` as the response type.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the server execution. If the server runs
    /// successfully, it never returns. If an error occurs, it returns an `IpcError`.
    pub fn execute<Req, Resp, S>(mut self, serve: &mut S) -> Result<(), IpcError>
    where
        Req: Serialize + for<'de> Deserialize<'de>,
        Resp: Serialize + for<'de> Deserialize<'de>,
        S: Serve<Req = Req, Resp = Resp>,
    {
        loop {
            let result = self
                .receive_request()
                .and_then(|req| serve.serve(req))
                .and_then(|resp| self.send_response(resp));

            match result {
                Ok(_) => continue,
                Err(e) => {
                    #[cfg(feature = "enable-logging")]
                    log::error!("Error in execute loop: {:?}", e);
                    // notify client
                    self.send_error_code(e.clone().into()).unwrap();
                    return Err(e);
                }
            }
        }
    }
    ///
    /// Sends a request to the server and waits for a response.
    ///
    /// This function serializes the request, sends it to the server, and then waits for the server's response.
    /// It returns the deserialized response or an `IpcError` if an error occurs during the process.
    ///
    /// # Arguments
    ///
    /// * `_method_name` - A static string slice representing the name of the method being called.
    /// * `req` - The request message to be sent to the server. It must implement `Serialize` and `Deserialize`.
    ///
    /// # Type Parameters
    ///
    /// * `Req` - The type of the request message. It must implement `Serialize` and `Deserialize`.
    /// * `Resp` - The type of the response message. It must implement `Serialize` and `Deserialize`.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized response message if the call is successful, or an `IpcError` if
    /// an error occurs during the process.
    /// # Example
    ///
    /// ```rust,ignore
    /// use ckb_script_ipc_common::channel::Channel;
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
    /// let mut channel = Channel::new(reader, writer);
    /// let request = MyRequest { /* fields */ };
    /// let response: MyResponse = channel.call("my_method", request).expect("Failed to call method");
    /// ```
    pub fn call<Req, Resp>(
        &mut self,
        _method_name: &'static str,
        req: Req,
    ) -> Result<Resp, IpcError>
    where
        Req: Serialize + for<'de> Deserialize<'de>,
        Resp: Serialize + for<'de> Deserialize<'de>,
    {
        let result = self.send_request(req).and_then(|_| self.receive_response());
        match result {
            Ok(resp) => Ok(resp),
            Err(e) => {
                #[cfg(feature = "enable-logging")]
                log::error!("Error in call({}): {:?}", _method_name, e);
                Err(e)
            }
        }
    }
    pub(crate) fn send_request<Req: Serialize>(&mut self, req: Req) -> Result<(), IpcError> {
        let serialized_req = to_vec(&req).map_err(|_| IpcError::SerializeError)?;
        let packet = RequestPacket::new(serialized_req);
        #[cfg(feature = "enable-logging")]
        log::info!("send request: {:?}", packet);

        let bytes = packet.serialize();
        self.writer.write(&bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Sends a raw JSON string request to the server.
    ///
    /// This function takes a JSON string and sends it directly as a request packet to the server,
    /// without performing any serialization. This is useful when working with raw JSON data that
    /// doesn't need to be converted from Rust types.
    ///
    /// # Arguments
    ///
    /// * `json` - A string slice containing the JSON request to be sent.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether the request was successfully sent, or an `IpcError` if
    /// writing to the channel fails.
    pub fn send_json_request(&mut self, json: &str) -> Result<(), IpcError> {
        let packet = RequestPacket::new(json.as_bytes().to_vec());
        #[cfg(feature = "enable-logging")]
        log::info!("send request: {:?}", packet);

        let bytes = packet.serialize();
        self.writer.write(&bytes)?;
        self.writer.flush()?;
        Ok(())
    }
    pub(crate) fn send_response<Resp: Serialize>(&mut self, resp: Resp) -> Result<(), IpcError> {
        let serialized_resp = to_vec(&resp).map_err(|_| IpcError::SerializeError)?;
        let packet = ResponsePacket::new(0, serialized_resp);
        #[cfg(feature = "enable-logging")]
        log::info!("send response: {:?}", packet);

        let bytes = packet.serialize();
        self.writer.write(&bytes)?;
        self.writer.flush()?;
        Ok(())
    }
    pub(crate) fn send_error_code(
        &mut self,
        error_code: ProtocolErrorCode,
    ) -> Result<(), IpcError> {
        let packet = ResponsePacket::new(error_code.clone() as u64, vec![]);
        #[cfg(feature = "enable-logging")]
        log::info!("send error code: {:?}", error_code as u64);
        let bytes = packet.serialize();
        self.writer.write(&bytes)?;
        self.writer.flush()?;
        Ok(())
    }
    pub(crate) fn receive_request<Req: for<'de> Deserialize<'de>>(
        &mut self,
    ) -> Result<Req, IpcError> {
        let packet = RequestPacket::read_from(&mut self.reader)?;
        #[cfg(feature = "enable-logging")]
        log::info!("receive request: {:?}", packet);
        let req = from_slice(packet.payload()).map_err(|_| IpcError::DeserializeError)?;
        Ok(req)
    }
    pub(crate) fn receive_response<Resp: for<'de> Deserialize<'de>>(
        &mut self,
    ) -> Result<Resp, IpcError> {
        let packet = ResponsePacket::read_from(&mut self.reader)?;

        #[cfg(feature = "enable-logging")]
        log::info!("Received response: {:?}", packet);

        let error_code = ProtocolErrorCode::from(packet.error_code());
        match error_code {
            ProtocolErrorCode::Ok => {}
            e => {
                #[cfg(feature = "enable-logging")]
                log::error!("Received error code: {:?}", e);
                return Err(IpcError::ProtocolError(e));
            }
        }
        from_slice(packet.payload()).map_err(|_| IpcError::DeserializeError)
    }

    /// Receives a JSON string response from the server.
    ///
    /// This function reads a response packet from the server and returns its payload as a String,
    /// without attempting to deserialize it into a specific type. This is useful when working
    /// with raw JSON responses that don't need to be converted into specific Rust types.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - A String containing the JSON response if successful
    /// * `Err(IpcError)` - An error if:
    ///   - Reading from the channel fails
    ///   - The server returns an error code
    ///   - The response payload contains invalid UTF-8
    ///
    pub fn receive_json_response(&mut self) -> Result<String, IpcError> {
        let packet = ResponsePacket::read_from(&mut self.reader)?;

        #[cfg(feature = "enable-logging")]
        log::info!("Received response: {:?}", packet);

        let error_code = ProtocolErrorCode::from(packet.error_code());
        match error_code {
            ProtocolErrorCode::Ok => {}
            e => {
                #[cfg(feature = "enable-logging")]
                log::error!("Received error code: {:?}", e);
                return Err(IpcError::ProtocolError(e));
            }
        }
        Ok(String::from_utf8_lossy(packet.payload()).into_owned())
    }
}
