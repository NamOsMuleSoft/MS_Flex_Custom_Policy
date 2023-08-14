// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::rc::Rc;

use crate::{runtime::value::Value, Location};

macro_rules! impl_from {
    ($into:ty => $($from:ty as $variant:ident),*) => {
        $(impl From<$from> for $into {
            fn from(value: $from) -> Self {
                Self::$variant(value)
            }
        })*
    };

    ($into:ty => $($from:ident),*) => {
        impl_from! {$into => $($from as $from),*}
    };
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expression {
    pub location: Location,
    pub body: Body,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Ref(Ref),
    Apply(Apply),
    Array(Vec<Expression>),
    DefaultOperator(DefaultOperator),
    Selection(Selection),
    IfElse(IfElse),
    UnaryOperation(UnaryOperation),
    Operation(Operation),
    Value(Value),
}

impl_from! {
    Body =>
        Ref,
        Apply,
        Selection,
        IfElse,
        DefaultOperator,
        UnaryOperation,
        Operation,
        Value
}

impl_from! {
    Body => Vec<Expression> as Array
}

impl Expression {
    pub fn new(location: Location, body: impl Into<Body>) -> Self {
        Self {
            location,
            body: body.into(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Symbol(Rc<str>);

impl Symbol {
    pub fn new(value: impl Into<Rc<str>>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ref(pub Symbol);

#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    pub target: Box<Expression>,
    pub selector: Box<Expression>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Apply {
    pub function: Box<Expression>,
    pub arguments: Vec<Expression>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefaultOperator {
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfElse {
    pub condition: Box<Expression>,
    pub true_branch: Box<Expression>,
    pub false_branch: Box<Expression>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    pub operator: Operator,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Operator {
    Eq,
    Neq,
    Lt,
    Gt,
    Let,
    Get,
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnaryOperation {
    pub operator: UnaryOperator,
    pub operand: Box<Expression>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnaryOperator {
    Not,
}
