//! `GET /_matrix/client/*/rooms/{roomId}/state/{eventType}/{stateKey}`
//!
//! Get state events associated with a given key.

pub mod v3 {
    //! `/v3/` ([spec])
    //!
    //! [spec]: https://spec.matrix.org/latest/client-server-api/#get_matrixclientv3roomsroomidstateeventtypestatekey

    use ruma_common::{
        api::{response, Metadata},
        metadata, OwnedRoomId,
    };
    use ruma_events::StateEventType;

    const METADATA: Metadata = metadata! {
        method: GET,
        rate_limited: false,
        authentication: AccessToken,
        history: {
            1.0 => "/_matrix/client/r0/rooms/:room_id/state/:event_type/:state_key",
            1.1 => "/_matrix/client/v3/rooms/:room_id/state/:event_type/:state_key",
        }
    };

    /// Request type for the `get_state_events_for_key` endpoint.
    #[derive(Clone, Debug)]
    #[cfg_attr(not(feature = "unstable-exhaustive-types"), non_exhaustive)]
    pub struct Request {
        /// The room to look up the state for.
        pub room_id: OwnedRoomId,

        /// The type of state to look up.
        pub event_type: StateEventType,

        /// The key of the state to look up.
        pub state_key: String,

        /// Optional parameter to return the event content
        /// or the full state event.
        pub format: Option<String>,
    }

    impl Request {
        /// Creates a new `Request` with the given room ID, event type and state key.
        pub fn new(room_id: OwnedRoomId, event_type: StateEventType, state_key: String) -> Self {
            Self { room_id, event_type, state_key, format: None }
        }
    }

    /// Response type for the `get_state_events_for_key` endpoint.
    #[response(error = crate::Error)]
    pub struct Response {
        /// The content of the state event.
        ///
        /// This is `serde_json::Value` due to complexity issues with returning only the
        /// actual JSON content without a top level key.
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        pub content: Option<serde_json::Value>,

        /// The full state event
        ///
        /// This is `serde_json::Value` due to complexity issues with returning only the
        /// actual JSON content without a top level key.
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        pub event: Option<serde_json::Value>,
    }

    impl Response {
        /// Creates a new `Response` with the given content.
        pub fn new(content: serde_json::Value, event: serde_json::Value) -> Self {
            Self { content: Some(content), event: Some(event) }
        }
    }

    #[cfg(feature = "client")]
    impl ruma_common::api::OutgoingRequest for Request {
        type EndpointError = crate::Error;
        type IncomingResponse = Response;

        const METADATA: Metadata = METADATA;

        fn try_into_http_request<T: Default + bytes::BufMut>(
            self,
            base_url: &str,
            access_token: ruma_common::api::SendAccessToken<'_>,
            considering_versions: &'_ [ruma_common::api::MatrixVersion],
        ) -> Result<http::Request<T>, ruma_common::api::error::IntoHttpError> {
            use http::header;

            let query_string = serde_html_form::to_string(RequestQuery { format: self.format })?;

            http::Request::builder()
                .method(http::Method::GET)
                .uri(METADATA.make_endpoint_url(
                    considering_versions,
                    base_url,
                    &[&self.room_id, &self.event_type, &self.state_key],
                    &query_string,
                )?)
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::AUTHORIZATION,
                    format!(
                        "Bearer {}",
                        access_token
                            .get_required_for_endpoint()
                            .ok_or(ruma_common::api::error::IntoHttpError::NeedsAuthentication)?,
                    ),
                )
                .body(T::default())
                .map_err(Into::into)
        }
    }

    #[cfg(feature = "server")]
    impl ruma_common::api::IncomingRequest for Request {
        type EndpointError = crate::Error;
        type OutgoingResponse = Response;

        const METADATA: Metadata = METADATA;

        fn try_from_http_request<B, S>(
            request: http::Request<B>,
            path_args: &[S],
        ) -> Result<Self, ruma_common::api::error::FromHttpRequestError>
        where
            B: AsRef<[u8]>,
            S: AsRef<str>,
        {
            // FIXME: find a way to make this if-else collapse with serde recognizing trailing
            // Option
            let (room_id, event_type, state_key): (OwnedRoomId, StateEventType, String) =
                if path_args.len() == 3 {
                    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                        _,
                        serde::de::value::Error,
                    >::new(
                        path_args.iter().map(::std::convert::AsRef::as_ref),
                    ))?
                } else {
                    let (a, b) =
                        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                            _,
                            serde::de::value::Error,
                        >::new(
                            path_args.iter().map(::std::convert::AsRef::as_ref),
                        ))?;

                    (a, b, "".into())
                };

            let request_query: RequestQuery =
                serde_html_form::from_str(request.uri().query().unwrap_or(""))?;

            Ok(Self { room_id, event_type, state_key, format: request_query.format })
        }
    }

    /// Data in the request's query string.
    #[derive(Debug)]
    #[cfg_attr(feature = "client", derive(serde::Serialize))]
    #[cfg_attr(feature = "server", derive(serde::Deserialize))]
    struct RequestQuery {
        /// Optional parameter to return the event content
        /// or the full state event.
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<String>,
    }
}
