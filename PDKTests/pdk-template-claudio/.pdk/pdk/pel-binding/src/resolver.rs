// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
};

use serde::{
    de::Visitor,
    {Deserialize, Deserializer},
};

use classy::event::{EventData, RequestHeaders, ResponseHeaders};
use pdk_core::log::debug;
use pdk_core::policy_context::PolicyContext;

use pel::{
    expression::Expression as InnerExpression,
    parser::{Parser, ParsingUnitError},
    runtime::{value::Value, Context, Evaluation, Runtime},
};

use crate::{
    convert::IntoValue, request_headers_context, response_headers_context, EvaluationMode,
    HeadersAccessor, OnPayloadContext, ExpressionError,
};

thread_local! {
    static PARSER: Parser = Parser::new();
    static RUNTIME: Runtime = Default::default();
}

//TODO: [AGW-5617] - Improve Expression display in log messages
#[derive(PartialEq, Clone, Debug)]
pub struct Expression {
    source: Option<String>,
    expression: InnerExpression,
}

impl Expression {
    fn new(expression: InnerExpression) -> Self {
        Self {
            source: None,
            expression,
        }
    }

    fn with_source(mut self, source: Option<String>) -> Self {
        self.source = source;
        self
    }

    // TODO W-12284881: Hidden method to avoid its usage in the front end.
    //   Will be of use when unit test tools get developed and parsing static DW expressions becomes available.
    #[allow(dead_code)]
    pub(crate) fn parse(pel: &str) -> Result<Self, ExpressionError> {
        PARSER.with(|parser| {
            parser
                .parse_str(pel)
                .map_err(ExpressionError::ParsingError)
                .map(Self::new)
        })
    }

    pub fn resolver(&self) -> ExpressionResolver {
        ExpressionResolver {
            source: self.source.clone(),
            expression: Some(self.expression.clone()),
        }
    }

    pub fn resolve_on_request_headers(
        &self,
        event_data: &EventData<RequestHeaders>,
    ) -> Result<Value, ExpressionError> {
        self.__resolve_on_request_headers(<dyn PolicyContext>::default(), event_data)
    }

    pub(crate) fn __resolve_on_request_headers(
        &self,
        policy_context: &dyn PolicyContext,
        accessor: &dyn HeadersAccessor,
    ) -> Result<Value, ExpressionError> {
        CompleteResolver::from_expression(self)
            .__resolve_on_request_headers(policy_context, accessor)
    }

    pub fn resolve_on_response_headers(
        &self,
        event_data: &EventData<ResponseHeaders>,
    ) -> Result<Value, ExpressionError> {
        self.__resolve_on_response_headers(<dyn PolicyContext>::default(), event_data)
    }

    pub(crate) fn __resolve_on_response_headers(
        &self,
        policy_context: &dyn PolicyContext,
        accessor: &dyn HeadersAccessor,
    ) -> Result<Value, ExpressionError> {
        CompleteResolver::from_expression(self)
            .__resolve_on_response_headers(policy_context, accessor)
    }

    #[allow(dead_code)]
    pub(crate) fn resolve_on_payload(&self, payload: String) -> Result<Value, ExpressionError> {
        CompleteResolver::from_expression(self).resolve_on_payload(payload)
    }

    pub fn with_var<'a>(&'a self, name: &'a str, value: impl IntoValue) -> CompleteResolver<'a> {
        CompleteResolver::from_expression(self).with_var(name, value)
    }

    pub fn with_vars<'a, V, I>(&'a self, vars: I) -> CompleteResolver<'a>
    where
        V: IntoValue,
        I: IntoIterator<Item = (&'a str, V)>,
    {
        CompleteResolver::from_expression(self).with_vars(vars)
    }
}

pub struct CompleteResolver<'a> {
    expression: &'a InnerExpression,
    source: Option<&'a str>,
    vars: HashMap<&'a str, Value>,
}

impl<'a> CompleteResolver<'a> {
    fn from_expression(expression: &'a Expression) -> Self {
        Self {
            expression: &expression.expression,
            source: expression.source.as_deref(),
            vars: HashMap::default(),
        }
    }

