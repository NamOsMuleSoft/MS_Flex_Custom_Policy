// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::{collections::HashMap, str::Split, vec::IntoIter};

use crate::{
    expression::{
        Apply, Body, DefaultOperator, Expression, IfElse, Operation, Operator, Ref, Selection,
        Symbol, UnaryOperation, UnaryOperator,
    },
    runtime::value::Value,
    Location,
};

use thiserror::Error;

#[derive(Error, Debug, Clone, Eq, PartialEq)]
pub enum ParsingUnitError {
    #[error("Empty content")]
    EmptyContent,

    #[error("Invalid container")]
    InvalidContainer,

    #[error("Invalid structure")]
    InvalidStructure,

    #[error("Invalid source format")]
    InvalidSourceFormat,

    #[error("Invalid source structure")]
    InvalidSourceStructure,

    #[error("Parsing error: {0}")]
    ParsingError(ParsingError),
}

#[derive(Error, Debug, Clone, Eq, PartialEq)]
#[error("{kind}")]
pub struct ParsingError {
    kind: ParsingErrorKind,
}

impl From<ParsingError> for ParsingUnitError {
    fn from(e: ParsingError) -> Self {
        Self::ParsingError(e)
    }
}

#[derive(Error, Debug, Clone, Eq, PartialEq)]
pub enum ParsingErrorKind {
    #[error("Bad formed expression")]
    BadFormedPelExpression,

    #[error("Unexpected structure")]
    UnexpectedPelStructure,

    #[error("Missing constructor")]
    MissingConstructor,

    #[error("Constructor type mismatch")]
    ConstructorTypeMismatch,

    #[error("Empty constructor")]
    EmptyConstructor,

    #[error("Unknown constructor")]
    UnknownConstructor,

    #[error("Missing location")]
    MissingLocation,

    #[error("Location type mismatch")]
    LocationTypeMismatch,

    #[error("Bad formed location")]
    BadFormedLocation,

    #[error("Missing function")]
    MissingFunction,

    #[error("Missing symbol")]
    MissingSymbol,

    #[error("Missing constructor argument")]
    MissingConstructorArgument,

    #[error("Missing selection target")]
    MissingSelectionTarget,

    #[error("Missing selector")]
    MissingSelector,

    #[error("Bad number format")]
    BadNumberFormat,

    #[error("Bad bool format")]
    BadBoolFormat,

    #[error("Missing condition")]
    MissingCondition,

    #[error("Missing true branch")]
    MissingTrueBranch,

    #[error("Missing false branch")]
    MissingFalseBranch,

    #[error("Missing operand")]
    MissingOperand,

    #[error("Missing left operand")]
    MissingLeftOperand,

    #[error("Missing right operand")]
    MissingRightOperand,
}

fn position(s: &mut Split<char>) -> Result<usize, ParsingError> {
    s.next()
        .ok_or(ParsingError {
            kind: ParsingErrorKind::MissingLocation,
        })
        .and_then(|l| {
            l.parse().map_err(|_| ParsingError {
                kind: ParsingErrorKind::BadFormedLocation,
            })
        })
}

fn location(json_value: serde_json::Value) -> Result<Location, ParsingError> {
    if let serde_json::Value::String(location) = json_value {
        if location.is_empty() {
            return Err(ParsingError {
                kind: ParsingErrorKind::MissingLocation,
            });
        }
        let positions = &mut location.split('-');
        let start = position(positions)?;
        let end = position(positions)?;
        Ok(Location { start, end })
    } else {
        Err(ParsingError {
            kind: ParsingErrorKind::LocationTypeMismatch,
        })
    }
}

fn constructor_id(json_value: serde_json::Value) -> Result<String, ParsingError> {
    if let serde_json::Value::String(constructor) = json_value {
        if constructor.is_empty() {
            return Err(ParsingError {
                kind: ParsingErrorKind::EmptyConstructor,
            });
        }
        Ok(constructor)
    } else {
        Err(ParsingError {
            kind: ParsingErrorKind::ConstructorTypeMismatch,
        })
    }
}

fn apply(
    parser: &Parser,
    location: Location,
    mut arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    let function = arguments
        .next()
        .ok_or(ParsingError {
            kind: ParsingErrorKind::MissingFunction,
        })
        .and_then(|value| parser.expression(value))?;
    let arguments = arguments
        .map(|value| parser.expression(value))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expression {
        location,
        body: Body::Apply(Apply {
            function: Box::new(function),
            arguments,
        }),
    })
}

