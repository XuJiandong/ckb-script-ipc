#![no_std]
#![cfg_attr(not(test), no_main)]

#[cfg(test)]
extern crate alloc;

#[cfg(not(test))]
use ckb_std::default_alloc;
#[cfg(not(test))]
ckb_std::entry!(program_entry);
#[cfg(not(test))]
default_alloc!();

use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};
use ckb_crypto_interface::{CkbCrypto, CryptoError, HasherCtx, HasherType};
use ckb_script_ipc_common::spawn::run_server;
use ckb_std::log::{error, info};

trait Hasher {
    fn update(&mut self, data: &[u8]);
    fn finalize(&mut self) -> Vec<u8>;
}

struct Blake2b {
    ctx: Option<blake2b_ref::Blake2b>,
}
impl Hasher for Blake2b {
    fn update(&mut self, data: &[u8]) {
        self.ctx.as_mut().unwrap().update(data);
    }
    fn finalize(&mut self) -> Vec<u8> {
        let ctx = self.ctx.take().unwrap();
        let mut buf = [0u8; 32];
        ctx.finalize(&mut buf);
        buf.to_vec()
    }
}

struct Sha256Hasher {
    ctx: Option<sha2::Sha256>,
}
impl Hasher for Sha256Hasher {
    fn update(&mut self, data: &[u8]) {
        use sha2::Digest;
        self.ctx.as_mut().unwrap().update(data);
    }
    fn finalize(&mut self) -> Vec<u8> {
        use sha2::Digest;
        let ctx = self.ctx.take().unwrap();
        ctx.finalize().to_vec()
    }
}

struct Ripemd160Hasher {
    ctx: Option<ripemd::digest::core_api::CoreWrapper<ripemd::Ripemd160Core>>,
}
impl Hasher for Ripemd160Hasher {
    fn update(&mut self, data: &[u8]) {
        use ripemd::Digest;
        self.ctx.as_mut().unwrap().update(data);
    }
    fn finalize(&mut self) -> Vec<u8> {
        use ripemd::Digest;
        let ctx = self.ctx.take().unwrap();
        ctx.finalize().to_vec()
    }
}

struct CryptoServer {
    hashers: BTreeMap<u64, Box<dyn Hasher>>,
    hasher_count: u64,
}

impl CryptoServer {
    fn new() -> Self {
        Self {
            hashers: Default::default(),
            hasher_count: 0,
        }
    }
}

impl CkbCrypto for CryptoServer {
    fn hasher_new(&mut self, hash_type: HasherType) -> HasherCtx {
        const CKB_HASH_PERSONALIZATION: &[u8] = b"ckb-default-hash";

        let hash: Box<dyn Hasher> = match hash_type {
            HasherType::CkbBlake2b => Box::new(Blake2b {
                ctx: Some(
                    blake2b_ref::Blake2bBuilder::new(32)
                        .personal(CKB_HASH_PERSONALIZATION)
                        .build(),
                ),
            }),
            HasherType::Blake2b => Box::new(Blake2b {
                ctx: Some(blake2b_ref::Blake2bBuilder::new(32).build()),
            }),
            HasherType::Sha256 => {
                use sha2::{Digest, Sha256};
                Box::new(Sha256Hasher {
                    ctx: Some(Sha256::new()),
                })
            }
            HasherType::Ripemd160 => {
                use ripemd::{Digest, Ripemd160};
                Box::new(Ripemd160Hasher {
                    ctx: Some(Ripemd160::new()),
                })
            }
        };

        let id = self.hasher_count;
        self.hasher_count += 1;
        self.hashers.insert(id, hash);
        HasherCtx(id)
    }
    fn hasher_update(&mut self, ctx: HasherCtx, data: Vec<u8>) -> Result<(), CryptoError> {
        if let Some(hasher) = self.hashers.get_mut(&ctx.0) {
            hasher.update(&data);
            Ok(())
        } else {
            Err(CryptoError::InvalidContext)
        }
    }
    fn hasher_finalize(&mut self, ctx: HasherCtx) -> Result<Vec<u8>, CryptoError> {
        if let Some(mut hasher) = self.hashers.remove(&ctx.0) {
            Ok(hasher.finalize())
        } else {
            Err(CryptoError::InvalidContext)
        }
    }