    pub fn with_var(mut self, name: &'a str, value: impl IntoValue) -> Self {
        self.vars.insert(name, value.into_value());
        self
    }

    pub fn with_vars<V, I>(mut self, vars: I) -> Self
    where
        V: IntoValue,
        I: IntoIterator<Item = (&'a str, V)>,
    {
        self.vars
            .extend(vars.into_iter().map(|(k, v)| (k, v.into_value())));
        self
    }

    pub fn resolve_on_request_headers(
        &self,
        event_data: &EventData<RequestHeaders>,
    ) -> Result<Value, ExpressionError> {
        self.__resolve_on_request_headers(<dyn PolicyContext>::default(), event_data)
    }

    pub(crate) fn __resolve_on_request_headers(
        &self,
        policy_context: &dyn PolicyContext,
        accessor: &dyn HeadersAccessor,
    ) -> Result<Value, ExpressionError> {
        self.resolve(&request_headers_context(
            policy_context,
            accessor,
            EvaluationMode::Complete,
            &self.vars,
        ))
    }

    pub fn resolve_on_response_headers(
        &self,
        event_data: &EventData<ResponseHeaders>,
    ) -> Result<Value, ExpressionError> {
        self.__resolve_on_response_headers(<dyn PolicyContext>::default(), event_data)
    }

    pub(crate) fn __resolve_on_response_headers(
        &self,
        policy_context: &dyn PolicyContext,
        accessor: &dyn HeadersAccessor,
    ) -> Result<Value, ExpressionError> {
        self.resolve(&response_headers_context(
            policy_context,
            accessor,
            EvaluationMode::Complete,
            &self.vars,
        ))
    }

    #[allow(dead_code)]
    pub(crate) fn resolve_on_payload(&self, payload: String) -> Result<Value, ExpressionError> {
        self.resolve(&OnPayloadContext { payload })
    }

    fn resolve(&self, context: &dyn Context) -> Result<Value, ExpressionError> {
        let evaluation = RUNTIME
            .with(|runtime| runtime.eval_with_context(self.expression, context))
            .map_err(|cause| ExpressionError::with_optional_source(cause, self.source))?;
        match evaluation {
            Evaluation::Complete(_, value) => Ok(value),
            Evaluation::Partial(_) => Err(ExpressionError::IncompleteEvaluation),
        }
    }
}

#[derive(Debug)]
pub struct PartialResolver {
    source: Option<String>,
    expression: Option<InnerExpression>,
}

pub type ExpressionResolver = PartialResolver;

impl PartialResolver {
    pub fn resolve_on_request_headers(
        &mut self,
        accessor: &EventData<RequestHeaders>,
    ) -> Result<Option<Value>, ExpressionError> {
        self.__resolve_on_request_headers(<dyn PolicyContext>::default(), accessor)
    }

    pub(crate) fn __resolve_on_request_headers(
        &mut self,
        policy_context: &dyn PolicyContext,
        accessor: &dyn HeadersAccessor,
    ) -> Result<Option<Value>, ExpressionError> {
        self.resolve(&request_headers_context(
            policy_context,
            accessor,
            EvaluationMode::Partial,
            &HashMap::default(),
        ))
    }

    pub fn resolve_on_response_headers(
        &mut self,
        accessor: &EventData<ResponseHeaders>,
    ) -> Result<Option<Value>, ExpressionError> {
        self.__resolve_on_response_headers(<dyn PolicyContext>::default(), accessor)
    }

    pub(crate) fn __resolve_on_response_headers(
        &mut self,
        policy_context: &dyn PolicyContext,
        accessor: &dyn HeadersAccessor,
    ) -> Result<Option<Value>, ExpressionError> {
        self.resolve(&response_headers_context(
            policy_context,
            accessor,
            EvaluationMode::Partial,
            &HashMap::default(),
        ))
    }

