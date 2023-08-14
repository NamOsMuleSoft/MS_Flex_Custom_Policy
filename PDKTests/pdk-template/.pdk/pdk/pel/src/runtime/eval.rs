// Copyright 2023 Salesforce, Inc. All rights reserved.
use crate::{
    expression::{
        Apply, Body, DefaultOperator, Expression, IfElse, Operation, Operator, Ref, Selection,
        UnaryOperation, UnaryOperator,
    },
    runtime::{
        coercion::Coerce, Binding, Context, Eval, Evaluation, RuntimeError, RuntimeErrorKind,
        Value, ValueHandler,
    },
    Location,
};

trait BodyEval {
    fn body_eval(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Evaluation, RuntimeError>;

    fn body_bind(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Expression, RuntimeError>;
}

impl BodyEval for Ref {
    fn body_eval(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Evaluation, RuntimeError> {
        match context.resolve(&self.0) {
            Binding::Available(v) => Ok(Evaluation::Complete(location, v)),
            Binding::Pending => Ok(Evaluation::Partial(Expression::new(
                location,
                (*self).clone(),
            ))),
            Binding::Unknown => Err(RuntimeError {
                location,
                kind: RuntimeErrorKind::UnknownSymbol(self.0.as_str().to_string()),
            }),
        }
    }

    fn body_bind(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Expression, RuntimeError> {
        Ok(self.body_eval(location, context)?.into_expression())
    }
}

impl BodyEval for Apply {
    fn body_eval(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Evaluation, RuntimeError> {
        let function = self.function.eval(context)?;
        let arguments = self
            .arguments
            .iter()
            .map(|exp| exp.eval(context))
            .collect::<Result<Vec<Evaluation>, _>>()?;

        let result = match function {
            Evaluation::Complete(_, function) if arguments.iter().all(Evaluation::is_complete) => {
                let function = function.as_function().ok_or(RuntimeError {
                    location,
                    kind: RuntimeErrorKind::TypeMismatch,
                })?;

                let arguments = arguments
                    .into_iter()
                    .map(|e| e.complete().unwrap())
                    .collect::<Vec<_>>();

                let result = function.apply(location, context, &arguments)?;

                Evaluation::Complete(location, result)
            }

            function => Evaluation::Partial(Expression::new(
                location,
                Apply {
                    function: Box::new(function.into_expression()),
                    arguments: arguments.into_iter().map(|e| e.into_expression()).collect(),
                },
            )),
        };
        Ok(result)
    }

    fn body_bind(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Expression, RuntimeError> {
        Ok(Expression::new(
            location,
            Self {
                function: Box::new(self.function.bind(context)?),
                arguments: self
                    .arguments
                    .iter()
                    .map(|e| e.bind(context))
                    .collect::<Result<_, _>>()?,
            },
        ))
    }
}

impl BodyEval for DefaultOperator {
    fn body_eval(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Evaluation, RuntimeError> {
        let result = match self.left.eval(context)? {
            Evaluation::Complete(left_location, left) => {
                if !left.is_null() {
                    Evaluation::Complete(left_location, left)
                } else {
                    self.right.eval(context)?
                }
            }
            Evaluation::Partial(left) => Evaluation::Partial(Expression::new(
                location,
                Self {
                    left: Box::new(left),
                    right: Box::new(self.right.bind(context)?),
                },
            )),
        };
        Ok(result)
    }

    fn body_bind(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Expression, RuntimeError> {
        Ok(Expression::new(
            location,
            Self {
                left: Box::new(self.left.bind(context)?),
                right: Box::new(self.right.bind(context)?),
            },
        ))
    }
}

impl BodyEval for Vec<Expression> {
    fn body_eval(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Evaluation, RuntimeError> {
        let evaluations = self
            .iter()
            .map(|e| e.eval(context))
            .collect::<Result<Vec<_>, _>>()?;

        let result = if evaluations.iter().all(Evaluation::is_complete) {
            Evaluation::Complete(
                location,
                Value::array(
                    evaluations
                        .into_iter()
                        .map(|e| e.complete().unwrap())
                        .collect(),
                ),
            )
        } else {
            Evaluation::Partial(Expression::new(
                location,
                evaluations
                    .into_iter()
                    .map(|e| e.into_expression())
                    .collect::<Vec<_>>(),
            ))
        };
        Ok(result)
    }

    fn body_bind(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Expression, RuntimeError> {
        Ok(Expression::new(
            location,
            self.iter()
                .map(|e| e.bind(context))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl BodyEval for IfElse {
    fn body_eval(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Evaluation, RuntimeError> {
        let result = match self.condition.eval(context)? {
            Evaluation::Complete(condition_location, condition) => {
                let evaluable_branch = if condition.coerce(condition_location)? {
                    &self.true_branch
                } else {
                    &self.false_branch
                };
                evaluable_branch.eval(context)?
            }
            Evaluation::Partial(condition) => Evaluation::Partial(Expression::new(
                location,
                Self {
                    condition: Box::new(condition),
                    true_branch: Box::new(self.true_branch.bind(context)?),
                    false_branch: Box::new(self.false_branch.bind(context)?),
                },
            )),
        };
        Ok(result)
    }

    fn body_bind(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Expression, RuntimeError> {
        Ok(Expression::new(
            location,
            Self {
                condition: Box::new(self.condition.bind(context)?),
                true_branch: Box::new(self.true_branch.bind(context)?),
                false_branch: Box::new(self.false_branch.bind(context)?),
            },
        ))
    }
}

impl BodyEval for Selection {
    fn body_eval(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Evaluation, RuntimeError> {
        let target = self.target.eval(context)?;
        let selector = self.selector.eval(context)?;

        match (target, selector) {
            (
                Evaluation::Complete(target_location, target),
                Evaluation::Complete(selector_location, selector),
            ) => {
                let target = target.to_value_handler(context).ok_or(RuntimeError {
                    location,
                    kind: RuntimeErrorKind::UnknownReference,
                })?;

                let value = if let Some(key) = selector.as_str() {
                    target.select_by_key(key).unwrap_or_else(Value::null)
                } else if let Some(index) = selector.as_f64() {
                    let index = index.trunc() as i32;
                    let size = target.size().ok_or(RuntimeError {
                        location: target_location,
                        kind: RuntimeErrorKind::UnsupportedSelection,
                    })?;
                    let index = if index < 0 {
                        size - -index as usize
                    } else {
                        index as usize
                    };
                    target.select_by_index(index).ok_or(RuntimeError {
                        location,
                        kind: RuntimeErrorKind::UnsupportedSelection,
                    })?
                } else {
                    return Err(RuntimeError {
                        location: selector_location,
                        kind: RuntimeErrorKind::UnsupportedSelection,
                    });
                };

                Ok(Evaluation::Complete(location, value))
            }
            (target, selector) => Ok(Evaluation::Partial(Expression::new(
                location,
                Self {
                    target: Box::new(target.into_expression()),
                    selector: Box::new(selector.into_expression()),
                },
            ))),
        }
    }

    fn body_bind(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Expression, RuntimeError> {
        Ok(Expression::new(
            location,
            Self {
                target: Box::new(self.target.bind(context)?),
                selector: Box::new(self.selector.bind(context)?),
            },
        ))
    }
}

impl BodyEval for UnaryOperation {
    fn body_eval(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Evaluation, RuntimeError> {
        let result = match self.operand.eval(context)? {
            Evaluation::Complete(operand_location, operand) => match self.operator {
                UnaryOperator::Not => {
                    Evaluation::Complete(location, Value::bool(!operand.coerce(operand_location)?))
                }
            },
            Evaluation::Partial(operand) => Evaluation::Partial(Expression::new(
                location,
                Self {
                    operator: self.operator,
                    operand: Box::new(operand),
                },
            )),
        };
        Ok(result)
    }

    fn body_bind(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Expression, RuntimeError> {
        Ok(Expression::new(
            location,
            Self {
                operator: self.operator,
                operand: Box::new(self.operand.bind(context)?),
            },
        ))
    }
}

impl BodyEval for Operation {
    fn body_eval(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Evaluation, RuntimeError> {
        match self.left.eval(context)? {
            Evaluation::Complete(left_location, left) => {
                let result: Evaluation = match self.operator {
                    Operator::Eq => self
                        .right
                        .eval(context)?
                        .map(|right| Value::bool(left == right)),
                    Operator::Neq => self
                        .right
                        .eval(context)?
                        .map(|right| Value::bool(left != right)),
                    Operator::And => {
                        if left.coerce(left_location)? {
                            match self.right.eval(context)? {
                                Evaluation::Complete(right_location, right) => {
                                    Evaluation::Complete(
                                        location,
                                        Value::bool(right.coerce(right_location)?),
                                    )
                                }
                                right => right,
                            }
                        } else {
                            Evaluation::Complete(location, Value::bool(false))
                        }
                    }
                    Operator::Or => {
                        if left.coerce(left_location)? {
                            Evaluation::Complete(location, Value::bool(true))
                        } else {
                            match self.right.eval(context)? {
                                Evaluation::Complete(right_location, right) => {
                                    Evaluation::Complete(
                                        location,
                                        Value::bool(right.coerce(right_location)?),
                                    )
                                }
                                right => right,
                            }
                        }
                    }
                    operator => match self.right.eval(context)? {
                        Evaluation::Complete(_, right) => Evaluation::Complete(
                            location,
                            eval_coercible_operation(location, operator, left, right)?,
                        ),
                        right => right,
                    },
                };
                Ok(result)
            }
            Evaluation::Partial(left) => Ok(Evaluation::Partial(Expression::new(
                location,
                Self {
                    operator: self.operator,
                    left: Box::new(left),
                    right: Box::new(self.right.bind(context)?),
                },
            ))),
        }
    }

    fn body_bind(
        &self,
        location: Location,
        context: &dyn Context,
    ) -> Result<Expression, RuntimeError> {
        Ok(Expression::new(
            location,
            Self {
                operator: self.operator,
                left: Box::new(self.left.bind(context)?),
                right: Box::new(self.right.bind(context)?),
            },
        ))
    }
}

impl BodyEval for Value {
    fn body_eval(
        &self,
        location: Location,
        _context: &dyn Context,
    ) -> Result<Evaluation, RuntimeError> {
        Ok(Evaluation::Complete(location, (*self).clone()))
    }

    fn body_bind(
        &self,
        location: Location,
        _context: &dyn Context,
    ) -> Result<Expression, RuntimeError> {
        Ok(Expression::new(location, (*self).clone()))
    }
}

fn eval_coercible_operation(
    location: Location,
    operator: Operator,
    left: Value,
    right: Value,
) -> Result<Value, RuntimeError> {
    let result = if let Some(right) = right.as_str() {
        let left: String = left.coerce(location)?;
        eval_coerced_operation(operator, left.as_str(), right)
    } else if let Some(right) = right.as_f64() {
        eval_coerced_operation(operator, &left.coerce(location)?, &right)
    } else if let Some(right) = right.as_bool() {
        eval_coerced_operation(operator, &left.coerce(location)?, &right)
    } else {
        return Err(RuntimeError {
            location,
            kind: RuntimeErrorKind::UncomparableTypes,
        });
    };
    Ok(Value::bool(result))
}

fn eval_coerced_operation<T>(operator: Operator, left: &T, right: &T) -> bool
where
    T: PartialOrd + ?Sized,
{
    match operator {
        Operator::Gt => left > right,
        Operator::Get => left >= right,
        Operator::Lt => left < right,
        Operator::Let => left <= right,
        _ => unreachable!(),
    }
}

impl Eval for Expression {
    fn eval(&self, context: &dyn Context) -> Result<Evaluation, RuntimeError> {
        self.body().body_eval(self.location, context)
    }
}

impl Expression {
    fn body(&self) -> &dyn BodyEval {
        match &self.body {
            Body::Ref(r) => r,
            Body::Apply(a) => a,
            Body::Array(a) => a,
            Body::IfElse(ie) => ie,
            Body::Selection(s) => s,
            Body::DefaultOperator(d) => d,
            Body::UnaryOperation(u) => u,
            Body::Operation(o) => o,
            Body::Value(v) => v,
        }
    }

    fn bind(&self, context: &dyn Context) -> Result<Self, RuntimeError> {
        self.body().body_bind(self.location, context)
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        expression::{Apply, Body, Expression, Ref, Symbol},
        runtime::Prelude,
    };

    use super::{BodyEval, Location, Value};

    const LOCATION: Location = Location {
        start: 100,
        end: 200,
    };

    #[test]
    fn eval_ref() {
        let mut context = Prelude::new();
        context.insert("a", Value::null());
        let r = Ref(Symbol::new("a"));
        let result = r.body_eval(LOCATION, &context).unwrap().complete().unwrap();
        assert_eq!(result, Value::null());
    }

    #[test]
    fn eval_ref_fail() {
        let context = Prelude::new();
        let r = Ref(Symbol::new("a"));
        let result = r.body_eval(LOCATION, &context);
        assert!(result.is_err());
    }

    #[test]
    fn eval_apply() {
        let mut context = Prelude::new();
        context.insert(
            "and",
            Value::function_from_fn(|_, _, arguments| {
                Ok(Value::bool(
                    arguments[0].as_bool().unwrap() && arguments[1].as_bool().unwrap(),
                ))
            }),
        );

        let function = Expression {
            location: LOCATION,
            body: Body::Ref(Ref(Symbol::new("and"))),
        };

        let arg1 = Expression {
            location: LOCATION,
            body: Body::Value(Value::bool(false)),
        };

        let arg2 = Expression {
            location: LOCATION,
            body: Body::Value(Value::bool(true)),
        };

        let application = Apply {
            function: Box::new(function),
            arguments: vec![arg1, arg2],
        };

        let result = application
            .body_eval(LOCATION, &context)
            .unwrap()
            .complete()
            .unwrap();
        assert_eq!(result, Value::bool(false));
    }
}
