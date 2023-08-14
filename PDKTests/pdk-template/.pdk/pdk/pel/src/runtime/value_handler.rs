// Copyright 2023 Salesforce, Inc. All rights reserved.
use super::{
    value::{Array, InternalValue, Object, Value},
    Context,
};

pub trait ValueHandler {
    fn detach(&self) -> Option<Value>;

    fn select_by_key(&self, _key: &str) -> Option<Value> {
        None
    }

    fn select_by_index(&self, _index: usize) -> Option<Value> {
        None
    }

    fn subselect_by_key(&self, key: &str) -> Option<Value> {
        self.select_by_key(key)
    }

    fn size(&self) -> Option<usize> {
        None
    }
}

enum ContextualHandler<'a> {
    Any(Any<'a>),
    Array(OwnedValueHandler<'a, Array>),
    Object(OwnedValueHandler<'a, Object>),
    Reference(&'a dyn ValueHandler),
}

impl<'a> ContextualHandler<'a> {
    fn internal_value_handler(&'a self) -> &'a dyn ValueHandler {
        match *self {
            Self::Any(ref a) => a,
            Self::Reference(r) => r,
            Self::Array(ref a) => a,
            Self::Object(ref o) => o,
        }
    }
}

impl ValueHandler for ContextualHandler<'_> {
    fn detach(&self) -> Option<Value> {
        self.internal_value_handler().detach()
    }

    fn select_by_key(&self, key: &str) -> Option<Value> {
        self.internal_value_handler().select_by_key(key)
    }

    fn select_by_index(&self, index: usize) -> Option<Value> {
        self.internal_value_handler().select_by_index(index)
    }

    fn subselect_by_key(&self, key: &str) -> Option<Value> {
        self.internal_value_handler().select_by_key(key)
    }

    fn size(&self) -> Option<usize> {
        self.internal_value_handler().size()
    }
}

struct Any<'a>(&'a Value);

impl ValueHandler for Any<'_> {
    fn detach(&self) -> Option<Value> {
        Some(self.0.clone())
    }
}

struct NullValueHandler;

impl ValueHandler for NullValueHandler {
    fn detach(&self) -> Option<Value> {
        Some(Value::null())
    }

    fn select_by_key(&self, _key: &str) -> Option<Value> {
        Some(Value::null())
    }

    fn select_by_index(&self, _index: usize) -> Option<Value> {
        Some(Value::null())
    }

    fn size(&self) -> Option<usize> {
        None
    }
}

impl ValueHandler for String {
    fn detach(&self) -> Option<Value> {
        Some(Value::string(self.clone()))
    }

    fn select_by_index(&self, index: usize) -> Option<Value> {
        Some(
            self.chars()
                .nth(index)
                .map(|c| Value::string(c.to_string()))
                .unwrap_or_else(Value::null),
        )
    }

    fn size(&self) -> Option<usize> {
        Some(self.len())
    }
}

struct OwnedValueHandler<'a, T> {
    context: &'a dyn Context,
    value: &'a T,
}

impl<'a, T> OwnedValueHandler<'a, T> {
    fn new(context: &'a dyn Context, value: &'a T) -> Self {
        Self { context, value }
    }
}

impl ValueHandler for OwnedValueHandler<'_, Array> {
    fn select_by_index(&self, index: usize) -> Option<Value> {
        Some(self.value.get(index).cloned().unwrap_or_else(Value::null))
    }

    fn select_by_key(&self, key: &str) -> Option<Value> {
        let array = self
            .value
            .iter()
            .filter_map(|v| v.to_value_handler(self.context))
            .filter_map(|vh| vh.select_by_key(key))
            .collect();

        Some(Value::array(array))
    }

    fn size(&self) -> Option<usize> {
        Some(self.value.len())
    }

    fn detach(&self) -> Option<Value> {
        let array = self
            .value
            .iter()
            .filter_map(|v| v.to_value_handler(self.context))
            .filter_map(|vh| vh.detach())
            .collect();

        Some(Value::array(array))
    }
}

impl ValueHandler for OwnedValueHandler<'_, Object> {
    fn select_by_index(&self, index: usize) -> Option<Value> {
        Some(
            self.value
                .values()
                .nth(index)
                .cloned()
                .unwrap_or_else(Value::null),
        )
    }

    fn select_by_key(&self, key: &str) -> Option<Value> {
        Some(self.value.get(key).cloned().unwrap_or_else(Value::null))
    }

    fn size(&self) -> Option<usize> {
        Some(self.value.len())
    }

    fn detach(&self) -> Option<Value> {
        let object = self
            .value
            .iter()
            .filter_map(|(k, v)| v.to_value_handler(self.context).map(|ch| (k, ch)))
            .filter_map(|(k, vh)| vh.detach().map(|v| (k.clone(), v)))
            .collect();
        Some(Value::object(object))
    }
}

impl Value {
    pub(crate) fn to_value_handler<'a>(
        &'a self,
        context: &'a dyn Context,
    ) -> Option<impl ValueHandler + 'a> {
        match &self.internal {
            InternalValue::Null => Some(ContextualHandler::Reference(&NullValueHandler)),
            InternalValue::String(s) => Some(ContextualHandler::Reference(s)),
            InternalValue::Array(array) => Some(ContextualHandler::Array(OwnedValueHandler::new(
                context, array,
            ))),
            InternalValue::Object(object) => Some(ContextualHandler::Object(
                OwnedValueHandler::new(context, object),
            )),
            InternalValue::Reference(reference) => context
                .value_handler(*reference)
                .map(ContextualHandler::Reference),
            _ => Some(ContextualHandler::Any(Any(self))),
        }
    }
}
