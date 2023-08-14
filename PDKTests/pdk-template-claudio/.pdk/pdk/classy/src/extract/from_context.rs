// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::convert::Infallible;

use crate::{all_the_tuples, BoxError};

pub trait FromContext<C>: Sized {
    type Error: Into<BoxError>;

    fn from_context(context: &C) -> Result<Self, Self::Error>;
}

pub trait Extract<T> {
    type Error;

    fn extract(&self) -> Result<T, Self::Error>;
}

impl<C, T> Extract<T> for C
where
    T: FromContext<C>,
{
    type Error = T::Error;

    fn extract(&self) -> Result<T, Self::Error> {
        T::from_context(self)
    }
}

impl<T, C> FromContext<C> for Option<T>
where
    T: FromContext<C>,
{
    type Error = Infallible;

    fn from_context(context: &C) -> Result<Self, Self::Error> {
        Ok(T::from_context(context).ok())
    }
}

impl<T, C> FromContext<C> for Result<T, T::Error>
where
    T: FromContext<C>,
{
    type Error = Infallible;

    fn from_context(context: &C) -> Result<Self, Self::Error> {
        Ok(T::from_context(context))
    }
}

impl<C> FromContext<C> for () {
    type Error = Infallible;

    fn from_context(_: &C) -> Result<Self, Self::Error> {
        Ok(())
    }
}

macro_rules! impl_from_context {
    (
        $($ty:ident),*
    ) => {
        #[allow(non_snake_case)]
        impl<C, $($ty, )* > FromContext<C> for ($($ty,)*)
        where
            $( $ty: FromContext<C>, )*
        {
            type Error = BoxError;

            fn from_context(context: &C) -> Result<Self, Self::Error> {
                $(
                    let $ty = $ty::from_context(context).map_err(|err| err.into())?;
                )*

                Ok(($($ty,)*))
            }
        }
    }
}

all_the_tuples!(impl_from_context);
