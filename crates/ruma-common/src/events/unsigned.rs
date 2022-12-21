use js_int::Int;
use serde::Deserialize;
use serde_json::{from_str as from_json_str, value::RawValue as RawJsonValue};

use super::{
    relation::BundledRelations, room::redaction::RoomRedactionEventContent, StateEventContent,
};
use crate::{
    serde::{CanBeEmpty, Raw},
    MilliSecondsSinceUnixEpoch, OwnedEventId, OwnedTransactionId, OwnedUserId,
};

/// Extra information about a message event that is not incorporated into the event's hash.
#[derive(Clone, Debug, Default, Deserialize)]
#[cfg_attr(not(feature = "unstable-exhaustive-types"), non_exhaustive)]
pub struct MessageLikeUnsigned {
    /// The time in milliseconds that has elapsed since the event was sent.
    ///
    /// This field is generated by the local homeserver, and may be incorrect if the local time on
    /// at least one of the two servers is out of sync, which can cause the age to either be
    /// negative or greater than it actually is.
    pub age: Option<Int>,

    /// The client-supplied transaction ID, if the client being given the event is the same one
    /// which sent it.
    pub transaction_id: Option<OwnedTransactionId>,

    /// [Bundled aggregations] of related child events.
    ///
    /// [Bundled aggregations]: https://spec.matrix.org/v1.4/client-server-api/#aggregations
    #[serde(rename = "m.relations", default)]
    pub relations: BundledRelations,
}

impl MessageLikeUnsigned {
    /// Create a new `Unsigned` with fields set to `None`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl CanBeEmpty for MessageLikeUnsigned {
    /// Whether this unsigned data is empty (all fields are `None`).
    ///
    /// This method is used to determine whether to skip serializing the `unsigned` field in room
    /// events. Do not use it to determine whether an incoming `unsigned` field was present - it
    /// could still have been present but contained none of the known fields.
    fn is_empty(&self) -> bool {
        self.age.is_none() && self.transaction_id.is_none() && self.relations.is_empty()
    }
}

/// Extra information about a state event that is not incorporated into the event's hash.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(not(feature = "unstable-exhaustive-types"), non_exhaustive)]
pub struct StateUnsigned<C: StateEventContent> {
    /// The time in milliseconds that has elapsed since the event was sent.
    ///
    /// This field is generated by the local homeserver, and may be incorrect if the local time on
    /// at least one of the two servers is out of sync, which can cause the age to either be
    /// negative or greater than it actually is.
    pub age: Option<Int>,

    /// The client-supplied transaction ID, if the client being given the event is the same one
    /// which sent it.
    pub transaction_id: Option<OwnedTransactionId>,

    /// Optional previous content of the event.
    pub prev_content: Option<C>,

    /// [Bundled aggregations] of related child events.
    ///
    /// [Bundled aggregations]: https://spec.matrix.org/v1.4/client-server-api/#aggregations
    #[serde(rename = "m.relations", default)]
    pub relations: BundledRelations,
}

impl<C: StateEventContent> StateUnsigned<C> {
    /// Create a new `Unsigned` with fields set to `None`.
    pub fn new() -> Self {
        Self { age: None, transaction_id: None, prev_content: None, relations: Default::default() }
    }
}

impl<C: StateEventContent> CanBeEmpty for StateUnsigned<C> {
    /// Whether this unsigned data is empty (all fields are `None`).
    ///
    /// This method is used to determine whether to skip serializing the `unsigned` field in room
    /// events. Do not use it to determine whether an incoming `unsigned` field was present - it
    /// could still have been present but contained none of the known fields.
    fn is_empty(&self) -> bool {
        self.age.is_none()
            && self.transaction_id.is_none()
            && self.prev_content.is_none()
            && self.relations.is_empty()
    }
}

/// Helper functions for proc-macro code.
///
/// Needs to be public for state events defined outside ruma-common.
#[doc(hidden)]
pub trait StateUnsignedFromParts: Sized {
    fn _from_parts(event_type: &str, object: &RawJsonValue) -> serde_json::Result<Self>;
}

impl<C: StateEventContent> StateUnsignedFromParts for StateUnsigned<C> {
    fn _from_parts(event_type: &str, object: &RawJsonValue) -> serde_json::Result<Self> {
        #[derive(Deserialize)]
        #[serde(bound = "")] // Disable default C: Deserialize bound
        struct WithRawPrevContent<C> {
            #[serde(skip_serializing_if = "Option::is_none")]
            age: Option<Int>,
            #[serde(skip_serializing_if = "Option::is_none")]
            transaction_id: Option<OwnedTransactionId>,
            prev_content: Option<Raw<C>>,
            #[serde(
                rename = "m.relations",
                default,
                skip_serializing_if = "BundledRelations::is_empty"
            )]
            relations: BundledRelations,
        }

        let raw: WithRawPrevContent<C> = from_json_str(object.get())?;
        let prev_content =
            raw.prev_content.map(|r| r.deserialize_content(event_type.into())).transpose()?;

        Ok(Self {
            age: raw.age,
            transaction_id: raw.transaction_id,
            relations: raw.relations,
            prev_content,
        })
    }
}

impl<C: StateEventContent> Default for StateUnsigned<C> {
    fn default() -> Self {
        Self::new()
    }
}

/// Extra information about a redacted event that is not incorporated into the event's hash.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(not(feature = "unstable-exhaustive-types"), non_exhaustive)]
pub struct RedactedUnsigned {
    /// The event that redacted this event, if any.
    pub redacted_because: UnsignedRoomRedactionEvent,
}

impl RedactedUnsigned {
    /// Create a new `RedactedUnsigned` with the given redaction event.
    pub fn new(redacted_because: UnsignedRoomRedactionEvent) -> Self {
        Self { redacted_because }
    }
}

/// A redaction event as found in `unsigned.redacted_because`.
///
/// While servers usually send this with the `redacts` field (unless nested), the ID of the event
/// being redacted is known from context wherever this type is used, so it's not reflected as a
/// field here.
///
/// It is intentionally not possible to create an instance of this type other than through `Clone`
/// or `Deserialize`.
#[derive(Clone, Debug, Deserialize)]
#[non_exhaustive]
pub struct UnsignedRoomRedactionEvent {
    /// Data specific to the event type.
    pub content: RoomRedactionEventContent,

    /// The globally unique event identifier for the user who sent the event.
    pub event_id: OwnedEventId,

    /// The fully-qualified ID of the user who sent this event.
    pub sender: OwnedUserId,

    /// Timestamp in milliseconds on originating homeserver when this event was sent.
    pub origin_server_ts: MilliSecondsSinceUnixEpoch,

    /// Additional key-value pairs not signed by the homeserver.
    #[serde(default)]
    pub unsigned: MessageLikeUnsigned,
}
