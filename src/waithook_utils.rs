use std::thread;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use std::sync::mpsc::{Receiver};
use std::ops::DerefMut;
use std::time::Duration;

use websocket::{Message};
use websocket::Sender as WsSender;
use websocket::result::WebSocketError;

use request_wrap::RequestWrap;
use waithook_server::{SharedSender, SenderAndIp};
use rustc_serialize::json;

fn extract_path(url : String) -> String {
    url[0 .. url.find('?').unwrap_or(url.len())].to_string()
}

fn pretty_json(request : RequestWrap) -> String {
    let encoder = json::as_pretty_json(&request);
    format!("{}", encoder)
}

pub fn keep_alive_ping(ping_local_ws_sender : SharedSender, client_ip : SocketAddr) {
    thread::spawn(move || {
        loop {
            println!("WS Sending PING to {}", client_ip);
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

pub fn handle_ping_message(incoming_message : Message, local_ws_sender : &SharedSender, client_ip : SocketAddr) {
    println!("WS Got PING from {}", client_ip);
    let message = Message::pong(incoming_message.payload);
    //sender.send_message(&message).unwrap();
    match local_ws_sender.lock().unwrap().deref_mut().send_message(&message) {
        Ok(_) => { println!("WS send pong ok") },
        Err(e) => { println!("WS Error while sending pong {:?} {}", e, e) }
    }
}

pub fn handle_close_message(local_ws_sender : &SharedSender, client_ip : SocketAddr) {
    let message = Message::close();
    println!("WS Client {} disconnected", client_ip);
    match local_ws_sender.lock() {
        Ok(mut res) => {
            match res.deref_mut().send_message(&message) {
                Ok(_) => { println!("WS send close ok") },
                Err(e) => { println!("WS Error while sending close message {:?} {}", e, e) }
            }
        },
        Err(e) => {
            println!("WS Error while sending close message {:?} {}", e, e);
        }
    }
}

pub fn listen_and_forward(ws_sender: SharedSender, channel_reciever: Receiver<RequestWrap>, path: String, client_ip: SocketAddr) {
    thread::spawn(move || {
        loop {
            let request = channel_reciever.recv();
            match request {
                Ok(request) => {
                    if extract_path(request.url.clone()) == path {
                        println!("WS {} Got message {:?}", path, request);

                        let message_row = pretty_json(request);
                        let message: Message = Message::text(message_row);

                        match ws_sender.lock().unwrap().deref_mut().send_message(&message) {
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
}

pub fn run_broadcast_broker(reciever: Receiver<RequestWrap>, broker_subscribers: Arc<Mutex<Vec<SenderAndIp>>>) {
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
}
