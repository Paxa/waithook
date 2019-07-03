use std::thread;
use std::net::{SocketAddr, TcpStream};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver};
use std::str;

use websocket::{Message, OwnedMessage};
use websocket::sync::Server as WsServer;
use websocket::sender::Writer as WsWriter;
use websocket::message::Type;

use time;

use webserver;
use request_wrap::RequestWrap;
use waithook_utils;
use waithook_stats;
use waithook_forward;
use hyper_utils;

pub type SharedSender = Arc<Mutex<WsWriter<TcpStream>>>;
pub type SubscribersLock = Arc<Mutex<Vec<Subscriber>>>;

// Just wrapper, so I can remove listener from array when connection is closed
pub struct Subscriber {
    pub sender: SharedSender,
    pub ip: SocketAddr,
    pub path: String
}

pub fn run_server(server_port : u16) {
    // Start listening for WebSocket connections
    let listen_address = format!("0.0.0.0:{}", server_port);
    println!("Starting server on {}", listen_address);
    let mut ws_server = WsServer::bind(listen_address.as_str()).unwrap();

    let (req_sender, req_reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();

    let subscribers : Vec<Subscriber> = Vec::new();
    let subscribers_shared = Arc::new(Mutex::new(subscribers));
    let start_time = time::now();

    let broker_subscribers = subscribers_shared.clone();
    waithook_utils::run_broadcast_broker(req_reciever, broker_subscribers);
    let forward_sender = waithook_forward::run_forwarder();

    loop {
        let connection_res = ws_server.accept();

        let local_subscribers = subscribers_shared.clone();
        let keep_alive_subscribers = subscribers_shared.clone();
        let req_sender = req_sender.clone();
        let forward_sender = forward_sender.clone();

        thread::spawn(move || {

            if connection_res.is_err() {

                let (tcp_stream, request_wrap) = hyper_utils::create_request_wrap(connection_res).unwrap();

                if request_wrap.url == "/@/stats" {
                    let mut listeners_wrap = local_subscribers.lock().unwrap();
                    waithook_stats::show_stats(request_wrap, tcp_stream, listeners_wrap.deref_mut(), start_time);
                } else {
                    webserver::handle(request_wrap.clone(), tcp_stream, req_sender);
                    if request_wrap.url != "/" && !request_wrap.url.starts_with("/@/") {
                        match forward_sender.send(request_wrap) {
                            Ok(_) => {},
                            Err(e) => { println!("FORWARDER: HTTP Channel send error: {}", e); }
                        }
                    }
                }
                return;
            } else {
                let connection = match connection_res {
                    Ok (conn) => conn,
                    Err(e) => {
                        println!("Connection Accept Error {:?}", e);
                        return;
                    }
                };
                let client_ip = connection.stream.peer_addr().unwrap().clone();
                let headers = connection.headers.clone();
                let path = connection.uri();

                //request.validate().unwrap(); // Validate the request

                // Form a response
                let client = match connection.accept() {
                    Ok (c) => c,
                    Err ((_, e)) => {
                        println!("Connection Accept Error {:?} {}", e, e);
                        return;
                    }
                };

                println!("WS Connection from {} on {}", client_ip, path);
                println!("WS Headers: {:?}", headers);

                let (mut ws_receiver, ws_sender) = client.split().unwrap();

                let ws_sender_shared = Arc::new(Mutex::new(ws_sender));

                let subscriber = Subscriber {
                    sender: ws_sender_shared.clone(),
                    ip: client_ip.clone(),
                    path: path.clone()
                };

                // block to make sure listeners are unblocked
                {
                    let mut listeners_wrap = local_subscribers.lock().unwrap();
                    let listeners = listeners_wrap.deref_mut();
                    listeners.push(subscriber);
                }

                let local_ws_sender = ws_sender_shared.clone();
                thread::spawn(move || {
                    for owned_message in ws_receiver.incoming_messages() {
                        let owned_message: OwnedMessage = match owned_message {
                            Ok(message) => message,
                            Err(e) => {
                                // This block should run when client disconnected and TCP socket is aware of if
                                // if TCP connection will not raise error on disconnect then it probably will in a ping loop
                                println!("WS Error receiving a message {:?} {}", e, e);
                                println!("WS Error probabaly client {} disconnected", client_ip);
                                waithook_utils::remove_listener(&local_subscribers, client_ip);
                                break;
                            }
                        };
                        let message = Message::from(owned_message);

                        if message.opcode != Type::Ping {
                            println!("WS Got message {:?} {:?} {:?}", message.opcode, message.cd_status_code, str::from_utf8(&message.payload));
                            //println!("WS Got message {}", message.opcode());
                        }

                        match message.opcode {
                            Type::Close => {
                                waithook_utils::handle_close_message(&local_ws_sender, client_ip);
                                waithook_utils::remove_listener(&local_subscribers, client_ip);
                                return;
                            },
                            Type::Ping => {
                                if !waithook_utils::handle_ping_message(message, &local_ws_sender, client_ip) {
                                    waithook_utils::remove_listener(&local_subscribers, client_ip);
                                }
                            },
                            Type::Pong => {
                                //println!("WS Got PONG from {}", client_ip);
                            },
                            _ => {
                                match local_ws_sender.lock().unwrap().deref_mut().send_message(&message) {
                                    Ok(_) => { println!("WS send same ok {:?}", message.opcode) },
                                    Err(e) => { println!("WS Error while sending same message {:?} {:?} {}", message.opcode, e, e) }
                                }
                            }
                        }
                    }
                });

                // Keep alive thing
                let ping_local_ws_sender = ws_sender_shared.clone();
                waithook_utils::keep_alive_ping(ping_local_ws_sender, client_ip.clone(), keep_alive_subscribers);
            }
        });
    }
}
