// https://github.com/joewalnes/reconnecting-websocket
!function(a,b){"function"==typeof define&&define.amd?define([],b):"undefined"!=typeof module&&module.exports?module.exports=b():a.ReconnectingWebSocket=b()}(this,function(){function a(b,c,d){function l(a,b){var c=document.createEvent("CustomEvent");return c.initCustomEvent(a,!1,!1,b),c}var e={debug:!1,automaticOpen:!0,reconnectInterval:1e3,maxReconnectInterval:3e4,reconnectDecay:1.5,timeoutInterval:2e3};d||(d={});for(var f in e)this[f]="undefined"!=typeof d[f]?d[f]:e[f];this.url=b,this.reconnectAttempts=0,this.readyState=WebSocket.CONNECTING,this.protocol=null;var h,g=this,i=!1,j=!1,k=document.createElement("div");k.addEventListener("open",function(a){g.onopen(a)}),k.addEventListener("close",function(a){g.onclose(a)}),k.addEventListener("connecting",function(a){g.onconnecting(a)}),k.addEventListener("message",function(a){g.onmessage(a)}),k.addEventListener("error",function(a){g.onerror(a)}),this.addEventListener=k.addEventListener.bind(k),this.removeEventListener=k.removeEventListener.bind(k),this.dispatchEvent=k.dispatchEvent.bind(k),this.open=function(b){h=new WebSocket(g.url,c||[]),b||k.dispatchEvent(l("connecting")),(g.debug||a.debugAll)&&console.debug("ReconnectingWebSocket","attempt-connect",g.url);var d=h,e=setTimeout(function(){(g.debug||a.debugAll)&&console.debug("ReconnectingWebSocket","connection-timeout",g.url),j=!0,d.close(),j=!1},g.timeoutInterval);h.onopen=function(){clearTimeout(e),(g.debug||a.debugAll)&&console.debug("ReconnectingWebSocket","onopen",g.url),g.protocol=h.protocol,g.readyState=WebSocket.OPEN,g.reconnectAttempts=0;var d=l("open");d.isReconnect=b,b=!1,k.dispatchEvent(d)},h.onclose=function(c){if(clearTimeout(e),h=null,i)g.readyState=WebSocket.CLOSED,k.dispatchEvent(l("close"));else{g.readyState=WebSocket.CONNECTING;var d=l("connecting");d.code=c.code,d.reason=c.reason,d.wasClean=c.wasClean,k.dispatchEvent(d),b||j||((g.debug||a.debugAll)&&console.debug("ReconnectingWebSocket","onclose",g.url),k.dispatchEvent(l("close")));var e=g.reconnectInterval*Math.pow(g.reconnectDecay,g.reconnectAttempts);setTimeout(function(){g.reconnectAttempts++,g.open(!0)},e>g.maxReconnectInterval?g.maxReconnectInterval:e)}},h.onmessage=function(b){(g.debug||a.debugAll)&&console.debug("ReconnectingWebSocket","onmessage",g.url,b.data);var c=l("message");c.data=b.data,k.dispatchEvent(c)},h.onerror=function(b){(g.debug||a.debugAll)&&console.debug("ReconnectingWebSocket","onerror",g.url,b),k.dispatchEvent(l("error"))}},1==this.automaticOpen&&this.open(!1),this.send=function(b){if(h)return(g.debug||a.debugAll)&&console.debug("ReconnectingWebSocket","send",g.url,b),h.send(b);throw"INVALID_STATE_ERR : Pausing to reconnect websocket"},this.close=function(a,b){"undefined"==typeof a&&(a=1e3),i=!0,h&&h.close(a,b)},this.refresh=function(){h&&h.close()}}return a.prototype.onopen=function(){},a.prototype.onclose=function(){},a.prototype.onconnecting=function(){},a.prototype.onmessage=function(){},a.prototype.onerror=function(){},a.debugAll=!1,a.CONNECTING=WebSocket.CONNECTING,a.OPEN=WebSocket.OPEN,a.CLOSING=WebSocket.CLOSING,a.CLOSED=WebSocket.CLOSED,a});

