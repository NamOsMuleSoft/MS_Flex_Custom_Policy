// Copyright 2023 Salesforce, Inc. All rights reserved.
use crate::{
    runtime::{
        coercion::CoerceArguments, value::Value, Context, Prelude, RuntimeError, RuntimeErrorKind,
        ValueHandler,
    },
    Location,
};

use super::coercion::Coerce;

fn concat(location: Location, _: &dyn Context, arguments: &[Value]) -> Result<Value, RuntimeError> {
    let (a, b): (String, String) = arguments.coerce_arguments(location)?;
    let b = b.as_str();
    Ok(Value::string(a + b))
}

fn contains(
    location: Location,
    _: &dyn Context,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    match arguments {
        [container, content] => {
            let result = if let Some(container) = container.as_str() {
                let pattern: String = content.coerce(location)?;
                container.contains(&pattern)
            } else if let Some(array) = container.as_slice() {
                array.contains(content)
            } else {
                let container: String = container.coerce(location)?;
                let pattern: String = content.coerce(location)?;
                container.contains(&pattern)
            };
            Ok(Value::bool(result))
        }
        [] => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::NotEnoughArguments,
        }),
        _ => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::TooManyArguments,
        }),
    }
}

fn lower(location: Location, _: &dyn Context, arguments: &[Value]) -> Result<Value, RuntimeError> {
    match arguments {
        [text] => {
            if text.is_null() {
                Ok(Value::null())
            } else {
                let text: String = text.coerce(location)?;
                Ok(Value::string(text.to_lowercase()))
            }
        }
        [] => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::NotEnoughArguments,
        }),
        _ => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::TooManyArguments,
        }),
    }
}

fn trim(location: Location, _: &dyn Context, arguments: &[Value]) -> Result<Value, RuntimeError> {
    let (s,): (String,) = arguments.coerce_arguments(location)?;
    Ok(Value::string(s.trim().to_string()))
}

fn uuid_v4(
    location: Location,
    _: &dyn Context,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    if !arguments.is_empty() {
        return Err(RuntimeError {
            location,
            kind: super::RuntimeErrorKind::TooManyArguments,
        });
    }

    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|_| RuntimeError {
        location,
        kind: RuntimeErrorKind::UnavailableRandomGenerator,
    })?;

    let mut builder = uuid::Builder::from_bytes(bytes);
    let uuid = builder
        .set_variant(uuid::Variant::RFC4122)
        .set_version(uuid::Version::Random)
        .as_uuid();

    Ok(Value::string(uuid.to_string()))
}

fn size_of(
    location: Location,
    context: &dyn Context,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    let object = arguments.get(0).ok_or(RuntimeError {
        location,
        kind: RuntimeErrorKind::TooManyArguments,
    })?;
    let size = object
        .to_value_handler(context)
        .and_then(|vh| vh.size())
        .ok_or(RuntimeError {
            location,
            kind: RuntimeErrorKind::UnavailableSize,
        })?;
    Ok(Value::number(size as f64))
}

fn split_by(
    location: Location,
    _: &dyn Context,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    match arguments {
        [text, separator] => {
            if text.is_null() {
                Ok(Value::null())
            } else {
                let text: String = text.coerce(location)?;
                let separator: String = separator.coerce(location)?;
                let s = text
                    .split(&separator)
                    .map(|s| Value::string(s.to_string()))
                    .collect();
                Ok(Value::array(s))
            }
        }
        [] => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::NotEnoughArguments,
        }),
        _ => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::TooManyArguments,
        }),
    }
}

fn substring_after(
    location: Location,
    _: &dyn Context,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    match arguments {
        [text, separator] => {
            if text.is_null() {
                Ok(Value::null())
            } else {
                let text: String = text.coerce(location)?;
                let separator: String = separator.coerce(location)?;
                if let Some((_, result)) = text.split_once(&separator) {
                    Ok(Value::string(result.to_string()))
                } else {
                    Ok(Value::null())
                }
            }
        }
        [] => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::NotEnoughArguments,
        }),
        _ => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::TooManyArguments,
        }),
    }
}

fn substring_after_last(
    location: Location,
    _: &dyn Context,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    match arguments {
        [text, separator] => {
            if text.is_null() {
                Ok(Value::null())
            } else {
                let text: String = text.coerce(location)?;
                let separator: String = separator.coerce(location)?;
                if let Some((_, result)) = text.rsplit_once(&separator) {
                    Ok(Value::string(result.to_string()))
                } else {
                    Ok(Value::null())
                }
            }
        }
        [] => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::NotEnoughArguments,
        }),
        _ => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::TooManyArguments,
        }),
    }
}

