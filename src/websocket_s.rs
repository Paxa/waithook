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
use websocket::result::WebSocketError;

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

// Just wrapper, so I can remove listener from array when connection is closed
struct SenderAndIp {
    pub sender: Sender<RequestWrap>,
    pub ip: SocketAddr
}

pub fn run_server(server_port : u16) {
    // Start listening for WebSocket connections
    let listen_address = format!("0.0.0.0:{}", server_port);
    println!("Starting server on {}", listen_address);
    let ws_server = Server::bind(SocketAddr::from_str(listen_address.as_str()).unwrap()).unwrap();

    let (sender, reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();

    let subscribers : Vec<SenderAndIp> = Vec::new();
    let subscribers_shared = Arc::new(Mutex::new(subscribers));

    let broker_subscribers = subscribers_shared.clone();
    thread::spawn(move || {
        loop {
            let request = reciever.recv();

            match request {
                Ok(request) => {
                    println!("Got message {:?} from {:?}", request, request.client_ip);

                    let mut listerners_wrap = broker_subscribers.lock().unwrap();
                    let mut listerners = listerners_wrap.deref_mut();

                    println!("Send message to {} listeners", listerners.len());

                    listerners.retain(|ref listerner| {
                        println!("Send message to listener {}", listerner.ip);
                        match listerner.sender.send(request.clone()) {
                            Ok(_) => { true },
                            Err(e) => {
                                println!("Channel send error: {}", e);
                                if format!("{}", e) == "sending on a closed channel" {
                                    println!("Remove listener from list {}", listerner.ip);
                                    false
                                } else {
                                    true
                                }
                            }
                        }
                    });
                    println!("total listeners: {}", listerners.len());
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
                    body: request.body.clone(),
                    client_ip: request.get_reader().peer_addr().unwrap().clone().ip()
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

                let (ws_sender, mut ws_receiver) = client.split();

                let (channel_sender, channel_reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();
                //let channel_sender_ref = &channel_sender;

                let sender_and_ip = SenderAndIp {
                    sender: channel_sender,
                    ip: client_ip.clone()
                };

                // block to make sure listeners are unblocked
                {
                    let mut listerners_wrap = local_subscribers.lock().unwrap();
                    let mut listerners = listerners_wrap.deref_mut();
                    //listerners.push(channel_sender);
                    listerners.push(sender_and_ip);
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

                                    match req_local_ws_sender.lock().unwrap().deref_mut().send_message(&message) {
                                        Ok(status) => {
                                            println!("WS Broadcast to {} success: {:?}", client_ip, status);
                                        },
                                        Err(e) => {
                                            println!("WS Broadcast to {} failed: {:?} {}", client_ip, e, e);
                                            match e {
                                                WebSocketError::IoError(err) => {
                                                    println!("WS Broadcast WebSocketError::IoError error: {:?} {}", err, err);
                                                    println!("WS Stoping broadcast loop");
                                                    break
                                                },
                                                _ => {
                                                    println!("WS Broadcast error: {:?} {}", e, e);
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                println!("WS Recieve Error: {:?} {}", e, e);
                                if format!("{}", e) == "receiving on a closed channel" {
                                    println!("WS Channel is closed, quiting!");
                                    break;
                                }
                            }
                        }
                    }
                });

                let local_ws_sender = ws_sender_shared.clone();
                thread::spawn(move || {
                    for message in ws_receiver.incoming_messages() {
                        println!("d1");
                        let message: Message = message.unwrap();
                        println!("WS Got message {} {:?} {:?}", message.opcode, message.cd_status_code, message.payload);
                        match message.opcode {
                            Type::Close => {
                                let message = Message::close();
                                println!("WS Client {} disconnected", client_ip);
                                println!("d2");
                                match local_ws_sender.lock().unwrap().deref_mut().send_message(&message) {
                                    Ok(_) => { println!("WS send close ok") },
                                    Err(e) => { println!("WS Error while sending close message {:?} {}", e, e) }
                                }

                                // block to make sure listeners are unblocked
                                {
                                    println!("WS Remove from listeners");
                                    let mut listerners_wrap = local_subscribers.lock().unwrap();
                                    let mut listerners = listerners_wrap.deref_mut();
                                    //listerners.push(channel_sender);
                                    let index = listerners.iter().position(|ref r| r.ip == client_ip );
                                    match index {
                                        Some(i) => {
                                            println!("WS Remove listener {}", i);
                                            listerners.remove(i);
                                        },
                                        None => {
                                            println!("WS Can not find listerner in a list");
                                        }
                                    }
                                }
                                return;
                            },
                            Type::Ping => {
                                println!("WS Got PING from {}", client_ip);
                                let message = Message::pong(message.payload);
                                //sender.send_message(&message).unwrap();
                                match local_ws_sender.lock().unwrap().deref_mut().send_message(&message) {
                                    Ok(_) => { println!("WS send pong ok") },
                                    Err(e) => { println!("WS Error while sending pong {:?} {}", e, e) }
                                }
                            },
                            Type::Pong => {
                                println!("WS Got PONG from {}", client_ip);
                            },
                            _ => {
                                match local_ws_sender.lock().unwrap().deref_mut().send_message(&message) {
                                    Ok(_) => { println!("WS send same ok {}", message.opcode) },
                                    Err(e) => { println!("WS Error while sending same message {} {:?} {}", message.opcode, e, e) }
                                }
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
                        match ping_local_ws_sender.lock().unwrap().deref_mut().send_message(&message) {
                            Ok(status) => {
                                println!("WS Ping success: {:?}", status);
                            },
                            Err(e) => {
                                println!("WS Ping failed: {:?} {}", e, e);
                                match e {
                                    WebSocketError::IoError(err) => {
                                        println!("WS Ping WebSocketError::IoError error: {:?} {}", err, err);
                                        println!("WS Stoping ping loop");
                                        break
                                    },
                                    _ => {
                                        println!("WS Ping error: {:?} {}", e, e);
                                    }
                                }
                            }
                        }
                        thread::sleep(Duration::from_millis(10 * 1000));
                    }
                });

            }
        });
    }
}
