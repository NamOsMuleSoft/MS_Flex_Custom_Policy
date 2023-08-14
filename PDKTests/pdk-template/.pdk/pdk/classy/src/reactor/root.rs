// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::{any::Any, cell::RefCell, collections::BTreeMap, rc::Rc, task::Waker};

use crate::{
    client::HttpCallResponse,
    event::EventKind,
    types::{Cid, HttpCid, RequestId, RootCid},
};

use super::http::HttpReactor;

pub type ResponseContent = Box<dyn Any>;
pub type BoxedExtractor = Box<dyn FnOnce(&HttpCallResponse) -> Box<dyn Any>>;

struct RawRootReactor {
    context_id: RootCid,
    active_cid: Cid,
    context_create_waker: Option<Waker>,
    new_http_reactor: Option<Rc<HttpReactor>>,
    http_reactors: BTreeMap<HttpCid, Rc<HttpReactor>>,
    extractors: BTreeMap<RequestId, BoxedExtractor>,
    clients: BTreeMap<RequestId, Waker>,
    responses: BTreeMap<RequestId, (HttpCallResponse, Option<ResponseContent>)>,
    done: bool,
}

impl RawRootReactor {
    fn context_id(&self) -> RootCid {
        self.context_id
    }

    fn current_event(&self) -> Option<EventKind> {
        match self.active_cid {
            Cid::Http(id) => self.http_reactors.get(&id).map(|r| r.current_event()),
            _ => None,
        }
    }

    fn active_cid(&self) -> Cid {
        self.active_cid
    }

    fn set_active_cid(&mut self, active_cid: Cid) {
        self.active_cid = active_cid;
    }

    fn done(&self) -> bool {
        self.done
    }

    fn set_done(&mut self) {
        self.done = true;
    }

    fn notify_response(&mut self, response: HttpCallResponse) {
        let request_id = response.request_id;
        let content = self.extractors.remove(&request_id).map(|e| e(&response));
        self.responses.insert(request_id, (response, content));
        if let Some(client) = self.clients.get(&request_id) {
            client.wake_by_ref();
        }
    }

    fn insert_create_waker(&mut self, waker: Waker) {
        self.context_create_waker = Some(waker);
    }

    fn take_create_waker(&mut self) -> Option<Waker> {
        self.context_create_waker.take()
    }

    fn set_paused(&self, cid: Cid, paused: bool) {
        match cid {
            Cid::Root(id) => {
                if id != self.context_id {
                    log::warn!("Trying to set pausing state from an invalid Context Reactor with id = {id:?}");
                }
            }
            Cid::Http(id) => {
                if let Some(reactor) = self.http_reactors.get(&id) {
                    reactor.set_paused(paused)
                } else {
                    log::warn!(
                        "Trying to set pausing state from a missing Http Reactor with id = {id:?}"
                    );
                }
            }
        }
    }

    fn set_http_context_done(&mut self, context_id: HttpCid) {
        self.http_reactors.remove(&context_id);
        self.set_active_cid(Cid::Root(self.context_id));
    }

    fn create_http_context(&mut self, context_id: HttpCid) -> Option<Rc<HttpReactor>> {
        let new_http_reactor = Rc::new(HttpReactor::new(context_id));
        self.new_http_reactor = Some(new_http_reactor.clone());
        self.http_reactors
            .insert(context_id, new_http_reactor.clone());

        let waker = self.context_create_waker.as_ref()?;

        waker.wake_by_ref();

        Some(new_http_reactor)
    }

    fn take_new_http_reactor(&mut self) -> Option<Rc<HttpReactor>> {
        self.new_http_reactor.take()
    }

    fn insert_client(&mut self, request_id: RequestId, waker: Waker) {
        self.clients.insert(request_id, waker);
    }

    fn insert_extractor(&mut self, request_id: RequestId, extractor: BoxedExtractor) {
        self.extractors.insert(request_id, extractor);
    }

    fn remove_extractor(&mut self, request_id: RequestId) -> Option<BoxedExtractor> {
        self.extractors.remove(&request_id)
    }

    fn remove_client(&mut self, request_id: RequestId) -> Option<Waker> {
        self.clients.remove(&request_id)
    }

    fn take_response(
        &mut self,
        request_id: RequestId,
    ) -> Option<(HttpCallResponse, Option<ResponseContent>)> {
        self.responses.remove(&request_id)
    }
}

pub struct RootReactor {
    raw: RefCell<RawRootReactor>,
}

impl RootReactor {
    pub fn new(context_id: RootCid) -> Self {
        Self {
            raw: RefCell::new(RawRootReactor {
                context_id,
                active_cid: Cid::Root(context_id),
                context_create_waker: None,
                new_http_reactor: None,
                http_reactors: BTreeMap::new(),
                extractors: BTreeMap::new(),
                clients: BTreeMap::new(),
                responses: BTreeMap::new(),
                done: false,
            }),
        }
    }

    pub fn active_cid(&self) -> Cid {
        self.raw.borrow().active_cid()
    }

    pub fn set_active_cid(&self, active_cid: Cid) {
        self.raw.borrow_mut().set_active_cid(active_cid);
    }

    pub fn create_http_context(&self, context_id: HttpCid) -> Option<Rc<HttpReactor>> {
        self.raw.borrow_mut().create_http_context(context_id)
    }

    pub fn context_id(&self) -> RootCid {
        self.raw.borrow().context_id()
    }

    pub fn current_event(&self) -> Option<EventKind> {
        self.raw.borrow().current_event()
    }

    pub fn insert_create_waker(&self, waker: Waker) {
        self.raw.borrow_mut().insert_create_waker(waker);
    }

    pub fn take_new_http_reactor(&self) -> Option<Rc<HttpReactor>> {
        self.raw.borrow_mut().take_new_http_reactor()
    }

    pub fn take_create_waker(&self) -> Option<Waker> {
        self.raw.borrow_mut().take_create_waker()
    }

    pub fn set_paused(&self, cid: Cid, paused: bool) {
        self.raw.borrow_mut().set_paused(cid, paused);
    }

    pub fn set_http_context_done(&self, context_id: HttpCid) {
        self.raw.borrow_mut().set_http_context_done(context_id);
    }

    pub fn done(&self) -> bool {
        self.raw.borrow().done()
    }

    pub fn set_done(&self) {
        self.raw.borrow_mut().set_done();
    }

    pub fn notify_response(&self, response: HttpCallResponse) {
        self.raw.borrow_mut().notify_response(response)
    }

    pub fn insert_client(&self, request_id: RequestId, waker: Waker) {
        self.raw.borrow_mut().insert_client(request_id, waker);
    }

    pub fn insert_extractor(&self, request_id: RequestId, extractor: BoxedExtractor) {
        self.raw
            .borrow_mut()
            .insert_extractor(request_id, extractor);
    }

    pub fn remove_client(&self, request_id: RequestId) -> Option<Waker> {
        self.raw.borrow_mut().remove_client(request_id)
    }

    pub fn remove_extractor(&self, request_id: RequestId) -> Option<BoxedExtractor> {
        self.raw.borrow_mut().remove_extractor(request_id)
    }

    pub fn remove_response(
        &self,
        request_id: RequestId,
    ) -> Option<(HttpCallResponse, Option<ResponseContent>)> {
        self.raw.borrow_mut().take_response(request_id)
    }
}
