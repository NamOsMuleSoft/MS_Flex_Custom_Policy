// Copyright 2023 Salesforce, Inc. All rights reserved.
use classy::event::HeadersAccessor;
use pdk_core::{
    log::trace,
    policy_context::{authentication::Authentication, PolicyContext},
};
use pel::{
    expression::Symbol,
    runtime::{value::Object, Binding, Context, ValueHandler},
    ContextId, Reference,
};
use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
};

pub mod convert;
mod custom_getrandom;
mod error;
mod resolver;

pub use error::ExpressionError;
pub use pel::runtime::value::Value;
pub use resolver::{Expression, ExpressionResolver};

// Keys
const ATTRIBUTES: &str = "attributes";
const AUTHENTICATION: &str = "authentication";
const HEADERS: &str = "headers";
const METHOD: &str = "method";
const PAYLOAD: &str = "payload";
const QUERY_PARAMS: &str = "queryParams";
const REQUEST_PATH: &str = "requestPath";
const REQUEST_URI: &str = "requestUri";
const REMOTE_ADDRESS: &str = "remoteAddress";
const STATUS_CODE: &str = "statusCode";
const LOCAL_ADDRESS: &str = "localAddress";
const QUERY_STRING: &str = "queryString";
const SCHEME: &str = "scheme";
const VARS: &str = "vars";
const VERSION: &str = "version";

// Authentication Keys
const CLIENT_ID: &str = "clientId";
const CLIENT_NAME: &str = "clientName";
const PRINCIPAL: &str = "principal";
const PROPERTIES: &str = "properties";

// References
const CONTEXT_ID: ContextId = ContextId::new(module_path!());
const ATTRIBUTES_REFERENCE: Reference = CONTEXT_ID.first_reference();
const AUTHENTICATION_REFERENCE: Reference = ATTRIBUTES_REFERENCE.next();
const HEADERS_REFERENCE: Reference = AUTHENTICATION_REFERENCE.next();
const QUERY_PARAMS_REFERENCE: Reference = HEADERS_REFERENCE.next();
const VARS_REFERENCE: Reference = QUERY_PARAMS_REFERENCE.next();

// Headers
const METHOD_HEADER: &str = ":method";
const PATH_HEADER: &str = ":path";
const STATUS_CODE_HEADER: &str = ":status";

pub(crate) type Vars<'a> = &'a HashMap<&'a str, Value>;

struct OnPayloadContext {
    payload: String,
}

impl Context for OnPayloadContext {
    fn resolve(&self, symbol: &Symbol) -> Binding {
        match symbol.as_str() {
            PAYLOAD => Binding::Available(Value::string(self.payload.clone())),
            _ => Binding::Unknown,
        }
    }

    fn value_handler(&self, _reference: Reference) -> Option<&dyn ValueHandler> {
        None
    }
}

trait OpsContext: Clone {
    fn header(&self, name: &str) -> Option<String>;

    fn headers(&self) -> Vec<(String, String)>;

    fn policy_context(&self) -> &dyn PolicyContext;
}

pub enum EvaluationMode {
    Complete,
    Partial,
}

impl EvaluationMode {
    fn resolve_pending(&self) -> Binding {
        match *self {
            EvaluationMode::Complete => Binding::Unknown,
            EvaluationMode::Partial => Binding::Pending,
        }
    }
}

#[derive(Clone)]
struct HeadersOpsContext<'a> {
    policy_context: &'a dyn PolicyContext,
    accessor: &'a dyn HeadersAccessor,
}

impl<'a> OpsContext for HeadersOpsContext<'a> {
    fn header(&self, name: &str) -> Option<String> {
        self.accessor.header(name)
    }

    fn headers(&self) -> Vec<(String, String)> {
        self.accessor.headers()
    }

    fn policy_context(&self) -> &dyn PolicyContext {
        self.policy_context
    }
}

struct RequestOpsContextWrapper<'a, C: OpsContext> {
    evaluation_mode: EvaluationMode,
    attributes: RequestAttributesHandler<C>,
    authentication: AuthenticationHandler<C>,
    vars: VarsHandler<'a>,
}

impl<'a, C: OpsContext> RequestOpsContextWrapper<'a, C> {
    pub fn new(evaluation_mode: EvaluationMode, source: C, vars: Vars<'a>) -> Self {
        Self {
            evaluation_mode,
            attributes: RequestAttributesHandler::new(source.clone()),
            authentication: AuthenticationHandler::new(source),
            vars: VarsHandler::new(vars),
        }
    }
}

struct ResponseOpsContextWrapper<'a, C: OpsContext> {
    evaluation_mode: EvaluationMode,
    attributes: ResponseAttributesHandler<C>,
    authentication: AuthenticationHandler<C>,
    vars: VarsHandler<'a>,
}

