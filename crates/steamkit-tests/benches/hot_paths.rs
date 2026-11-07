use std::hint::black_box;
use std::time::Instant;

use steamkit::base::client_msg::{ClientMsgProtobuf, IClientMsg};
use steamkit::base::generated::steam_language::EMsg;
use steamkit::networking::steam3::net_filter_encryption_with_hmac::NetFilterEncryptionWithHMAC;
use steamkit::networking::steam3::udp_packet::UdpHeader;
use steamkit_protos::steam::CMsgClientHello;

fn bench<T, F>(name: &str, iterations: u32, mut f: F)
where
    F: FnMut() -> T,
{
    let start = Instant::now();

    for _ in 0..iterations {
        black_box(f());
    }

    let elapsed = start.elapsed();
    let per_op = elapsed.as_nanos() as f64 / f64::from(iterations);
    println!("{name:<26} {per_op:>12.1} ns/op");
}

fn main() {
    let key = [7u8; 32];
    let filter = NetFilterEncryptionWithHMAC::new(&key).expect("a 32-byte key is valid");
    let payload = vec![0xABu8; 1024];
    let encrypted = filter
        .process_outgoing(&payload)
        .expect("the payload must encrypt");

    bench("encrypt 1 KiB", 20_000, || {
        filter.process_outgoing(&payload).expect("encrypt")
    });
    bench("decrypt 1 KiB", 20_000, || {
        filter.process_incoming(&encrypted).expect("decrypt")
    });

    let message = ClientMsgProtobuf::<CMsgClientHello>::new(EMsg::ClientHello);
    bench("serialize ClientHello", 50_000, || message.serialize());

    let header = UdpHeader {
        magic: UdpHeader::MAGIC,
        payload_size: 1024,
        ..UdpHeader::default()
    };
    bench("udp header serialize", 200_000, || {
        let mut out = Vec::with_capacity(UdpHeader::SERIALIZED_SIZE);
        header.serialize_into(&mut out);
        out
    });
}
