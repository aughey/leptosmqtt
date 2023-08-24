// Provide a simplistic C like interface to mqtt

// list of created clients, caller manages lifetime
const clients = {};
let next_id = 0;

function mqtt_connect(hostname, port, client_id, onDisconnect, onMessage) {
    var client = new Paho.Client(hostname, Number(port), "/mqtt", client_id);

    let id = next_id++;
    id = id.toString();
    clients[id] = client;

    return new Promise((resolve, reject) => {
        client.onConnectionLost = onDisconnect;

        const onMessageArrived = (message) => {
            onMessage(message.destinationName, message.payloadString);
        };
        client.onMessageArrived = onMessageArrived;

        client.connect({
            onSuccess: () => {
                resolve(id);
            }, onFailure: () => {
                reject();
            }, reconnect: false
        });
    });
}

function mqtt_subscribe(id, topic) {
    id = id.toString();
    let client = clients[id];

    return new Promise((resolve, reject) => {
        console.log("subscribing to topic: " + topic);
        client.subscribe(topic, {
            onSuccess: () => {
                console.log("js: subscribed to topic: " + topic);
                resolve();
            },
            onFailure: () => {
                console.log("js: failed to subscribe to topic: " + topic);
                reject();
            }
        });
    });
}

function mqtt_unsubscribe(id, topic) {
    id = Number(id);
    let client = clients[id];

    return new Promise((resolve, reject) => {
        console.log("unsubscribing from topic: " + topic);
        client.subscribe(topic, {
            onSuccess: () => {
                console.log("js: unsubscribed from topic: " + topic);
                resolve();
            },
            onFailure: () => {
                console.log("js: failed to unsubscribe from topic: " + topic);
                reject();
            }
        });
    });

}

function mqtt_unsubscribe(id, topic, onSuccess) {
    id = Number(id);
    let client = clients[id];
    client.unsubscribe(topic, {
        onSuccess: () => {
            onSuccess(true);
        }, onFailure: () => {
            onSuccess(false);
        }
    });


    console.log("Unsubscribed from topic: " + topic);
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