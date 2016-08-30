var $ = function (selector) { return document.querySelector(selector); };

function subscribe() {
  var path = $('#subscribe_path').value;
  $('#subscribe_path').disabled = true;

  var wsProtocol = window.location.protocol == "http:" ? "ws:" : "wss:";
  var wsURL = wsProtocol + "//" + window.location.host + "/" + path;
  var websocket = new WebSocket(wsURL);

  console.log('Socket Status: ' + websocket.readyState);

  websocket.onmessage = function (event) {
    console.log("Socket Message: " + event.data);
    addMessageToLog(event.data, true);
  };

  websocket.onopen = function () {
    console.log('Socket Status: ' + websocket.readyState + ' (open)');
    //websocket.send("Hello Server");
    $('#subscribe_simulate').disabled = false;
    $('#subscribe_simulate').focus();
    $('#sample_link').href = "/" + path + "?query_args=123";
    $('#sample_link').removeAttribute('disabled');
    addMessageToLog("Subscribed to " + wsURL);
  };
}

function padNum(a, b) {
  return (1e15 + a + "").slice(-b);
}

function addMessageToLog(data, hightlight) {
  var line = document.createElement("DIV");
  line.className = "in";
  line.innerText = data;
  if (hightlight) {
    try {
      hljs.highlightBlock(line);
    } catch (e) {
      setTimeout(function () {
        throw e;
      }, 10);
    }
  }
  var date = new Date();
  line.title = "-> @ " + [padNum(date.getHours(), 2), padNum(date.getMinutes(), 2), padNum(date.getSeconds(), 2)].join(":");
  $('#log').appendChild(line);
}

$('#subscribe_start').addEventListener('click', function () {
  subscribe();
  $('#subscribe_start').disabled = true;
}, false);

$('#subscribe_simulate').addEventListener('click', function () {
  $('#subscribe_simulate').disabled = true;

  var xhr = new XMLHttpRequest();
  xhr.onreadystatechange = function() {
    if (xhr.readyState == 4 && xhr.status == 200) {
      $('#subscribe_simulate').disabled = false;
    }
  };

  xhr.open("POST", "/" + $('#subscribe_path').value + "?foo=bar", true);
  xhr.setRequestHeader("Content-type", "application/json");
  xhr.send(JSON.stringify({
    type: "Testing Request",
    time: new Date()
  }, null, 2));
}, false);

$('#subscribe_path').value = "testing_" + Math.round(Math.random() * 1000);
