use crate::json_encoder::JsonEncoder;
use crate::never::Never;
use futures::{future, Future, IntoFuture};
use hyper::{service::Service, Body, Method, Request, Response, Server, StatusCode};
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;

struct MetricServer {
    path_for_prom: String,
    path_for_http: String,
}

impl MetricServer {
    pub fn new(path_for_prom: String, path_for_http: String) -> Self {
        MetricServer {
            path_for_prom,
            path_for_http,
        }
    }
}

impl Service for MetricServer {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Never;
    type Future = future::FutureResult<Response<Body>, Never>;

    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        let mut resp = Response::new(Body::empty());
        if req.method() == &Method::GET {
            let path = req.uri().path();
            if path == self.path_for_prom {
                *resp.body_mut() = Body::from(encode_metrics(TextEncoder::new()));
            } else if path == self.path_for_http {
                *resp.body_mut() = Body::from(encode_metrics(JsonEncoder));
            } else {
                *resp.status_mut() = StatusCode::NOT_FOUND;
            }
        } else {
            *resp.status_mut() = StatusCode::NOT_FOUND;
        }

        future::ok(resp)
    }
}

impl IntoFuture for MetricServer {
    type Future = future::FutureResult<Self::Item, Never>;
    type Item = Self;
    type Error = Never;

    fn into_future(self) -> Self::Future {
        future::ok(self)
    }
}

fn encode_metrics(encoder: impl Encoder) -> Vec<u8> {
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    buffer
}

pub fn start_metric_server(
    addr: SocketAddr,
    path_for_prom: String,
    path_for_http: String,
) -> impl Future<Item = (), Error = ()> {
    let srv = Server::try_bind(&addr).unwrap();
    srv.serve(move || MetricServer::new(path_for_prom.clone(), path_for_http.clone()))
        .map_err(|e| println!("server error: {}", e))
}
