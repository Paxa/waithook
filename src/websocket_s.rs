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


pub struct Channels {
    pub senders : Vec<Sender<RequestWrap>>
    //pub recievers : Ven<Receiver<RequestWrap>>
}

/*
impl Channels {
    pub fn new() -> Channels {
        Channels {
            senders: Vec::new()
        }
    }

    pub fn subscribe(&mut self) -> Receiver<RequestWrap> {
        let (sender, reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();
        self.senders.push(sender);
        reciever
    }

    pub fn publish(&mut self, request : RequestWrap) {
        for sender in &self.senders {
            sender.send(request);
        }
    }
}
*/

pub fn run_server() {
    // Start listening for WebSocket connections
    let ws_server = Server::bind("127.0.0.1:3012").unwrap();
    let addr = SocketAddr::from_str("127.0.0.1:3012").unwrap();

    /*
    let mut pipes = HashMap::new();
    let shared_pipes = Arc::new(pipes);
    */

    //let (sender, reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();
    //let mut listeners: Vec<SenderBox> = Vec::new();

    /*
    thread::spawn(move || {
        for hook in reciever.recv() {
            let message: Message = Message::text(hook.body);
            for listener in listeners.iter() {
                listener.send_message(message).unwrap();
            }
        }
    });
    */

    //let channels = Channels::new();
    //let shared_channels = Arc::new(channels);
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

            /*
            let mut channels = match shared_pipes.get(&path) {
                Some(channels) => {
                    channels
                },
                None => {
                    let channels = Channels::new();
                    pipes.insert(path, channels);
                    &channels
                }
            };
            */

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

                /*
                for message in receiver.incoming_messages() {
                    let message: Message = message.unwrap();

                    match message.opcode {
                        Type::Close => {
                            let message = Message::close();
                            sender.send_message(&message).unwrap();
                            println!("Client {} disconnected", ip);
                            return;
                        },
                        Type::Ping => {
                            let message = Message::pong(message.payload);
                            sender.send_message(&message).unwrap();
                        }
                        _ => sender.send_message(&message).unwrap(),
                    }
                }
                */
            }
        });
    }
}
