use crate::router::RouterId;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

#[derive(Debug)]
pub struct AppData(Box<dyn Any + Send + Sync>);

impl Default for AppData {
    fn default() -> Self {
        Self::new(())
    }
}

impl Deref for AppData {
    type Target = Box<dyn Any + Send + Sync>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AppData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AppData {
    pub fn new<T: Any + Send + Sync>(app_data: T) -> Self {
        Self(Box::new(app_data))
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransportListenIp {
    pub ip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announced_ip: Option<String>,
}

#[derive(Debug, Serialize, Copy, Clone)]
pub struct NumSctpStreams {
    /// Initially requested number of outgoing SCTP streams.
    #[serde(rename = "OS")]
    pub os: u16,
    /// Maximum number of incoming SCTP streams.
    #[serde(rename = "MIS")]
    pub mis: u16,
}

impl Default for NumSctpStreams {
    fn default() -> Self {
        Self {
            os: 1024,
            mis: 1024,
        }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IceRole {
    Controlled,
    Controlling,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IceParameters {
    pub username_fragment: String,
    pub password: String,
    pub ice_lite: Option<bool>,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IceCandidateType {
    Host,
    Srflx,
    Prflx,
    Relay,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IceCandidateTcpType {
    Passive,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TransportProtocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IceCandidate {
    pub foundation: String,
    pub priority: u32,
    pub ip: String,
    pub protocol: TransportProtocol,
    pub port: u16,
    pub r#type: IceCandidateType,
    pub tcp_type: Option<IceCandidateTcpType>,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IceState {
    New,
    Connected,
    Completed,
    Disconnected,
    Closed,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum TransportTuple {
    LocalOnly {
        // TODO: Maybe better type for IP address?
        local_ip: String,
        local_port: u16,
        protocol: TransportProtocol,
    },
    WithRemote {
        // TODO: Maybe better type for IP address?
        local_ip: String,
        local_port: u16,
        // TODO: Maybe better type for IP address?
        remote_ip: String,
        remote_port: u16,
        protocol: TransportProtocol,
    },
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DtlsState {
    New,
    Connecting,
    Connected,
    Failed,
    Closed,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SctpParameters {
    /// Must always equal 5000.
    pub port: u16,
    /// Initially requested number of outgoing SCTP streams.
    #[serde(rename = "OS")]
    pub os: u16,
    /// Maximum number of incoming SCTP streams.
    #[serde(rename = "MIS")]
    pub mis: u16,
    /// Maximum allowed size for SCTP messages.
    pub max_message_size: usize,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SctpState {
    New,
    Connecting,
    Connected,
    Failed,
    Closed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouterCreateAudioLevelObserverInternal {
    pub router_id: RouterId,
    pub rtp_observer_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum DtlsRole {
    Auto,
    Client,
    Server,
}

impl Default for DtlsRole {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DtlsFingerprint {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DtlsParameters {
    pub role: DtlsRole,
    pub fingerprints: Vec<DtlsFingerprint>,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EventDirection {
    In,
    Out,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub enum RtpType {
    #[serde(rename = "inbound-rtp")]
    Inbound,
    #[serde(rename = "outbound-rtp")]
    Outbound,
}
