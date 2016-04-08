use iron::prelude::{IronResult, Request, Response, Set};
use iron::status;

use hbs::{Template};

pub fn index(req: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    resp.set_mut(Template::new("index", "Q: ".to_string())).set_mut(status::Ok);
    Ok(resp)
}

pub fn recieve_webhook(req: &mut Request) -> IronResult<Response> {
    println!("{:?}", req);
    let query = req.url.to_string();
    Ok(Response::with((status::Ok, "Q: ".to_string() + &query)))
}