    pub fn resolve_on_payload(&mut self, payload: String) -> Result<Option<Value>, ExpressionError> {
        self.resolve(&OnPayloadContext { payload })
    }

    fn resolve(&mut self, context: &dyn Context) -> Result<Option<Value>, ExpressionError> {
        let expression = self.expression.as_ref().ok_or(ExpressionError::AlreadyResolved)?;
        let evaluation = RUNTIME
            .with(|runtime| runtime.eval_with_context(expression, context))
            .map_err(|cause| ExpressionError::with_optional_source(cause, self.source.as_deref()))?;
        match evaluation {
            Evaluation::Complete(_, value) => {
                self.expression = None;
                Ok(Some(value))
            }
            Evaluation::Partial(expression) => {
                self.expression = Some(expression);
                Ok(None)
            }
        }
    }
}

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ResolverVisitor;

        impl<'de> Visitor<'de> for ResolverVisitor {
            type Value = Expression;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("PEL Expression")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let (expression, source) = PARSER.with(|parser| parser.parse_unit(value)).map_err(
                    |error| match error {
                        ParsingUnitError::InvalidContainer => {
                            serde::de::Error::invalid_type(serde::de::Unexpected::Str(value), &self)
                        }
                        _ => serde::de::Error::custom(format_args!(
                            "Unexpected error when parsing expression '{value}': {error}"
                        )),
                    },
                )?;

                debug!("Expression {value} successfully parsed");

                Ok(Expression::new(expression).with_source(source))
            }
        }

        deserializer.deserialize_str(ResolverVisitor)
    }
}

#[cfg(test)]
mod tests {

    use crate::tests::{MockAccessor, MockPolicyContext};
    use mockall::predicate::eq;
    use pel::runtime::value::Value;
    use serde::Deserialize;

    use crate::resolver::PARSER;
    use pel::expression::Expression as InnerExpression;

    use super::{PartialResolver, ExpressionError, Expression};

    #[derive(Deserialize)]
    struct TestStruct {
        expression: Expression,
        string: String,
        inner_struct: InnerStruct,
    }

    #[derive(Deserialize)]
    struct InnerStruct {
        inner_expression: Expression,
    }

    #[test]
    fn deserialize_config() {
        let config_json = serde_json::json!({
            "expression": "P[[\":ref\", \"0-10\", \"attributes\"]]",
            "string": "Some String",
            "inner_struct": {
                "inner_expression": "P[[\":ref\", \"0-7\", \"payload\"]]"
            }
        });

        let parsing_result: Result<TestStruct, _> = serde_json::from_value(config_json);

        let expression = r#"[":ref", "0-10", "attributes"]"#;
        let parsed_expression = parse(expression);

        let inner_expression = r#"[":ref", "0-7", "payload"]"#;
        let parsed_inner_expression = parse(inner_expression);

        assert!(parsing_result.is_ok());

        let parsed_config = parsing_result.unwrap();

        assert_eq!(parsed_config.string, "Some String");
        assert_eq!(parsed_config.expression.expression, parsed_expression);
        assert_eq!(
            parsed_config.inner_struct.inner_expression.expression,
            parsed_inner_expression
        );
    }

