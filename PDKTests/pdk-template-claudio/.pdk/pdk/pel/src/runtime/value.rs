// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::{collections::HashMap, fmt::Debug, rc::Rc};

use crate::{
    runtime::{Context, RuntimeError},
    Location, Reference,
};

pub type Object = HashMap<String, Value>;
pub type Array = Vec<Value>;

pub trait Function {
    fn apply(
        &self,
        location: Location,
        context: &dyn Context,
        arguments: &[Value],
    ) -> Result<Value, RuntimeError>;
}

struct DefaultFunction<F>(F);

impl<F> Function for DefaultFunction<F>
where
    F: Fn(Location, &dyn Context, &[Value]) -> Result<Value, RuntimeError>,
{
    fn apply(
        &self,
        location: Location,
        context: &dyn Context,
        arguments: &[Value],
    ) -> Result<Value, RuntimeError> {
        self.0(location, context, arguments)
    }
}

#[derive(Clone)]
pub(super) struct FunctionValue(Rc<dyn Function>);

impl PartialEq for FunctionValue {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Debug for FunctionValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeFunction")
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeNumber {
    value: f64,
    representation: String,
}

impl RuntimeNumber {
    pub fn representation(&self) -> &str {
        &self.representation
    }
}

impl PartialEq for RuntimeNumber {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum InternalValue {
    Null,
    Bool(bool),
    Number(RuntimeNumber),
    String(String),
    Array(Rc<Array>),
    Object(Rc<Object>),
    Function(FunctionValue),
    Reference(Reference),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    pub(super) internal: InternalValue,
}

impl Value {
    pub fn null() -> Self {
        Self {
            internal: InternalValue::Null,
        }
    }

    pub fn bool(b: bool) -> Self {
        Self {
            internal: InternalValue::Bool(b),
        }
    }

    pub fn number(value: f64) -> Self {
        // TODO: AGW-5356 - Improve number coercion
        Self::number_with_representation(value, value.to_string())
    }

    pub(crate) fn number_with_representation(value: f64, representation: String) -> Self {
        Self {
            internal: InternalValue::Number(RuntimeNumber {
                value,
                representation,
            }),
        }
    }

    pub fn string(s: String) -> Self {
        Self {
            internal: InternalValue::String(s),
        }
    }

    pub fn array(v: Vec<Value>) -> Self {
        Self {
            internal: InternalValue::Array(Rc::new(v)),
        }
    }

    pub fn object(o: Object) -> Self {
        Self {
            internal: InternalValue::Object(Rc::new(o)),
        }
    }

    pub fn reference(reference: Reference) -> Self {
        Self {
            internal: InternalValue::Reference(reference),
        }
    }

    pub fn function<F>(f: F) -> Self
    where
        F: 'static + Function,
    {
        Self {
            internal: InternalValue::Function(FunctionValue(Rc::new(f))),
        }
    }

    pub fn function_from_fn<F>(f: F) -> Self
    where
        F: 'static + Fn(Location, &dyn Context, &[Value]) -> Result<Value, RuntimeError>,
    {
        Self::function(DefaultFunction(f))
    }

    pub fn is_null(&self) -> bool {
        matches!(self.internal, InternalValue::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match &self.internal {
            InternalValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match &self.internal {
            InternalValue::Number(n) => Some(n.value),
            _ => None,
        }
    }

    pub(super) fn as_number(&self) -> Option<&RuntimeNumber> {
        match &self.internal {
            InternalValue::Number(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match &self.internal {
            InternalValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_slice(&self) -> Option<&[Value]> {
        match &self.internal {
            InternalValue::Array(array) => Some(array),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Object> {
        match &self.internal {
            InternalValue::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn as_function(&self) -> Option<&dyn Function> {
        match &self.internal {
            InternalValue::Function(FunctionValue(function)) => Some(function.as_ref()),
            _ => None,
        }
    }

    pub fn as_reference(&self) -> Option<Reference> {
        match self.internal {
            InternalValue::Reference(reference) => Some(reference),
            _ => None,
        }
    }
}