fn default(
    parser: &Parser,
    location: Location,
    mut arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    let left = arguments
        .next()
        .ok_or(ParsingError {
            kind: ParsingErrorKind::MissingLeftOperand,
        })
        .and_then(|value| parser.expression(value))?;

    let right = arguments
        .next()
        .ok_or(ParsingError {
            kind: ParsingErrorKind::MissingRightOperand,
        })
        .and_then(|value| parser.expression(value))?;

    Ok(Expression {
        location,
        body: Body::DefaultOperator(DefaultOperator {
            left: Box::new(left),
            right: Box::new(right),
        }),
    })
}

fn reference(
    _: &Parser,
    location: Location,
    mut arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    let symbol = arguments.next().ok_or(ParsingError {
        kind: ParsingErrorKind::MissingSymbol,
    })?;
    if let serde_json::Value::String(symbol) = symbol {
        Ok(Expression {
            location,
            body: Body::Ref(Ref(Symbol::new(symbol))),
        })
    } else {
        Err(ParsingError {
            kind: ParsingErrorKind::UnexpectedPelStructure,
        })
    }
}

fn null(
    _: &Parser,
    location: Location,
    _arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    Ok(Expression {
        location,
        body: Body::Value(Value::null()),
    })
}

fn string(
    _: &Parser,
    location: Location,
    mut arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    let s = arguments.next().ok_or(ParsingError {
        kind: ParsingErrorKind::MissingConstructorArgument,
    })?;
    if let serde_json::Value::String(s) = s {
        Ok(Expression {
            location,
            body: Body::Value(Value::string(s)),
        })
    } else {
        Err(ParsingError {
            kind: ParsingErrorKind::UnexpectedPelStructure,
        })
    }
}

fn bool(
    _: &Parser,
    location: Location,
    mut arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    let b = arguments.next().ok_or(ParsingError {
        kind: ParsingErrorKind::MissingConstructorArgument,
    })?;
    if let serde_json::Value::String(b) = b {
        let b: bool = b.parse().map_err(|_| ParsingError {
            kind: ParsingErrorKind::BadBoolFormat,
        })?;
        Ok(Expression {
            location,
            body: Body::Value(Value::bool(b)),
        })
    } else {
        Err(ParsingError {
            kind: ParsingErrorKind::UnexpectedPelStructure,
        })
    }
}

fn number(
    _: &Parser,
    location: Location,
    mut arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    let nbr = arguments.next().ok_or(ParsingError {
        kind: ParsingErrorKind::MissingConstructorArgument,
    })?;
    if let serde_json::Value::String(nbr) = nbr {
        let value: f64 = nbr.parse().map_err(|_| ParsingError {
            kind: ParsingErrorKind::BadNumberFormat,
        })?;
        Ok(Expression {
            location,
            // TODO: AGW-5356 - Improve number coercion
            body: Body::Value(Value::number_with_representation(value, nbr)),
        })
    } else {
        Err(ParsingError {
            kind: ParsingErrorKind::UnexpectedPelStructure,
        })
    }
}

fn array(
    parser: &Parser,
    location: Location,
    arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    let array = arguments
        .map(|value| parser.expression(value))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expression {
        location,
        body: Body::Array(array),
    })
}

fn selection(
    parser: &Parser,
    location: Location,
    mut arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    let target = arguments
        .next()
        .ok_or(ParsingError {
            kind: ParsingErrorKind::MissingSelectionTarget,
        })
        .and_then(|value| parser.expression(value))?;
    let selector = arguments
        .next()
        .ok_or(ParsingError {
            kind: ParsingErrorKind::MissingSelector,
        })
        .and_then(|value| parser.expression(value))?;
    Ok(Expression {
        location,
        body: Body::Selection(Selection {
            target: Box::new(target),
            selector: Box::new(selector),
        }),
    })
}