impl<'a, C: OpsContext> ResponseOpsContextWrapper<'a, C> {
    pub fn new(evaluation_mode: EvaluationMode, source: C, vars: Vars<'a>) -> Self {
        Self {
            evaluation_mode,
            attributes: ResponseAttributesHandler::new(source.clone()),
            authentication: AuthenticationHandler::new(source),
            vars: VarsHandler::new(vars),
        }
    }
}

fn fake_url(uri: &str) -> Option<url::Url> {
    url::Url::parse("http://fake_base").ok()?.join(uri).ok()
}

fn extract_query_param(uri: &str, name: &str) -> Option<String> {
    fake_url(uri)?
        .query_pairs()
        .find_map(|(key, value)| (key == name).then(|| value.to_string()))
}

fn extract_query_params(uri: &str) -> Option<Object> {
    Some(
        fake_url(uri)?
            .query_pairs()
            .map(|(k, v)| (k.to_string(), Value::string(v.to_string())))
            .collect(),
    )
}

fn extract_query_string(uri: &str) -> Option<String> {
    fake_url(uri)?.query().map(|s| s.to_string())
}

fn extract_path(uri: &str) -> Option<String> {
    let mut url = fake_url(uri)?;
    url.set_query(None);
    Some(url.path().to_string())
}

struct RequestAttributesHandler<C> {
    source: C,
    headers: HeadersHandler<C>,
    query_params: QueryParamsHandler<C>,
}

impl<C: OpsContext> RequestAttributesHandler<C> {
    fn new(source: C) -> Self {
        Self {
            source: source.clone(),
            headers: HeadersHandler {
                source: source.clone(),
            },
            query_params: QueryParamsHandler { source },
        }
    }

    fn header(&self, name: &str) -> Option<Value> {
        self.source.header(name).map(Value::string)
    }

    fn method(&self) -> Option<Value> {
        self.header(METHOD_HEADER)
    }

    fn uri(&self) -> Option<Value> {
        self.header(PATH_HEADER)
    }

    fn path(&self) -> Option<Value> {
        self.source
            .header(PATH_HEADER)
            .and_then(|s| extract_path(&s))
            .map(Value::string)
    }

    fn remote_address(&self) -> Option<Value> {
        let address = self
            .source
            .policy_context()
            .connection_properties()
            .source()
            .address()
            .ok()?
            .map(Value::string)
            .unwrap_or_else(Value::null);
        Some(address)
    }

    fn local_address(&self) -> Option<Value> {
        let address = self
            .source
            .policy_context()
            .connection_properties()
            .destination()
            .address()
            .ok()?
            .map(Value::string)
            .unwrap_or_else(Value::null);
        Some(address)
    }

    fn query_string(&self) -> Option<Value> {
        let path = self.source.header(PATH_HEADER)?;
        let query_string = extract_query_string(&path)
            .map(Value::string)
            .unwrap_or_else(Value::null);
        Some(query_string)
    }

    fn scheme(&self) -> Option<Value> {
        let address = self
            .source
            .policy_context()
            .connection_properties()
            .request()
            .scheme()
            .ok()?
            .map(Value::string)
            .unwrap_or_else(Value::null);
        Some(address)
    }

    fn version(&self) -> Option<Value> {
        let address = self
            .source
            .policy_context()
            .connection_properties()
            .request()
            .protocol()
            .ok()?
            .map(Value::string)
            .unwrap_or_else(Value::null);
        Some(address)
    }
}

impl<C: OpsContext> ValueHandler for RequestAttributesHandler<C> {
    fn detach(&self) -> Option<Value> {
        let values = [
            (HEADERS, self.headers.detach()),
            (METHOD, self.method()),
            (QUERY_PARAMS, self.query_params.detach()),
            (REQUEST_PATH, self.path()),
            (REQUEST_URI, self.uri()),
            (REMOTE_ADDRESS, self.remote_address()),
            (LOCAL_ADDRESS, self.local_address()),
            (QUERY_STRING, self.query_string()),
            (SCHEME, self.scheme()),
            (VERSION, self.version()),
        ]
        .map(|(k, v)| (k.to_string(), v.unwrap_or_else(Value::null)));

        Some(Value::object(values.into()))
    }

    fn select_by_key(&self, key: &str) -> Option<Value> {
        let selection = match key {
            HEADERS => Some(Value::reference(HEADERS_REFERENCE)),
            METHOD => self.method(),
            QUERY_PARAMS => Some(Value::reference(QUERY_PARAMS_REFERENCE)),
            REQUEST_PATH => self.path(),
            REQUEST_URI => self.uri(),
            REMOTE_ADDRESS => self.remote_address(),
            LOCAL_ADDRESS => self.local_address(),
            QUERY_STRING => self.query_string(),
            SCHEME => self.scheme(),
            VERSION => self.version(),
            _ => None,
        };
        Some(selection.unwrap_or_else(Value::null))
    }
}

