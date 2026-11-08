//! The Rust analogue of SteamKit2's `ProtobufModelFacts`.
//!
//! C# checks that the protobuf-net `TypeModel` can serialize every generated contract. Rust has
//! no runtime model: `prost` derives the codec at compile time, so the equivalent check is that
//! the generated messages round-trip through that codec.

use prost::Message;
use steamkit_protos::steam::c_msg_client_games_played::GamePlayed;
use steamkit_protos::steam::{
    c_msg_ip_address, CMsgClientGamesPlayed, CMsgClientHello, CMsgIpAddress,
};

fn round_trip<M>(message: &M) -> M
where
    M: Message + Default,
{
    let encoded = message.encode_to_vec();
    M::decode(encoded.as_slice()).expect("a message must decode what it encoded")
}

#[test]
fn a_scalar_message_round_trips() {
    let message = CMsgClientHello {
        protocol_version: Some(65_580),
    };

    assert_eq!(message, round_trip(&message));
}

#[test]
fn a_oneof_message_round_trips() {
    let message = CMsgIpAddress {
        ip: Some(c_msg_ip_address::Ip::V4(0x0102_0304)),
    };
    assert_eq!(message, round_trip(&message));

    let message = CMsgIpAddress {
        ip: Some(c_msg_ip_address::Ip::V6(vec![0xAB; 16])),
    };
    assert_eq!(message, round_trip(&message));
}

#[test]
fn a_repeated_nested_message_round_trips() {
    let message = CMsgClientGamesPlayed {
        games_played: vec![
            GamePlayed {
                game_id: Some(440),
                ..GamePlayed::default()
            },
            GamePlayed {
                game_id: Some(570),
                ..GamePlayed::default()
            },
        ],
        ..CMsgClientGamesPlayed::default()
    };

    assert_eq!(message, round_trip(&message));
}

#[test]
fn an_empty_message_round_trips() {
    let message = CMsgClientHello::default();

    assert_eq!(message, round_trip(&message));
}
