//! Steamworks unified messages (the RPC-style "services").
//!
//! prost doesn't generate service clients, so [`UnifiedService`] is a generic
//! client: you register the response and notification body types of the
//! methods you use with [`UnifiedService::set_response_type`] and
//! [`UnifiedService::set_notification_type`]. `subscribe_service_*` and
//! `send_message` do that for you.
//!
//! Responses are decoded by the service they belong to, like the C# typed
//! `UnifiedService`; a response for a service that was never created is
//! dropped, and the job waiting for it times out.

use std::any::TypeId;
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;

use crate::base::client_msg::{ClientMsgProtobuf, IClientMsg};
use crate::base::generated::steam_language::{EMsg, EResult};
use crate::base::generated::steam_language_internal::{
    MsgHdrProtoBuf, SteamSerializableDeserialize,
};
use crate::base::msg_base::Reader;
use crate::base::packet_base::IPacketMsg;
use crate::error::{SteamError, SteamResult};
use crate::steam::handlers::client_msg_handler::ClientMsgHandler;
use crate::steam::steam_client::callback_mgr::callback_msg::CallbackMsg;
use crate::steam::steam_client::steam_client::SteamClient;
use crate::types::job_id::{AsyncJob, JobID};

use super::callbacks::{ServiceMethodNotification, ServiceMethodResponse};

/// The version of the service methods.
pub const SERVICE_METHOD_VERSION: u32 = 1;

/// Builds the name of a service method, `Service.Method#Version`. That's what
/// Steam expects in the `target_job_name` field of the protobuf header.
pub fn service_method_name(service: &str, method: &str) -> String {
    format!("{service}.{method}#{SERVICE_METHOD_VERSION}")
}

/// The raw parts of a unified service message.
#[derive(Clone, Debug)]
pub struct ServiceMessage {
    /// The job id of the message.
    pub job_id: JobID,
    /// The full job name, `Service.Method#Version`.
    pub job_name: String,
    /// The service the message belongs to.
    pub service_name: String,
    /// The method the message belongs to.
    pub method_name: String,
    /// The result of the message.
    pub result: EResult,
    /// The raw protobuf body of the message.
    pub body: Vec<u8>,
}

/// Builds a callback from the raw parts of a service message.
type MessageDecoder =
    Arc<dyn Fn(&ServiceMessage) -> SteamResult<Box<dyn CallbackMsg>> + Send + Sync>;

/// A decoder together with the body type it decodes, so a second registration
/// with a different type can be rejected instead of silently replacing the
/// first one.
struct DecoderEntry {
    type_id: TypeId,
    decoder: MessageDecoder,
}

/// A client for one unified service.
pub struct UnifiedService {
    service_name: String,
    client: Weak<SteamClient>,
}

impl UnifiedService {
    /// The name of the service.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Registers the response body type of one of this service's methods, so
    /// responses can be turned into a [`ServiceMethodResponse`].
    pub fn set_response_type<T>(&self, method: &str)
    where
        T: prost::Message + Default + Clone + 'static,
    {
        let entry = DecoderEntry {
            type_id: TypeId::of::<T>(),
            decoder: Arc::new(
                |message: &ServiceMessage| -> SteamResult<Box<dyn CallbackMsg>> {
                    let callback = ServiceMethodResponse::<T>::from_parts(
                        message.job_id,
                        message.result,
                        message.body.as_slice(),
                    )?;

                    Ok(Box::new(callback) as Box<dyn CallbackMsg>)
                },
            ),
        };

        if let Ok(unified) = self.unified_messages() {
            unified.register_response_decoder(&self.service_name, method, entry);
        }
    }

    /// Registers the notification body type of one of this service's methods.
    pub fn set_notification_type<T>(&self, method: &str)
    where
        T: prost::Message + Default + Clone + 'static,
    {
        let entry = DecoderEntry {
            type_id: TypeId::of::<T>(),
            decoder: Arc::new(
                |message: &ServiceMessage| -> SteamResult<Box<dyn CallbackMsg>> {
                    let callback = ServiceMethodNotification::<T>::from_parts(
                        &message.job_name,
                        message.body.as_slice(),
                    )?;

                    Ok(Box::new(callback) as Box<dyn CallbackMsg>)
                },
            ),
        };

        if let Ok(unified) = self.unified_messages() {
            unified.register_notification_decoder(&self.service_name, method, entry);
        }
    }

    /// Handles a response for this service.
    pub fn handle_response_msg(
        &self,
        method_name: &str,
        message: &ServiceMessage,
    ) -> SteamResult<()> {
        self.unified_messages()?
            .handle_service_response(&self.service_name, method_name, message)
    }

    /// Handles a notification for this service.
    pub fn handle_notification_msg(
        &self,
        method_name: &str,
        message: &ServiceMessage,
    ) -> SteamResult<()> {
        self.unified_messages()?
            .handle_service_notification(&self.service_name, method_name, message)
    }