struct ResponseAttributesHandler<C> {
    source: C,
    headers: HeadersHandler<C>,
}

impl<C: OpsContext> ResponseAttributesHandler<C> {
    fn new(source: C) -> Self {
        Self {
            source: source.clone(),
            headers: HeadersHandler { source },
        }
    }

    fn header(&self, name: &str) -> Option<Value> {
        self.source.header(name).map(Value::string)
    }

    fn status_code(&self) -> Option<Value> {
        self.header(STATUS_CODE_HEADER).map(|value| {
            match value.as_str().map(|value| value.parse::<u32>()) {
                None => Value::null(),
                Some(Err(err)) => {
                    trace!("Unexpected error parsing status code: {:?}", err);
                    Value::null()
                }
                Some(Ok(number)) => Value::number(number as f64),
            }
        })
    }
}

impl<C: OpsContext> ValueHandler for ResponseAttributesHandler<C> {
    fn detach(&self) -> Option<Value> {
        let values = [
            (HEADERS, self.headers.detach()),
            (STATUS_CODE, self.status_code()),
        ]
        .map(|(k, v)| (k.to_string(), v.unwrap_or_else(Value::null)));

        Some(Value::object(values.into()))
    }

    fn select_by_key(&self, key: &str) -> Option<Value> {
        let selection = match key {
            HEADERS => Some(Value::reference(HEADERS_REFERENCE)),
            STATUS_CODE => self.status_code(),
            _ => None,
        };

        Some(selection.unwrap_or_else(Value::null))
    }
}

struct AuthenticationHandler<C> {
    source: C,
    authentication: RefCell<Option<Option<Authentication>>>,
    properties: RefCell<Option<Option<Value>>>,
}

impl<C: OpsContext> AuthenticationHandler<C> {
    fn new(source: C) -> Self {
        Self {
            source,
            authentication: RefCell::new(None),
            properties: RefCell::new(None),
        }
    }

    fn authentication(&self) -> RefMut<Option<Authentication>> {
        RefMut::map(self.authentication.borrow_mut(), |a| {
            a.get_or_insert_with(|| {
                self.source
                    .policy_context()
                    .authentication_handler()
                    .authentication()
            })
        })
    }

    fn client_id(&self) -> Option<Value> {
        let authentication = self.authentication();
        let authentication = authentication.as_ref()?;
        let client_id = authentication.client_id()?;
        Some(Value::string(client_id.to_string()))
    }

    fn client_name(&self) -> Option<Value> {
        let authentication = self.authentication();
        let authentication = authentication.as_ref()?;
        let client_name = authentication.client_name()?;
        Some(Value::string(client_name.to_string()))
    }

    fn principal(&self) -> Option<Value> {
        let authentication = self.authentication();
        let authentication = authentication.as_ref()?;
        let principal = authentication.principal()?;
        Some(Value::string(principal.to_string()))
    }

    fn properties(&self) -> Option<Value> {
        self.properties
            .borrow_mut()
            .get_or_insert_with(|| {
                let authentication = self.authentication();
                let authentication = authentication.as_ref()?;
                let properties =
                    convert::authentication_object_to_value(authentication.properties());
                Some(properties)
            })
            .clone()
    }
}

impl<C: OpsContext> ValueHandler for AuthenticationHandler<C> {
    fn select_by_key(&self, key: &str) -> Option<Value> {
        let result = match key {
            CLIENT_ID => self.client_id(),
            CLIENT_NAME => self.client_name(),
            PRINCIPAL => self.principal(),
            PROPERTIES => self.properties(),
            _ => None,
        };
        Some(result.unwrap_or_else(Value::null))
    }

    fn detach(&self) -> Option<Value> {
        let values = [
            (CLIENT_ID, self.client_id()),
            (CLIENT_NAME, self.client_name()),
            (PRINCIPAL, self.principal()),
            (PROPERTIES, self.properties()),
        ]
        .map(|(k, v)| (k.to_string(), v.unwrap_or_else(Value::null)));

        Some(Value::object(values.into()))
    }
}

struct HeadersHandler<C> {
    source: C,
}

impl<C: OpsContext> ValueHandler for HeadersHandler<C> {
    fn detach(&self) -> Option<Value> {
        Some(Value::object(
            self.source
                .headers()
                .into_iter()
                .map(|(k, v)| (k, Value::string(v)))
                .collect(),
        ))
    }

    fn select_by_key(&self, key: &str) -> Option<Value> {
        Some(
            self.source
                .header(key)
                .map(Value::string)
                .unwrap_or_else(Value::null),
        )
    }
}

