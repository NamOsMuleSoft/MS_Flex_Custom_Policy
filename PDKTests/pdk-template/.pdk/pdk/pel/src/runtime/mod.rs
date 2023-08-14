// Copyright 2023 Salesforce, Inc. All rights reserved.
mod coercion;
mod eval;
mod prelude;
mod value_handler;

pub mod value;

use std::collections::HashMap;
use thiserror::Error;

use crate::{
    expression::{Expression, Symbol},
    runtime::value::Value,
    Location, Reference,
};

pub use value_handler::ValueHandler;

#[derive(Error, Debug)]
#[error("{kind}")]
pub struct RuntimeError {
    location: Location,
    kind: RuntimeErrorKind,
}

impl RuntimeError {
    pub fn new(location: Location, kind: RuntimeErrorKind) -> Self {
        Self { location, kind }
    }

    pub fn kind(&self) -> &RuntimeErrorKind {
        &self.kind
    }

    pub fn location(&self) -> Location {
        self.location
    }
}

#[derive(Error, Debug, Clone, Eq, PartialEq)]
pub enum RuntimeErrorKind {
    #[error("Unknown reference")]
    UnknownReference,

    #[error("Unknown symbol `{0}`")]
    UnknownSymbol(String),

    #[error("Type mismatch")]
    TypeMismatch,

    #[error("Not enough arguments")]
    NotEnoughArguments,

    #[error("Too many arguments")]
    TooManyArguments,

    #[error("Unsupported selection")]
    UnsupportedSelection,

    #[error("Unavailable random generator")]
    UnavailableRandomGenerator,

    #[error("Unavailable size")]
    UnavailableSize,

    #[error("Uncomparable types")]
    UncomparableTypes,
}

pub enum Binding {
    Available(Value),
    Pending,
    Unknown,
}

pub trait Context {
    fn resolve(&self, symbol: &Symbol) -> Binding;

    fn value_handler(&self, reference: Reference) -> Option<&dyn ValueHandler>;
}

type Prelude = HashMap<&'static str, Value>;

impl Context for Prelude {
    fn resolve(&self, symbol: &Symbol) -> Binding {
        self.get(symbol.as_str())
            .cloned()
            .map(Binding::Available)
            .unwrap_or(Binding::Unknown)
    }

    fn value_handler(&self, _reference: Reference) -> Option<&dyn ValueHandler> {
        None
    }
}

struct MergedContext<'a> {
    root: &'a dyn Context,
    current: &'a dyn Context,
}

struct EmptyContext;

impl Context for EmptyContext {
    fn resolve(&self, _: &Symbol) -> Binding {
        Binding::Unknown
    }

    fn value_handler(&self, _reference: Reference) -> Option<&dyn ValueHandler> {
        None
    }
}

impl<'a> MergedContext<'a> {
    fn new(root: &'a dyn Context, current: &'a dyn Context) -> Self {
        Self { root, current }
    }
}

