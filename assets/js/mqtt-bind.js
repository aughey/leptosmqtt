// Provide a simplistic C like interface to mqtt

// list of created clients, caller manages lifetime
const clients = {};
let next_id = 0;

function mqtt_connect(hostname, port, client_id, onConnect, onFail, onDisconnect, onMessage) {
    var client = new Paho.Client(hostname, Number(port), "/mqtt", client_id);
    client.onConnectionLost = onDisconnect;

    const onMessageArrived = (message) => {
        console.log("Got message");
        console.log(message);
        onMessage(message.destinationName, message.payloadString);
    };
    client.onMessageArrived = onMessageArrived;

    client.connect({ onSuccess: onConnect, onFailure: onFail, reconnect: false});
    let id = next_id++;
    clients[id] = client;
    return id;
}

function mqtt_subscribe(id, topic) {
    id = Number(id);
    let client = clients[id];
    client.subscribe(topic);
    console.log("Subscribed to topic: " + topic);
}

function mqtt_close(id) {
    id = Number(id);
    let client = clients[id];
    delete clients[id];
    console.log("Closing client");
    try {
        client.disconnect();
    } catch (e) {
    }
}