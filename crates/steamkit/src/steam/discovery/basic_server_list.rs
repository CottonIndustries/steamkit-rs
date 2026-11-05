//! The server list as stored by `FileStorageServerListProvider` and
//! `IsolatedStorageServerListProvider`.
//!
//! It is the repeated field 1 of the protobuf message, written as a bare
//! sequence of length-prefixed entries:
//!
//! ```text
//! file  := entry*
//! entry := 0x0A varint(len) payload
//! ```
//!
//! This is the same byte stream SteamKit2 writes, so cache files are
//! interchangeable.

use crate::error::SteamError;

use super::basic_server_list_proto::{
    write_tag, write_varint, BasicServerListProto, Cursor, WireType,
};
use super::server_record::ServerRecord;

/// The server list as stored by the file and isolated-storage providers.
pub(crate) struct BasicServerList {
    servers: Vec<BasicServerListProto>,
}

impl BasicServerList {
    /// The servers in the list.
    pub(crate) fn servers(&self) -> &[BasicServerListProto] {
        &self.servers
    }

    /// Builds the list from server records.
    pub(crate) fn from_server_records(endpoints: &[ServerRecord]) -> Self {
        BasicServerList {
            servers: endpoints
                .iter()
                .map(BasicServerListProto::from_server_record)
                .collect(),
        }
    }

    /// Encodes the list as repeated message field 1.
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();

        for item in &self.servers {
            let payload = item.encode();
            write_tag(&mut out, 1, WireType::LengthDelimited);
            write_varint(&mut out, payload.len() as u64);
            out.extend_from_slice(&payload);
        }

        out
    }

    /// Decodes a list written as repeated message field 1.
    pub(crate) fn decode(data: &[u8]) -> Result<Self, SteamError> {
        let mut servers = Vec::new();
        let mut cursor = Cursor::new(data);

        while !cursor.is_empty() {
            let tag = cursor.read_varint()?;
            let field = tag >> 3;
            let wire_type = WireType::from_tag(tag)?;

            if field == 1 && wire_type == WireType::LengthDelimited {
                let length = cursor.read_varint()? as usize;
                let bytes = cursor.read_bytes(length)?;
                servers.push(BasicServerListProto::decode(bytes)?);
            } else {
                cursor.skip(wire_type)?;
            }
        }

        Ok(BasicServerList { servers })
    }
}
