// Copyright 2023 Salesforce, Inc. All rights reserved.
use futures::{Stream, StreamExt};
use std::task::{Poll, Waker};

use crate::{
    event::{After, Exchange, Start},
    extract::{context::FilterContext, FromContext},
    handler::Handler,
    host::Host,
    reactor::{
        http::{ExchangePhase, HttpReactor},
        root::RootReactor,
    },
};
use std::rc::Rc;

/// Launcher for asynchronous dispatch of filters through the `launch` method.
///
/// This launcher is sent as parameter for configuration functions during `on_configure` event.
///
/// # Example
///
/// ```
/// use classy::event::{Exchange, RequestHeaders};
/// use classy::bootstrap::Launcher;
///
/// async fn my_filter(exchange: Exchange<RequestHeaders>) {
/// }
///
/// async fn my_configuration(launcher: Launcher) {
///     launcher.launch(my_filter).await.unwrap();
/// }
/// ```
pub struct Launcher {
    reactor: Rc<RootReactor>,
    host: Rc<dyn Host>,
}

#[derive(thiserror::Error, Debug)]
#[error("Launch Error")]
pub struct LaunchError {}

impl Launcher {
    pub(crate) fn new(reactor: Rc<RootReactor>, host: Rc<dyn Host>) -> Self {
        Self { reactor, host }
    }

    pub async fn launch<H, T, E>(self, filter: H) -> Result<(), LaunchError>
    where
        H: Handler<Exchange<E>, T, Result = ()>,
        T: FromContext<FilterContext>,
        E: After<Start>,
    {
        let reactor = self.reactor.clone();
        let host = &self.host;

        let contexts = ContextCreateStream::new(reactor.clone());

        contexts
            .map(|http_reactor| {
                let context =
                    FilterContext::new(host.clone(), reactor.clone(), http_reactor.clone());
                let exchange: Exchange<Start> = Exchange::new(http_reactor.clone(), host.clone());
                let extraction_result = T::from_context(&context).map(|args| async {
                    let exchange = exchange.wait_for_event::<E>().await;
                    filter.call(exchange, args).await
                });
                (http_reactor, extraction_result)
            })
            .for_each_concurrent(None, |(http_reactor, extraction_result)| async move {
                match extraction_result {
                    Ok(result) => {
                        result.await;

                        if http_reactor.paused() && !http_reactor.cancelled_request() {
                            http_reactor.set_paused(false);
                            match http_reactor.phase() {
                                ExchangePhase::Request => host.resume_http_request(),
                                ExchangePhase::Response => {
                                    host.resume_http_response();
                                }
                            }
                        }
                    }
                    Err(extraction_error) => {
                        log::error!(
                            "Extraction problem in filter: {:?}",
                            extraction_error.into()
                        );
                    }
                }
            })
            .await;

        Ok(())
    }
}

struct ContextCreateStream {
    reactor: Rc<RootReactor>,
    waker: Option<Waker>,
}

impl ContextCreateStream {
    fn new(reactor: Rc<RootReactor>) -> Self {
        Self {
            reactor,
            waker: None,
        }
    }
}

impl Unpin for ContextCreateStream {}

impl Stream for ContextCreateStream {
    type Item = Rc<HttpReactor>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if self.reactor.done() {
            self.reactor.take_create_waker();
            return Poll::Ready(None);
        }

        if let Some(http_reactor) = self.reactor.take_new_http_reactor() {
            return Poll::Ready(Some(http_reactor));
        }

        match &self.waker {
            None => {
                // Register the waker in the reactor.
                self.reactor.insert_create_waker(cx.waker().clone());
                self.waker = Some(cx.waker().clone());
            }
            Some(waker) if !waker.will_wake(cx.waker()) => {
                self.reactor.insert_create_waker(waker.clone());
                self.waker = Some(cx.waker().clone());
            }
            Some(_) => {}
        }
        Poll::Pending
    }
}
