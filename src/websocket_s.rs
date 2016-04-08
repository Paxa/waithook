use std::thread;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use std::str::FromStr;
use std::collections::HashMap;

use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::ops::DerefMut;

use websocket::{Message, Server};
use websocket::Receiver as WsiReceiver;
use websocket::Sender as WsiSender;
use websocket::server::Response as WsResponse;
use websocket::message::Type;
use websocket::header::{WebSocketProtocol};
use websocket::stream::WebSocketStream;

use hyper::net::NetworkStream;
use hyper::header;

use webserver;
use request_wrap::RequestWrap;
use rustc_serialize::json;

pub fn run_server(server_port : u16) {
    // Start listening for WebSocket connections
    let listen_address = format!("0.0.0.0:{}", server_port);
    println!("Starting server on {}", listen_address);
    let ws_server = Server::bind(SocketAddr::from_str(listen_address.as_str()).unwrap()).unwrap();

    let (sender, reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();

    let subscribers : Vec<Sender<RequestWrap>> = Vec::new();
    let subscribers_shared = Arc::new(Mutex::new(subscribers));

    let broker_subscribers = subscribers_shared.clone();
    thread::spawn(move || {
        loop {
            let request = reciever.recv();

            match request {
                Ok(request) => {
                    println!("Got message {:?}", request);

                    let mut guard = broker_subscribers.lock();
                    let mut listerners_wrap = guard.unwrap();
                    let mut listerners = listerners_wrap.deref_mut();

                    for listerner in listerners {
                        println!("Send message to listener");
                        listerner.send(request.clone());
                    }
                },
                Err(e) => {
                    println!("Recieve Error: {}", e);
                }
            }

        }
    });

    for connection in ws_server {
        let local_subscribers = subscribers_shared.clone();
        let sender = sender.clone();

        thread::spawn(move || {
            let request = connection.unwrap().read_request().unwrap();
            let headers = request.headers.clone();
            let path = request.url.to_string();

            if !headers.has::<header::Upgrade>() {

                let web_request = RequestWrap {
                    method: request.method.as_ref().to_string(),
                    url: request.url.to_string(),
                    headers: request.headers.clone(),
                    body: request.body.clone()
                };

                let mut response = WsResponse::bad_request(request);
                let (reader, writer) = response.into_inner();

                println!("NOT WEB SOCKET!!!!");
                webserver::handle(web_request, writer, sender);
                return;
            } else {
                request.validate().unwrap(); // Validate the request

                let mut response = request.accept(); // Form a response

                if let Some(&WebSocketProtocol(ref protocols)) = headers.get() {
                    if protocols.contains(&("rust-websocket".to_string())) {
                        // We have a protocol we want to use
                        response.headers.set(WebSocketProtocol(vec!["rust-websocket".to_string()]));
                    }
                }

                let mut client = response.send().unwrap(); // Send the response

                let ip = client.get_mut_sender()
                    .get_mut()
                    .peer_addr()
                    .unwrap();

                println!("Connection from {}", ip);

                let message: Message = Message::text("Hello".to_string());
                client.send_message(&message).unwrap();

                let (mut ws_sender, mut ws_receiver) = client.split();

                let (channel_sender, channel_reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();

                if true {
                    let mut guard = local_subscribers.lock();
                    let mut listerners_wrap = guard.unwrap();
                    let mut listerners = listerners_wrap.deref_mut();
                    listerners.push(channel_sender);
                }

                //let reciever = shared_channels.subscribe();
                loop {
                    let request = channel_reciever.recv();
                    match request {
                        Ok(request) => {
                            if request.url == path {
                                println!("Web Socket {} Got message {:?}", path, request);

                                let message_row = format!("{} {}\n{:?}\n\n{}",
                                    request.method, request.url, request.headers, request.body);
                                let message: Message = Message::text(message_row);
                                ws_sender.send_message(&message).unwrap();
                            }
                        },
                        Err(e) => {
                            println!("Recieve Error: {}", e);
                        }
                    }
                }
            }
        });
    }
}
