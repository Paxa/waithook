use std::thread;
use std::net::SocketAddr;
use std::str::FromStr;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver};
use std::time::Instant;

use websocket::{Message, Server, WebSocketStream};
use websocket::Receiver as WsReceiver;
use websocket::Sender as WsSender;
use websocket::client::Sender as WsClientSender;
use websocket::server::Response as WsResponse;
use websocket::message::Type;

use hyper::header;
use time;

use webserver;
use request_wrap::RequestWrap;
use waithook_utils;
use waithook_stats;
use waithook_forward;

pub type SharedSender = Arc<Mutex<WsClientSender<WebSocketStream>>>;
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
    let ws_server = Server::bind(SocketAddr::from_str(listen_address.as_str()).unwrap()).unwrap();

    let (sender, reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();

    let subscribers : Vec<Subscriber> = Vec::new();
    let subscribers_shared = Arc::new(Mutex::new(subscribers));
    let start_time = time::now();

    let broker_subscribers = subscribers_shared.clone();
    waithook_utils::run_broadcast_broker(reciever, broker_subscribers);
    let forward_sender = waithook_forward::run_forwarder();

    for connection in ws_server {
        let local_subscribers = subscribers_shared.clone();
        let keep_alive_subscribers = subscribers_shared.clone();
        let sender = sender.clone();
        let forward_sender = forward_sender.clone();

        thread::spawn(move || {
            //let request = connection.unwrap().read_request().unwrap();
            let request = match connection.unwrap().read_request() {
                Ok(request) => request,
                Err(e) => {
                    // This block should run when client disconnected and TCP socket is aware of if
                    // if TCP connection will not raise error on disconnect then it probably will in a ping loop
                    println!("HTTP Error reading request {:?} {}", e, e);
                    return;
                }
            };
            let headers = request.headers.clone();
            let path = request.url.to_string();

            if !headers.has::<header::Upgrade>() {

                let web_request = RequestWrap {
                    method: request.method.as_ref().to_string(),
                    url: request.url.to_string(),
                    headers: request.headers.clone(),
                    body: request.body.clone(),
                    client_ip: request.get_reader().peer_addr().unwrap().clone(),
                    time: Instant::now()
                };

                let response = WsResponse::bad_request(request);
                let (_, writer) = response.into_inner();

                if path == "/@/stats" {
                    let mut listeners_wrap = local_subscribers.lock().unwrap();
                    waithook_stats::show_stats(web_request, writer, listeners_wrap.deref_mut(), start_time);
                } else {
                    webserver::handle(web_request.clone(), writer, sender);
                    if web_request.url != "/" && !web_request.url.starts_with("/@/") {
                        match forward_sender.send(web_request) {
                            Ok(_) => {},
                            Err(e) => { println!("FORWARDER: HTTP Channel send error: {}", e); }
                        }
                    }
                }
                return;
            } else {
                request.validate().unwrap(); // Validate the request

                let response = request.accept(); // Form a response

                let mut client = response.send().unwrap(); // Send the response

                let client_ip = client.get_mut_sender()
                    .get_mut()
                    .peer_addr()
                    .unwrap();

                println!("WS Connection from {} on {}", client_ip, path);
                println!("WS Headers: {:?}", headers);

                let (ws_sender, mut ws_receiver) = client.split();

                let ws_sender_shared = Arc::new(Mutex::new(ws_sender));

                let subscriber = Subscriber {
                    sender: ws_sender_shared.clone(),
                    ip: client_ip.clone(),
                    path: path.clone()
                };

                // block to make sure listeners are unblocked
                {
                    let mut listeners_wrap = local_subscribers.lock().unwrap();
                    let mut listeners = listeners_wrap.deref_mut();
                    listeners.push(subscriber);
                }

                let local_ws_sender = ws_sender_shared.clone();
                thread::spawn(move || {
                    for message in ws_receiver.incoming_messages() {
                        let message: Message = match message {
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
                        if message.opcode != Type::Ping {
                            println!("WS Got message {} {:?} {:?}", message.opcode, message.cd_status_code, message.payload);
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
                                    Ok(_) => { println!("WS send same ok {}", message.opcode) },
                                    Err(e) => { println!("WS Error while sending same message {} {:?} {}", message.opcode, e, e) }
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
