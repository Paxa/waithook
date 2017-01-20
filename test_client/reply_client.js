#!/usr/bin/env node

var WebSocketClient = require('websocket').client;
var request = require('request');

if (process.argv[2] === undefined || process.argv[3] === undefined) {
  throw "required 2 command line arguments";
}

var LISTEN_TO = process.argv[2];

var SEND_TO = process.argv[3];

if (!SEND_TO.match(/^https?:\/\//)) {
  SEND_TO = "http://" + SEND_TO;
}

console.log("Subscribing for 'ws://waithook.com/%s", LISTEN_TO);
console.log("Forward to ", SEND_TO);

var client = new WebSocketClient();

client.on('connectFailed', function(error) {
    console.log('Connect Error: ' + error.toString());
});
 
client.on('connect', function(connection) {
    console.log('WebSocket Client Connected');
    connection.on('error', function(error) {
        console.log("Connection Error: " + error.toString());
    });
    connection.on('close', function() {
        console.log('echo-protocol Connection Closed');
    });
    connection.on('message', function(message) {
        if (message.type === 'utf8') {
            console.log("Received: '" + message.utf8Data + "'");
            if (message.utf8Data[0] == "{") {
              var data = JSON.parse(message.utf8Data);

              var url = SEND_TO;
              if (data.url.includes('?')) {
                url += data.url.substring(data.url.indexOf('?'), data.url.length - 1);
              }

              request({
                method: data.method,
                url: url,
                headers: data.headers,
                body: data.body
              }, function (error, response, body) {
                if (error) {
                  console.error("HTTP Error: ", error);
                } else {
                  console.log("HTTP Server Response: %s\n%s", response.statusCode, body);
                }
              });
            }
        }
    });
});

//client.connect('ws://localhost:3012/' + LISTEN_TO);
client.connect('ws://waithook.com/' + LISTEN_TO);