//

var $ = function (selector) { return document.querySelector(selector); };

var websocket = null;

function subscribe() {
  $('#subscribe_start').value = "Connecting...";
  $('#subscribe_start').disabled = true;
  var path = $('#subscribe_path').value;
  $('#subscribe_path').disabled = true;

  var wsProtocol = window.location.protocol == "http:" ? "ws:" : "wss:";
  var wsURL = wsProtocol + "//" + window.location.host + "/" + path;
  websocket = new ReconnectingWebSocket(wsURL);

  var newUrl = "/?" + buildQuery({
    path: path,
    local_url: $('#local_url').value || null,
    forward_url: $('#forward_url').value || null
  });

  window.history.replaceState(history.state, '', newUrl);

  console.log('Socket Status: ' + websocket.readyState);

  websocket.onmessage = function (event) {
    console.log("Socket Message: " + event.data);
    addMessageToLog(event.data, {hightlight: true});

    var localUrl = $('#local_url').value;
    if (localUrl && localUrl != '') {
      sendXhrToLocal(localUrl, event);
    }

  };

  websocket.onerror = function (event) {
    console.log("Socket Error: ", event);
    addMessageToLog("Websocket Error: " + JSON.stringify(event));
  };

  websocket.onclose = function (event) {
    console.log("Socket Closed: ", event);
    addMessageToLog("Websocket Closed: " + JSON.stringify({code: event.code}));
    $('#subscribe_start').value = "Subscribe";
    $('#subscribe_start').disabled = false;
  };

  websocket.onopen = function () {
    console.log('Socket Status: ' + websocket.readyState + ' (open)');
    //websocket.send("Hello Server");
    $('#subscribe_start').value = "Stop";
    $('#subscribe_start').disabled = false;
    $('#subscribe_simulate').disabled = false;
    $('#subscribe_simulate').focus();
    /*
    $('#sample_link').href = "/" + path + "?query_args=123";
    $('#sample_link').removeAttribute('disabled');
    */
    var targetUrl = window.location.origin + "/" + path;
    if ($('#forward_url').value) {
      targetUrl += "?" + buildQuery({forward: $('#forward_url').value});
    }
    addMessageToLog("Subscribed to " + wsURL +
      "\nYou may send request to: " +
      "<strong><a href='" + targetUrl + "' target='_blank'>" + targetUrl + "</a></strong>",
      {hightlight: false, htmlSafe: true});
  };

  // Load saved events
  if (window.fetch) {
    fetch("/@/history?path=" + path).then(function (res) {
      res.json().then(function (parsed) {
        console.log('res', parsed);
        parsed.forEach(function (message) {
          var date = new Date(message.created_at);
          var data = {
            method: message.method,
            url: message.url,
            headers: message.headers
          };
          if (message.body) {
            data.body = message.body;
          }
          addMessageToLog(JSON.stringify(data, null, 2), {
            hightlight: true,
            htmlSafe: false,
            date: date,
            title: "(Previously Saved)"
          });
        });
      });
    });
  }
}

function unsubscribe() {
  websocket.close();
  $('#subscribe_start').value = "Stopping...";
  $('#subscribe_start').disabled = true;
}

function sendXhrToLocal(localUrl, event) {
  var parsed = JSON.parse(event.data);
  addMessageToLog("Sending " + parsed.method + " request to: " + localUrl);

  var xhr = new XMLHttpRequest();
  xhr.open(parsed.method, localUrl, true);
  var skipHeaders = [
    "host", "connection", "origin", "referer", "cookie", "user-agent", "accept-encoding", "content-length"
  ];
  Object.keys(parsed.headers).forEach(function (key) {
    if (skipHeaders.indexOf(key.toLowerCase()) == -1) {
      xhr.setRequestHeader(key, parsed.headers[key]);
    }
  });
  xhr.onreadystatechange = function() {
    if (xhr.readyState == 4) {
      if (xhr.status == 0) {
        addMessageToLog(
          "Can not send " + parsed.method + " request to " + localUrl + ". " +
          "Missing <a href='https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/"+
          "Access-Control-Allow-Origin' target='_blank'>CORS headers<a>",
          {hightlight: false, htmlSafe: true});
      } else {
        addMessageToLog(
          "Reponse from " + localUrl + " -> " + xhr.status + " " +
          xhr.statusText + " (" + xhr.responseText.length + " bytes.)");
      }
    }
  };
  xhr.send(parsed.body);
}

