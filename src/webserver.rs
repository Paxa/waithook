use websocket::stream::WebSocketStream;

//use hyper::header;

use std::sync::mpsc::Sender;
use std::net::Shutdown;
use std::io::{self, Write, Read};
use std::fs::File;

use request_wrap::RequestWrap;

fn get_file_body(filepath: &str) -> Result<String, io::Error> {
    let filename = format!("public/{}", filepath.replace("..", ""));

    /*
    if filepath.starts_with("/") {
        return Err(io::Error::new(io::ErrorKind::Other, "За тобой уже выехали!"))
    }
    */

    println!("Reading {}", filename);
    let mut file = try!(File::open(filename));
    let mut body = String::new();
    try!(file.read_to_string(&mut body));

    Ok(body)
}

pub fn create_http_response(body: String, extra_headers: &str) -> String {
    if extra_headers == "" {
        format!("{}\r\n{}\r\n{}: {}\r\n\r\n{}",
            "HTTP/1.1 200 OK",
            "Connection: close",
            "Content-Length", body.len(),
            body
        )
    } else {
        format!("{}\r\n{}\r\n{}: {}\r\n{}\r\n\r\n{}",
            "HTTP/1.1 200 OK",
            "Connection: close",
            "Content-Length", body.len(),
            extra_headers,
            body
        )
    }
}

pub fn handle(request: RequestWrap, mut writter: WebSocketStream, sender: Sender<RequestWrap>) {
    println!("HTTP {} {}", request.method, request.url);

    let raw_response = if request.url == "/" {
        let body = match get_file_body("index.html") {
            Ok(b) => b,
            Err(e) => {
                format!("Read file error: {}", e)
            }
        };

        create_http_response(body, "Content-Type: text/html")
    } else if request.url.starts_with("/@/") {
        let (_, filename) = request.url.split_at(3);
        let body = match get_file_body(filename) {
            Ok(b) => b,
            Err(e) => {
                format!("Read file error: {}", e)
            }
        };

        let mut content_type = "text/plain";

        if filename.ends_with(".css") {
            content_type = "text/css"
        } else if filename.ends_with(".js") {
            content_type = "application/javascript; charset=utf-8"
        } else if filename.ends_with(".html") {
            content_type = "text/html"
        } else if filename.ends_with(".ico") {
            content_type = "image/x-icon"
        }

        create_http_response(body, &format!("Content-Type: {}", content_type))
    } else {
        match sender.send(request) {
            Ok(_) => {},
            Err(e) => { println!("HTTP Channel send error: {}", e); }
        }

        create_http_response("OK\n".to_string(), "Content-Type: text/plain")
    };

    match writter.write(raw_response.as_bytes()) {
        Ok(_) => {},
        Err(e) => { println!("HTTP Socket write error: {}", e); }
    }

    writter.flush().unwrap();
    writter.shutdown(Shutdown::Both).unwrap();
}
