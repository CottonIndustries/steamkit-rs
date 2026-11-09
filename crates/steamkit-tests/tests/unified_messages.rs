//! `SteamUnifiedMessages` subscription tests: subscribing creates the service
//! and registers the decoder, the way the C# `SubscribeServiceNotification`
//! does.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use steamkit::base::generated::steam_language::EResult;
use steamkit::steam::handlers::steam_unified_messages::callbacks::ServiceMethodNotification;
use steamkit::steam::handlers::steam_unified_messages::steam_unified_messages::{
    ServiceMessage, SteamUnifiedMessages,
};
use steamkit::steam::steam_client::callback_mgr::callback_msg::CallbackMsg;
use steamkit::steam::steam_client::steam_client::SteamClient;
use steamkit::types::job_id::JobID;
use steamkit_protos::steam::{
    CAuthenticationGetPasswordRsaPublicKeyRequest, CPlayerGetGameBadgeLevelsRequest,
};

/// The service and method the subscription names.
const SERVICE: &str = "Authentication";
const METHOD: &str = "GetPasswordRSAPublicKey";

#[tokio::test]
async fn subscribe_service_notification_creates_the_service_and_routes_notifications() {
    let client = SteamClient::new();

    let unified = client
        .get_required_handler::<SteamUnifiedMessages>()
        .expect("SteamUnifiedMessages is a default handler");

    let seen = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&seen);

    // One call: the service is created and the decoder registered on the way.
    let _subscription = client
        .callback_manager()
        .subscribe_service_notification::<CAuthenticationGetPasswordRsaPublicKeyRequest>(
            SERVICE,
            METHOD,
            Box::new(move |_notification| flag.store(true, Ordering::SeqCst)),
        )
        .expect("the subscription should succeed");

    let service = unified
        .create_service(SERVICE)
        .expect("the service should exist after subscribing");

    let message = ServiceMessage {
        job_id: JobID::INVALID,
        job_name: format!("{SERVICE}.{METHOD}#1"),
        service_name: SERVICE.to_string(),
        method_name: METHOD.to_string(),
        result: EResult::OK,
        body: Vec::new(),
    };

    service
        .handle_notification_msg(METHOD, &message)
        .expect("an empty body decodes to the default message");

    let callback = client
        .get_callback()
        .expect("the notification should be posted as a callback");
    client.callback_manager().dispatch(callback);

    assert!(seen.load(Ordering::SeqCst), "the subscriber should have run");
}

#[tokio::test]
async fn remove_and_recreate_keeps_the_registered_decoders() {
    let client = SteamClient::new();

    let unified = client
        .get_required_handler::<SteamUnifiedMessages>()
        .expect("SteamUnifiedMessages is a default handler");

    let seen = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&seen);

    let _subscription = client
        .callback_manager()
        .subscribe_service_notification::<CAuthenticationGetPasswordRsaPublicKeyRequest>(
            SERVICE,
            METHOD,
            Box::new(move |_notification| flag.store(true, Ordering::SeqCst)),
        )
        .expect("the subscription should succeed");

    unified.remove_service(SERVICE);

    let service = unified
        .create_service(SERVICE)
        .expect("the service can be created again");

    let message = ServiceMessage {
        job_id: JobID::INVALID,
        job_name: format!("{SERVICE}.{METHOD}#1"),
        service_name: SERVICE.to_string(),
        method_name: METHOD.to_string(),
        result: EResult::OK,
        body: Vec::new(),
    };

    service
        .handle_notification_msg(METHOD, &message)
        .expect("an empty body decodes to the default message");

    let callback = client
        .get_callback()
        .expect("the notification should still be posted");
    client.callback_manager().dispatch(callback);

    assert!(
        seen.load(Ordering::SeqCst),
        "recreating the service must not lose the registered body type"
    );
}

#[tokio::test]
async fn a_second_body_type_for_the_same_method_is_ignored() {
    let client = SteamClient::new();

    let unified = client
        .get_required_handler::<SteamUnifiedMessages>()
        .expect("SteamUnifiedMessages is a default handler");

    let service = unified
        .create_service(SERVICE)
        .expect("the service can be created");

    service.set_notification_type::<CAuthenticationGetPasswordRsaPublicKeyRequest>(METHOD);
    service.set_notification_type::<CPlayerGetGameBadgeLevelsRequest>(METHOD);

    let message = ServiceMessage {
        job_id: JobID::INVALID,
        job_name: format!("{SERVICE}.{METHOD}#1"),
        service_name: SERVICE.to_string(),
        method_name: METHOD.to_string(),
        result: EResult::OK,
        body: Vec::new(),
    };

    service
        .handle_notification_msg(METHOD, &message)
        .expect("an empty body decodes to the default message");

    let callback = client
        .get_callback()
        .expect("the notification should be posted");

    assert!(
        callback
            .as_any()
            .downcast_ref::<ServiceMethodNotification<CAuthenticationGetPasswordRsaPublicKeyRequest>>()
            .is_some(),
        "the first registered body type must win"
    );
}
