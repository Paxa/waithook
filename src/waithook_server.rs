use std::thread;
use std::net::SocketAddr;
use std::str::FromStr;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver};

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

pub type SharedSender = Arc<Mutex<WsClientSender<WebSocketStream>>>;

// Just wrapper, so I can remove listener from array when connection is closed
pub struct SenderAndIp {
    pub sender: Sender<RequestWrap>,
    pub ip: SocketAddr,
    pub path: String
}

pub fn run_server(server_port : u16) {
    // Start listening for WebSocket connections
    let listen_address = format!("0.0.0.0:{}", server_port);
    println!("Starting server on {}", listen_address);
    let ws_server = Server::bind(SocketAddr::from_str(listen_address.as_str()).unwrap()).unwrap();

    let (sender, reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();

    let subscribers : Vec<SenderAndIp> = Vec::new();
    let subscribers_shared = Arc::new(Mutex::new(subscribers));
    let start_time = time::now();

    let broker_subscribers = subscribers_shared.clone();
    waithook_utils::run_broadcast_broker(reciever, broker_subscribers);

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

                if path == "/@/stats" {
                    let mut listerners_wrap = local_subscribers.lock().unwrap();
                    waithook_stats::show_stats(web_request, writer, listerners_wrap.deref_mut(), start_time);
                } else {
                    webserver::handle(web_request, writer, sender);
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

                println!("WS Connection from {}", client_ip);
                println!("WS Headers: {:?}", headers);

                let (ws_sender, mut ws_receiver) = client.split();

                let (channel_sender, channel_reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();
                //let channel_sender_ref = &channel_sender;

                let sender_and_ip = SenderAndIp {
                    sender: channel_sender,
                    ip: client_ip.clone(),
                    path: path.clone()
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
                waithook_utils::listen_and_forward(req_local_ws_sender, channel_reciever, path, client_ip);

                let local_ws_sender = ws_sender_shared.clone();
                thread::spawn(move || {
                    for message in ws_receiver.incoming_messages() {
                        let message: Message = message.unwrap();
                        println!("WS Got message {} {:?} {:?}", message.opcode, message.cd_status_code, message.payload);
                        match message.opcode {
                            Type::Close => {
                                waithook_utils::handle_close_message(&local_ws_sender, client_ip);

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
                                waithook_utils::handle_ping_message(message, &local_ws_sender, client_ip);
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
                waithook_utils::keep_alive_ping(ping_local_ws_sender, client_ip.clone());
            }
        });
    }
}
