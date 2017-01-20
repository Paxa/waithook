# Waithook - HTTP to web-socket transmitter

An excelent tool to debug incoming HTTP notifications. Let's say your application need to receive HTTP request from 3rd parties.
It can payment provider or incoming email processing or GitHub push notifications, etc.
Developing it on localhost can be hard, using SSH tunnels always gave me hard times.

## Client Libraries

* https://github.com/Paxa/waithook-ruby

## How it Works

1. Open web-socket connection to http://waithook.com/something (choose your namespace)
2. Send HTTP request to http://waithook.com/something
3. Waithook will forward it as json to all listening web-socket connections

Example of message send to websocket:
```json
{
  "method": "POST",
  "url": "/testing_346?foo=bar",
  "headers": {
    "X-Request-Start": "1478935535042",
    "X-Forwarded-For": "182.253.140.80",
    "Total-Route-Time": "0",
    "Accept": "*/*",
    "Connection": "close",
    "Via": "1.1 vegur",
    "Content-Type": "application/json",
    "Content-Length": "69",
    "Connect-Time": "1",
    "Origin": "https://waithook.com",
    "Referer": "https://waithook.com/",
    "Host": "waithook.herokuapp.com",
    "X-Request-Id": "5d12e9b2-31d1-47b1-abc9-f9bca03dc4bf",
    "X-Forwarded-Proto": "https",
    "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/54.0.2840.59 Safari/537.36",
    "Accept-Language": "en-US,en;q=0.8",
    "X-Forwarded-Port": "443",
    "Accept-Encoding": "gzip, deflate, br"
  },
  "body": "{\n  \"type\": \"Testing Request\",\n  \"time\": \"2016-11-12T07:25:34.280Z\"\n}"
}
```

## Development

```
cargo run
```

[![Deploy](https://www.herokucdn.com/deploy/button.svg)](https://heroku.com/deploy)
