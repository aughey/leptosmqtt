use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn mqtt_connect(
        hostname: &str,
        port: u16,
        client_id: &str,
        onDisconnect: &Closure<dyn FnMut()>,
        onMessage: &Closure<dyn FnMut(String, String)>,
    ) -> Result<JsValue, JsValue>;
    fn mqtt_close(id: u32);
    #[wasm_bindgen(catch)]
    async fn mqtt_subscribe(id: u32, topics: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(catch)]
    async fn mqtt_unsubscribe(id: u32, topics: &str) -> Result<(), JsValue>;
}

// Automatically drops the connection when it goes out of scope
struct MqttID(u32);
impl Drop for MqttID {
    fn drop(&mut self) {
        mqtt_close(self.0);
    }
}

pub struct MqttConnection {
    id: Rc<RefCell<Option<MqttID>>>,
    _on_disconnect: Closure<dyn FnMut()>,
    _on_message: Closure<dyn FnMut(String, String)>,
}
impl MqttConnection {
    pub async fn subscribe(&mut self, topic: &str) -> anyhow::Result<()> {
        let id = self.id.borrow();
        let id = id.as_ref().ok_or(anyhow::anyhow!("Disconnected"))?;

        mqtt_subscribe(id.0, topic)
            .await
            .map_err(|_| anyhow::anyhow!("Error in javascript subscribe"))
    }
}

async fn rs_mqtt_connect(
    hostname: &str,
    port: u16,
    client_id: &str,
    on_message: impl FnMut(String, String) + 'static,
    caller_on_disconnect: impl FnMut() + 'static,
) -> anyhow::Result<MqttConnection> {
    let on_message = Closure::new(on_message);

    // The connection id is an optional u32 where the optional state indicates connection
    let connect_id = Rc::new(RefCell::new(None));

    let mut caller_on_disconnect = Some(caller_on_disconnect);

    let on_disconnect = {
        let connect_id = connect_id.clone();
        Closure::new(move || {
            _ = connect_id.borrow_mut().take();
            caller_on_disconnect.take().unwrap()();
            log!("Disconnected");
        })
    };

    // Set our connection id
    let id = mqtt_connect(hostname, port, client_id, &on_disconnect, &on_message)
        .await
        .map_err(|_| anyhow::anyhow!("Failed to connect"))?;
    let id = id
        .as_string()
        .ok_or_else(|| anyhow::anyhow!("Could not get string from id is mqtt_connect"))?
        .parse::<u32>()?;

    *connect_id.borrow_mut() = Some(MqttID(id));

    Ok(MqttConnection {
        id: connect_id,
        _on_disconnect: on_disconnect,
        _on_message: on_message,
    })
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/leptos_start.css"/>
        <Script src="/assets/js/paho-mqtt.js" />
        <Script src="/assets/js/mqtt-bind.js" />
        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes>
                    <Route path="/" view=HomePage/>
                    <Route path="/*any" view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(0);
    let on_click = move |_| set_count.update(|count| *count += 1);

    let (value, set_value) = create_signal("NONE".to_string());

    // let params = use_params_map();
    // let id = params.with(|params| params.get("id").unwrap().to_owned());


    let connection = create_local_resource(
        || (),
        move |_| async move {
            let on_message = move |topic: String, message: String| {
                set_value(format!("{}: {}", topic, message));
                //                log!("Got message: {} on topic {}", message, topic);
            };
            // let (done_tx, done_rx) = futures::channel::oneshot::channel();
            // let mut done_tx = Some(done_tx);
            let on_disconnect = move || {
                log!("Disconnected");
                // done_tx.take().map(|tx| tx.send(()));
            };
            log!("STarting connect");
            let mut res =
                rs_mqtt_connect("localhost", 9002, "leptosclient", on_message, on_disconnect)
                    .await
                    .map_err(|e| {
                        log!("Got error: {:?}", e);
                        e
                    })?;
            res.subscribe("test").await?;
            log!("finished subscribing");

            // done_rx.await?;

            Ok::<MqttConnection,anyhow::Error>(res)
        },
    );

    let connected_msg = move||connection.with(|_| view! { <h1>"Connected"</h1> });

    view! {
        <h1>"Welcome to Leptos!"</h1>
        {value}
        {connected_msg}
        <button on:click=on_click>"Click Me: " {count}</button>
    }
}

/// 404 - Not Found
#[component]
fn NotFound() -> impl IntoView {
    // set an HTTP status code 404
    // this is feature gated because it can only be done during
    // initial server-side rendering
    // if you navigate to the 404 page subsequently, the status
    // code will not be set because there is not a new HTTP request
    // to the server
    #[cfg(feature = "ssr")]
    {
        // this can be done inline because it's synchronous
        // if it were async, we'd use a server function
        let resp = expect_context::<leptos_actix::ResponseOptions>();
        resp.set_status(actix_web::http::StatusCode::NOT_FOUND);
    }

    view! {
        <h1>"Not Found"</h1>
    }
}