    /// Calls a method of this service. The response comes as a
    /// [`ServiceMethodResponse`].
    pub fn send_message<TRequest, TResult>(
        &self,
        method: &str,
        message: TRequest,
    ) -> SteamResult<AsyncJob<ServiceMethodResponse<TResult>>>
    where
        TRequest: prost::Message + Default,
        TResult: prost::Message + Default + Clone + 'static,
    {
        self.set_response_type::<TResult>(method);

        let name = service_method_name(&self.service_name, method);

        self.unified_messages()?.send_message(&name, message)
    }

    /// Sends a notification to a method of this service.
    pub fn send_notification<TRequest>(&self, method: &str, message: TRequest) -> SteamResult<()>
    where
        TRequest: prost::Message + Default,
    {
        let name = service_method_name(&self.service_name, method);
        let handler = self.unified_messages()?;

        handler.send_notification(&name, message)
    }

    /// The unified messages handler this service came from.
    pub fn unified_messages(&self) -> SteamResult<Arc<SteamUnifiedMessages>> {
        let client = self.client()?;
        client.get_required_handler::<SteamUnifiedMessages>()
    }

    /// Upgrades the weak client reference.
    fn client(&self) -> SteamResult<Arc<SteamClient>> {
        self.client
            .upgrade()
            .ok_or_else(|| SteamError::Connection("SteamClient has been dropped".to_string()))
    }
}

/// Steamworks unified messages.
pub struct SteamUnifiedMessages {
    client: Weak<SteamClient>,
    services: DashMap<String, Arc<UnifiedService>>,
    response_decoders: DashMap<(String, String), DecoderEntry>,
    notification_decoders: DashMap<(String, String), DecoderEntry>,
}

impl SteamUnifiedMessages {
    /// Creates the handler and stores a weak back-reference to the client.
    pub fn new(client: &Arc<SteamClient>) -> Self {
        Self {
            client: Arc::downgrade(client),
            services: DashMap::new(),
            response_decoders: DashMap::new(),
            notification_decoders: DashMap::new(),
        }
    }

    /// Upgrades the weak client reference.
    fn client(&self) -> SteamResult<Arc<SteamClient>> {
        self.client
            .upgrade()
            .ok_or_else(|| SteamError::Connection("SteamClient has been dropped".to_string()))
    }

    /// Creates a client for the named service, or returns the existing one.
    pub fn create_service(
        &self,
        service_name: impl Into<String>,
    ) -> SteamResult<Arc<UnifiedService>> {
        let client = self.client()?;
        let service_name = service_name.into();

        let entry = self.services.entry(service_name.clone()).or_insert_with(|| {
            Arc::new(UnifiedService {
                service_name,
                client: Arc::downgrade(&client),
            })
        });

        Ok(Arc::clone(entry.value()))
    }

    /// Removes a service; it can't send messages or get notifications anymore.
    /// The registered body types are kept, so creating the service again
    /// resumes delivery, like the C# generated switches.
    pub fn remove_service(&self, service_name: &str) {
        let _ = self.services.remove(service_name);
    }

    fn register_response_decoder(&self, service_name: &str, method: &str, entry: DecoderEntry) {
        let key = (service_name.to_string(), method.to_string());

        match self.response_decoders.entry(key) {
            Entry::Occupied(existing) => {
                if existing.get().type_id != entry.type_id {
                    tracing::warn!(
                        "Unified message response for {service_name}.{method} is already registered with another type"
                    );
                }
            }
            Entry::Vacant(slot) => {
                slot.insert(entry);
            }
        }
    }

    fn register_notification_decoder(&self, service_name: &str, method: &str, entry: DecoderEntry) {
        let key = (service_name.to_string(), method.to_string());

        match self.notification_decoders.entry(key) {
            Entry::Occupied(existing) => {
                if existing.get().type_id != entry.type_id {
                    tracing::warn!(
                        "Unified message notification for {service_name}.{method} is already registered with another type"
                    );
                }
            }
            Entry::Vacant(slot) => {
                slot.insert(entry);
            }
        }
    }

    fn handle_service_response(
        &self,
        service_name: &str,
        method_name: &str,
        message: &ServiceMessage,
    ) -> SteamResult<()> {
        let key = (service_name.to_string(), method_name.to_string());

        let Some(entry) = self.response_decoders.get(&key) else {
            return Ok(());
        };
        let decoder = Arc::clone(&entry.value().decoder);
        drop(entry);

        let callback = decoder(message)?;
        self.client()?.post_callback(callback);

        Ok(())
    }

    fn handle_service_notification(
        &self,
        service_name: &str,
        method_name: &str,
        message: &ServiceMessage,
    ) -> SteamResult<()> {
        let key = (service_name.to_string(), method_name.to_string());

        let Some(entry) = self.notification_decoders.get(&key) else {
            return Ok(());
        };
        let decoder = Arc::clone(&entry.value().decoder);
        drop(entry);

        let callback = decoder(message)?;
        self.client()?.post_callback(callback);

        Ok(())
    }