impl Context for MergedContext<'_> {
    fn resolve(&self, symbol: &Symbol) -> Binding {
        match self.current.resolve(symbol) {
            Binding::Unknown => self.root.resolve(symbol),
            b => b,
        }
    }

    fn value_handler(&self, reference: Reference) -> Option<&dyn ValueHandler> {
        self.current
            .value_handler(reference)
            .or_else(|| self.root.value_handler(reference))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Evaluation {
    Complete(Location, Value),
    Partial(Expression),
}

impl Evaluation {
    pub fn map<F: FnOnce(Value) -> Value>(self, op: F) -> Self {
        match self {
            Self::Complete(l, v) => Self::Complete(l, op(v)),
            e => e,
        }
    }

    pub fn complete(self) -> Option<Value> {
        match self {
            Self::Complete(_, v) => Some(v),
            _ => None,
        }
    }

    pub fn partial(self) -> Option<Expression> {
        match self {
            Self::Partial(expression) => Some(expression),
            _ => None,
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete(_, _))
    }

    pub fn into_expression(self) -> Expression {
        match self {
            Self::Complete(location, value) => Expression::new(location, value),
            Self::Partial(expression) => expression,
        }
    }
}

pub trait Eval {
    fn eval(&self, context: &dyn Context) -> Result<Evaluation, RuntimeError>;
}

pub struct Runtime<P = Prelude> {
    prelude: P,
}

impl Runtime {
    pub fn new() -> Self {
        Self::with_prelude(prelude::prelude())
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Context> Runtime<P> {
    pub fn with_prelude(prelude: P) -> Self {
        Self { prelude }
    }
}

impl Runtime {
    pub fn eval_with_context(
        &self,
        e: &dyn Eval,
        context: &dyn Context,
    ) -> Result<Evaluation, RuntimeError> {
        let context = MergedContext::new(&self.prelude, context);
        e.eval(&context).map(|ev| {
            ev.map(|v| {
                v.to_value_handler(&context)
                    .and_then(|vh| vh.detach())
                    .unwrap_or_else(Value::null)
            })
        })
    }

    pub fn eval(&self, e: &dyn Eval) -> Result<Evaluation, RuntimeError> {
        self.eval_with_context(e, &EmptyContext)
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use super::value::{Object, Value};
    use crate::{
        expression::Symbol,
        parser::Parser,
        runtime::{Binding, Context, RuntimeErrorKind, ValueHandler},
        ContextId, Reference,
    };

    use super::Runtime;

    #[test]
    fn inline_array() {
        // DW: ["one", 2, false]
        let json = r#"[":array", "0-17", [":str", "1-6", "one"], [":nbr", "8-9", "2"], [":bool", "11-16", "false"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let actual = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();
        let expected = &[
            Value::string("one".to_string()),
            Value::number(2.0),
            Value::bool(false),
        ];

        assert_eq!(actual.as_slice().unwrap(), expected);
    }

    #[test]
    fn unknown_symbol_fail() {
        // DW: [":ref", "unknown"]
        let pel = r#"[":ref", "0-7", "unknown"]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let error = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(
            error.kind(),
            &RuntimeErrorKind::UnknownSymbol("unknown".to_string())
        );
    }

    #[test]
    fn null_value() {
        // DW: null
        let pel = r#"[":null", "0-4"]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn apply() {
        // DW: "hi" ++ "by"
        let json = r#"[":apply", "5-7", [":ref", "5-7", "++"], [":str", "0-4", "hi"], [":str", "8-12", "by"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("hiby", result.as_str().unwrap());
    }

    #[test]
    fn if_else() {
        // DW: if (false) "a" else "b"
        let json = r#"[":if", "0-23", [":bool", "4-9", "false"], [":str", "11-14", "a"], [":str", "20-23", "b"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("b", result.as_str().unwrap());
    }

    #[test]
    fn default_left_null() {
        // DW: null default "right"
        let pel = r#"
            [":default", "0-6", 
                [":null", "0-1"], 
                [":str", "5-6", "right"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "right");
    }

    #[test]
    fn default_left_not_null() {
        // DW: 700 default true
        let pel = r#"
            [":default", "0-6", 
                [":nbr", "0-1", "700"], 
                [":bool", "5-6", "true"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_f64().unwrap(), 700.0);
    }

    #[test]
    fn array_positive_index() {
        // DW: ["accept","accept-encoding","user-agent"][2]
        let pel = r#"
            [".", "0-45", 
                [":array", "0-41", 
                    [":str", "1-9", "accept"], 
                    [":str", "10-27", "accept-encoding"], 
                    [":str", "28-40", "user-agent"]
                ], [":nbr", "42-44", "2"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("user-agent", result.as_str().unwrap());
    }

    #[test]
    fn array_negative_index() {
        // DW: ["accept","accept-encoding","user-agent"][-2]
        let pel = r#"
            [".", "0-45", 
                [":array", "0-41", 
                    [":str", "1-9", "accept"], 
                    [":str", "10-27", "accept-encoding"], 
                    [":str", "28-40", "user-agent"]
                ], [":nbr", "42-44", "-2"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("accept-encoding", result.as_str().unwrap());
    }

    #[test]
    fn array_out_of_bounds_index() {
        // DW: ["accept","accept-encoding","user-agent"][5]
        let pel = r#"
            [".", "0-45", 
                [":array", "0-41", 
                    [":str", "1-9", "accept"], 
                    [":str", "10-27", "accept-encoding"], 
                    [":str", "28-40", "user-agent"]
                ], [":nbr", "42-44", "5"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn string_positive_index() {
        // DW: "peregrine expression language"[10]
        let pel = r#"
            [".", "0-45", 
                [":str", "0-41", "peregrine expression language"], 
                [":nbr", "42-44", "10"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("e", result.as_str().unwrap());
    }

    #[test]
    fn string_negative_index() {
        // DW: "peregrine expression language"[-5]
        let pel = r#"
            [".", "0-45", 
                [":str", "0-41", "peregrine expression language"], 
                [":nbr", "42-44", "-5"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("g", result.as_str().unwrap());
    }

    #[test]
    fn string_out_of_bounds_index() {
        // DW: "peregrine expression language"[100]
        let pel = r#"
            [".", "0-45", 
                [":str", "0-41", "peregrine expression language"], 
                [":nbr", "42-44", "100"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn selection_by_key() {
        // Context
        let mut b_object = Object::new();
        b_object.insert("c".to_string(), Value::string("foo".to_string()));

        let mut a_object = Object::new();
        a_object.insert("b".to_string(), Value::object(b_object));

        let a_object = Value::object(a_object);

        let mut context = HashMap::new();
        context.insert("a", a_object);

        // DW: a.b.c
        let json = r#"[".", "0-5", [".", "0-3", [":ref", "0-1", "a"], [":str", "2-3", "b"]], [":str", "4-5", "c"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval_with_context(&expression, &context)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("foo", result.as_str().unwrap());
    }

    #[test]
    fn selection_by_key_in_array() {
        // Context
        let mut b_object = Object::new();
        b_object.insert("c".to_string(), Value::string("foo".to_string()));

        let mut a_object = Object::new();
        a_object.insert("b".to_string(), Value::object(b_object));

        let a_object = Value::object(a_object);

        let array = Value::array(vec![a_object]);

        let mut context = HashMap::new();
        context.insert("array", array);

        // DW: array.b.c
        let pel = r#"
            [".", "0-3", 
                [".", "0-5", 
                    [":ref", "0-5", "array"],
                    [":str", "0-1", "b"]
                ],
                [":str", "2-3", "c"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval_with_context(&expression, &context)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(&[Value::string("foo".into())], result.as_slice().unwrap());
    }

    #[test]
    fn select_by_key_inexistent_in_object() {
        // Context
        let b_object = Object::new();

        let mut a_object = Object::new();
        a_object.insert("b".to_string(), Value::object(b_object));

        let a_object = Value::object(a_object);

        let mut context = HashMap::new();
        context.insert("a", a_object);

        // DW: a.b.inexistent
        let pel = r#"
            [".", "0-5", 
                [".", "0-3", 
                    [":ref", "0-1", "a"], 
                    [":str", "2-3", "b"]
                ], 
                [":str", "4-5", "inexistent"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval_with_context(&expression, &context)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn select_by_key_inexistent_in_reference() {
        struct TestContext;
        struct TestValueHandler;

        const CONTEXT_ID: ContextId = ContextId::new("TestContext");
        const REFERENCE: Reference = CONTEXT_ID.first_reference();

        impl ValueHandler for TestValueHandler {
            fn detach(&self) -> Option<Value> {
                unimplemented!()
            }

            fn select_by_key(&self, _key: &str) -> Option<Value> {
                None
            }
        }

        impl Context for TestContext {
            fn resolve(&self, symbol: &Symbol) -> Binding {
                match symbol.as_str() {
                    "a" => Binding::Available(Value::reference(REFERENCE)),
                    _ => Binding::Unknown,
                }
            }

            fn value_handler(&self, reference: Reference) -> Option<&dyn ValueHandler> {
                match reference {
                    REFERENCE => Some(&TestValueHandler),
                    _ => None,
                }
            }
        }

        // DW: a.b.inexistent
        let pel = r#"
            [".", "0-5", 
                [":ref", "0-1", "a"], 
                [":str", "2-3", "inexistent"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval_with_context(&expression, &TestContext)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn operation_eq() {
        // DW: 10 == 10.0
        let json = r#"["==", "0-6", [":nbr", "0-1", "10"], [":nbr", "5-6", "10.0"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn operation_eq_mismatch() {
        // DW: 10 == "10.0"
        let json = r#"["==", "0-6", [":nbr", "0-1", "10"], [":str", "5-6", "10.0"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(!result.as_bool().unwrap());
    }

    #[test]
    fn operation_neq() {
        // DW: 10 != 10.0
        let json = r#"["!=", "0-6", [":nbr", "0-1", "10"], [":nbr", "5-6", "10.0"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(!result.as_bool().unwrap());
    }

    #[test]
    fn operation_neq_mismatch() {
        // DW: 10 != "10"
        let json = r#"["!=", "0-6", [":nbr", "0-1", "10"], [":str", "5-6", "10"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn operation_gt() {
        // DW: 10 > 100.0
        let json = r#"[">", "0-6", [":nbr", "0-1", "10"], [":nbr", "5-6", "100.0"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(!result.as_bool().unwrap());
    }

    #[test]
    fn operation_get() {
        // DW: "left" >= "left"
        let json = r#"[">=", "0-6", [":str", "0-1", "left"], [":str", "5-6", "left"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn operation_lt() {
        // DW: "2.0" < 10
        let json = r#"["<", "0-6", [":str", "0-1", "2.0"], [":nbr", "5-6", "10"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn operation_let() {
        // DW: true <= "true"
        let json = r#"["<=", "0-6", [":bool", "0-1", "true"], [":str", "5-6", "true"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn operation_type_mismatch_fail() {
        // DW: "11.0" <= true
        let json = r#"["<=", "0-14", [":str", "0-6", "11.0"], [":bool", "9-13", "true"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let error = Runtime::new().eval(&expression).err().unwrap();

        assert_eq!(error.kind(), &RuntimeErrorKind::TypeMismatch);
        assert_eq!(error.location().start, 0);
        assert_eq!(error.location().end, 14);
    }

    #[test]
    fn operation_and() {
        // DW: false && true
        let json = r#"["&&", "0-6", [":bool", "0-1", "false"], [":bool", "5-6", "true"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(!result.as_bool().unwrap());
    }

    #[test]
    fn operation_or() {
        // DW: false || true
        let json = r#"["||", "0-6", [":bool", "0-1", "false"], [":bool", "5-6", "true"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn operation_not() {
        // DW: !false
        let json = r#"["!", "0-6", [":bool", "0-1", "false"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn lower_null() {
        // DW: lower(null)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "lower"], 
                [":null", "0-4"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn lower_string() {
        // DW: lower("Peregrine Expression Language")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "lower"], 
                [":str", "0-4", "Peregrine Expression Language"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("peregrine expression language", result.as_str().unwrap());
    }

    #[test]
    fn lower_boolean() {
        // DW: lower(true)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "lower"], 
                [":bool", "0-4", "true"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("true", result.as_str().unwrap());
    }

    #[test]
    fn lower_number() {
        // DW: lower(1944.07)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "lower"], 
                [":nbr", "0-4", "1944.07"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("1944.07", result.as_str().unwrap());
    }

    #[test]
    fn lower_array_fail() {
        // DW: lower(["accept", "accept-encoding", "user-agent"])
        let pel = r#"
            [":apply", "0-50", 
                [":ref", "0-5", "lower"], 
                [":array", "6-49", 
                    [":str", "7-15", "accept"], 
                    [":str", "17-34", "accept-encoding"], 
                    [":str", "36-48", "user-agent"]
                ]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let error = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(error.kind(), &RuntimeErrorKind::TypeMismatch);
        assert_eq!(error.location().start, 0);
        assert_eq!(error.location().end, 50);
    }

    #[test]
    fn selection_by_key_in_lookup_context() {
        struct LookupValueHandler(fn(&str) -> Option<Value>);
        struct LookupContext;

        const CONTEXT_ID: ContextId = ContextId::new("lookup_context");
        const A_REFERENCE: Reference = CONTEXT_ID.first_reference();
        const B_REFERENCE: Reference = A_REFERENCE.next();

        static A_VALUE_HANDLER: LookupValueHandler =
            LookupValueHandler(|key| (key == "d").then(|| Value::reference(B_REFERENCE)));
        static B_VALUE_HANDLER: LookupValueHandler =
            LookupValueHandler(|key| (key == "c").then(|| Value::number(111.0)));

        impl ValueHandler for LookupValueHandler {
            fn select_by_key(&self, key: &str) -> Option<Value> {
                self.0(key)
            }

            fn detach(&self) -> Option<Value> {
                None
            }
        }

        impl Context for LookupContext {
            fn resolve(&self, symbol: &Symbol) -> Binding {
                match symbol.as_str() {
                    "a" => Binding::Available(Value::reference(A_REFERENCE)),
                    _ => Binding::Unknown,
                }
            }

            fn value_handler(
                &self,
                reference: Reference,
            ) -> Option<&dyn crate::runtime::ValueHandler> {
                match reference {
                    A_REFERENCE => Some(&A_VALUE_HANDLER),
                    B_REFERENCE => Some(&B_VALUE_HANDLER),
                    _ => None,
                }
            }
        }

        // DW: a.d.c
        let json = r#"[".", "0-5", [".", "0-3", [":ref", "0-1", "a"], [":str", "2-3", "d"]], [":str", "4-5", "c"]]"#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval_with_context(&expression, &LookupContext)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(111, result.as_f64().unwrap() as i32);
    }

    #[test]
    fn contains_string_string_is_true() {
        // DW: contains("Peregrine expression language", "lang")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-8", "contains"],
                [":str", "5-7", "Peregrine expression language"], 
                [":str", "0-4", "lang"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn contains_string_number_is_true() {
        // DW: contains("Peregrine expression language 1.0", 1.0)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-8", "contains"],
                [":str", "5-7", "Peregrine expression language 1.0"], 
                [":nbr", "0-4", "1.0"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn contains_string_string_is_false() {
        // DW: contains("Peregrine expression language", "PEREGRINE")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-8", "contains"],
                [":str", "5-7", "Peregrine expression language"], 
                [":str", "0-4", "PEREGRINE"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(!result.as_bool().unwrap());
    }

    #[test]
    fn contains_boolean_string_is_true() {
        // DW: contains(true, "ru")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-8", "contains"],
                [":bool", "5-7", "true"], 
                [":str", "0-4", "ru"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn contains_boolean_string_is_false() {
        // DW: contains(true, "fal")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-8", "contains"],
                [":bool", "5-7", "true"], 
                [":str", "0-4", "fal"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(!result.as_bool().unwrap());
    }

    #[test]
    fn contains_number_string_is_true() {
        // DW: contains(3.141516, ".141")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-8", "contains"],
                [":nbr", "5-7", "3.141516"], 
                [":str", "0-4", ".141"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn contains_number_number_is_false() {
        // DW: contains(3.141516, 9)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-8", "contains"],
                [":nbr", "5-7", "3.141516"], 
                [":nbr", "0-4", "9"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(!result.as_bool().unwrap());
    }

    #[test]
    fn contains_array_is_true() {
        // DW: contains(["a", 10], 10)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-8", "contains"],
                [":array", "5-7", 
                    [":str", "5-7", "a"],
                    [":nbr", "5-7", "10"]
                ], 
                [":nbr", "0-4", "10"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.as_bool().unwrap());
    }

    #[test]
    fn contains_array_is_false() {
        // DW: contains(["a", false], 10)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-8", "contains"],
                [":array", "5-7", 
                    [":str", "5-7", "a"],
                    [":bool", "5-7", "false"]
                ], 
                [":nbr", "0-4", "10"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(!result.as_bool().unwrap());
    }

    #[test]
    fn trim_array() {
        // DW: trim(["one", 2, false])
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "trim"], 
                [":array", "0-17", 
                    [":str", "1-6", "one"], 
                    [":nbr", "8-9", "2"], 
                    [":bool", "11-16", "false"]
                ]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let error = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(error.kind, RuntimeErrorKind::TypeMismatch);
    }

    #[test]
    fn trim_boolean() {
        // DW: trim(true)
        let json = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "trim"], 
                [":bool", "0-4", "true"]
            ]
        "#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("true", result.as_str().unwrap());
    }

    #[test]
    fn trim_number() {
        // DW: trim(1000.0)
        let json = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "trim"], 
                [":nbr", "0-4", "1000.0"]
            ]
        "#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        // TODO: AGW-5356 - This should return "1000" when improve number coercion
        assert_eq!("1000.0", result.as_str().unwrap());
    }

    #[test]
    fn trim_string() {
        // DW: trim("   hello world   ")
        let json = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "trim"], 
                [":str", "0-4", "   hello world   "]
            ]
        "#;

        let expression = Parser::new().parse_str(json).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("hello world", result.as_str().unwrap());
    }

    #[test]
    fn generate_uuid() {
        // DW: uuid()
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "uuid"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        let uuid = result.as_str().unwrap();

        assert!(uuid::Uuid::parse_str(uuid).is_ok());
    }

    #[test]
    fn size_of_string() {
        // DW: sizeOf("hello world")
        let pel = r#"
            [":apply", "0-45", 
                [":ref", "0-10", "sizeOf"],
                [":str", "0-41", "hello world"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(11, result.as_f64().unwrap() as usize);
    }

    #[test]
    fn size_of_array() {
        // DW: sizeOf(["accept", "accept-encoding", "user-agent"])
        let pel = r#"
            [":apply", "0-45", 
                [":ref", "0-10", "sizeOf"],
                [":array", "0-41", 
                    [":str", "1-9", "accept"], 
                    [":str", "10-27", "accept-encoding"], 
                    [":str", "28-40", "user-agent"]
                ]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(3, result.as_f64().unwrap() as usize);
    }

    #[test]
    fn size_of_object() {
        let object = Value::object(Object::from([("foo".to_string(), Value::bool(true))]));

        let context = HashMap::from([("object", object)]);

        // DW: sizeOf(object)
        let pel = r#"
            [":apply", "0-45", 
                [":ref", "0-10", "sizeOf"],
                [":ref", "0-41", "object"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval_with_context(&expression, &context)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(1, result.as_f64().unwrap() as usize);
    }

    #[test]
    fn split_by_string_string() {
        // DW: splitBy("Peregrine Expression Language", "re")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "splitBy"], 
                [":str", "0-4", "Peregrine Expression Language"], 
                [":str", "8-12", "re"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        let expected = [
            Value::string("Pe".to_string()),
            Value::string("grine Exp".to_string()),
            Value::string("ssion Language".to_string()),
        ];

        assert_eq!(&expected, result.as_slice().unwrap());
    }

    #[test]
    fn split_by_number_string() {
        // DW: splitBy(1946.03, ".")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "splitBy"], 
                [":nbr", "0-4", "1946.03"], 
                [":str", "8-12", "."]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        let expected = [
            Value::string("1946".to_string()),
            Value::string("03".to_string()),
        ];

        assert_eq!(&expected, result.as_slice().unwrap());
    }

    #[test]
    fn split_by_boolean_string() {
        // DW: splitBy(true, "r")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "splitBy"], 
                [":bool", "0-4", "true"], 
                [":str", "8-12", "r"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        let expected = [
            Value::string("t".to_string()),
            Value::string("ue".to_string()),
        ];

        assert_eq!(&expected, result.as_slice().unwrap());
    }

    #[test]
    fn split_by_null_string() {
        // DW: splitBy(null, "re")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "splitBy"], 
                [":null", "0-41"], 
                [":str", "8-12", "r"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn split_by_string_null_faiil() {
        // DW: splitBy("Peregrine", null)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "splitBy"], 
                [":str", "0-41", "Peregrine"], 
                [":null", "8-12"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let error = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(error.kind, RuntimeErrorKind::TypeMismatch);
    }

    #[test]
    fn split_by_array_fail() {
        // DW: splitBy(["accept", "accept-encoding", "user-agent"], "re")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "splitBy"], 
                [":array", "0-41", 
                    [":str", "1-9", "accept"], 
                    [":str", "10-27", "accept-encoding"], 
                    [":str", "28-40", "user-agent"]
                ], 
                [":str", "8-12", "r"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let error = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(error.kind, RuntimeErrorKind::TypeMismatch);
    }

    #[test]
    fn substring_after_null_string() {
        // DW: substringAfter(null, "peregrine")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfter"], 
                [":null", "0-4"], 
                [":str", "8-12", "peregrine"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn substring_after_string_string() {
        // DW: substringAfter("peregrine", "gr")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfter"], 
                [":str", "0-4", "peregrine"], 
                [":str", "8-12", "gr"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "ine");
    }

    #[test]
    fn substring_after_string_null_fail() {
        // DW: substringAfter("peregrine", null)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfter"], 
                [":str", "0-4", "peregrine"], 
                [":null", "8-12"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let error = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(error.kind, RuntimeErrorKind::TypeMismatch);
    }

    #[test]
    fn substring_after_boolean_string() {
        // DW: substringAfter(true, "r")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfter"], 
                [":bool", "0-4", "true"], 
                [":str", "8-12", "r"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "ue");
    }

    #[test]
    fn substring_after_number_number() {
        // DW: substringAfter(1234.567, 4.5)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfter"], 
                [":nbr", "0-4", "1234.567"], 
                [":nbr", "8-12", "4.5"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "67");
    }

    #[test]
    fn substring_after_array_fail() {
        // DW: substringAfter(["accept", "accept-encoding", "user-agent"], "re")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfter"], 
                [":array", "0-41", 
                    [":str", "1-9", "accept"], 
                    [":str", "10-27", "accept-encoding"], 
                    [":str", "28-40", "user-agent"]
                ],
                [":str", "1-10", "re"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(result.kind, RuntimeErrorKind::TypeMismatch);
    }

    #[test]
    fn substring_after_last_null_string() {
        // DW: substringAfterLast(null, "peregrine")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfterLast"], 
                [":null", "0-4"], 
                [":str", "8-12", "peregrine"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn substring_after_last_string_string() {
        // DW: substringAfterLast("Peregrine Expression Language", "re")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfterLast"], 
                [":str", "0-4", "Peregrine Expression Language"], 
                [":str", "8-12", "re"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "ssion Language");
    }

    #[test]
    fn substring_after_last_string_null_fail() {
        // DW: substringAfterLast("peregrine", null)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfterLast"], 
                [":str", "0-4", "peregrine"], 
                [":null", "8-12"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let error = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(error.kind, RuntimeErrorKind::TypeMismatch);
    }

    #[test]
    fn substring_after_last_boolean_string() {
        // DW: substringAfterLast(true, "r")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfterLast"], 
                [":bool", "0-4", "true"], 
                [":str", "8-12", "r"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "ue");
    }

    #[test]
    fn substring_after_last_number_number() {
        // DW: substringAfterLast(12123512.3512, 35)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfterLast"], 
                [":nbr", "0-4", "12123512.3512"], 
                [":nbr", "8-12", "35"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "12");
    }

    #[test]
    fn substring_after_last_array_fail() {
        // DW: substringAfterLast(["accept", "accept-encoding", "user-agent"], "re")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringAfterLast"], 
                [":array", "0-41", 
                    [":str", "1-9", "accept"], 
                    [":str", "10-27", "accept-encoding"], 
                    [":str", "28-40", "user-agent"]
                ],
                [":str", "1-10", "re"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(result.kind, RuntimeErrorKind::TypeMismatch);
    }

    #[test]
    fn substring_before_null_string() {
        // DW: substringBefore(null, "peregrine")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBefore"], 
                [":null", "0-4"], 
                [":str", "8-12", "peregrine"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn substring_before_string_string() {
        // DW: substringBefore("peregrine", "gr")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBefore"], 
                [":str", "0-4", "peregrine"], 
                [":str", "8-12", "gr"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "pere");
    }

    #[test]
    fn substring_before_string_null_fail() {
        // DW: substringBefore("peregrine", null)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBefore"], 
                [":str", "0-4", "peregrine"], 
                [":null", "8-12"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(result.kind, RuntimeErrorKind::TypeMismatch);
    }

    #[test]
    fn substring_before_boolean_string() {
        // DW: substringBefore(true, "ue")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBefore"], 
                [":bool", "0-4", "true"], 
                [":str", "8-12", "ue"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "tr");
    }

    #[test]
    fn substring_before_number_number() {
        // DW: substringBefore(1234.56, 4.5)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBefore"], 
                [":nbr", "0-4", "1234.56"], 
                [":nbr", "8-12", "4.5"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "123");
    }

    #[test]
    fn substring_before_array_fail() {
        // DW: substringBefore(["accept", "accept-encoding", "user-agent"], "re")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBefore"], 
                [":array", "0-41", 
                    [":str", "1-9", "accept"], 
                    [":str", "10-27", "accept-encoding"], 
                    [":str", "28-40", "user-agent"]
                ],
                [":str", "1-10", "re"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(result.kind, RuntimeErrorKind::TypeMismatch);
    }

    #[test]
    fn substring_before_last_null_string() {
        // DW: substringBeforeLast(null, "peregrine")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBeforeLast"], 
                [":null", "0-4"], 
                [":str", "8-12", "peregrine"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn substring_before_last_string_string() {
        // DW: substringBeforeLast("Peregrine Expression Language", "re")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBeforeLast"], 
                [":str", "0-4", "Peregrine Expression Language"], 
                [":str", "8-12", "re"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "Peregrine Exp");
    }

    #[test]
    fn substring_before_last_string_null_fail() {
        // DW: substringBeforeLast("peregrine", null)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBeforeLast"], 
                [":str", "0-4", "peregrine"], 
                [":null", "8-12"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(result.kind, RuntimeErrorKind::TypeMismatch);
    }

    #[test]
    fn substring_before_last_boolean_string() {
        // DW: substringBeforeLast(true, "ue")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBeforeLast"], 
                [":bool", "0-4", "true"], 
                [":str", "8-12", "ue"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "tr");
    }

    #[test]
    fn substring_before_last_number_number() {
        // DW: substringBeforeLast(121235.123512, 4.5)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBeforeLast"], 
                [":nbr", "0-4", "121235.123512"], 
                [":nbr", "8-12", "12"]
            ]"#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!(result.as_str().unwrap(), "121235.1235");
    }

    #[test]
    fn substring_before_last_array_fail() {
        // DW: substringBeforeLast(["accept", "accept-encoding", "user-agent"], "re")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "substringBeforeLast"], 
                [":array", "0-41", 
                    [":str", "1-9", "accept"], 
                    [":str", "10-27", "accept-encoding"], 
                    [":str", "28-40", "user-agent"]
                ],
                [":str", "1-10", "re"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(result.kind, RuntimeErrorKind::TypeMismatch);
    }

    #[test]
    fn upper_null() {
        // DW: upper(null)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "upper"], 
                [":null", "0-4"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert!(result.is_null());
    }

    #[test]
    fn upper_string() {
        // DW: upper("Peregrine Expression Language")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "upper"], 
                [":str", "0-4", "Peregrine Expression Language"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("PEREGRINE EXPRESSION LANGUAGE", result.as_str().unwrap());
    }

    #[test]
    fn upper_boolean() {
        // DW: upper(true)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "upper"], 
                [":bool", "0-4", "true"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("TRUE", result.as_str().unwrap());
    }

    #[test]
    fn upper_number() {
        // DW: upper(123.4)
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "upper"], 
                [":nbr", "0-4", "123.4"]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let result = Runtime::new()
            .eval(&expression)
            .unwrap()
            .complete()
            .unwrap();

        assert_eq!("123.4", result.as_str().unwrap());
    }

    #[test]
    fn upper_array_fail() {
        // DW: upper(["accept", "accept-encoding", "user-agent"], "re")
        let pel = r#"
            [":apply", "5-7", 
                [":ref", "5-7", "upper"], 
                [":array", "0-41", 
                    [":str", "1-9", "accept"], 
                    [":str", "10-27", "accept-encoding"], 
                    [":str", "28-40", "user-agent"]
                ]
            ]
        "#;

        let expression = Parser::new().parse_str(pel).unwrap();
        let error = Runtime::new().eval(&expression).unwrap_err();

        assert_eq!(error.kind, RuntimeErrorKind::TypeMismatch);
    }

    mod partial_evaluation {
        use std::collections::HashMap;

        use crate::{
            expression::Symbol,
            parser::Parser,
            runtime::{value::Value, Binding, Context, Runtime, ValueHandler},
            Reference,
        };

        struct TestContextChain {
            contexts: Vec<HashMap<String, Value>>,
        }

        impl TestContextChain {
            fn new<const N: usize>(entries: [(&str, Value); N]) -> Self {
                Self { contexts: vec![] }.then(entries)
            }

            fn then<const N: usize>(mut self, entries: [(&str, Value); N]) -> Self {
                self.contexts
                    .push(entries.map(|(k, v)| (k.to_string(), v)).into());
                self
            }

            fn next(mut self) -> Self {
                self.contexts.remove(0);
                self
            }
        }

        impl Context for TestContextChain {
            fn resolve(&self, symbol: &Symbol) -> Binding {
                let s = symbol.as_str();
                match self.contexts.get(0).expect("Empty context").get(s) {
                    Some(value) => Binding::Available(value.clone()),
                    None => {
                        if self.contexts.iter().any(|c| c.contains_key(s)) {
                            Binding::Pending
                        } else {
                            Binding::Unknown
                        }
                    }
                }
            }

            fn value_handler(&self, _reference: Reference) -> Option<&dyn ValueHandler> {
                None
            }
        }

        #[test]
        fn if_else_pending_condition() {
            let runtime = Runtime::new();
            let parser = Parser::new();

            let context_1 = TestContextChain::new([("a", Value::string("ctx1".to_string()))])
                .then([("condition", Value::bool(true))])
                .then([("b", Value::string("ctx3".to_string()))]);

            // DW: if (condition) a else b
            let pel_1 = r#"[":if", "0-23", [":ref", "4-9", "condition"], [":ref", "11-14", "a"], [":ref", "20-23", "b"]]"#;

            let expression_1 = parser.parse_str(pel_1).unwrap();
            let expression_2 = runtime
                .eval_with_context(&expression_1, &context_1)
                .unwrap()
                .partial()
                .unwrap();

            // DW: if (condition) "ctx1" else b
            let pel_2 = r#"[":if", "0-23", [":ref", "4-9", "condition"], [":str", "11-14", "ctx1"], [":ref", "20-23", "b"]]"#;

            assert_eq!(expression_2, parser.parse_str(pel_2).unwrap());

            let context_2 = context_1.next();
            let result = runtime
                .eval_with_context(&expression_2, &context_2)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(result.as_str().unwrap(), "ctx1");
        }

        #[test]
        fn if_else_pending_true_branch() {
            let runtime = Runtime::new();
            let parser = Parser::new();

            let context_1 = TestContextChain::new([("a", Value::string("ctx1".to_string()))])
                .then([("b", Value::string("ctx2".to_string()))]);

            // DW: if (false) a else b
            let pel_1 = r#"[":if", "0-23", [":bool", "4-9", "true"], [":ref", "11-14", "b"], [":ref", "20-23", "a"]]"#;

            let expression_1 = parser.parse_str(pel_1).unwrap();
            let expression_2 = runtime
                .eval_with_context(&expression_1, &context_1)
                .unwrap()
                .partial()
                .unwrap();

            let pel_2 = r#"[":ref", "11-14", "b"]"#;

            assert_eq!(expression_2, parser.parse_str(pel_2).unwrap());

            let context_2 = context_1.next();
            let result = runtime
                .eval_with_context(&expression_2, &context_2)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(result.as_str().unwrap(), "ctx2");
        }

        #[test]
        fn if_else_pending_false_branch() {
            let runtime = Runtime::new();
            let parser = Parser::new();

            let context_1 = TestContextChain::new([("a", Value::string("ctx1".to_string()))])
                .then([("b", Value::string("ctx2".to_string()))]);

            // DW: if (false) a else b
            let pel_1 = r#"[":if", "0-23", [":bool", "4-9", "false"], [":ref", "11-14", "a"], [":ref", "20-23", "b"]]"#;

            let expression_1 = parser.parse_str(pel_1).unwrap();
            let expression_2 = runtime
                .eval_with_context(&expression_1, &context_1)
                .unwrap()
                .partial()
                .unwrap();

            let pel_2 = r#"[":ref", "20-23", "b"]"#;

            assert_eq!(expression_2, parser.parse_str(pel_2).unwrap());

            let context_2 = context_1.next();
            let result = runtime
                .eval_with_context(&expression_2, &context_2)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(result.as_str().unwrap(), "ctx2");
        }

        #[test]
        fn default_left_unavailable() {
            let runtime = Runtime::new();
            let parser = Parser::new();

            let context_1 = TestContextChain::new([("right", Value::string("ctx1".to_string()))])
                .then([("left", Value::null())]);

            // DW: left default right
            let pel_1 = r#"
                [":default", "0-6", 
                    [":ref", "0-1", "left"], 
                    [":ref", "5-6", "right"]
                ]
            "#;

            let expression_1 = parser.parse_str(pel_1).unwrap();
            let expression_2 = runtime
                .eval_with_context(&expression_1, &context_1)
                .unwrap()
                .partial()
                .unwrap();

            // DW: left default "ctx1"
            let pel_2 = r#"
                [":default", "0-6", 
                    [":ref", "0-1", "left"], 
                    [":str", "5-6", "ctx1"]
                ]
            "#;

            assert_eq!(expression_2, parser.parse_str(pel_2).unwrap());

            let context_2 = context_1.next();
            let result = runtime
                .eval_with_context(&expression_2, &context_2)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(result.as_str().unwrap(), "ctx1");
        }

        #[test]
        fn default_right_unavailable() {
            let runtime = Runtime::new();
            let parser = Parser::new();

            let context_1 = TestContextChain::new([("left", Value::null())])
                .then([("right", Value::string("ctx1".to_string()))]);

            // DW: left default right
            let pel_1 = r#"
                [":default", "0-6", 
                    [":ref", "0-1", "left"], 
                    [":ref", "5-6", "right"]
                ]
            "#;

            let expression_1 = parser.parse_str(pel_1).unwrap();
            let expression_2 = runtime
                .eval_with_context(&expression_1, &context_1)
                .unwrap()
                .partial()
                .unwrap();

            // DW: right
            let pel_2 = r#"
                [":ref", "5-6", "right"]
            "#;

            assert_eq!(expression_2, parser.parse_str(pel_2).unwrap());

            let context_2 = context_1.next();
            let result = runtime
                .eval_with_context(&expression_2, &context_2)
                .unwrap()
                .complete()
                .unwrap();

            assert_eq!(result.as_str().unwrap(), "ctx1");
        }
    }
}