fn if_else(
    parser: &Parser,
    location: Location,
    mut arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    let condition = arguments
        .next()
        .ok_or(ParsingError {
            kind: ParsingErrorKind::MissingCondition,
        })
        .and_then(|value| parser.expression(value))?;
    let true_branch = arguments
        .next()
        .ok_or(ParsingError {
            kind: ParsingErrorKind::MissingTrueBranch,
        })
        .and_then(|value| parser.expression(value))?;
    let false_branch = arguments
        .next()
        .ok_or(ParsingError {
            kind: ParsingErrorKind::MissingFalseBranch,
        })
        .and_then(|value| parser.expression(value))?;
    Ok(Expression {
        location,
        body: Body::IfElse(IfElse {
            condition: Box::new(condition),
            true_branch: Box::new(true_branch),
            false_branch: Box::new(false_branch),
        }),
    })
}

fn unary_operation(
    operator: UnaryOperator,
    parser: &Parser,
    location: Location,
    mut arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    Ok(Expression {
        location,
        body: Body::UnaryOperation(UnaryOperation {
            operator,
            operand: arguments
                .next()
                .ok_or(ParsingError {
                    kind: ParsingErrorKind::MissingOperand,
                })
                .and_then(|value| parser.expression(value))?
                .into(),
        }),
    })
}

macro_rules! unary_operation {
    ($operator:expr) => {{
        fn applied_operation(
            parser: &Parser,
            location: Location,
            arguments: IntoIter<serde_json::Value>,
        ) -> Result<Expression, ParsingError> {
            unary_operation($operator, parser, location, arguments)
        }
        applied_operation
    }};
}

fn operation(
    operator: Operator,
    parser: &Parser,
    location: Location,
    mut arguments: IntoIter<serde_json::Value>,
) -> Result<Expression, ParsingError> {
    Ok(Expression {
        location,
        body: Body::Operation(Operation {
            operator,
            left: arguments
                .next()
                .ok_or(ParsingError {
                    kind: ParsingErrorKind::MissingLeftOperand,
                })
                .and_then(|value| parser.expression(value))?
                .into(),
            right: arguments
                .next()
                .ok_or(ParsingError {
                    kind: ParsingErrorKind::MissingRightOperand,
                })
                .and_then(|value| parser.expression(value))?
                .into(),
        }),
    })
}

macro_rules! operation {
    ($operator:expr) => {{
        fn applied_operation(
            parser: &Parser,
            location: Location,
            arguments: IntoIter<serde_json::Value>,
        ) -> Result<Expression, ParsingError> {
            operation($operator, parser, location, arguments)
        }
        applied_operation
    }};
}

type Constructor =
    fn(&Parser, Location, IntoIter<serde_json::Value>) -> Result<Expression, ParsingError>;

static CONSTRUCTORS: &[(&str, Constructor)] = &[
    (".", selection),
    (":apply", apply),
    (":null", null),
    (":str", string),
    (":nbr", number),
    (":bool", bool),
    (":array", array),
    (":ref", reference),
    (":if", if_else),
    (":default", default),
    ("!", unary_operation!(UnaryOperator::Not)),
    ("==", operation!(Operator::Eq)),
    ("!=", operation!(Operator::Neq)),
    ("<", operation!(Operator::Lt)),
    (">", operation!(Operator::Gt)),
    ("<=", operation!(Operator::Let)),
    (">=", operation!(Operator::Get)),
    ("&&", operation!(Operator::And)),
    ("||", operation!(Operator::Or)),
];

