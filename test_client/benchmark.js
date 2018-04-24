// npm install websocket request

var WebSocket = require('ws');
var EventEmitter = require('events');
var request = require('request-promise-native');

var events = new EventEmitter();
var messages = [];
var websocket = new WebSocket("ws://127.0.0.1:3012/test_notif");

websocket.onmessage = (event) => {
  //console.log(event.data);
  messages.push(JSON.parse(event.data));
  events.emit('message');
};

websocket.addEventListener('close', e => {
  console.log('ws close');
});

websocket.onerror = (event) => {
  console.log('ws error');
};

var getMessage = () => {
  return new Promise((resolve, reject) => {
    var msg = messages.shift();
    if (msg) {
      resolve(msg);
    } else {
      events.once('message', () => {
        resolve(messages.shift());
      });
    }
  });
};

(async () => {
  await new Promise((resolve, reject) => {
    websocket.onopen = resolve;
  });

  var startTime = Date.now();
  var count = 1000;

  for (var i = 1; i <= count; i++) {
    var content = `I: ${i}`;
    await request({
      url: 'http://127.0.0.1:3012/test_notif',
      method: 'POST',
      body: content
    });
    var msg = await getMessage();
    if (msg.body != content) {
      throw new Error(`content mismatch: ${msg.body} != ${content}`);
    }
    if (i % 100 == 0 || i == count) {
      console.log(msg.body);
    }
  }

  var diff = Date.now() - startTime;
  console.log(`complete ${count} in ${(diff / 1000.0).toFixed(3)} sec. ${(count / diff * 1000.0).toFixed(3)} msg/sec`);

  websocket.close(1000, "bye bye");
  process.exit();
})().catch(error => {
  console.error(error);
});
