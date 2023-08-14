// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::collections::HashMap;

use log::warn;
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};

pub type Object = HashMap<String, Value>;
pub type Array = Vec<Value>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Array),
    Object(Object),
}

impl Value {
    pub fn as_str(&self) -> Option<String> {
        match &self {
            Value::String(value) => Some(value.to_string()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match &self {
            Value::Bool(value) => Some(value.to_owned()),
            _ => None,
        }
    }

    // TODO: W-10705600: Allow Authentication Value to be parsed as f64, i64 and u64
    pub fn as_f64(&self) -> Option<f64> {
        match &self {
            Value::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_slice(&self) -> Option<&[Value]> {
        match &self {
            Value::Array(array) => Some(array),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Object> {
        match &self {
            Value::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn is_string(&self) -> bool {
        self.as_str().is_some()
    }

    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    pub fn is_f64(&self) -> bool {
        self.as_f64().is_some()
    }

    pub fn is_slice(&self) -> bool {
        self.as_slice().is_some()
    }

    pub fn is_object(&self) -> bool {
        self.as_object().is_some()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Authentication {
    principal: Option<String>,
    client_id: Option<String>,
    client_name: Option<String>,
    properties: Object,
}

impl Authentication {
    pub(crate) fn new(
        principal: Option<String>,
        client_id: Option<String>,
        client_name: Option<String>,
        properties: Object,
    ) -> Authentication {
        Self {
            principal,
            client_id,
            client_name,
            properties,
        }
    }

    pub fn principal(&self) -> Option<&str> {
        self.principal.as_deref()
    }

    pub fn properties(&self) -> &Object {
        &self.properties
    }

    pub fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
    }

    pub fn client_name(&self) -> Option<&str> {
        self.client_name.as_deref()
    }
}

#[derive(Default)]
pub struct AuthenticationBuilder {
    principal: Option<String>,
    client_id: Option<String>,
    client_name: Option<String>,
    properties: Object,
}

impl AuthenticationBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn principal(mut self, principal: &str) -> Self {
        self.principal = Some(principal.to_string());
        self
    }

    pub fn client_id(mut self, client_id: &str) -> Self {
        self.client_id = Some(client_id.to_string());
        self
    }

    pub fn client_name(mut self, client_name: &str) -> Self {
        self.client_name = Some(client_name.to_string());
        self
    }

    pub fn properties(mut self, properties: Object) -> Self {
        self.properties = properties;
        self
    }

    pub fn build(self) -> Authentication {
        Authentication {
            principal: self.principal,
            client_id: self.client_id,
            client_name: self.client_name,
            properties: self.properties,
        }
    }
}

pub struct AuthenticationUpdater<'a> {
    handler: &'a dyn AuthenticationHandler,
    principal: Option<String>,
    client_id: Option<String>,
    client_name: Option<String>,
    properties: Object,
}

/// An utility struct for easy [`Authentication`] updating.
///
/// [`AuthenticationUpdater`] is responsible for
/// * Updating authentication data,
///
impl<'a> AuthenticationUpdater<'a> {
    pub fn new(original: Authentication, handler: &'a dyn AuthenticationHandler) -> Self {
        AuthenticationUpdater {
            handler,
            principal: original.principal,
            client_id: original.client_id,
            client_name: original.client_name,
            properties: original.properties,
        }
    }

    pub fn with_client(mut self, client_id: Option<String>, client_name: Option<String>) -> Self {
        self.principal = client_id.clone();
        self.client_id = client_id;
        self.client_name = client_name;
        self
    }

    pub fn with_principal(mut self, principal: Option<String>) -> Self {
        self.principal = principal;
        self
    }

    pub fn with_properties(mut self, properties: Object) -> Self {
        let mut updated_properties = self.properties.clone();
        updated_properties.extend(properties);
        self.properties = updated_properties;
        self
    }

    pub fn update(self) -> Authentication {
        let authentication = Authentication::new(
            self.principal,
            self.client_id,
            self.client_name,
            self.properties,
        );
        self.handler.set_authentication(&authentication);
        authentication
    }
}

/// An interface of the `Flex` `Policy Context`.
///
/// [`AuthenticationHandler`] is responsible for
/// * Access authentication data,
/// * Update authentication data,
///
/// At the moment, [`AuthenticationHandler`] can be used for the [`HttpFilter`]
/// extension.
pub trait AuthenticationHandler {
    /// Returns the current authentication data if present.
    fn authentication(&self) -> Option<Authentication>;

    /// Sets the authentication data.
    ///
    /// # Arguments
    ///
    /// * `authentication` - authentication data.
    ///
    fn set_authentication(&self, authentication: &Authentication);

    /// Returns an [AuthenticationUpdater] for easy updating of the current authentication data.
    fn update_authentication(&self) -> AuthenticationUpdater;
}

impl dyn AuthenticationHandler {
    pub fn default() -> &'static dyn AuthenticationHandler {
        &impls::Host
    }
}

mod impls {
    use super::*;
    use crate::host::property::PropertyAccessor;
    use crate::policy_context::AUTHENTICATION_PROPERTY;

    struct DefaultAuthenticationHandler<'a> {
        property_accessor: &'a dyn PropertyAccessor,
    }

    impl Default for DefaultAuthenticationHandler<'static> {
        fn default() -> Self {
            Self {
                property_accessor: <dyn PropertyAccessor>::default(),
            }
        }
    }

    impl<'a> DefaultAuthenticationHandler<'a> {
        fn read_authentication(&self) -> Option<Authentication> {
            let bytes = self.property_accessor.read_property(AUTHENTICATION_PROPERTY)?;
            AuthenticationStreamSerializer::deserialize(bytes.as_slice())
        }

        fn write_authentication(&self, authentication: &Authentication) {
            if let Some(bytes) = AuthenticationStreamSerializer::serialize(authentication) {
                self.property_accessor
                    .set_property(AUTHENTICATION_PROPERTY, bytes.as_slice());
            }
        }
    }

    impl AuthenticationHandler for DefaultAuthenticationHandler<'_> {
        fn authentication(&self) -> Option<Authentication> {
            self.read_authentication()
        }

        fn set_authentication(&self, authentication: &Authentication) {
            self.write_authentication(authentication);
        }

        fn update_authentication(&self) -> AuthenticationUpdater {
            AuthenticationUpdater::new(self.authentication().unwrap_or_default(), self)
        }
    }

    /// Serializes and deserializes Authentication objects so that can be propagated between policies.
    /// The chosen serialization format is MessagePack. Using a cross-language format allows to
    /// propagate the object between filters that were coded in any language
    struct AuthenticationStreamSerializer;

    impl AuthenticationStreamSerializer {
        pub fn deserialize(bytes: &[u8]) -> Option<Authentication> {
            match rmp_serde::decode::from_read(bytes) {
                Ok(authentication) => Some(authentication),
                Err(err) => {
                    warn!(
                        "Unexpected error deserializing Authentication object: {}",
                        err
                    );
                    None
                }
            }
        }

        pub fn serialize(authentication: &Authentication) -> Option<Vec<u8>> {
            let mut buf = Vec::new();
            let result = authentication.serialize(&mut Serializer::new(&mut buf));

            match result {
                Ok(_) => Some(buf),
                Err(err) => {
                    warn!(
                        "Unexpected error serializing Authentication object: {}",
                        err
                    );
                    None
                }
            }
        }
    }

    pub(super) struct Host;

    impl AuthenticationHandler for Host {
        fn authentication(&self) -> Option<Authentication> {
            DefaultAuthenticationHandler::default().authentication()
        }

        fn set_authentication(&self, authentication: &Authentication) {
            DefaultAuthenticationHandler::default().set_authentication(authentication)
        }

        fn update_authentication(&self) -> AuthenticationUpdater {
            AuthenticationUpdater::new(self.authentication().unwrap_or_default(), self)
        }
    }

    #[cfg(test)]
    mod tests {
        use std::cell::RefCell;
        use std::collections::HashMap;

        use classy::proxy_wasm::types::Bytes;
        use serde_json::Error;

        use super::*;

        const KEY_1: &str = "key1";
        const KEY_2: &str = "key2";

        const VALUE: &str = "value2";

        const PRINCIPAL: &str = "principal";
        const CLIENT_ID: &str = "client_id";
        const CLIENT_NAME: &str = "client_name";

        #[derive(Default)]
        struct MockPropertyAccessor {
            properties: RefCell<HashMap<Vec<String>, Bytes>>,
        }

        impl MockPropertyAccessor {
            fn mock_handler(&self) -> DefaultAuthenticationHandler {
                DefaultAuthenticationHandler { property_accessor: self }
            }
        }

        impl PropertyAccessor for MockPropertyAccessor {
            fn read_property(&self, path: &[&str]) -> Option<Bytes> {
                let path: Vec<String> = path.to_vec().iter().map(|x| x.to_string()).collect();
                self.properties.take().get(&path).cloned()
            }

            fn set_property(&self, path: &[&str], value: &[u8]) {
                let path: Vec<String> = path.to_vec().iter().map(|x| x.to_string()).collect();
                let bytes = Bytes::from(value);
                self.properties.borrow_mut().insert(path.to_vec(), bytes);
            }
        }

        #[test]
        fn serialize_and_deserialize_authentication_to_bytes() {
            let auth = create_authentication();
            let property_accessor = MockPropertyAccessor::default();
            let auth_handler = property_accessor.mock_handler();

            auth_handler.set_authentication(&auth);
            let auth = auth_handler.authentication();

            assert_authentication(auth.clone());
            assert_eq!(auth.unwrap().properties().len(), 2);
        }

        #[test]
        fn handler_get_empty() {
            let property_accessor = MockPropertyAccessor::default();
            let auth_handler = property_accessor.mock_handler();

            let auth = auth_handler.authentication();

            assert!(auth.is_none())
        }

        #[test]
        fn handler_new_authentication_creates_auth_when_no_previous_data() {
            let property_accessor = MockPropertyAccessor::default();
            let auth_handler = property_accessor.mock_handler();

            let new_auth = auth_handler
                .update_authentication()
                .with_client(Some(CLIENT_ID.to_string()), Some(CLIENT_NAME.to_string()))
                .with_principal(Some(PRINCIPAL.to_string()))
                .with_properties(HashMap::from([
                    (KEY_1.to_string(), Value::Bool(true)),
                    (KEY_2.to_string(), Value::String(VALUE.to_string())),
                ]))
                .update();

            let auth = auth_handler.authentication();

            assert_authentication(auth.clone());
            assert_eq!(new_auth, auth.unwrap());
            assert_eq!(new_auth.properties().len(), 2);
        }

        #[test]
        fn handler_update_authentication_adding_properties_maintains_previous_data() {
            let property_accessor = MockPropertyAccessor::default();
            let auth_handler = property_accessor.mock_handler();

            auth_handler.set_authentication(&create_authentication());

            let new_property = ("new_property".to_string(), Value::Number(10_f64));

            let auth = auth_handler
                .update_authentication()
                .with_properties(HashMap::from([new_property.clone()]))
                .update();

            assert_eq!(
                auth.properties.get(new_property.0.as_str()),
                Some(&new_property.1)
            );
            assert_eq!(auth.properties().len(), 3);
            assert_authentication(Some(auth));
        }

        #[test]
        fn handler_update_authentication_property_is_overridden() {
            let property_accessor = MockPropertyAccessor::default();
            let auth_handler = property_accessor.mock_handler();

            auth_handler.set_authentication(&create_authentication());

            let property_to_override = (KEY_1.to_string(), Value::Number(10_f64));

            let auth = auth_handler
                .update_authentication()
                .with_properties(HashMap::from([property_to_override.clone()]))
                .update();

            assert_eq!(auth.principal, Some(PRINCIPAL.to_string()));
            assert_eq!(auth.client_id, Some(CLIENT_ID.to_string()));
            assert_eq!(auth.client_name, Some(CLIENT_NAME.to_string()));
            assert_eq!(auth.properties.get(KEY_1), Some(&property_to_override.1));
            assert_eq!(
                auth.properties.get(KEY_2),
                Some(&Value::String(VALUE.to_string()))
            );
            assert_eq!(auth.properties().len(), 2);
        }

        #[test]
        fn handler_update_authentication_updating_client_maintains_the_properties() {
            let property_accessor = MockPropertyAccessor::default();
            let auth_handler = property_accessor.mock_handler();

            auth_handler.set_authentication(&create_authentication());

            let new_client_id = Some(String::from("new client id"));
            let new_client_name = Some(String::from("new client id"));

            let auth = auth_handler
                .update_authentication()
                .with_client(new_client_id.clone(), new_client_name.clone())
                .update();

            assert_eq!(auth.principal, new_client_id);
            assert_eq!(auth.client_id, new_client_id);
            assert_eq!(auth.client_name, new_client_name);
            assert_eq!(auth.properties.get(KEY_1), Some(&Value::Bool(true)));
            assert_eq!(
                auth.properties.get(KEY_2),
                Some(&Value::String(VALUE.to_string()))
            );
            assert_eq!(auth.properties().len(), 2);
        }

        #[test]
        fn handler_update_authentication_updating_principal_maintains_the_client_and_properties() {
            let property_accessor = MockPropertyAccessor::default();
            let auth_handler = property_accessor.mock_handler();

            auth_handler.set_authentication(&create_authentication());

            let new_principal = Some(String::from("new principal"));

            let auth = auth_handler
                .update_authentication()
                .with_principal(new_principal.clone())
                .update();

            assert_eq!(auth.principal, new_principal);
            assert_eq!(auth.client_id, Some(CLIENT_ID.to_string()));
            assert_eq!(auth.client_name, Some(CLIENT_NAME.to_string()));
            assert_eq!(auth.properties.get(KEY_1), Some(&Value::Bool(true)));
            assert_eq!(
                auth.properties.get(KEY_2),
                Some(&Value::String(VALUE.to_string()))
            );
            assert_eq!(auth.properties().len(), 2);
        }

        #[test]
        fn deserialize_json_into_sdk_value() {
            let input = r#"{
                "scope":["read"],
                "exp": 1643981305,
                "active": false,
                "floating": 123.23,
                "signed": -123.23,
                "uid":"emmet.brown",
                "mail":"pablo.carballo+emmet@mulesoft.com",
                "sn":"Brown",
                "cn":"Emmet Brown Full",
                "realm":"/",
                "token_type":"Bearer",
                "client_id":"abdfo1RGs9Cgedi1RxTl8o8209821",
                "access_token":"e50ab980-d0af-4e1a-8aa5-422d8d1b2ef3",
                "property_with_object_value": {
                    "innerKey": "innerValue"
                },
                "null_property": null,
                "empty_object": {},
                "empty_array": []
           }"#;

            let result: Result<Value, Error> = serde_json::from_str(input);

            assert!(result.is_ok());

            let value: Value = result.unwrap();
            let object_option = value.as_object();

            assert!(object_option.is_some());

            let object = object_option.unwrap();
            let expected = expected_value();

            object
                .iter()
                .for_each(|(key, value)| assert_eq!(value, expected.get(key).unwrap()));
            assert_eq!(object.len(), expected.len());
        }

        #[test]
        fn deserialize_json_into_sdk_value_invalid_json() {
            let input = r#"{notAJson}"#;

            let result: Result<Value, Error> = serde_json::from_str(input);

            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err().to_string(),
                "key must be a string at line 1 column 2".to_string()
            );
        }

        #[test]
        fn deserialize_empty_string_json_into_sdk_value() {
            let input = r#""""#;

            let result: Result<Value, Error> = serde_json::from_str(input);

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Value::String("".to_string()));
        }

        #[test]
        fn deserialize_empty_object_json_into_sdk_value() {
            let input = r#"{}"#;

            let result: Result<Value, Error> = serde_json::from_str(input);

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Value::Object(HashMap::new()));
        }

