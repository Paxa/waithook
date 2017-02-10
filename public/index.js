var $ = function (selector) { return document.querySelector(selector); };

function subscribe() {
  var path = $('#subscribe_path').value;
  $('#subscribe_path').disabled = true;

  var wsProtocol = window.location.protocol == "http:" ? "ws:" : "wss:";
  var wsURL = wsProtocol + "//" + window.location.host + "/" + path;
  var websocket = new WebSocket(wsURL);

  var newUrl = "/?" + buildQuery({
    path: path,
    local_url: $('#local_url').value || null,
    forward_url: $('#forward_url').value || null
  });

  window.history.replaceState(history.state, '', newUrl);

  console.log('Socket Status: ' + websocket.readyState);

  websocket.onmessage = function (event) {
    console.log("Socket Message: " + event.data);
    addMessageToLog(event.data, true);

    var localUrl = $('#local_url').value;
    if (localUrl && localUrl != '') {
      var parsed = JSON.parse(event.data);
      addMessageToLog("Sending " + parsed.method + " request to: " + localUrl);

      var xhr = new XMLHttpRequest();
      var updatedUrl = localUrl.replace('localhost', 'local.waithook.com').replace('127.0.0.1', 'local.waithook.com');
      xhr.open(parsed.method, updatedUrl, true);
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
          addMessageToLog("Reponse from " + localUrl + " -> " + xhr.status + " " + xhr.statusText + " (" + xhr.responseText.length + " bytes.)");
        }
      };
      xhr.send(parsed.body);
      $xhr = xhr;
    }

  };

  websocket.onerror = function (event) {
    console.log("Socket Error: ", event);
    addMessageToLog("Websocket Error: " + JSON.stringify(event));
  };

  websocket.onclose = function (event) {
    console.log("Socket Closed: ", event);
    addMessageToLog("Websocket Closed: " + JSON.stringify({code: event.code}));
  };

  websocket.onopen = function () {
    console.log('Socket Status: ' + websocket.readyState + ' (open)');
    //websocket.send("Hello Server");
    $('#subscribe_simulate').disabled = false;
    $('#subscribe_simulate').focus();
    /*
    $('#sample_link').href = "/" + path + "?query_args=123";
    $('#sample_link').removeAttribute('disabled');
    */
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
  // to make page laod faster
  setTimeout(function () {
    document.querySelector('.github-btn').src = 'https://ghbtns.com/github-btn.html?user=paxa&repo=waithook&type=star&count=true';
  }, 200);

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
      $('#subscribe_start').disabled = true;
      subscribe();
    }
  }
};