    /// Calls a service method. `name` is `Service.Method#Version` (see
    /// [`service_method_name`]). The response comes as a
    /// [`ServiceMethodResponse`] through the registered service.
    pub fn send_message<TRequest, TResult>(
        &self,
        name: &str,
        message: TRequest,
    ) -> SteamResult<AsyncJob<ServiceMethodResponse<TResult>>>
    where
        TRequest: prost::Message + Default,
        TResult: prost::Message + Default + Clone + 'static,
    {
        let client = self.client()?;

        let job_id = client.get_next_job_id();
        let job = client.start_job::<ServiceMethodResponse<TResult>>(job_id);

        let e_msg = if client.steam_id().is_some() {
            EMsg::ServiceMethodCallFromClient
        } else {
            EMsg::ServiceMethodCallFromClientNonAuthed
        };

        let mut msg = ClientMsgProtobuf::<TRequest>::new(e_msg);
        msg.set_source_job_id(job_id);
        msg.header_mut().proto.target_job_name = Some(name.to_string());
        msg.set_body(message);

        if let Err(error) = client.send(&msg) {
            client.job_manager().fail_job(job_id);
            return Err(error);
        }

        Ok(job)
    }

    /// Sends a notification. `name` is `Service.Method#Version` (see
    /// [`service_method_name`]).
    pub fn send_notification<TRequest>(&self, name: &str, message: TRequest) -> SteamResult<()>
    where
        TRequest: prost::Message + Default,
    {
        let client = self.client()?;

        // Notifications don't set a source job id; Steam rejects a
        // "Notification" method call that has one.
        let e_msg = if client.steam_id().is_some() {
            EMsg::ServiceMethodCallFromClient
        } else {
            EMsg::ServiceMethodCallFromClientNonAuthed
        };

        let mut msg = ClientMsgProtobuf::<TRequest>::new(e_msg);
        msg.header_mut().proto.target_job_name = Some(name.to_string());
        msg.set_body(message);

        client.send(&msg)
    }

    /// Handles the response to a service method.
    fn handle_response_msg(&self, packet_msg: &dyn IPacketMsg) -> SteamResult<()> {
        let message = read_service_message(packet_msg)?;

        if message.job_name.is_empty() {
            return Ok(());
        }

        if !self.services.contains_key(&message.service_name) {
            return Ok(());
        }

        self.handle_service_response(&message.service_name, &message.method_name, &message)
    }

    /// Handles a service notification.
    fn handle_notification_msg(&self, packet_msg: &dyn IPacketMsg) -> SteamResult<()> {
        let message = read_service_message(packet_msg)?;

        if message.job_name.is_empty() {
            return Ok(());
        }

        if !self.services.contains_key(&message.service_name) {
            return Ok(());
        }

        self.handle_service_notification(&message.service_name, &message.method_name, &message)
    }
}

/// Reads the header and body of an incoming unified message.
fn read_service_message(packet_msg: &dyn IPacketMsg) -> SteamResult<ServiceMessage> {
    let data = packet_msg.get_data();
    let mut reader = Reader::new(data);
    let header = MsgHdrProtoBuf::read(&mut reader)?;

    let body = data.get(reader.position()..).unwrap_or_default().to_vec();

    let job_name = header.proto.target_job_name.clone().unwrap_or_default();
    let result =
        EResult::from_value(header.proto.eresult.unwrap_or_default()).unwrap_or(EResult::Invalid);

    let (service_name, method_name) = split_job_name(&job_name).unwrap_or_default();

    Ok(ServiceMessage {
        job_id: packet_msg.target_job_id(),
        job_name,
        service_name,
        method_name,
        result,
        body,
    })
}

/// Splits a `Service.Method#Version` job name into service and method.
fn split_job_name(job_name: &str) -> Option<(String, String)> {
    let dot = job_name.find('.')?;
    let hash = job_name.rfind('#')?;

    if hash <= dot {
        return None;
    }

    Some((
        job_name[..dot].to_string(),
        job_name[dot + 1..hash].to_string(),
    ))
}

#[async_trait]
impl ClientMsgHandler for SteamUnifiedMessages {
    async fn handle_msg(
        &self,
        _client: &SteamClient,
        packet_msg: &dyn IPacketMsg,
    ) -> SteamResult<()> {
        match packet_msg.msg_type() {
            EMsg::ServiceMethodResponse => {
                if !packet_msg.is_proto() {
                    return Ok(());
                }

                self.handle_response_msg(packet_msg)
            }
            EMsg::ServiceMethod => {
                if !packet_msg.is_proto() {
                    return Ok(());
                }

                self.handle_notification_msg(packet_msg)
            }
            // Not a message we handle.
            _ => Ok(()),
        }
    }
}