struct QueryParamsHandler<S> {
    source: S,
}

impl<C: OpsContext> ValueHandler for QueryParamsHandler<C> {
    fn detach(&self) -> Option<Value> {
        self.source
            .header(PATH_HEADER)
            .and_then(|path| extract_query_params(&path))
            .map(Value::object)
    }

    fn select_by_key(&self, key: &str) -> Option<Value> {
        self.source
            .header(PATH_HEADER)
            .and_then(|path| extract_query_param(&path, key))
            .map(Value::string)
    }
}

struct VarsHandler<'a> {
    vars: Vars<'a>,
}

impl<'a> VarsHandler<'a> {
    fn new(vars: Vars<'a>) -> Self {
        Self { vars }
    }
}

impl<'a> ValueHandler for VarsHandler<'a> {
    fn select_by_key(&self, key: &str) -> Option<Value> {
        self.vars
            .get(key)
            .cloned()
            .unwrap_or_else(Value::null)
            .into()
    }

    fn detach(&self) -> Option<Value> {
        Value::object(
            self.vars
                .iter()
                .map(|(key, value)| (key.to_string(), value.clone()))
                .collect(),
        )
        .into()
    }
}

impl<C: OpsContext> Context for RequestOpsContextWrapper<'_, C> {
    fn resolve(&self, symbol: &Symbol) -> Binding {
        match symbol.as_str() {
            ATTRIBUTES => Binding::Available(Value::reference(ATTRIBUTES_REFERENCE)),
            AUTHENTICATION => Binding::Available(Value::reference(AUTHENTICATION_REFERENCE)),
            PAYLOAD => self.evaluation_mode.resolve_pending(),
            VARS => Binding::Available(Value::reference(VARS_REFERENCE)),
            _ => Binding::Unknown,
        }
    }

    fn value_handler(&self, reference: Reference) -> Option<&dyn ValueHandler> {
        match reference {
            ATTRIBUTES_REFERENCE => Some(&self.attributes),
            AUTHENTICATION_REFERENCE => Some(&self.authentication),
            HEADERS_REFERENCE => Some(&self.attributes.headers),
            QUERY_PARAMS_REFERENCE => Some(&self.attributes.query_params),
            VARS_REFERENCE => Some(&self.vars),
            _ => None,
        }
    }
}

impl<'a, C: OpsContext> Context for ResponseOpsContextWrapper<'a, C> {
    fn resolve(&self, symbol: &Symbol) -> Binding {
        match symbol.as_str() {
            ATTRIBUTES => Binding::Available(Value::reference(ATTRIBUTES_REFERENCE)),
            AUTHENTICATION => Binding::Available(Value::reference(AUTHENTICATION_REFERENCE)),
            PAYLOAD => self.evaluation_mode.resolve_pending(),
            VARS => Binding::Available(Value::reference(VARS_REFERENCE)),
            _ => Binding::Unknown,
        }
    }

    fn value_handler(&self, reference: Reference) -> Option<&dyn ValueHandler> {
        match reference {
            ATTRIBUTES_REFERENCE => Some(&self.attributes),
            AUTHENTICATION_REFERENCE => Some(&self.authentication),
            HEADERS_REFERENCE => Some(&self.attributes.headers),
            VARS_REFERENCE => Some(&self.vars),
            _ => None,
        }
    }
}

fn request_headers_context<'a>(
    policy_context: &'a dyn PolicyContext,
    accessor: &'a dyn HeadersAccessor,
    evaluation_mode: EvaluationMode,
    vars: Vars<'a>,
) -> impl Context + 'a {
    RequestOpsContextWrapper::new(
        evaluation_mode,
        HeadersOpsContext {
            policy_context,
            accessor,
        },
        vars,
    )
}