    #[test]
    fn deserialize_config_with_source() {
        let config_json = serde_json::json!({
            "expression": r##"P[[":ref", "0-10", "attributes"], "#[attributes]"]"##,
            "string": "Some String",
            "inner_struct": {
                "inner_expression": r##"P[[":str", "0-7", "some string"], "#[\"some string\"]"]"##
            }
        });

        let parsing_result: Result<TestStruct, _> = serde_json::from_value(config_json);

        let expression = r#"[":ref", "0-10", "attributes"]"#;
        let parsed_expression = parse(expression);

        let inner_expression = r#"[":str", "0-7", "some string"]"#;
        let parsed_inner_expression = parse(inner_expression);

        assert!(parsing_result.is_ok());

        let parsed_config = parsing_result.unwrap();

        assert_eq!(parsed_config.string, "Some String");
        assert_eq!(parsed_config.expression.expression, parsed_expression);
        assert_eq!(
            parsed_config.expression.source.as_deref(),
            Some("attributes")
        );
        assert_eq!(
            parsed_config.inner_struct.inner_expression.expression,
            parsed_inner_expression
        );
        assert_eq!(
            parsed_config
                .inner_struct
                .inner_expression
                .source
                .as_deref(),
            Some(r#""some string""#)
        );
    }

    #[test]
    fn config_with_invalid_pel() {
        let config_json =
            serde_json::json!({"inner_expression": "P[[\":invalid\", \"0-10\", \"attributes\"]]"});

        let parsing_result: Result<InnerStruct, _> = serde_json::from_value(config_json);

        assert!(parsing_result.is_err());
        assert_eq!(
            parsing_result.err().unwrap().to_string(),
            "Unexpected error when parsing expression \
            \'P[[\":invalid\", \"0-10\", \"attributes\"]]\': Parsing error: Unknown constructor"
        );
    }

    #[test]
    fn config_with_non_expression_as_pel() {
        let config_json = serde_json::json!({"inner_expression": "plain string"});

        let parsing_result: Result<InnerStruct, _> = serde_json::from_value(config_json);

        assert!(parsing_result.is_err());
        assert_eq!(
            parsing_result.err().unwrap().to_string(),
            "invalid type: string \"plain string\", expected PEL Expression"
        );
    }

    #[test]
    fn config_with_invalid_json_as_pel() {
        let config_json =
            serde_json::json!({"inner_expression": "P[{[\":ref\", \"0-10\", \"attributes\"]]"});

        let parsing_result: Result<InnerStruct, _> = serde_json::from_value(config_json);

        assert!(parsing_result.is_err());
        assert_eq!(
            parsing_result.err().unwrap().to_string(),
            "Unexpected error when parsing expression \
            \'P[{[\":ref\", \"0-10\", \"attributes\"]]\': Invalid structure"
        );
    }

    #[test]
    fn config_with_invalid_type_as_pel() {
        let config_json = serde_json::json!({"inner_expression": 100});

        let parsing_result: Result<InnerStruct, _> = serde_json::from_value(config_json);

        assert!(parsing_result.is_err());
        assert_eq!(
            parsing_result.err().unwrap().to_string(),
            "invalid type: integer `100`, expected PEL Expression"
        );
    }

    #[test]
    fn display_error_snippet() {
        let config_json = serde_json::json!({
            "expression": r##"P[[":null", "0-4"], "#[null]"]"##,
            "string": "Some String",
            "inner_struct": {
                "inner_expression":
                    r##"P[
                        [":apply", "0-14", [":ref", "5-7", "++"], [":null", "0-4"], [":str", "8-14", " bye"]],
                        "#[null ++ ' bye']"
                    ]"##
            }
        });

        let parsing_result: Result<TestStruct, _> = serde_json::from_value(config_json);
        let parsing_result = parsing_result.unwrap();
        let expression = parsing_result.inner_struct.inner_expression;
        let error = expression
            .__resolve_on_request_headers(&MockPolicyContext, &MockAccessor::new())
            .unwrap_err();

        let expected =
            "Runtime error:\n\tType mismatch\nLocation:\n\tline: 1, column: 1\n1| null ++ ' bye'\n";

        assert_eq!(error.to_string(), expected);
    }

