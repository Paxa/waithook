use std::thread;

use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use request_wrap::RequestWrap;

use url::Url;
use hyper::client::Client;
use hyper::net::HttpsConnector;
use hyper::header;
use hyper_native_tls::NativeTlsClient;

pub fn run_forwarder() -> Sender<RequestWrap> {
    // 5 threads to process forward requests
    let (forward_sender, forward_reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();
    let mut workers : Vec<Sender<RequestWrap>> = Vec::new();
    let mut current_worker = 0;

    for _ in 0..5 {
        let (worker_sender, worker_reciever): (Sender<RequestWrap>, Receiver<RequestWrap>) = mpsc::channel();
        workers.push(worker_sender);
        thread::spawn(move || {
            forwarder_processing(worker_reciever);
        });
    }

    thread::spawn(move || {
        loop {
            let request = forward_reciever.recv();

            match request {
                Ok(request) => {
                    current_worker = (current_worker + 1) % workers.len();
                    println!("FORWARD: sending to worker {}", current_worker);
                    match workers[current_worker].send(request) {
                        Ok(_) => { }
                        Err(e) => {
                            println!("FORWARD: Error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("FORWARD: Recieve Error: {}", e);
                }
            }
        }
    });

    return forward_sender;
}

fn forwarder_processing(reciever: Receiver<RequestWrap>) {
    loop {
        let request = reciever.recv();

        match request {
            Ok(request) => {
                match Url::parse(&format!("http://example.com{}", &request.url)) {
                    Ok(url) => {
                        let forward_arg = url.query_pairs().find(|ref x| x.0 == "forward" );
                        match forward_arg {
                            Some(value) => {
                                forward_request(request, value.1.into_owned());
                            },
                            None => {
                                println!("FORWARD: No 'forward' argument {}", request.url);
                            }
                        }
                    },
                    Err(e) => {
                        println!("FORWARD: Parse Query Error: {}", e);
                    }
                };

                //println!("FOWRWARD: Got message {:?} from {:?}", request, request.client_ip);
            }
            Err(e) => {
                println!("FORWARD: Recieve Error: {}", e);
            }
        }
    }
}

fn forward_request(request: RequestWrap, target: String) {
    println!("FORWARD: Sending {} request to {:?}", request.method, target);

    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let mut client = Client::with_connector(connector);

    client.set_read_timeout(Some(Duration::new(10, 0)));
    client.set_write_timeout(Some(Duration::new(10, 0)));

    let mut headers = request.headers.clone();
    headers.remove::<header::Host>();

    let req = match request.method.as_ref() {
        "GET" => {
            client.get(&target).headers(headers)
        },
        "HEAD" => {
            client.head(&target).headers(headers).body(&request.body)
        },
        "POST" => {
            client.post(&target).headers(headers).body(&request.body)
        },
        "PATCH" => {
            client.patch(&target).headers(headers).body(&request.body)
        },
        "PUT" => {
            client.put(&target).headers(headers).body(&request.body)
        },
        "DELETE" => {
            client.delete(&target).headers(headers).body(&request.body)
        },
        _ => {
            client.post(&target).headers(headers).body(&request.body)
        }
    };

    match req.send() {
        Ok(res) => {
            println!("FORWARD: Response {:?}", res);
        },
        Err(e) => {
            println!("FORWARD: Error {:?}", e);
        }
    }
}