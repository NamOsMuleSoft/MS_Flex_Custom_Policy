// Copyright 2023 Salesforce, Inc. All rights reserved.
use pdk_core::policy_context::authentication;
use pel::runtime::value::Value;

pub trait IntoValue {
    fn into_value(self) -> Value;
}

impl IntoValue for Value {
    fn into_value(self) -> Value {
        self
    }
}

impl IntoValue for bool {
    fn into_value(self) -> Value {
        Value::bool(self)
    }
}

impl IntoValue for f64 {
    fn into_value(self) -> Value {
        Value::number(self)
    }
}

impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::string(self.to_string())
    }
}

impl IntoValue for String {
    fn into_value(self) -> Value {
        Value::string(self)
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(self) -> Value {
        self.map(|v| v.into_value()).unwrap_or_else(Value::null)
    }
}

impl<T: IntoValue> IntoValue for Vec<T> {
    fn into_value(self) -> Value {
        Value::array(self.into_iter().map(|v| v.into_value()).collect())
    }
}

pub(crate) fn authentication_object_to_value(o: &authentication::Object) -> Value {
    Value::object(o.iter().map(|(k, v)| (k.clone(), v.into_value())).collect())
}

impl IntoValue for authentication::Value {
    fn into_value(self) -> Value {
        match self {
            authentication::Value::Null => Value::null(),
            authentication::Value::Bool(b) => Value::bool(b),
            authentication::Value::Number(n) => Value::number(n),
            authentication::Value::String(s) => Value::string(s),
            authentication::Value::Array(a) => {
                Value::array(a.into_iter().map(|v| v.into_value()).collect())
            }
            authentication::Value::Object(o) => {
                Value::object(o.into_iter().map(|(k, v)| (k, v.into_value())).collect())
            }
        }
    }
}

impl IntoValue for &authentication::Value {
    fn into_value(self) -> Value {
        match self {
            authentication::Value::Null => Value::null(),
            authentication::Value::Bool(b) => Value::bool(*b),
            authentication::Value::Number(n) => Value::number(*n),
            authentication::Value::String(s) => Value::string(s.clone()),
            authentication::Value::Array(a) => {
                Value::array(a.iter().map(|v| v.into_value()).collect())
            }
            authentication::Value::Object(o) => authentication_object_to_value(o),
        }
    }
}

impl IntoValue for serde_json::Value {
    fn into_value(self) -> Value {
        match self {
            serde_json::Value::Null => Value::null(),
            serde_json::Value::Bool(b) => Value::bool(b),
            serde_json::Value::String(s) => Value::string(s),
            serde_json::Value::Number(n) => {
                n.as_f64().map(Value::number).unwrap_or_else(Value::null)
            }
            serde_json::Value::Array(a) => {
                Value::array(a.into_iter().map(|v| v.into_value()).collect())
            }
            serde_json::Value::Object(o) => {
                Value::object(o.into_iter().map(|(k, v)| (k, v.into_value())).collect())
            }
        }
    }
}

impl IntoValue for &serde_json::Value {
    fn into_value(self) -> Value {
        match self {
            serde_json::Value::Null => Value::null(),
            serde_json::Value::Bool(b) => Value::bool(*b),
            serde_json::Value::String(s) => Value::string(s.clone()),
            serde_json::Value::Number(n) => {
                n.as_f64().map(Value::number).unwrap_or_else(Value::null)
            }
            serde_json::Value::Array(a) => Value::array(a.iter().map(|v| v.into_value()).collect()),
            serde_json::Value::Object(o) => {
                Value::object(o.iter().map(|(k, v)| (k.clone(), v.into_value())).collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use pel::runtime::value::{Object, Value};
    use serde_json::json;

    #[test]
    fn option_into_value() {
        let value: Option<&str> = None;
        assert_eq!(Value::null(), value.into_value());
        assert_eq!(Value::number(123.4), Some(123.4).into_value());
    }

    #[test]
    fn bool_into_value() {
        assert_eq!(Value::bool(false), false.into_value());
        assert_eq!(Value::bool(true), true.into_value());
    }

    #[test]
    fn f64_into_value() {
        assert_eq!(Value::number(456.7), 456.7.into_value());
    }

    #[test]
    fn str_into_value() {
        assert_eq!(Value::string("peregrine".into()), "peregrine".into_value());
    }

    #[test]
    fn string_into_value() {
        assert_eq!(
            Value::string("peregrine".into()),
            String::from("peregrine").into_value()
        );
    }

    #[test]
    fn vec_into_value() {
        assert_eq!(
            Value::array(vec![Value::number(12.3), Value::number(45.6)]),
            vec![12.3, 45.6].into_value()
        );
    }

    #[test]
    fn authentication_value_into_value() {
        assert_eq!(Value::null(), authentication::Value::Null.into_value());
        assert_eq!(
            Value::bool(true),
            authentication::Value::Bool(true).into_value()
        );
        assert_eq!(
            Value::bool(false),
            authentication::Value::Bool(false).into_value()
        );
        assert_eq!(
            Value::number(11.5),
            authentication::Value::Number(11.5f64).into_value()
        );
        assert_eq!(
            Value::string("peregrine".into()),
            authentication::Value::String("peregrine".into()).into_value()
        );

        assert_eq!(
            Value::array(vec![
                Value::null(),
                Value::bool(true),
                Value::number(11.5),
                Value::string("peregrine".into())
            ]),
            authentication::Value::Array(authentication::Array::from([
                authentication::Value::Null,
                authentication::Value::Bool(true),
                authentication::Value::Number(11.5),
                authentication::Value::String("peregrine".into())
            ]))
            .into_value()
        );

        assert_eq!(
            Value::object(Object::from([
                ("foo".into(), Value::null()),
                ("bar".into(), Value::bool(true)),
                ("baz".into(), Value::number(11.5)),
                ("baf".into(), Value::string("peregrine".into())),
            ])),
            authentication::Value::Object(authentication::Object::from([
                ("foo".into(), authentication::Value::Null),
                ("bar".into(), authentication::Value::Bool(true)),
                ("baz".into(), authentication::Value::Number(11.5)),
                (
                    "baf".into(),
                    authentication::Value::String("peregrine".into())
                ),
            ]))
            .into_value()
        );
    }

    #[test]
    fn authentication_value_ref_into_value() {
        assert_eq!(Value::null(), (&authentication::Value::Null).into_value());
        assert_eq!(
            Value::bool(true),
            (&authentication::Value::Bool(true)).into_value()
        );
        assert_eq!(
            Value::bool(false),
            (&authentication::Value::Bool(false)).into_value()
        );
        assert_eq!(
            Value::number(11.5),
            (&authentication::Value::Number(11.5f64)).into_value()
        );
        assert_eq!(
            Value::string("peregrine".into()),
            (&authentication::Value::String("peregrine".into())).into_value()
        );

        assert_eq!(
            Value::array(vec![
                Value::null(),
                Value::bool(true),
                Value::number(11.5),
                Value::string("peregrine".into())
            ]),
            (&authentication::Value::Array(authentication::Array::from([
                authentication::Value::Null,
                authentication::Value::Bool(true),
                authentication::Value::Number(11.5),
                authentication::Value::String("peregrine".into())
            ])))
                .into_value()
        );

        assert_eq!(
            Value::object(Object::from([
                ("foo".into(), Value::null()),
                ("bar".into(), Value::bool(true)),
                ("baz".into(), Value::number(11.5)),
                ("baf".into(), Value::string("peregrine".into())),
            ])),
            (&authentication::Value::Object(authentication::Object::from([
                ("foo".into(), authentication::Value::Null),
                ("bar".into(), authentication::Value::Bool(true)),
                ("baz".into(), authentication::Value::Number(11.5)),
                (
                    "baf".into(),
                    authentication::Value::String("peregrine".into())
                ),
            ])))
                .into_value()
        );
    }

    #[test]
    fn serde_json_into_value() {
        assert_eq!(Value::null(), json!(null).into_value());
        assert_eq!(Value::bool(true), json!(true).into_value());
        assert_eq!(Value::number(11.5), json!(11.5f64).into_value());
        assert_eq!(
            Value::string("peregrine".into()),
            json!("peregrine").into_value()
        );

        assert_eq!(
            Value::array(vec![
                Value::null(),
                Value::bool(true),
                Value::number(11.5),
                Value::string("peregrine".into())
            ]),
            json!([null, true, 11.5f64, "peregrine"]).into_value()
        );

        assert_eq!(
            Value::object(Object::from([
                ("foo".into(), Value::null()),
                ("bar".into(), Value::bool(true)),
                ("baz".into(), Value::number(11.5)),
                ("baf".into(), Value::string("peregrine".into())),
            ])),
            json!({
                "foo": null,
                "bar": true,
                "baz": 11.5f64,
                "baf": "peregrine"
            })
            .into_value()
        );
    }

    #[test]
    fn serde_json_ref_into_value() {
        assert_eq!(Value::null(), (&json!(null)).into_value());
        assert_eq!(Value::bool(true), (&json!(true)).into_value());
        assert_eq!(Value::number(11.5), (&json!(11.5f64)).into_value());
        assert_eq!(
            Value::string("peregrine".into()),
            (&json!("peregrine")).into_value()
        );

        assert_eq!(
            Value::array(vec![
                Value::null(),
                Value::bool(true),
                Value::number(11.5),
                Value::string("peregrine".into())
            ]),
            (&json!([null, true, 11.5f64, "peregrine"])).into_value()
        );

        assert_eq!(
            Value::object(Object::from([
                ("foo".into(), Value::null()),
                ("bar".into(), Value::bool(true)),
                ("baz".into(), Value::number(11.5)),
                ("baf".into(), Value::string("peregrine".into())),
            ])),
            (&json!({
                "foo": null,
                "bar": true,
                "baz": 11.5f64,
                "baf": "peregrine"
            }))
                .into_value()
        );
    }
}
