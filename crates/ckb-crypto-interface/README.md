# CKB Crypto service interface

## 1. Starting the Crypto Service

To start the service, use the following code:

```rust
let service = CkbCryptoClient::new(read_pipe, write_pipe);
```

- `read_pipe` and `write_pipe` should be created using the `spawn_cell_server` method.
- For implementation details, refer to the `ckb-script-ipc` and `ckb-script-ipc-common` libraries.

---

## 2. Hash Computation Interface

The service supports the following hash algorithms:
- **CkbBlake2b**
- **Blake2b**
- **Sha256**
- **Ripemd160**

### Usage

1. **Create a hasher context**:
   ```rust
   let hasher_ctx = service.hasher_new(HasherType::CkbBlake2b);
   ```

2. **Update data**:
   ```rust
   service.hasher_update(hasher_ctx, data);
   ```

3. **Retrieve the hash value**:
   ```rust
   let hash = service.hasher_finalize(hasher_ctx);
   ```
   The length of `hash` will vary according to different algorithms.

---

## 3. Recover secp256k1 Public Key

### Function Signature

```rust
fn secp256k1_recovery(
    prehash: Vec<u8>,
    signature: Vec<u8>,
    recovery_id: u8,
) -> Result<Vec<u8>, CryptoError>;
```

### Parameters

- `prehash`:
  - The hash of the input data.
  - This value will not be hashed again. It is recommended to use **32 bytes**. The minimum is **16 bytes**, and if the length exceeds 32 bytes, only the first **32 bytes** will be used.
- `signature`:
  - secp256k1 signature, **64 bytes** in length.
- `recovery_id`:
  - Recovery ID used to restore the public key from the signature.

### Return Value

- On success, it returns the recovered public key as a `Vec<u8>`.
- On failure, it returns a `CryptoError`.

---

## 4. Verify secp256k1 Signature

### Function Signature

```rust
fn secp256k1_verify(
    public_key: Vec<u8>,
    prehash: Vec<u8>,
    signature: Vec<u8>,
) -> Result<(), CryptoError>;
```

### Parameters

- `public_key`:
  - The plaintext format of the public key.
- `prehash`:
  - Same as the `prehash` parameter in `secp256k1_recovery`.
- `signature`:
  - secp256k1 signature, **64 bytes** in length.

### Return Value

- Returns `Ok(())` on success.
- Returns `CryptoError` on failure.

---

## 5. Verify Schnorr Signature

### Function Signature

```rust
fn schnorr_verify(
    public_key: Vec<u8>,
    msg: Vec<u8>,
    signature: Vec<u8>,
) -> Result<(), CryptoError>;
```

### Parameters

- `public_key`:
  - The plaintext format of the public key, **32 bytes** in length.
- `msg`:
  - Input message of arbitrary length. A **Sha256** hash will be applied during verification.
- `signature`:
  - Schnorr signature, **64 bytes** in length.

### Return Value

- Returns `Ok(())` on success.
- Returns `CryptoError` on failure.

---

## 6. Verify Ed25519 Signature

### Function Signature

```rust
fn ed25519_verify(
    public_key: Vec<u8>,
    msg: Vec<u8>,
    signature: Vec<u8>,
) -> Result<(), CryptoError>;
```

### Parameters

- `public_key`:
  - The plaintext format of the public key, **32 bytes** in length.
- `msg`:
  - Input message of arbitrary length. A **Sha512** hash will be applied during verification.
- `signature`:
  - Ed25519 signature, **64 bytes** in length.

### Return Value

- Returns `Ok(())` on success.
- Returns `CryptoError` on failure.
