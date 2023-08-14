// Copyright 2023 Salesforce, Inc. All rights reserved.
use crate::Location;

use crate::runtime::{value::Value, RuntimeError, RuntimeErrorKind};

pub trait Coerce<T> {
    fn coerce(&self, location: Location) -> Result<T, RuntimeError> {
        self.cast().ok_or(RuntimeError {
            location,
            kind: RuntimeErrorKind::TypeMismatch,
        })
    }

    fn cast(&self) -> Option<T>;
}

impl Coerce<f64> for Value {
    fn cast(&self) -> Option<f64> {
        if let Some(n) = self.as_f64() {
            Some(n)
        } else if let Some(s) = self.as_str() {
            s.parse().ok()
        } else {
            None
        }
    }
}

impl Coerce<bool> for Value {
    fn cast(&self) -> Option<bool> {
        if let Some(b) = self.as_bool() {
            Some(b)
        } else if let Some(s) = self.as_str() {
            s.parse().ok()
        } else {
            None
        }
    }
}

impl Coerce<String> for Value {
    // In order to improve readbility, we skip the manual map check, to avoid complex abstractions.
    #[allow(clippy::manual_map)]
    fn cast(&self) -> Option<String> {
        if let Some(s) = self.as_str() {
            Some(s.to_owned())
        } else if let Some(b) = self.as_bool() {
            Some(b.to_string())
        } else if let Some(n) = self.as_number() {
            Some(n.representation().to_string())
        } else {
            None
        }
    }
}

pub trait CoerceArguments<T> {
    fn coerce_arguments(&self, location: Location) -> Result<T, RuntimeError>;
}

impl<T> CoerceArguments<(T,)> for &[Value]
where
    Value: Coerce<T>,
{
    fn coerce_arguments(&self, location: Location) -> Result<(T,), RuntimeError> {
        match self.len() {
            0 => Err(RuntimeError {
                location,
                kind: RuntimeErrorKind::NotEnoughArguments,
            }),
            1 => Ok((self[0].coerce(location)?,)),
            _ => Err(RuntimeError {
                location,
                kind: RuntimeErrorKind::TooManyArguments,
            }),
        }
    }
}

impl<A, B> CoerceArguments<(A, B)> for &[Value]
where
    Value: Coerce<A> + Coerce<B>,
{
    fn coerce_arguments(&self, location: Location) -> Result<(A, B), RuntimeError> {
        match self.len() {
            0 => Err(RuntimeError {
                location,
                kind: RuntimeErrorKind::NotEnoughArguments,
            }),
            2 => Ok((self[0].coerce(location)?, self[1].coerce(location)?)),
            _ => Err(RuntimeError {
                location,
                kind: RuntimeErrorKind::TooManyArguments,
            }),
        }
    }
}

impl<A, B, C> CoerceArguments<(A, B, C)> for &[Value]
where
    Value: Coerce<A> + Coerce<B> + Coerce<C>,
{
    fn coerce_arguments(&self, location: Location) -> Result<(A, B, C), RuntimeError> {
        match self.len() {
            0 => Err(RuntimeError {
                location,
                kind: RuntimeErrorKind::NotEnoughArguments,
            }),
            3 => Ok((
                self[0].coerce(location)?,
                self[1].coerce(location)?,
                self[2].coerce(location)?,
            )),
            _ => Err(RuntimeError {
                location,
                kind: RuntimeErrorKind::TooManyArguments,
            }),
        }
    }
}
