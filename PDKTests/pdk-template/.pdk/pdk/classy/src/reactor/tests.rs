// Copyright 2023 Salesforce, Inc. All rights reserved.
mod root {
    use std::{
        rc::Rc,
        sync::{Arc, Mutex},
        task::{Wake, Waker},
    };

    use futures::task::noop_waker;

    use crate::{
        client::HttpCallResponse,
        reactor::root::RootReactor,
        types::{HttpCid, RequestId, RootCid},
    };

    /// This Waker counts how many times it was woken up
    struct CountingWaker {
        count: Mutex<usize>,
    }

    impl CountingWaker {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                count: Mutex::new(0),
            })
        }

        fn count(&self) -> usize {
            *self.count.lock().unwrap()
        }

        fn to_waker(self: &Arc<Self>) -> Waker {
            Waker::from(Arc::clone(self))
        }
    }

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            *self.count.lock().unwrap() += 1;
        }
    }

    #[test]
    fn active_cid() {
        let root_cid: RootCid = RootCid::from(1);
        let reactor = RootReactor::new(root_cid);

        assert_eq!(reactor.context_id(), root_cid);
        assert_eq!(reactor.active_cid(), root_cid.into());
    }

    #[test]
    fn create_http_context() {
        let root_cid = RootCid::from(1);
        let http_cid = HttpCid::from(10);

        let root_reactor = RootReactor::new(root_cid);

        let create_waker = CountingWaker::new();

        root_reactor.insert_create_waker(create_waker.to_waker());

        // create context
        let http_reactor = root_reactor.create_http_context(http_cid).unwrap();

        assert_eq!(create_waker.count(), 1);
        assert_eq!(http_reactor.context_id(), http_cid);
        assert_eq!(root_reactor.active_cid(), root_cid.into());

        let new_http_reactor = root_reactor.take_new_http_reactor().unwrap();
        assert!(Rc::ptr_eq(&http_reactor, &new_http_reactor));

        assert!(root_reactor.take_new_http_reactor().is_none());
    }

    #[test]
    fn notify_response_in_root() {
        let root_cid = RootCid::from(1);
        let root_reactor = RootReactor::new(root_cid);

        let request_id = RequestId::from(100);
        let content = Box::new(1000);
        let response = HttpCallResponse {
            request_id,
            num_headers: 10,
            body_size: 2000,
            num_trailers: 5,
        };

        let expected_content = Some(content.clone());
        let expected_response = response.clone();
        let extractor_response = response.clone();

        let client_waker = CountingWaker::new();
        root_reactor.insert_extractor(
            request_id,
            Box::new(move |event| {
                assert_eq!(&extractor_response, event);
                content
            }),
        );
        root_reactor.insert_client(request_id, client_waker.to_waker());
        root_reactor.notify_response(response);

        assert_eq!(client_waker.count(), 1);
        let (actual_response, actual_content) = root_reactor.remove_response(request_id).unwrap();
        let actual_content = actual_content.and_then(|c| c.downcast::<i32>().ok());

        assert_eq!(expected_response, actual_response);
        assert_eq!(expected_content, actual_content);
        assert!(root_reactor.remove_response(request_id).is_none());

        root_reactor.remove_client(request_id);
        root_reactor.notify_response(expected_response);

        assert_eq!(
            client_waker.count(),
            1,
            "After removing client waker, it should not be notified"
        );
    }

    #[test]
    fn notify_response_in_http() {
        let root_cid = RootCid::from(1);
        let http_cid = HttpCid::from(100);

        let root_reactor = RootReactor::new(root_cid);

        let request_id = RequestId::from(100);

        // Make reactor happy when creating http context
        root_reactor.insert_create_waker(noop_waker());
        let _ = root_reactor.create_http_context(http_cid);

        let client_waker = CountingWaker::new();
        root_reactor.insert_client(request_id, client_waker.to_waker());

        let response = HttpCallResponse {
            request_id,
            num_headers: 10,
            body_size: 2000,
            num_trailers: 5,
        };
        let twice_response = response.clone();

        root_reactor.notify_response(response);

        assert_eq!(client_waker.count(), 1);

        root_reactor.remove_client(request_id);
        root_reactor.notify_response(twice_response);

        assert_eq!(
            client_waker.count(),
            1,
            "After removing client waker, it should not be notified"
        );
    }
}
