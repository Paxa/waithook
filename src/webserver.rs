use websocket::stream::WebSocketStream;

//use hyper::header;

use std::sync::mpsc::Sender;
use std::net::Shutdown;
use std::io::{self, Write, Read};
use std::fs::File;

use request_wrap::RequestWrap;

fn get_index_body() -> Result<String, io::Error> {
    let mut file = try!(File::open("views/index.html"));
    let mut body = String::new();
    try!(file.read_to_string(&mut body));

    Ok(body)
}

pub fn handle(request: RequestWrap, mut writter: WebSocketStream, sender: Sender<RequestWrap>) {
    println!("HTTP {} {}", request.method, request.url);

    let raw_response = if request.url == "/" {
        let body = match get_index_body() {
            Ok(b) => b,
            Err(e) => {
                format!("Read file error: {}", e)
            }
        };

        format!("{}\r\n{}: {}\r\n{}\r\n\r\n{}",
            "HTTP/1.1 200 OK",
            "Content-Length", body.len(),
            "Content-Type: text/html",
            body
        )
    } else {
        match sender.send(request) {
            Ok(_) => {},
            Err(e) => { println!("HTTP Channel send error: {}", e); }
        }

        let body = "OK\n";

        format!("{}\r\n{}: {}\r\n\r\n{}",
            "HTTP/1.1 200 OK",
            "Content-Length", body.len(),
            body
        )
    };

    //println!("Sending:\n{}", response);
    match writter.write(raw_response.as_bytes()) {
        Ok(_) => {},
        Err(e) => { println!("HTTP Socket write error: {}", e); }
    }

    writter.flush().unwrap();
    writter.shutdown(Shutdown::Both).unwrap();
}