use websocket::stream::WebSocketStream;

//use hyper::header;

use std::net::{SocketAddr, Shutdown, TcpStream};
use std::io::Write;
use std::io::Read;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::Arc;

use websocket_s;
use request_wrap::RequestWrap;

pub fn handle(request: RequestWrap, mut writter: WebSocketStream, mut sender: Sender<RequestWrap>) {
    println!("{} {}", request.method, request.url);

    //println!("Request from {}", writter.peer_addr().unwrap());

    //channels.publish(request);
    sender.send(request);

    let body = "OK\n";

    let response = format!("{}\r\n{}: {}\r\n\r\n{}",
        "HTTP/1.1 200 OK",
        "Content-Length", body.len(),
        body
    );

    println!("Sending:\n{}", response);
    writter.write(response.as_bytes());
    //try!(write!(writter, "{}", response));
    writter.flush();
    writter.shutdown(Shutdown::Both);
}