        #[test]
        fn boolean_deserialized_is_not_string() {
            let input = r#"{ "active": false }"#;

            let result: Result<Value, Error> = serde_json::from_str(input);

            assert!(result.is_ok());

            let value: Value = result.unwrap();
            let map = value.as_object().unwrap();
            let active = map.get("active").unwrap();

            assert!(!active.is_string());
        }

        #[test]
        fn string_deserialized_is_string() {
            let input = r#"{ "uid":"emmet.brown" }"#;

            let result: Result<Value, Error> = serde_json::from_str(input);

            assert!(result.is_ok());

            let value: Value = result.unwrap();
            let map = value.as_object().unwrap();
            let uid = map.get("uid").unwrap();

            assert!(uid.is_string());
        }

        #[test]
        fn string_deserialized_is_not_slice() {
            let input = r#"{ "uid":"emmet.brown" }"#;

            let result: Result<Value, Error> = serde_json::from_str(input);

            assert!(result.is_ok());

            let value: Value = result.unwrap();
            let map = value.as_object().unwrap();
            let uid = map.get("uid").unwrap();

            assert!(!uid.is_slice());
        }

        #[test]
        fn array_deserialized_is_slice() {
            let input = r#"{ "scope":["read"] }"#;

            let result: Result<Value, Error> = serde_json::from_str(input);

            assert!(result.is_ok());

            let value: Value = result.unwrap();
            let map = value.as_object().unwrap();
            let scopes = map.get("scope").unwrap();

            assert!(scopes.is_slice());
        }

