use std::thread;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use std::str::FromStr;

use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::ops::DerefMut;
use std::time::Duration;

use websocket::{Message, Server};
use websocket::Receiver as WsiReceiver;
use websocket::Sender as WsiSender;
use websocket::server::Response as WsResponse;
use websocket::message::Type;
use websocket::header::{WebSocketProtocol};

use hyper::net::NetworkStream;
use hyper::header;

use webserver;
use request_wrap::RequestWrap;
use rustc_serialize::json;

fn extract_path(url : String) -> String {
    url[0 .. url.find('?').unwrap_or(url.len())].to_string()
}

fn pretty_json(request : RequestWrap) -> String {
    let encoder = json::as_pretty_json(&request);
    format!("{}", encoder)
}

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

                    let mut listerners_wrap = broker_subscribers.lock().unwrap();
                    let listerners = listerners_wrap.deref_mut();

                    println!("Send message to {} listeners", listerners.len());
                    for listerner in listerners {
                        //println!("Send message to listener");
                        match listerner.send(request.clone()) {
                            Ok(_) => {},
                            Err(e) => { println!("Channel send error: {}", e); }
                        }
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

                let response = WsResponse::bad_request(request);
                let (_, writer) = response.into_inner();

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

                let client_ip = client.get_mut_sender()
                    .get_mut()
                    .peer_addr()
                    .unwrap();

                println!("WS Connection from {}", client_ip);
                println!("WS Headers: {:?}", headers);

                let message: Message = Message::text("Hello".to_string());
                client.send_message(&message).unwrap();

                let (ws_sender, mut ws_receiver) = client.split();

                let (channel_sender, channel_reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();

                // block to make sure listeners are unblocked
                {
                    let mut listerners_wrap = local_subscribers.lock().unwrap();
                    let mut listerners = listerners_wrap.deref_mut();
                    listerners.push(channel_sender);
                }

                let ws_sender_shared = Arc::new(Mutex::new(ws_sender));

                let req_local_ws_sender = ws_sender_shared.clone();
                thread::spawn(move || {
                    loop {
                        let request = channel_reciever.recv();
                        match request {
                            Ok(request) => {
                                if extract_path(request.url.clone()) == path {
                                    println!("WS {} Got message {:?}", path, request);

                                    let message_row = pretty_json(request);
                                    let message: Message = Message::text(message_row);

                                    req_local_ws_sender.lock().unwrap().deref_mut().send_message(&message).unwrap();
                                }
                            },
                            Err(e) => {
                                println!("WS Recieve Error: {}", e);
                            }
                        }
                    }
                });

                // Keep alive thing
                let pong_local_ws_sender = ws_sender_shared.clone();
                thread::spawn(move || {
                    for message in ws_receiver.incoming_messages() {
                        let message: Message = message.unwrap();

                        match message.opcode {
                            Type::Close => {
                                let message = Message::close();
                                //sender.send_message(&message).unwrap();
                                println!("WS Client {} disconnected", client_ip);
                                pong_local_ws_sender.lock().unwrap().deref_mut().send_message(&message).unwrap();
                                return;
                            },
                            Type::Ping => {
                                println!("WS Got PING from {}", client_ip);
                                let message = Message::pong(message.payload);
                                //sender.send_message(&message).unwrap();
                                pong_local_ws_sender.lock().unwrap().deref_mut().send_message(&message).unwrap()
                            },
                            Type::Pong => {
                                println!("WS Got PONG from {}", client_ip);
                            },
                            _ => {
                                pong_local_ws_sender.lock().unwrap().deref_mut().send_message(&message).unwrap()
                                //sender.send_message(&message).unwrap()
                            }
                        }
                    }
                });

                // Keep alive thing
                let ping_local_ws_sender = ws_sender_shared.clone();
                let ping_client_ip = client_ip.clone();
                thread::spawn(move || {
                    loop {
                        println!("WS Sending PING to {}", ping_client_ip);
                        let message = Message::ping(b"PING".to_vec());
                        ping_local_ws_sender.lock().unwrap().deref_mut().send_message(&message).unwrap();
                        thread::sleep(Duration::from_millis(30 * 1000));
                    }
                });

            }
        });
    }
}
