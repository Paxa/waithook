use std::io::Read;
use std::net::TcpStream;
use std::time::Instant;

use hyper::header;
use hyper::http::h1::parse_request;
use hyper::http::h1::Incoming;
use hyper::buffer::BufReader;
use hyper::http::h1::HttpReader::{SizedReader, ChunkedReader, EmptyReader};

use websocket::server::InvalidConnection;
use websocket::server::upgrade::sync::Buffer;
use websocket::server::sync::AcceptResult;

use request_wrap::RequestWrap;

pub fn create_request_wrap(connection_res : AcceptResult<TcpStream>) -> Result<(TcpStream, RequestWrap), &'static str> {
    let connection : InvalidConnection<TcpStream, Buffer> = match connection_res {
        Ok(_) => {
            println!("Should not have happen");
            return Err("Should not have happen");
        },
        Err(e) => { e }
    };

    let tcp_stream = connection.stream.unwrap();
    let client_ip = tcp_stream.peer_addr().unwrap().clone();

    let buffer = connection.buffer.unwrap();
    let mut last = 0;
    for (i, &x) in buffer.buf.iter().enumerate() {
        if x == 0u8 && buffer.buf[i + 1] == 0u8 {
            last = i;
            break;
        }
    }

    let mut reader = BufReader::from_parts(tcp_stream.try_clone().unwrap(), buffer.buf, 0, last);

    let Incoming { version, subject: (http_method, request_uri), headers } = parse_request(&mut reader).unwrap();

    let mut body_reader = if headers.has::<header::ContentLength>() {
        match headers.get::<header::ContentLength>() {
            Some(&header::ContentLength(len)) => SizedReader(reader, len),
            None => unreachable!()
        }
    } else if headers.has::<header::TransferEncoding>() {
        //todo!("check for Transfer-Encoding: chunked");
        ChunkedReader(reader, None)
    } else {
        EmptyReader(reader)
    };
    let mut body = String::new();
    body_reader.read_to_string(&mut body).unwrap();

    //println!("{:?} {} {:?} {:?}", http_method, request_uri, headers, body);

    let web_request = RequestWrap {
        method: http_method.as_ref().to_string(),
        url: request_uri.to_string(),
        headers: headers.clone(),
        body: body,
        client_ip: client_ip,
        time: Instant::now()
    };

    Ok((tcp_stream, web_request))
}