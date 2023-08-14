// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::{cell::RefCell, collections::BTreeMap, task::Waker};

use crate::{event::EventKind, types::HttpCid};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExchangePhase {
    Request,
    Response,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WakerId(usize);

impl PartialOrd for WakerId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for WakerId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

struct WakerIdGenerator {
    last_id: WakerId,
}

impl WakerIdGenerator {
    fn new() -> Self {
        Self {
            last_id: WakerId(0),
        }
    }

    fn generate(&mut self) -> WakerId {
        let last_id = self.last_id;
        self.last_id = WakerId(last_id.0 + 1);
        last_id
    }
}

struct RawHttpReactor {
    id_generator: WakerIdGenerator,
    cancelled_request: bool,
    paused_request: bool,
    paused_response: bool,
    current_event: EventKind,
    wakers: BTreeMap<(EventKind, WakerId), Waker>,
}

impl RawHttpReactor {
    pub fn notify(&mut self, event: EventKind) {
        self.current_event = event;
        self.wakers
            .iter()
            .filter(|((e, _), _)| e == &event)
            .for_each(|((_, _id), w)| {
                w.wake_by_ref();
            });
    }

    pub fn insert_waker(&mut self, event: EventKind, waker: Waker) -> WakerId {
        let id = self.id_generator.generate();
        self.wakers.insert((event, id), waker);
        id
    }

    pub fn remove_waker(&mut self, event: EventKind, id: WakerId) {
        self.wakers.remove(&(event, id));
    }
}

pub struct HttpReactor {
    context_id: HttpCid,
    raw: RefCell<RawHttpReactor>,
}

impl HttpReactor {
    pub fn new(context_id: HttpCid) -> Self {
        HttpReactor {
            context_id,
            raw: RefCell::new(RawHttpReactor {
                id_generator: WakerIdGenerator::new(),
                cancelled_request: false,
                paused_request: false,
                paused_response: false,
                current_event: EventKind::Start,
                wakers: BTreeMap::new(),
            }),
        }
    }

    pub fn context_id(&self) -> HttpCid {
        self.context_id
    }

    pub fn notify(&self, event: EventKind) {
        self.raw.borrow_mut().notify(event);
    }

    pub fn cancel_request(&self) {
        self.raw.borrow_mut().cancelled_request = true;
    }

    pub fn cancelled_request(&self) -> bool {
        self.raw.borrow().cancelled_request
    }

    pub fn paused(&self) -> bool {
        match self.phase() {
            ExchangePhase::Request => self.raw.borrow().paused_request,
            ExchangePhase::Response => self.raw.borrow().paused_response,
        }
    }

    pub fn set_paused(&self, paused: bool) {
        match self.phase() {
            ExchangePhase::Request => self.raw.borrow_mut().paused_request = paused,
            ExchangePhase::Response => self.raw.borrow_mut().paused_response = paused,
        }
    }

    pub fn current_event(&self) -> EventKind {
        self.raw.borrow().current_event
    }

    pub fn insert_waker(&self, event: EventKind, waker: Waker) -> WakerId {
        self.raw.borrow_mut().insert_waker(event, waker)
    }

    pub fn remove_waker(&self, event: EventKind, id: WakerId) {
        self.raw.borrow_mut().remove_waker(event, id)
    }

    pub fn phase(&self) -> ExchangePhase {
        match self.current_event() {
            EventKind::Start
            | EventKind::RequestHeaders
            | EventKind::RequestBody
            | EventKind::RequestTrailers => ExchangePhase::Request,
            EventKind::ResponseHeaders
            | EventKind::ResponseBody
            | EventKind::ResponseTrailers
            | EventKind::ExchangeComplete => ExchangePhase::Response,
        }
    }
}
