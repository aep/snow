extern crate snow;
extern crate rustc_serialize;

use std::ops::Deref;
use rustc_serialize::{Encodable, Decodable, Decoder};
use rustc_serialize::hex::{FromHex, ToHex};
use rustc_serialize::json::{self, DecoderError, Encoder};
use snow::*;

struct HexBytes {
    original: String,
    payload: Vec<u8>,
}

impl Deref for HexBytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl Decodable for HexBytes {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        let hex = d.read_str()?;
        let bytes = hex.from_hex().map_err(|_| d.error("field is an invalid binary hex encoding"))?;
        Ok(HexBytes {
            original: hex,
            payload: bytes,
        })
    }
}

#[derive(RustcDecodable)]
struct TestMessage {
    payload: HexBytes,
    ciphertext: HexBytes,
}

#[derive(RustcDecodable)]
struct TestVector {
    name: String,
    pattern: String,
    dh: String,
    cipher: String,
    hash: String,
    init_prologue: HexBytes,
    init_psk: Option<HexBytes>,
    init_static: Option<HexBytes>,
    init_remote_static: Option<HexBytes>,
    init_ephemeral: Option<HexBytes>,
    resp_prologue: HexBytes,
    resp_static: Option<HexBytes>,
    resp_remote_static: Option<HexBytes>,
    resp_ephemeral: Option<HexBytes>,
    resp_psk: Option<HexBytes>,
    messages: Vec<TestMessage>,
}

#[derive(RustcDecodable)]
struct TestVectors {
    vectors: Vec<TestVector>,
}

fn build_session_pair(vector: &TestVector) -> Result<(NoiseSession<HandshakeState>, NoiseSession<HandshakeState>), NoiseError> {

}

#[test]
fn test_vectors_noise_c_basic() {
    let vectors_json = include_str!("vectors/noise-c-basic.txt");
    let test_vectors: TestVectors = json::decode(vectors_json).unwrap();

    let mut ignored_448 = 0;
    let mut ignored_oneway = 0;

    for vector in test_vectors.vectors {
        let params: NoiseParams = vector.name.parse().unwrap();
        if params.dh == DHChoice::Ed448 {
            ignored_448 += 1;
            continue;
        }
        if params.handshake.is_oneway() {
            ignored_oneway += 1;
            continue;
        }
        println!("testing {}...", vector.name);

        let mut init_builder = NoiseBuilder::new(params.clone());
        let mut resp_builder = NoiseBuilder::new(params.clone());

        match (params.base, vector.init_psk, vector.resp_psk) {
            (BaseChoice::NoisePSK, Some(init_psk), Some(resp_psk)) => {
                init_builder = init_builder.preshared_key(&*init_psk);
                resp_builder = resp_builder.preshared_key(&*resp_psk);
            },
            (BaseChoice::NoisePSK, _, _) => {
                panic!("NoisePSK case missing PSKs for init and/or resp");
            },
            _ => {}
        }

        if let Some(ref init_s) = vector.init_static {
            init_builder = init_builder.local_private_key(&*init_s);
        }
        if let Some(ref resp_s) = vector.resp_static {
            resp_builder = resp_builder.local_private_key(&*resp_s);
        }
        if let Some(ref init_remote_static) = vector.init_remote_static {
            init_builder = init_builder.remote_public_key(&*init_remote_static);
        }
        if let Some(ref resp_remote_static) = vector.resp_remote_static {
            resp_builder = resp_builder.remote_public_key(&*resp_remote_static);
        }
        if let Some(ref init_e) = vector.init_ephemeral {
            init_builder = init_builder.fixed_ephemeral_key_for_testing_only(&*init_e);
        }
        if let Some(ref resp_e) = vector.resp_ephemeral {
            resp_builder = resp_builder.fixed_ephemeral_key_for_testing_only(&*resp_e);
        }

        let mut init = init_builder.build_initiator().unwrap();
        let mut resp = resp_builder.build_responder().unwrap();

        let (mut sendbuf, mut recvbuf) = ([0u8; 65535], [0u8; 65535]);
        for (i, message) in vector.messages.iter().enumerate() {
            println!("  message {}", i);
            let (send, recv) = if i % 2 == 0 {
                (&mut init, &mut resp)
            } else {
                (&mut resp, &mut init)
            };

            let (len, _) = send.write_message(&*message.payload, &mut sendbuf).unwrap();
            println!("ciphertext: {}", &sendbuf[..len].to_hex());
            assert!(&sendbuf[..len] == &(*message.ciphertext)[..]);
            recv.read_message(&sendbuf[..len], &mut recvbuf).unwrap();
        }
        println!("  passed!");
    }

    println!("\n* ignored {} unsupported Ed448-Goldilocks variants\n", ignored_448);
    panic!("done");
}