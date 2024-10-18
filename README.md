# CKB Script IPC

This project consists of two main components: a proc-macro library for
generating Inter-Process Communication (IPC) code for CKB scripts, and a common
runtime library for CKB script IPC functionality. The proc-macro library is
inspired from [tarpc](https://github.com/google/tarpc).


## Overview

The `ckb-script-ipc` crate provides procedural macros that simplify the process
of creating IPC code for CKB scripts. It automates the generation of
serialization, deserialization, and communication boilerplate, allowing
developers to focus on the core logic of their scripts.

The `ckb-script-ipc-common` crate offers a set of tools and runtime support for
IPC in CKB scripts. It includes necessary dependencies and features to
facilitate communication between different parts of a CKB script. It is used by
`ckb-script-ipc` crate.

## Features

- Automatic generation of IPC message structures
- Serialization and deserialization of IPC messages
- Easy-to-use macros for defining IPC interfaces

## Usage
1. Import necessary crates:

```toml
ckb-script-ipc = { version = "..." }
ckb-script-ipc-common = { version = "..." }
serde = { version = "...", default-features = false, features = ["derive"] }
```
Replace "..." with the latest versions.

2. Define IPC interface:

```rust,ignore
#[ckb_script_ipc::service]
pub trait World {
    fn hello(name: String) -> Result<String, u64>;
}
```

Place this trait definition in a library that is shared by both client and
server scripts. Note that this trait does not involve `self`. The proc-macro
will automatically append the necessary implementations.

The arguments and returned types in methods should implement the Serialize and
Deserialize traits from serde.

3. Start the server:

```rust,ignore
use ckb_script_ipc_common::spawn::spawn_server;

let (read_pipe, write_pipe) = spawn_server(
    0,
    Source::CellDep,
    &[CString::new("demo").unwrap().as_ref()],
)?;
```
You can also use `spawn_cell_server` with `code_hash/hash_type`. The pipes
returned will be used in client side.

4. Implement and run the server:

```rust,ignore
use crate::def::World;
use ckb_script_ipc_common::spawn::run_server;

struct WorldServer;

impl World for WorldServer {
    fn hello(&mut self, name: String) -> Result<String, u64> {
        if name == "error" {
            Err(1)
        } else {
            Ok(format!("hello, {}", name))
        }
    }
}

run_server(WorldServer.server()).map_err(|_| Error::ServerError)
```
The `run_server` contains a infinite loop and never returns. The method `server`
is implemented by proc-macro implicitly.

5. Create and use the client:

```rust,ignore
use crate::def::WorldClient;

let mut client = WorldClient::new(read_pipe.into(), write_pipe.into());
let ret = client.hello("world".into()).unwrap();
```
The `read_pipe`/`write_pipe` are passed into server from `spawn_server`.

For a complete example, see [ckb-script-ipc-demo](https://github.com/XuJiandong/ckb-script-ipc/tree/main/contracts/ckb-script-ipc-demo).

## Wire format
### Concept of Packet

Since the `read`/`write` operations related to `spawn` are based on streams,
they don't fully meet our requirements. Therefore, we introduce a concept
similar to packets. Each packet needs to contain a header to indicate basic
information such as packet length, service information, error codes, etc.

We use [VLQ](https://en.wikipedia.org/wiki/Variable-length_quantity) to define
the length information in the packet header. Compared to fixed-length
representations, VLQ is more compact and suitable for this scenario. Packets are
divided into the following two categories:

### Request

Contains the following fields without any format. That is, all fields are
directly arranged without any additional header. Therefore, in the shortest
case, version + method id + length only occupies 3 bytes.
- version (VLQ)
- method id (VLQ)
- length (VLQ)
- payload (variable length data)

### Response

- version (VLQ)
- error code (VLQ)
- length (VLQ)
- payload (variable length data)

### Communication Details

- Difference between Request and Response:
    - All sent data belongs to Request; all received data belongs to Response.
    - After a Request is sent, it will be processed immediately and a Response
      will be returned. Therefore, the Response does not need to specify which
      Request it corresponds to.
- Packet Field Parsing:
    - version: Indicates the version, currently 0.
    - length: Indicates the length of the subsequent payload.
    - method id: Represents the method ID. Some services require multiple
      Request/Response interactions, so they are composed of multiple method
      IDs. The range of `method id` is defined as 0 to 2^64. This field can be
      not used because users can include it in the payload through
      serialization/deserialization.
    - error code: Only appears in Response, range is 0 to 2^64.
    - payload: Defined by the service provider, developers can choose freely.
      You can use `molecule` to define the data, or choose other methods.

In theory, VLQ can represent integers of any length, but considering practical
implementation, we need to set a boundary for easier code processing. Currently,
we define the range of all VLQ from 0 to 2^64, including length, method_id, error_code.


## FAQ
Q: What types can be used in IPC methods?

A: Any arguments and returned types to IPC methods should be implemented by trait
Serialize and Deserialize from serde. By default, all primitive types and types
from standard libraries should be fine. Any user defined structure type should
be annotated. For example:
```rust,ignore
#[derive(Serialize, Deserialize)]
pub struct Struct0 {
    pub f0: u8,
    pub f1: u64,
    pub f2: [u8; 3],
}
```

Q: What serialize/deserialize format is used for message packing and unpacking?

A: [serde_molecule](https://github.com/XuJiandong/serde_molecule)

Q: How can I view code expanded by `#[ckb_script_ipc::service]`?

A: [cargo expand](https://github.com/dtolnay/cargo-expand)

Q: Why is it called IPC and not RPC?

A: The code operates within a script process that is part of a transaction, and
it can only run on the same machine. This makes it more akin to Inter-Process
Communication (IPC) rather than Remote Procedure Call (RPC). RPC encompasses
additional features such as encryption, authentication, error propagation,
retries and timeouts, scaling, and more. This crate focuses on a limited subset of
these features, primarily those relevant to IPC.


## Vector issue
By default, `Vec` is serialized into a [molecule
fixvec](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0008-serialization/0008-serialization.md#fixvec---fixed-vector).
This works well for simple types like `Vec<u8>` or `Vec<[u8; 4]>`. However, it
does not support complex element types such as `Vec<Vec<u8>>`. These should be
serialized into a [molecule
dynvec](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0008-serialization/0008-serialization.md#dynvec---dynamic-vector).
In `serde_molecule`, this can be annotated using `#[serde(with = "dynvec_serde")]`.
Since this crate does not provide a way to annotate method types, it is recommended to define a new type to avoid this issue:

```rust,ignore
use serde_molecule::dynvec_serde;

#[derive(Serialize, Deserialize)]
struct TwoDimVec {
    #[serde(with = "dynvec_serde")]
    pub inner: Vec<Vec<u8>>
}
```