fn substring_before(
    location: Location,
    _: &dyn Context,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    match arguments {
        [text, separator] => {
            if text.is_null() {
                Ok(Value::null())
            } else {
                let text: String = text.coerce(location)?;
                let separator: String = separator.coerce(location)?;
                if let Some((result, _)) = text.split_once(&separator) {
                    Ok(Value::string(result.to_string()))
                } else {
                    Ok(Value::null())
                }
            }
        }
        [] => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::NotEnoughArguments,
        }),
        _ => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::TooManyArguments,
        }),
    }
}

fn substring_before_last(
    location: Location,
    _: &dyn Context,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    match arguments {
        [text, separator] => {
            if text.is_null() {
                Ok(Value::null())
            } else {
                let text: String = text.coerce(location)?;
                let separator: String = separator.coerce(location)?;
                if let Some((result, _)) = text.rsplit_once(&separator) {
                    Ok(Value::string(result.to_string()))
                } else {
                    Ok(Value::null())
                }
            }
        }
        [] => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::NotEnoughArguments,
        }),
        _ => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::TooManyArguments,
        }),
    }
}

fn upper(location: Location, _: &dyn Context, arguments: &[Value]) -> Result<Value, RuntimeError> {
    match arguments {
        [text] => {
            if text.is_null() {
                Ok(Value::null())
            } else {
                let text: String = text.coerce(location)?;
                Ok(Value::string(text.to_uppercase()))
            }
        }
        [] => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::NotEnoughArguments,
        }),
        _ => Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::TooManyArguments,
        }),
    }
}

type PreludeFunction = fn(Location, &dyn Context, &[Value]) -> Result<Value, RuntimeError>;

static PRELUDE: &[(&str, PreludeFunction)] = &[
    ("++", concat),
    ("contains", contains),
    ("lower", lower),
    ("sizeOf", size_of),
    ("splitBy", split_by),
    ("substringAfter", substring_after),
    ("substringAfterLast", substring_after_last),
    ("substringBefore", substring_before),
    ("substringBeforeLast", substring_before_last),
    ("trim", trim),
    ("upper", upper),
    ("uuid", uuid_v4),
];

pub fn prelude() -> Prelude {
    PRELUDE
        .iter()
        .map(|(key, function)| (*key, Value::function_from_fn(function)))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::runtime::{Binding, ValueHandler};

    use super::{concat, split_by, trim, Context, Location, Value};

    struct TestContext;

    impl Context for TestContext {
        fn resolve(&self, _: &crate::expression::Symbol) -> Binding {
            unreachable!()
        }

        fn value_handler(&self, _reference: crate::Reference) -> Option<&dyn ValueHandler> {
            unreachable!()
        }
    }

    const LOCATION: Location = Location {
        start: 100,
        end: 200,
    };

    const CONTEXT: &dyn Context = &TestContext;

    #[test]
    fn trim_string() {
        let result = trim(
            LOCATION,
            CONTEXT,
            &[Value::string("  hello world  ".to_string())],
        )
        .unwrap();
        assert_eq!("hello world", result.as_str().unwrap());
    }

    #[test]
    fn concat_strings() {
        let result = concat(
            LOCATION,
            CONTEXT,
            &[
                Value::string("hello ".to_string()),
                Value::string("world".to_string()),
            ],
        )
        .unwrap();
        assert_eq!("hello world", result.as_str().unwrap());
    }

    #[test]
    fn concat_numbers() {
        let result = concat(
            LOCATION,
            CONTEXT,
            &[Value::number(15.04), Value::number(12.01)],
        )
        .unwrap();
        assert_eq!("15.0412.01", result.as_str().unwrap());
    }

    #[test]
    fn concat_booleans() {
        let result = concat(LOCATION, CONTEXT, &[Value::bool(true), Value::bool(false)]).unwrap();
        assert_eq!("truefalse", result.as_str().unwrap());
    }

    #[test]
    fn fail_when_concat_null() {
        let result = concat(LOCATION, CONTEXT, &[Value::null(), Value::bool(false)]);
        assert!(result.is_err());
    }

    #[test]
    fn split_by_strings() {
        let result = split_by(
            LOCATION,
            CONTEXT,
            &[
                Value::string("Peregrine Expression Language".to_string()),
                Value::string("re".to_string()),
            ],
        )
        .unwrap();
        let expected = &[
            Value::string("Pe".to_string()),
            Value::string("grine Exp".to_string()),
            Value::string("ssion Language".to_string()),
        ];
        assert_eq!(expected, result.as_slice().unwrap());
    }
}
