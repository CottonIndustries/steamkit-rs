//! The blocking `SmartCMServerList` accessors: they fetch the list first, the
//! way the C# `WaitForServersFetched` does.

use std::net::SocketAddr;
use std::sync::Arc;

use steamkit::networking::steam3::protocol_type::ProtocolTypes;
use steamkit::steam::discovery::i_server_list_provider::IServerListProvider;
use steamkit::steam::discovery::memory_server_list_provider::MemoryServerListProvider;
use steamkit::steam::discovery::server_record::ServerRecord;
use steamkit::steam::discovery::smart_cm_server_list::SmartCMServerList;

/// The endpoint the provider holds.
const END_POINT: ([u8; 4], u16) = ([10, 1, 2, 3], 27017);

/// A list whose provider already holds one server, with the Steam Directory
/// disabled so nothing reaches the network.
fn list_from_provider() -> SmartCMServerList {
    let provider = MemoryServerListProvider::new();

    // The provider is filled through its async API. Its lock is uncontended, so
    // polling the future once is enough and no runtime is needed - building one
    // here would panic when the caller already runs inside a runtime.
    futures::executor::block_on(
        provider.update_server_list(vec![ServerRecord::create_socket_server(
            SocketAddr::from(END_POINT),
        )]),
    )
    .expect("the provider should accept the list");

    SmartCMServerList::with_provider(Arc::new(provider), false)
}

fn expected() -> Option<SocketAddr> {
    Some(SocketAddr::from(END_POINT))
}

/// No runtime at all: the fetch runs on a runtime of its own.
#[test]
fn get_next_server_candidate_fetches_the_list_without_a_runtime() {
    let list = list_from_provider();

    assert_eq!(
        list.get_next_server_candidate(ProtocolTypes::TCP)
            .and_then(|record| record.end_point()),
        expected()
    );
}

/// A current-thread runtime has no other worker, so the fetch must not
/// deadlock.
#[tokio::test]
async fn get_next_server_candidate_fetches_the_list_on_a_current_thread_runtime() {
    let list = list_from_provider();

    assert_eq!(
        list.get_next_server_candidate(ProtocolTypes::TCP)
            .and_then(|record| record.end_point()),
        expected()
    );
}

/// A multi-threaded runtime keeps driving the fetch on its other workers.
#[tokio::test(flavor = "multi_thread")]
async fn get_next_server_candidate_fetches_the_list_on_a_multi_thread_runtime() {
    let list = list_from_provider();

    assert_eq!(
        list.get_next_server_candidate(ProtocolTypes::TCP)
            .and_then(|record| record.end_point()),
        expected()
    );
}

#[test]
fn get_all_end_points_fetches_the_list() {
    let list = list_from_provider();

    let end_points: Vec<SocketAddr> = list
        .get_all_end_points()
        .into_iter()
        .filter_map(|record| record.end_point())
        .collect();

    assert_eq!(end_points, vec![SocketAddr::from(END_POINT)]);
}