function padNum(a, b) {
  return (1e15 + a + "").slice(-b);
}

function addMessageToLog(data, options) {
  if (!options) options = {};

  var line = document.createElement("DIV");
  line.className = "in";
  if (options.htmlSafe) {
    line.innerHTML = data;
  } else {
    line.innerText = data;
  }
  if (options.hightlight) {
    try {
      hljs.highlightBlock(line);
    } catch (e) {
      setTimeout(function () {
        throw e;
      }, 10);
    }
  }
  var date = options.date || new Date();
  line.title = "-> @ " + [
    padNum(date.getHours(), 2),
    padNum(date.getMinutes(), 2),
    padNum(date.getSeconds(), 2)
  ].join(":");
  if (options.title) {
    line.title += " " + options.title;
  }
  $('#log').appendChild(line);
}

function parseQuery(qstr) {
  qstr = qstr.replace(/^\?/, '');
  var query = {};
  var a = (qstr[0] === '?' ? qstr.substr(1) : qstr).split('&');
  for (var i = 0; i < a.length; i++) {
    var b = a[i].split('=');
    query[decodeURIComponent(b[0])] = decodeURIComponent(b[1] || '');
  }
  return query;
}

function buildQuery(params) {
  var esc = encodeURIComponent;
  return Object.keys(params)
    .map(function (k) {
      if (params[k] !== null) {
        return esc(k) + '=' + esc(params[k]);
      }
    })
    .filter(function (pair) { return pair !== null && pair !== undefined; })
    .join('&');
}

$('#subscribe_start').addEventListener('click', function () {
  if ($('#subscribe_start').value == 'Stop') {
    unsubscribe();
  } else {
    subscribe();
  }
}, false);

$('#subscribe_simulate').addEventListener('click', function () {
  $('#subscribe_simulate').disabled = true;

  var xhr = new XMLHttpRequest();
  xhr.onreadystatechange = function() {
    if (xhr.readyState == 4 && xhr.status == 200) {
      $('#subscribe_simulate').disabled = false;
    }
  };

  var reqUrl = '/' + $('#subscribe_path').value + "?" + buildQuery({foo: 'bar', forward: $('#forward_url').value || null});

  xhr.open("PATCH", reqUrl, true);
  xhr.setRequestHeader("Content-type", "application/json");
  xhr.send(JSON.stringify({
    type: "Testing Request",
    time: new Date()
  }, null, 2));
}, false);

$('#send_to_localhost').addEventListener('change', function (event) {
  $('.local-url-form').style.display = event.target.checked ? 'block' : 'none';
});

$('#do_forward').addEventListener('change', function (event) {
  $('.forward-to-form').style.display = event.target.checked ? 'block' : 'none';
});

var onPageLoad = function () {
  if (window.location) {
    var params = parseQuery(window.location.search);

    if (params.local_url && params.local_url != '') {
      $('#local_url').value = params.local_url;
      $('#send_to_localhost').checked = true;
      $('.local-url-form').style.display = 'block';
    }

    if (params.forward_url && params.forward_url != '') {
      $('#forward_url').value = params.forward_url;
      $('#do_forward').checked = true;
      $('.forward-to-form').style.display = 'block';
    }

    $('#subscribe_path').value = params.path || "testing_" + Math.round(Math.random() * 1000);
    if (params.path) {
      subscribe();
    }
  }
};