    fn secp256k1_recovery(
        &mut self,
        prehash: Vec<u8>,
        signature: Vec<u8>,
        recovery_id: u8,
    ) -> Result<Vec<u8>, CryptoError> {
        use k256::ecdsa::hazmat::bits2field;
        // use k256::ecdsa::signature::Result;
        use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
        use k256::elliptic_curve::bigint::CheckedAdd;
        use k256::elliptic_curve::ops::{Invert, LinearCombination, Reduce};
        use k256::elliptic_curve::point::DecompressPoint;
        use k256::elliptic_curve::{
            AffinePoint, Curve, FieldBytesEncoding, PrimeField, ProjectivePoint,
        };
        use k256::{Scalar, Secp256k1};

        let signature = Signature::from_slice(&signature).map_err(|_| CryptoError::InvalidSig)?;
        let (r, s) = signature.split_scalars();
        let z = <Scalar as Reduce<<Secp256k1 as k256::elliptic_curve::Curve>::Uint>>::reduce_bytes(
            &bits2field::<Secp256k1>(&prehash).map_err(|_| CryptoError::InvalidPrehash)?,
        );

        let recovery_id = RecoveryId::from_byte(recovery_id);
        if recovery_id.is_none() {
            return Err(CryptoError::InvalidRecoveryId);
        }
        let recovery_id = recovery_id.unwrap();

        let mut r_bytes = r.to_repr();
        if recovery_id.is_x_reduced() {
            match Option::<<Secp256k1 as k256::elliptic_curve::Curve>::Uint>::from(
                <Secp256k1 as k256::elliptic_curve::Curve>::Uint::decode_field_bytes(&r_bytes)
                    .checked_add(&Secp256k1::ORDER),
            ) {
                Some(restored) => r_bytes = restored.encode_field_bytes(),
                // No reduction should happen here if r was reduced
                None => return Err(CryptoError::InvalidRecoveryId),
            };
        }
        #[allow(non_snake_case)]
        let R =
            AffinePoint::<Secp256k1>::decompress(&r_bytes, u8::from(recovery_id.is_y_odd()).into());

        if R.is_none().into() {
            return Err(CryptoError::RecoveryFailed);
        }

        #[allow(non_snake_case)]
        let R = ProjectivePoint::<Secp256k1>::from(R.unwrap());
        let r_inv = *r.invert();
        let u1 = -(r_inv * z);
        let u2 = r_inv * *s;
        let pk = ProjectivePoint::<Secp256k1>::lincomb(
            &ProjectivePoint::<Secp256k1>::GENERATOR,
            &u1,
            &R,
            &u2,
        );
        let vk = VerifyingKey::from_affine(pk.into()).map_err(|_| CryptoError::RecoveryFailed)?;
        Ok(vk.to_sec1_bytes().to_vec())
    }

    fn secp256k1_verify(
        &mut self,
        public_key: Vec<u8>,
        prehash: Vec<u8>,
        signature: Vec<u8>,
    ) -> Result<(), CryptoError> {
        use k256::ecdsa::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey};

        let signature = Signature::from_slice(&signature).map_err(|_| CryptoError::InvalidSig)?;
        let verify_key =
            VerifyingKey::from_sec1_bytes(&public_key).map_err(|_| CryptoError::InvalidPubkey)?;

        verify_key
            .verify_prehash(&prehash, &signature)
            .map_err(|_| CryptoError::VerifyFailed)?;
        Ok(())
    }

    fn schnorr_verify(
        &mut self,
        public_key: Vec<u8>,
        messge: Vec<u8>,
        signature: Vec<u8>,
    ) -> Result<(), CryptoError> {
        use k256::schnorr::{signature::Verifier, Signature, VerifyingKey};

        let signature =
            Signature::try_from(signature.as_slice()).map_err(|_| CryptoError::InvalidSig)?;

        let verify_key =
            VerifyingKey::from_bytes(&public_key).map_err(|_| CryptoError::InvalidPubkey)?;

        verify_key
            .verify(&messge, &signature)
            .map_err(|_| CryptoError::VerifyFailed)?;

        Ok(())
    }

    fn ed25519_verify(
        &mut self,
        public_key: Vec<u8>,
        prehash: Vec<u8>,
        signature: Vec<u8>,
    ) -> Result<(), CryptoError> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let signature = Signature::from_slice(&signature).map_err(|_| CryptoError::InvalidSig)?;
        let public_key = public_key
            .try_into()
            .map_err(|_| CryptoError::InvalidPubkey)?;

        VerifyingKey::from_bytes(&public_key)
            .map_err(|_| CryptoError::InvalidPubkey)?
            .verify(&prehash, &signature)
            .map_err(|_| CryptoError::VerifyFailed)?;

        Ok(())
    }
}

pub fn program_entry() -> i8 {
    drop(ckb_std::logger::init());

    info!("server started");
    let world = CryptoServer::new();
    let err = run_server(world.server());

    if err.is_ok() {
        0
    } else {
        error!("Server failed: {:?}", err.unwrap_err());
        1
    }
}