pub struct Parser {
    constructors: HashMap<&'static str, Constructor>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            constructors: CONSTRUCTORS.iter().cloned().collect(),
        }
    }

    pub fn parse_slice(&self, source: &[u8]) -> Result<Expression, ParsingError> {
        let json_value: serde_json::Value =
            serde_json::from_slice(source).map_err(|_| ParsingError {
                kind: ParsingErrorKind::BadFormedPelExpression,
            })?;
        self.expression(json_value)
    }

    pub fn parse_str(&self, source: &str) -> Result<Expression, ParsingError> {
        let json_value: serde_json::Value =
            serde_json::from_str(source).map_err(|_| ParsingError {
                kind: ParsingErrorKind::BadFormedPelExpression,
            })?;
        self.expression(json_value)
    }

    pub fn parse_unit(&self, unit: &str) -> Result<(Expression, Option<String>), ParsingUnitError> {
        if !(unit.starts_with("P[") && unit.ends_with(']')) {
            return Err(ParsingUnitError::InvalidContainer);
        }

        let unit = &unit[1..];
        let unit: serde_json::Value =
            serde_json::from_str(unit).map_err(|_| ParsingUnitError::InvalidStructure)?;
        let unit = if let serde_json::Value::Array(array) = unit {
            array
        } else {
            return Err(ParsingUnitError::InvalidStructure);
        };

        let mut unit = unit.into_iter();

        let expression = unit.next().ok_or(ParsingUnitError::EmptyContent)?;
        let expression = self.expression(expression)?;

        let source = if let Some(source) = unit.next() {
            let source = if let serde_json::Value::String(source) = source {
                source
            } else {
                return Err(ParsingUnitError::InvalidSourceStructure);
            };

            if !(source.starts_with("#[") && source.ends_with(']')) {
                return Err(ParsingUnitError::InvalidSourceFormat);
            }

            Some((source[2..source.len() - 1]).to_string())
        } else {
            None
        };

        Ok((expression, source))
    }

    fn expression(&self, json_value: serde_json::Value) -> Result<Expression, ParsingError> {
        match json_value {
            serde_json::Value::Array(array) => {
                let mut iter = array.into_iter();
                let constructor_id = iter
                    .next()
                    .ok_or(ParsingError {
                        kind: ParsingErrorKind::MissingConstructor,
                    })
                    .and_then(constructor_id)?;

                let constructor =
                    self.constructors
                        .get(constructor_id.as_str())
                        .ok_or(ParsingError {
                            kind: ParsingErrorKind::UnknownConstructor,
                        })?;

                let location = iter
                    .next()
                    .ok_or(ParsingError {
                        kind: ParsingErrorKind::MissingLocation,
                    })
                    .and_then(location)?;

                constructor(self, location, iter)
            }
            _ => Err(ParsingError {
                kind: ParsingErrorKind::UnexpectedPelStructure,
            }),
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const LOCATION: Location = Location {
        start: 100,
        end: 200,
    };

    #[test]
    fn parse_location() {
        let input = serde_json::Value::String("100-200".to_string());
        let result = location(input).unwrap();
        assert_eq!(result, LOCATION);
    }

    #[test]
    fn parse_location_fail() {
        let input = serde_json::Value::String("100,200".to_string());
        let result = location(input);
        assert!(result.is_err());
    }

    #[test]
    fn parse_unit() {
        let unit = r##"P[
            [":null", "0-4"], 
            "#[null]"
        ]"##;
        let (expression, source) = Parser::new().parse_unit(unit).unwrap();

        assert_eq!(source, Some("null".to_string()));
        assert_eq!(expression.body, Body::Value(Value::null()));
    }

    #[test]
    fn parse_unit_without_source() {
        let unit = r##"P[
            [":null", "0-4"]
        ]"##;
        let (expression, source) = Parser::new().parse_unit(unit).unwrap();

        assert_eq!(source, None);
        assert_eq!(expression.body, Body::Value(Value::null()));
    }

    #[test]
    fn parse_unit_with_invalid_container() {
        let unit = r##"X[
            [":null", "0-4"],
            "#[null]"
        ]"##;
        let error = Parser::new().parse_unit(unit).unwrap_err();

        assert_eq!(error, ParsingUnitError::InvalidContainer);
    }

    #[test]
    fn parse_unit_with_invalid_structure() {
        let unit = r##"P["expression": "fail"]"##;
        let error = Parser::new().parse_unit(unit).unwrap_err();

        assert_eq!(error, ParsingUnitError::InvalidStructure);
    }

    #[test]
    fn parse_unit_with_parsing_error() {
        let unit = r##"P[
            ":null",
            "#[null]"
        ]"##;
        let error = Parser::new().parse_unit(unit).unwrap_err();

        assert!(matches!(error, ParsingUnitError::ParsingError(_)));
    }

    #[test]
    fn parse_unit_with_invalid_source_structure() {
        let unit = r##"P[
            [":null", "0-4"],
            [null]
        ]"##;
        let error = Parser::new().parse_unit(unit).unwrap_err();

        assert_eq!(error, ParsingUnitError::InvalidSourceStructure);
    }

    #[test]
    fn parse_unit_with_invalid_source_format() {
        let unit = r##"P[
            [":null", "0-4"],
            "null"
        ]"##;
        let error = Parser::new().parse_unit(unit).unwrap_err();

        assert_eq!(error, ParsingUnitError::InvalidSourceFormat);
    }

    #[test]
    fn parse_unit_with_empty_content() {
        let unit = r#"P[]"#;
        let error = Parser::new().parse_unit(unit).unwrap_err();

        assert_eq!(error, ParsingUnitError::EmptyContent);
    }
}
