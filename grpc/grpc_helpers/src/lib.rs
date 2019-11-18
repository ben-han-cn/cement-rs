// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use failure::{prelude::*, Result};
use futures::{compat::Future01CompatExt, future::Future, prelude::*};
use futures_01::future::Future as Future01;
use grpcio::{EnvBuilder, ServerBuilder};
use std::{
    str::from_utf8,
    sync::{
        mpsc::{self, Sender},
        Arc,
    },
    thread, time,
};

pub fn spawn_service_thread(
    service: ::grpcio::Service,
    service_host_address: String,
    service_public_port: u16,
    service_name: impl Into<String>,
) -> ServerHandle {
    spawn_service_thread_with_drop_closure(
        service,
        service_host_address,
        service_public_port,
        service_name,
        || { /* no code, to make compiler happy */ },
    )
}

pub fn spawn_service_thread_with_drop_closure<F>(
    service: ::grpcio::Service,
    service_host_address: String,
    service_public_port: u16,
    service_name: impl Into<String>,
    service_drop_closure: F,
) -> ServerHandle
where
    F: FnOnce() + 'static,
{
    let env = Arc::new(EnvBuilder::new().name_prefix(service_name).build());
    let server = ServerBuilder::new(env)
        .register_service(service)
        .bind(service_host_address, service_public_port)
        .build()
        .expect("Unable to create grpc server");
    ServerHandle::setup_with_drop_closure(server, Some(Box::new(service_drop_closure)))
}

pub struct ServerHandle {
    stop_sender: Sender<()>,
    drop_closure: Option<Box<dyn FnOnce()>>,
}

impl ServerHandle {
    pub fn setup_with_drop_closure(
        mut server: ::grpcio::Server,
        drop_closure: Option<Box<dyn FnOnce()>>,
    ) -> Self {
        let (start_sender, start_receiver) = mpsc::channel();
        let (stop_sender, stop_receiver) = mpsc::channel();
        let handle = Self {
            stop_sender,
            drop_closure,
        };
        thread::spawn(move || {
            server.start();
            start_sender.send(()).unwrap();
            loop {
                if stop_receiver.try_recv().is_ok() {
                    return;
                }
                thread::sleep(time::Duration::from_millis(100));
            }
        });

        start_receiver.recv().unwrap();
        handle
    }
    pub fn setup(server: ::grpcio::Server) -> Self {
        Self::setup_with_drop_closure(server, None)
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.stop_sender.send(()).unwrap();
        if let Some(f) = self.drop_closure.take() {
            f()
        }
    }
}

pub fn convert_grpc_response<T>(
    response: grpcio::Result<impl Future01<Item = T, Error = grpcio::Error>>,
) -> impl Future<Output = Result<T>> {
    future::ready(response.map_err(convert_grpc_err))
        .map_ok(Future01CompatExt::compat)
        .and_then(|x| x.map_err(convert_grpc_err))
}

fn convert_grpc_err(e: ::grpcio::Error) -> Error {
    format_err!("grpc error: {}", e)
}

pub fn provide_grpc_response<ResponseType: std::fmt::Debug>(
    resp: Result<ResponseType>,
    ctx: ::grpcio::RpcContext<'_>,
    sink: ::grpcio::UnarySink<ResponseType>,
) {
    match resp {
        Ok(resp) => ctx.spawn(sink.success(resp).map_err(default_reply_error_logger)),
        Err(e) => {
            let f = sink
                .fail(create_grpc_invalid_arg_status(
                    from_utf8(ctx.method()).expect("Unable to convert function name to string"),
                    e,
                ))
                .map_err(default_reply_error_logger);
            ctx.spawn(f)
        }
    }
}

pub fn create_grpc_invalid_arg_status(method: &str, err: ::failure::Error) -> ::grpcio::RpcStatus {
    let msg = format!("Request {} failed {}", method, err);
    ::grpcio::RpcStatus::new(::grpcio::RpcStatusCode::InvalidArgument, Some(msg))
}

pub fn default_reply_error_logger<T: std::fmt::Debug>(e: T) {
    println!("Failed to reply error due to {:?}", e)
}