fn response_headers_context<'a>(
    policy_context: &'a dyn PolicyContext,
    accessor: &'a dyn HeadersAccessor,
    evaluation_mode: EvaluationMode,
    vars: Vars<'a>,
) -> impl Context + 'a {
    ResponseOpsContextWrapper::new(
        evaluation_mode,
        HeadersOpsContext {
            policy_context,
            accessor,
        },
        vars,
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::convert::IntoValue;
    use mockall::mock;
    use mockall::predicate::eq;
    use pdk_core::host::property::PropertyAccessor;
    use pdk_core::policy_context::authentication;
    use pdk_core::policy_context::authentication::AuthenticationBuilder;
    use pdk_core::policy_context::metadata::PolicyMetadata;
    use pel::{parser::Parser, runtime::Runtime};
    use std::rc::Rc;

    use super::*;

    static HEADERS: &[(&str, &str)] = &[
        (":method", "GET"),
        (":path", "/something?baz=bal&foo=bar"),
        ("Content-Length", "1024"),
        ("Content-Type", "application/json"),
        ("Content-Type", "text/html"),
        (":status", "207"),
    ];

    #[derive(Debug)]
    struct MockPropertyAccessor;

    impl PropertyAccessor for MockPropertyAccessor {
        fn read_property(&self, path: &[&str]) -> Option<Vec<u8>> {
            match path {
                ["destination", "address"] => Some("172.25.0.7:7890".as_bytes().to_vec()),
                ["request", "query"] => Some("baz=bal&foo=bar".as_bytes().to_vec()),
                ["request", "scheme"] => Some("http".as_bytes().to_vec()),
                ["request", "protocol"] => Some("HTTP/1.1".as_bytes().to_vec()),
                ["source", "address"] => Some("172.18.0.1:60686".as_bytes().to_vec()),
                _ => None,
            }
        }

        fn set_property(&self, _: &[&str], _: &[u8]) {
            unimplemented!()
        }
    }

    #[derive(Debug)]
    pub struct MockPolicyContext;

    impl PolicyContext for MockPolicyContext {
        fn policy_metadata(&self) -> Rc<PolicyMetadata> {
            unimplemented!()
        }

        fn connection_properties(&self) -> &dyn PropertyAccessor {
            &MockPropertyAccessor
        }

        fn authentication_handler(&self) -> &dyn authentication::AuthenticationHandler {
            &MockAuthenticationHandler
        }
    }

    #[derive(Debug)]
    struct MockAuthenticationHandler;

    impl authentication::AuthenticationHandler for MockAuthenticationHandler {
        fn authentication(&self) -> Option<Authentication> {
            let authentication = AuthenticationBuilder::new()
                .principal("PRINCIPAL")
                .client_id("CLIENT_ID")
                .client_name("CLIENT_NAME")
                .properties(authentication::Object::from([(
                    "foo".to_string(),
                    authentication::Value::Number(100.0),
                )]))
                .build();
            Some(authentication)
        }

        fn set_authentication(&self, _authentication: &Authentication) {
            unimplemented!()
        }

        fn update_authentication(&self) -> authentication::AuthenticationUpdater {
            unimplemented!()
        }
    }

    fn header_map() -> Vec<(String, String)> {
        HEADERS
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<Vec<_>>()
    }

    fn value_to_json(value: &Value) -> serde_json::Value {
        if let Some(b) = value.as_bool() {
            b.into()
        } else if let Some(n) = value.as_f64() {
            n.into()
        } else if let Some(s) = value.as_str() {
            s.into()
        } else if let Some(a) = value.as_slice() {
            serde_json::Value::Array(a.iter().map(value_to_json).collect())
        } else if let Some(o) = value.as_object() {
            serde_json::Value::Object(
                o.iter()
                    .map(|(k, v)| (k.clone(), value_to_json(v)))
                    .collect(),
            )
        } else {
            serde_json::Value::Null
        }
    }

    mock! {
        pub Accessor {}
        impl HeadersAccessor for Accessor {
            fn header(&self, name: &str) -> Option<String>;
            fn headers(&self) -> Vec<(String, String)>;
            fn add_header(&self, name: &str, value: &str);
            fn set_header(&self, name: &str, value: &str);
            fn set_headers<'a>(&self, headers: Vec<(&'a str, &'a str)>);
            fn remove_header(&self, name: &str);
        }
    }

    pub struct Ops {
        pub request: MockAccessor,
        pub response: MockAccessor,
    }

    impl Ops {
        pub fn new() -> Self {
            Self {
                request: MockAccessor::new(),
                response: MockAccessor::new(),
            }
        }
    }

    fn mock_request_header(
        ops: &mut Ops,
        header: &'static str,
        header_value: Option<&'static str>,
    ) {
        ops.request
            .expect_header()
            .with(eq(header))
            .return_const(header_value.map(|s| s.to_string()));
    }

    fn mock_response_header(
        ops: &mut Ops,
        header: &'static str,
        header_value: Option<&'static str>,
    ) {
        ops.response
            .expect_header()
            .with(eq(header))
            .return_const(header_value.map(|s| s.to_string()));
    }

    fn mock_header(ops: &mut Ops, header: &'static str, header_value: Option<&'static str>) {
        mock_request_header(ops, header, header_value);
        mock_response_header(ops, header, header_value);
    }

    fn lazy_mock_ops() -> Ops {
        let mut ops = Ops::new();
        HEADERS
            .iter()
            .for_each(|(header, header_value)| mock_header(&mut ops, header, Some(header_value)));
        mock_header(&mut ops, "inexistent", None);

        ops
    }

    fn mock_request_headers(ops: &mut Ops) {
        ops.request.expect_headers().returning(header_map);
    }

    fn mock_response_headers(ops: &mut Ops) {
        ops.response.expect_headers().returning(header_map);
    }

    fn detached_mock_ops() -> Ops {
        let mut ops = lazy_mock_ops();

        {
            let ops = &mut ops;
            mock_request_headers(ops);
            mock_response_headers(ops);
        }

        ops
    }

    fn foreach_request_context(ops: &Ops, test: impl Fn(&dyn Context)) {
        test(&request_headers_context(
            &MockPolicyContext,
            &ops.request,
            EvaluationMode::Complete,
            &HashMap::default(),
        ));
    }

    fn foreach_response_context(ops: &Ops, test: impl Fn(&dyn Context)) {
        test(&response_headers_context(
            &MockPolicyContext,
            &ops.response,
            EvaluationMode::Complete,
            &HashMap::default(),
        ));
    }

    fn foreach_context(ops: &Ops, test: impl Fn(&dyn Context)) {
        let vars = &HashMap::from([(
            "claimSet",
            serde_json::json!({
                "foo": "bar",
                "baz": "bam",
            })
            .into_value(),
        )]);
        test(&request_headers_context(
            &MockPolicyContext,
            &ops.request,
            EvaluationMode::Complete,
            vars,
        ));
        test(&response_headers_context(
            &MockPolicyContext,
            &ops.response,
            EvaluationMode::Complete,
            vars,
        ));
    }

    #[test]
    fn attributes_inexistent() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_context(&lazy_mock_ops(), |context| {
            // DW: attributes.inexistent
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "inexistent"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let result = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert!(result.is_null());
        });
    }

    #[test]
    fn authentication_client_id() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        // DW: authentication.clientId
        let pel = r#"
            [".", "0-30",
                [":ref", "11-22", "authentication"],
                [":str", "0-20", "clientId"]
            ]
        "#;

        let expression = parser.parse_str(pel).unwrap();

        foreach_context(&lazy_mock_ops(), |context| {
            let result = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(result.as_str().unwrap(), "CLIENT_ID");
        });
    }

    #[test]
    fn authentication_client_name() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        // DW: authentication.clientName
        let pel = r#"
            [".", "0-30",
                [":ref", "11-22", "authentication"],
                [":str", "0-20", "clientName"]
            ]
        "#;

        let expression = parser.parse_str(pel).unwrap();

        foreach_context(&lazy_mock_ops(), |context| {
            let result = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(result.as_str().unwrap(), "CLIENT_NAME");
        });
    }

    #[test]
    fn authentication_principal() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        // DW: authentication.principal
        let pel = r#"
            [".", "0-30",
                [":ref", "11-22", "authentication"],
                [":str", "0-20", "principal"]
            ]
        "#;

        let expression = parser.parse_str(pel).unwrap();

        foreach_context(&lazy_mock_ops(), |context| {
            let result = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(result.as_str().unwrap(), "PRINCIPAL");
        });
    }

    #[test]
    fn authentication_properties() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        // DW: authentication.properties['foo']
        let pel = r#"
            [".", "0-40",
                [".", "0-30",
                    [":ref", "11-22", "authentication"],
                    [":str", "0-20", "properties"]
                ],
                [":str", "0-20", "foo"]
            ]
        "#;

        let expression = parser.parse_str(pel).unwrap();

        foreach_context(&lazy_mock_ops(), |context| {
            let result = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(result.as_f64().unwrap(), 100.0);
        });
    }

    #[test]
    fn authentication_properties_detach() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        // DW: authentication.properties['foo']
        let pel = r#"
            [".", "0-30",
                [":ref", "11-22", "authentication"],
                [":str", "0-20", "properties"]
            ]
        "#;

        let expression = parser.parse_str(pel).unwrap();

        let expected = serde_json::json!({
            "foo": 100.0
        });

        foreach_context(&lazy_mock_ops(), |context| {
            let result = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let actual = value_to_json(&result);

            assert_eq!(actual, expected);
        });
    }

    #[test]
    fn authentication_detach() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        // DW: authentication
        let pel = r#"[":ref", "0-10", "authentication"]"#;

        let expression = parser.parse_str(pel).unwrap();

        let expected = serde_json::json!({
            "clientId": "CLIENT_ID",
            "clientName": "CLIENT_NAME",
            "principal": "PRINCIPAL",
            "properties": {
                "foo": 100.0
            }
        });

        foreach_context(&lazy_mock_ops(), |context| {
            let result = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let actual = value_to_json(&result);

            assert_eq!(actual, expected);
        });
    }

    #[test]
    fn attributes_headers() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_context(&lazy_mock_ops(), |context| {
            // DW: attributes.headers["Content-Type"]
            let pel = r#"
                [".", "0-24",
                    [".", "0-22",
                        [":ref", "0-10", "attributes"],
                        [":str", "11-22", "headers"]
                    ],
                    [":str", "23-24", "Content-Type"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let header = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let content_type = header.as_str().unwrap();

            assert_eq!(content_type, "application/json");
        });
    }

    #[test]
    fn attributes_headers_inexistent() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_context(&lazy_mock_ops(), |context| {
            // DW: attributes.headers["inexistent"]
            let pel = r#"
                [".", "0-24",
                    [".", "0-22",
                        [":ref", "0-10", "attributes"],
                        [":str", "11-22", "headers"]
                    ],
                    [":str", "23-24", "inexistent"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let result = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert!(result.is_null());
        });
    }

    #[test]
    fn authentication_inexistent() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        // DW: authentication["inexistent"]
        let pel = r#"
            [".", "0-24",
                [":ref", "11-22", "authentication"],
                [":str", "23-24", "inexistent"]
            ]
        "#;

        let expression = parser.parse_str(pel).unwrap();

        foreach_context(&lazy_mock_ops(), |context| {
            let query_param = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert!(query_param.is_null());
        });
    }

    #[test]
    fn attributes_method() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&lazy_mock_ops(), |context| {
            // DW: attributes.method
            let pel = r#"
                [".", "0-24",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "method"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let method = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let method = method.as_str().unwrap();

            assert_eq!(method, "GET");
        });
    }

    #[test]
    fn attributes_query_param() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&lazy_mock_ops(), |context| {
            // DW: attributes.queryParams["foo"]
            let pel = r#"
                [".", "0-24",
                    [".", "0-22",
                        [":ref", "0-10", "attributes"],
                        [":str", "11-22", "queryParams"]
                    ],
                    [":str", "23-24", "foo"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let query_param = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let query_param = query_param.as_str().unwrap();

            assert_eq!(query_param, "bar");
        });
    }

    #[test]
    fn attributes_query_param_inexistent() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&lazy_mock_ops(), |context| {
            // DW: attributes.queryParams["inexistent"]
            let pel = r#"
                [".", "0-24",
                    [".", "0-22",
                        [":ref", "0-10", "attributes"],
                        [":str", "11-22", "queryParams"]
                    ],
                    [":str", "23-24", "inexistent"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let query_param = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert!(query_param.is_null());
        });
    }

    #[test]
    fn attributes_query_string() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&lazy_mock_ops(), |context| {
            // DW: attributes.queryString
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "12-22", "queryString"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let query_string = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let query_string = query_string.as_str().unwrap();

            assert_eq!(query_string, "baz=bal&foo=bar");
        });
    }

    #[test]
    fn attributes_request_path() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&lazy_mock_ops(), |context| {
            // DW: attributes.requestPath
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "requestPath"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let request_path = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let request_path = dbg!(request_path.as_str()).unwrap();

            assert_eq!(request_path, "/something");
        });
    }

    #[test]
    fn attributes_request_uri() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&lazy_mock_ops(), |context| {
            // DW: attributes.requestUri
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "requestUri"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let request_uri = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let request_uri = request_uri.as_str().unwrap();

            assert_eq!(request_uri, "/something?baz=bal&foo=bar");
        });
    }

    #[test]
    fn attributes_remote_address() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&lazy_mock_ops(), |context| {
            // DW: attributes.remoteAddress
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "remoteAddress"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let remote_address = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let remote_address = remote_address.as_str().unwrap();

            assert_eq!(remote_address, "172.18.0.1:60686");
        });
    }

    #[test]
    fn attributes_local_address() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&lazy_mock_ops(), |context| {
            // DW: attributes.remoteAddress
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "localAddress"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let remote_address = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let remote_address = remote_address.as_str().unwrap();

            assert_eq!(remote_address, "172.25.0.7:7890");
        });
    }

    #[test]
    fn attributes_query() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&lazy_mock_ops(), |context| {
            // DW: attributes.remoteAddress
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "queryString"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let remote_address = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let remote_address = remote_address.as_str().unwrap();

            assert_eq!(remote_address, "baz=bal&foo=bar");
        });
    }

    #[test]
    fn attributes_scheme() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&lazy_mock_ops(), |context| {
            // DW: attributes.remoteAddress
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "scheme"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let remote_address = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let remote_address = remote_address.as_str().unwrap();

            assert_eq!(remote_address, "http");
        });
    }

    #[test]
    fn attributes_version() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&lazy_mock_ops(), |context| {
            // DW: attributes.remoteAddress
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "version"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let remote_address = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let remote_address = remote_address.as_str().unwrap();

            assert_eq!(remote_address, "HTTP/1.1");
        });
    }

    #[test]
    fn attributes_status_code() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_response_context(&lazy_mock_ops(), |context| {
            // DW: attributes.statusCode
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "statusCode"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let status_code = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let request_uri = status_code.as_f64().unwrap();

            assert_eq!(request_uri, 207_f64);
        });
    }

    #[test]
    fn attributes_status_code_nan() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        let mut ops = Ops::new();
        mock_header(&mut ops, ":status", Some("Not a number"));

        foreach_response_context(&ops, |context| {
            // DW: attributes.statusCode
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "statusCode"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let status_code = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(status_code, Value::null());
        });
    }

    #[test]
    fn attributes_status_code_missing() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        let mut ops = Ops::new();
        mock_header(&mut ops, ":status", None);

        foreach_response_context(&ops, |context| {
            // DW: attributes.statusCode
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-22", "statusCode"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let status_code = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(status_code, Value::null());
        });
    }

    #[test]
    fn vars_select() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_context(&lazy_mock_ops(), |context| {
            // DW: vars.claimSet.foo
            let pel = r#"
                [".", "0-22",
                    [".", "0-22",
                        [":ref", "0-10", "vars"],
                        [":str", "11-22", "claimSet"]
                    ],
                    [":str", "11-22", "foo"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let result = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(Some("bar"), result.as_str());
        });
    }

    #[test]
    fn vars_inexistent() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_context(&lazy_mock_ops(), |context| {
            // DW: vars.inexistent
            let pel = r#"
                [".", "0-22",
                    [":ref", "0-10", "vars"],
                    [":str", "11-22", "inexistent"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let result = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            assert!(result.is_null());
        });
    }

    #[test]
    fn headers_detach() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_context(&detached_mock_ops(), |context| {
            // DW: attributes.headers
            let pel = r#"
                [".", "0-17",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-17", "headers"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let headers = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let actual = value_to_json(&headers);

            let expected = serde_json::json!({
                "Content-Length": "1024",
                "Content-Type": "text/html",
                ":path":  "/something?baz=bal&foo=bar",
                ":method": "GET",
                ":status": "207"
            });

            assert_eq!(actual, expected);
        });
    }

    #[test]
    fn query_params_detach() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&detached_mock_ops(), |context| {
            // DW: attributes.queryParams
            let pel = r#"
                [".", "0-21",
                    [":ref", "0-10", "attributes"],
                    [":str", "11-21", "queryParams"]
                ]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let query_params = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let actual = value_to_json(&query_params);

            let expected = serde_json::json!({
                "baz": "bal",
                "foo": "bar"
            });

            assert_eq!(actual, expected);
        });
    }

    #[test]
    fn request_attributes_detach() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_request_context(&detached_mock_ops(), |context| {
            // DW: attributes
            let pel = r#"
                [":ref", "0-10", "attributes"]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let attributes = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let actual = value_to_json(&attributes);

            let expected = serde_json::json!({
                "headers": {
                    "Content-Length": "1024",
                    "Content-Type": "text/html",
                    ":path":  "/something?baz=bal&foo=bar",
                    ":method": "GET",
                    ":status": "207"
                },
                "localAddress": "172.25.0.7:7890",
                "method": "GET",
                "queryParams": {
                    "baz": "bal",
                    "foo": "bar"
                },
                "queryString": "baz=bal&foo=bar",
                "remoteAddress": "172.18.0.1:60686",
                "requestPath": "/something",
                "requestUri": "/something?baz=bal&foo=bar",
                "scheme": "http",
                "version": "HTTP/1.1",
            });

            assert_eq!(actual, expected);
        });
    }

    #[test]
    fn response_attributes_detach() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_response_context(&detached_mock_ops(), |context| {
            // DW: attributes
            let pel = r#"
                [":ref", "0-10", "attributes"]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let attributes = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let actual = value_to_json(&attributes);

            let expected = serde_json::json!({
                "headers": {
                    "Content-Length": "1024",
                    "Content-Type": "text/html",
                    ":path":  "/something?baz=bal&foo=bar",
                    ":method": "GET",
                    ":status": "207"
                },
                // TODO: AGW-5356 - Improve number coercion
                "statusCode": 207.0
            });

            assert_eq!(actual, expected);
        });
    }

    #[test]
    fn vars_detach() {
        let parser = Parser::new();
        let runtime = Runtime::new();

        foreach_context(&detached_mock_ops(), |context| {
            // DW: vars
            let pel = r#"
                [":ref", "0-10", "vars"]
            "#;

            let expression = parser.parse_str(pel).unwrap();
            let attributes = runtime
                .eval_with_context(&expression, context)
                .unwrap()
                .complete()
                .unwrap();

            let actual = value_to_json(&attributes);

            let expected = serde_json::json!({
                "claimSet": {
                    "foo": "bar",
                    "baz": "bam",
                },
            });

            assert_eq!(actual, expected);
        });
    }
}
