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
use waithook_server::{SharedSender, Subscriber, SubscribersLock};
use rustc_serialize::json;


fn extract_path(url: String) -> String {
    url[0 .. url.find('?').unwrap_or(url.len())].to_string()
}

fn pretty_json(request: &RequestWrap) -> String {
    let encoder = json::as_pretty_json(&request);
    format!("{}", encoder)
}

pub fn keep_alive_ping(ping_local_ws_sender: SharedSender, client_ip: SocketAddr, local_subscribers: SubscribersLock) {
    thread::spawn(move || {
        loop {
            //println!("WS Sending PING to {}", client_ip);
            let message = Message::ping(b"PING".to_vec());
            match ping_local_ws_sender.lock().unwrap().deref_mut().send_message(&message) {
                Ok(_) => {
                    //println!("WS Ping success: {:?}", status);
                },
                Err(e) => {
                    println!("WS Ping failed: {:?} {}", e, e);
                    match e {
                        WebSocketError::IoError(err) => {
                            println!("WS Ping WebSocketError::IoError error: {:?} {}", err, err);
                            println!("WS Stoping ping loop");
                            remove_listener(&local_subscribers, client_ip);
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

pub fn handle_ping_message(incoming_message: Message, local_ws_sender: &SharedSender, client_ip: SocketAddr) -> bool {
    println!("WS Got PING from {}", client_ip);
    let message = Message::pong(incoming_message.payload);
    //sender.send_message(&message).unwrap();
    match local_ws_sender.lock().unwrap().deref_mut().send_message(&message) {
        Ok(_) => {
            println!("WS send pong ok");
            true
        },
        Err(e) => {
            println!("WS Error while sending pong {:?} {}", e, e);
            false
        }
    }
}

pub fn handle_close_message(local_ws_sender: &SharedSender, client_ip: SocketAddr) {
    let message = Message::close();
    println!("WS Client {} disconnected", client_ip);
    match local_ws_sender.lock() {
        Ok(mut res) => {
            match res.deref_mut().send_message(&message) {
                Ok(_) => { println!("WS send close ok") },
                Err(e) => {
                    println!("WS Error while sending close message {:?} {}", e, e)
                }
            }
        },
        Err(e) => {
            println!("WS Error while sending close message {:?} {}", e, e);
        }
    }
}


pub fn run_broadcast_broker(reciever: Receiver<RequestWrap>, broker_subscribers: Arc<Mutex<Vec<Subscriber>>>) {
    thread::spawn(move || {
        loop {
            let request = reciever.recv();

            match request {
                Ok(request) => {
                    println!("Got message {:?} from {:?}", request, request.client_ip);

                    let mut listeners_wrap = broker_subscribers.lock().unwrap();
                    let mut listeners = listeners_wrap.deref_mut();

                    let message_row = pretty_json(&request);
                    println!("MESSAGE: {}", message_row);
                    let message: Message = Message::text(message_row);
                    let path = extract_path(request.url.clone());

                    println!("Send message to {} listeners", listeners.len());

                    listeners.retain(|ref listerner| {
                        if extract_path(listerner.path.clone()) == path {
                            match listerner.sender.lock().unwrap().deref_mut().send_message(&message) {
                                Ok(status) => {
                                    let diff = request.time.elapsed();
                                    println!("WS Send time: {}.{:09}", diff.as_secs(), diff.subsec_nanos());
                                    println!("WS Broadcast to {} success: {:?}", listerner.ip, status);
                                    true
                                },
                                Err(e) => {
                                    println!("WS Broadcast to {} failed: {:?} {}", listerner.ip, e, e);
                                    match e {
                                        WebSocketError::IoError(err) => {
                                            println!("WS Broadcast WebSocketError::IoError error: {:?} {}", err, err);
                                            println!("WS Stoping broadcast loop");
                                            false
                                        },
                                        _ => {
                                            println!("WS Broadcast error: {:?} {}", e, e);
                                            false
                                        }
                                    }
                                }
                            }
                        } else {
                            true
                        }
                    });

                    println!("Broker: total listeners: {}", listeners.len());
                },
                Err(e) => {
                    println!("Broker: Recieve Error: {}", e);
                }
            }

        }
    });
}

pub fn remove_listener(ref mut subscribers_lock: &SubscribersLock, client_ip: SocketAddr) {
    println!("WS Remove {} from listeners", client_ip);
    let mut subscribers_wrap = subscribers_lock.lock().unwrap();
    let mut listeners = subscribers_wrap.deref_mut();

    let index = listeners.iter().position(|ref r| r.ip == client_ip );
    match index {
        Some(i) => {
            println!("WS Remove listener {}", i);
            listeners.remove(i);
        },
        None => {
            println!("WS Can not find listerner in a list");
        }
    }
}
