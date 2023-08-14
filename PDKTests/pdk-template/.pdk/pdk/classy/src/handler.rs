// Copyright 2023 Salesforce, Inc. All rights reserved.
use crate::{all_the_tuples, BoxError};
use std::future::Future;

pub type HandlerResult = Result<(), BoxError>;

/// Result for generic handlers.
///
/// This Result type enables several return types for filters and configurations.
pub trait IntoHandlerResult {
    fn into_handler_result(self) -> HandlerResult;
}

impl IntoHandlerResult for () {
    fn into_handler_result(self) -> HandlerResult {
        Ok(())
    }
}

impl<E> IntoHandlerResult for Result<(), E>
where
    E: Into<BoxError>,
{
    fn into_handler_result(self) -> HandlerResult {
        match self {
            Ok(()) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

pub trait Handler<S, T> {
    type Result: IntoHandlerResult;
    type Future: Future<Output = Self::Result>;

    fn call(&self, selector: S, args: T) -> Self::Future;
}

impl<F, Fut, Res, S> Handler<S, ()> for F
where
    F: Fn(S) -> Fut,
    Fut: Future<Output = Res>,
    Res: IntoHandlerResult,
{
    type Result = Res;

    type Future = Fut;

    fn call(&self, selector: S, _: ()) -> Self::Future {
        self(selector)
    }
}

macro_rules! impl_handler {
    (
        $($ty:ident),*
    ) => {
        #[allow(non_snake_case)]
        impl<F, Fut, Res, S, $($ty,)* > Handler<S, ($( $ty, )*)> for F
        where
            F: Fn(S, $( $ty, )* )-> Fut,
            Fut: Future<Output = Res>,
            Res: IntoHandlerResult,
        {
            type Result = Res;

            type Future = Fut;

            fn call(&self, selector: S, ($( $ty, )*) : ($( $ty, )*) ) -> Self::Future {
                self(selector, $( $ty, )* )
            }
        }
    }
}

all_the_tuples!(impl_handler);
