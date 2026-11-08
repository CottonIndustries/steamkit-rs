//! Robustness and round-trip tests for the hand-written decoders.
//!
//! SteamKit2 has no equivalent: its protobuf codecs are generated and its compression wrappers
//! throw. These tests are the Rust equivalent of fuzzing those paths in CI, with a deterministic
//! generator so any failure reproduces.

use std::io::Cursor;

use steamkit::networking::steam3::protocol_type::ProtocolTypes;
use steamkit::steam::discovery::basic_server_list::BasicServerList;
use steamkit::steam::discovery::basic_server_list_proto::BasicServerListProto;
use steamkit::steam::discovery::server_record::ServerRecord;
use steamkit::util::lzma_util::LzmaUtil;
use steamkit::util::v_zip_util::VZipUtil;
use steamkit::util::v_zstd_util::VZstdUtil;

/// A deterministic xorshift generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn bytes(&mut self, max_len: usize) -> Vec<u8> {
        let len = (self.next() as usize) % (max_len + 1);
        (0..len).map(|_| (self.next() & 0xFF) as u8).collect()
    }
}

#[test]
fn the_decoders_do_not_panic_on_random_input() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut destination = vec![0u8; 4096];

    for _ in 0..2_000 {
        let data = rng.bytes(512);

        let _ = BasicServerList::decode(&data);
        let _ = BasicServerListProto::decode(&data);
        let _ = VZipUtil::decompress(&data, &mut destination, true);
        let _ = VZstdUtil::decompress(&data, &mut destination, true);
        let _ = LzmaUtil::decompress(
            &mut Cursor::new(&data),
            &mut Vec::new(),
            data.len(),
            4096,
        );
    }
}

#[test]
fn a_server_list_round_trips() {
    let records = vec![
        ServerRecord::create_socket_server("1.2.3.4:27017".parse().expect("a valid address")),
        ServerRecord::create_server("cm.example.com", 27017, ProtocolTypes::TCP),
        ServerRecord::create_server("cm2.example.com", 443, ProtocolTypes::WEB_SOCKET),
    ];

    let list = BasicServerList::from_server_records(&records);
    let encoded = list.encode();
    let decoded = BasicServerList::decode(&encoded).expect("the list must decode");

    assert_eq!(list.servers().len(), decoded.servers().len());

    for (left, right) in list.servers().iter().zip(decoded.servers()) {
        assert_eq!(left.address(), right.address());
        assert_eq!(left.port(), right.port());
        assert_eq!(left.protocols(), right.protocols());
    }
}

#[test]
fn an_empty_server_list_is_valid() {
    let decoded = BasicServerList::decode(&[]).expect("an empty list is valid");

    assert!(decoded.servers().is_empty());
}

#[test]
fn a_truncated_server_list_is_rejected() {
    let list = BasicServerList::from_server_records(&[ServerRecord::create_server(
        "cm.example.com",
        27017,
        ProtocolTypes::TCP,
    )]);
    let mut encoded = list.encode();
    encoded.pop();

    assert!(BasicServerList::decode(&encoded).is_err());
}

#[test]
fn garbage_is_rejected_by_the_decompressors() {
    let garbage = [0xFFu8; 32];
    let mut destination = vec![0u8; 4096];

    assert!(VZipUtil::decompress(&garbage, &mut destination, true).is_err());
    assert!(VZstdUtil::decompress(&garbage, &mut destination, true).is_err());
    assert!(LzmaUtil::decompress(
        &mut Cursor::new(&garbage),
        &mut Vec::new(),
        garbage.len(),
        4096,
    )
    .is_err());
}

#[test]
fn vzip_round_trips() {
    for length in [0usize, 1, 7, 1024] {
        let data: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
        let compressed = VZipUtil::compress(&data).expect("compression must succeed");
        let mut destination = vec![0u8; data.len()];

        let written = VZipUtil::decompress(&compressed, &mut destination, true)
            .expect("decompression must succeed");

        assert_eq!(data.len(), written as usize);
        assert_eq!(data, destination);
    }
}