        fn assert_authentication(auth: Option<Authentication>) {
            assert!(auth.is_some());
            let unwrapped = auth.unwrap();
            assert_eq!(unwrapped.principal, Some(PRINCIPAL.to_string()));
            assert_eq!(unwrapped.client_id, Some(CLIENT_ID.to_string()));
            assert_eq!(unwrapped.client_name, Some(CLIENT_NAME.to_string()));
            assert_eq!(unwrapped.properties.get(KEY_1), Some(&Value::Bool(true)));
            assert_eq!(
                unwrapped.properties.get(KEY_2),
                Some(&Value::String(VALUE.to_string()))
            );
        }

        fn expected_value() -> HashMap<String, Value> {
            let array = vec![Value::String("read".to_string())];

            let inner = HashMap::from([(
                "innerKey".to_string(),
                Value::String("innerValue".to_string()),
            )]);

            HashMap::from([
                ("scope".to_string(), Value::Array(array)),
                ("exp".to_string(), Value::Number(1643981305_f64)),
                ("active".to_string(), Value::Bool(false)),
                ("floating".to_string(), Value::Number(123.23)),
                ("signed".to_string(), Value::Number(-123.23)),
                ("uid".to_string(), Value::String("emmet.brown".to_string())),
                (
                    "mail".to_string(),
                    Value::String("pablo.carballo+emmet@mulesoft.com".to_string()),
                ),
                ("sn".to_string(), Value::String("Brown".to_string())),
                (
                    "cn".to_string(),
                    Value::String("Emmet Brown Full".to_string()),
                ),
                ("realm".to_string(), Value::String("/".to_string())),
                (
                    "token_type".to_string(),
                    Value::String("Bearer".to_string()),
                ),
                (
                    "client_id".to_string(),
                    Value::String("abdfo1RGs9Cgedi1RxTl8o8209821".to_string()),
                ),
                (
                    "access_token".to_string(),
                    Value::String("e50ab980-d0af-4e1a-8aa5-422d8d1b2ef3".to_string()),
                ),
                (
                    "property_with_object_value".to_string(),
                    Value::Object(inner),
                ),
                ("null_property".to_string(), Value::Null),
                ("empty_object".to_string(), Value::Object(HashMap::new())),
                ("empty_array".to_string(), Value::Array(vec![])),
            ])
        }

        fn create_authentication() -> Authentication {
            Authentication {
                principal: Some(PRINCIPAL.to_string()),
                client_id: Some(CLIENT_ID.to_string()),
                client_name: Some(CLIENT_NAME.to_string()),
                properties: HashMap::from([
                    (KEY_1.to_string(), Value::Bool(true)),
                    (KEY_2.to_string(), Value::String(VALUE.to_string())),
                ]),
            }
        }
    }
}
