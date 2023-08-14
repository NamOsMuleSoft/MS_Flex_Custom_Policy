// Copyright 2023 Salesforce, Inc. All rights reserved.
pub mod expression;
pub mod parser;
pub mod runtime;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Location {
    pub start: usize,
    pub end: usize,
}

impl Location {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ContextId(&'static str);

impl ContextId {
    pub const fn new(raw: &'static str) -> Self {
        Self(raw)
    }

    pub const fn first_reference(&self) -> Reference {
        Reference {
            context_id: *self,
            offset: 0,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Reference {
    context_id: ContextId,
    offset: usize,
}

impl Reference {
    pub const fn next(&self) -> Self {
        Self {
            context_id: self.context_id,
            offset: self.offset + 1,
        }
    }

    pub const fn context_id(&self) -> ContextId {
        self.context_id
    }

    pub const fn offset(&self) -> usize {
        self.offset
    }
}