    #[test]
    fn resolve_on_request_headers() {
        let pel = r#"
                [".", "0-24",
                    [".", "0-22",
                        [":ref", "0-10", "attributes"],
                        [":str", "11-22", "headers"]
                    ],
                    [":str", "23-24", "Content-Length"]
                ]
            "#;
        let resolver = Expression::new(parse(pel));
        let mut ops = MockAccessor::new();
        ops.expect_header()
            .with(eq("Content-Length"))
            .returning(move |_x: &str| Some("1024".to_string()));

        let result = resolver.__resolve_on_request_headers(&MockPolicyContext, &ops);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str().unwrap(), "1024");
    }

    #[test]
    fn resolve_on_response_headers() {
        let pel = r#"
                [".", "0-24",
                    [".", "0-22",
                        [":ref", "0-10", "attributes"],
                        [":str", "11-22", "headers"]
                    ],
                    [":str", "23-24", "status-code"]
                ]
            "#;
        let resolver = Expression::new(parse(pel));
        let mut ops = MockAccessor::new();
        ops.expect_header()
            .with(eq("status-code"))
            .returning(move |_x: &str| Some("201".to_string()));

        let result = resolver.__resolve_on_response_headers(&MockPolicyContext, &ops);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str().unwrap(), "201");
    }

    #[test]
    fn resolve_with_vars() {
        // DW: vars.claimSet.foo
        let pel = r#"
                [".", "0-24",
                    [".", "0-22",
                        [":ref", "0-10", "vars"],
                        [":str", "11-22", "claimSet"]
                    ],
                    [":str", "23-24", "foo"]
                ]
            "#;
        let expression = Expression::new(parse(pel));
        let ops = MockAccessor::new();
        let result = expression
            .with_var("peregrine-version", "1.0")
            .with_vars([("n1", 123.4), ("n2", 56.7), ("n3", 9023.3)])
            .with_var(
                "claimSet",
                serde_json::json!({
                    "foo": "bar",
                    "baz": "bam"
                }),
            )
            .__resolve_on_request_headers(&MockPolicyContext, &ops);

        assert!(result.is_ok());
        assert_eq!(Some("bar"), result.unwrap().as_str());
    }

    fn resolve_partial<F>(resolve: F)
    where
        F: Fn(&mut PartialResolver) -> Result<Option<Value>, ExpressionError>,
    {
        // DW: payload ++ attributes.headers["status-code"]
        let pel = r#"
            [":apply", "0-44",
                [":ref", "8-10", "++"],
                [":ref", "0-7", "payload"],
                [".", "29-44",
                    [".", "21-22",
                        [":ref", "11-21", "attributes"],
                        [":str", "22-29", "headers"]
                    ],
                    [":str", "30-43", "status-code"]
                ]
            ]
        "#;
        let mut resolver = Expression::new(parse(pel)).resolver();

        let round_1 = resolve(&mut resolver);

        assert!(round_1.unwrap().is_none());

        let round_2 = resolver.resolve_on_payload("test".to_string()).unwrap();

        assert_eq!(round_2.unwrap().as_str().unwrap(), "test201");
    }

    #[test]
    fn resolve_partial_on_request_headers() {
        let mut ops = MockAccessor::new();

        ops.expect_header()
            .with(eq("status-code"))
            .return_const(Some("201".to_string()));

        resolve_partial(|resolver| resolver.__resolve_on_request_headers(&MockPolicyContext, &ops));
    }

    #[test]
    fn resolve_partial_on_response_headers() {
        let mut ops = MockAccessor::new();

        ops.expect_header()
            .with(eq("status-code"))
            .return_const(Some("201".to_string()));

        resolve_partial(|resolver| {
            resolver.__resolve_on_response_headers(&MockPolicyContext, &ops)
        });
    }

    #[test]
    fn resolve_returns_error() {
        let pel = r#"
                [":ref", "0-10", "undefined"]
            "#;

        let expression = Expression::new(parse(pel));

        let result =
            expression.__resolve_on_request_headers(&MockPolicyContext, &MockAccessor::new());

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Runtime error: Unknown symbol `undefined`"
        );
    }

    #[test]
    fn resolve_pending_in_pel_expression_returns_runtime_error() {
        let pel = r#"
            [":ref", "0-10", "payload"]
        "#;

        let expression = Expression::new(parse(pel));

        let result =
            expression.__resolve_on_request_headers(&MockPolicyContext, &MockAccessor::new());

        assert!(matches!(result, Err(ExpressionError::RuntimeError(_))));
    }

    fn parse(expression: &str) -> InnerExpression {
        PARSER
            .with(|parser| parser.parse_slice(expression.as_bytes()))
            .unwrap()
    }
}
