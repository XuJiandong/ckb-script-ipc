use crate::error::ProtocolErrorCode;
use crate::io::Write;
use crate::ipc::Serve;
use crate::packet::{Packet, RequestPacket, ResponsePacket};
use crate::{error::IpcError, pipe::Pipe};
use alloc::vec;
use serde::{Deserialize, Serialize};
use serde_molecule::{from_slice, to_vec};

pub struct Channel {
    reader: Pipe,
    writer: Pipe,
}

impl Channel {
    pub fn new(reader: Pipe, writer: Pipe) -> Self {
        Self { reader, writer }
    }
}

impl Channel {
    /// Execute a server loop
    /// 1. receive request
    /// 2. call serve method
    /// 3. send response
    /// 4. continue
    pub fn execute<Req, Resp, S>(mut self, serve: &mut S) -> Result<(), IpcError>
    where
        Req: Serialize + for<'de> Deserialize<'de>,
        Resp: Serialize + for<'de> Deserialize<'de>,
        S: Serve<Req = Req, Resp = Resp>,
    {
        loop {
            let result = self
                .receive_request()
                .and_then(|req| serve.serve(req).map_err(Into::into))
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
    // used for client
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
    pub fn send_request<Req: Serialize>(&mut self, req: Req) -> Result<(), IpcError> {
        let serialized_req = to_vec(&req, false).map_err(|_| IpcError::SerializeError)?;
        let packet = RequestPacket::new(serialized_req);
        #[cfg(feature = "enable-logging")]
        log::info!("send request: {:?}", packet);

        let bytes = packet.serialize();
        self.writer.write(&bytes)?;
        Ok(())
    }
    pub fn send_response<Resp: Serialize>(&mut self, resp: Resp) -> Result<(), IpcError> {
        let serialized_resp = to_vec(&resp, false).map_err(|_| IpcError::SerializeError)?;
        let packet = ResponsePacket::new(0, serialized_resp);
        #[cfg(feature = "enable-logging")]
        log::info!("send response: {:?}", packet);

        let bytes = packet.serialize();
        self.writer.write(&bytes)?;
        Ok(())
    }
    pub fn send_error_code(&mut self, error_code: ProtocolErrorCode) -> Result<(), IpcError> {
        let packet = ResponsePacket::new(error_code as u64, vec![]);
        #[cfg(feature = "enable-logging")]
        log::info!("send error code: {:?}", error_code as u64);
        let bytes = packet.serialize();
        self.writer.write(&bytes)?;
        Ok(())
    }
    pub fn receive_request<Req: for<'de> Deserialize<'de>>(&mut self) -> Result<Req, IpcError> {
        let packet = RequestPacket::read_from(&mut self.reader)?;
        #[cfg(feature = "enable-logging")]
        log::info!("receive request: {:?}", packet);
        let req = from_slice(packet.payload(), false).map_err(|_| IpcError::DeserializeError)?;
        Ok(req)
    }
    pub fn receive_response<Resp: for<'de> Deserialize<'de>>(&mut self) -> Result<Resp, IpcError> {
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
        from_slice(packet.payload(), false).map_err(|_| IpcError::DeserializeError)
    }
}
