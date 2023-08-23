use std::{cell::RefCell, rc::Rc};

use anyhow::Ok;
use futures::StreamExt;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use wasm_bindgen::prelude::*;

// Import the `window.alert` function from the Web.
#[wasm_bindgen] //(module = "/js/mqtt-bind.js")]
extern "C" {
    fn mqtt_connect(
        hostname: &str,
        port: u16,
        client_id: &str,
        onConnect: &Closure<dyn FnMut()>,
        onFail: &Closure<dyn FnMut()>,
        onDisconnect: &Closure<dyn FnMut()>,
        onMessage: &Closure<dyn FnMut(String, String)>,
    ) -> u32;
    fn mqtt_close(id: u32);
    fn mqtt_subscribe(id: u32, topics: &str);
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
    msgrx: futures::channel::mpsc::Receiver<(String, String)>,
    _on_disconnect: Closure<dyn FnMut()>,
    _on_message: Closure<dyn FnMut(String, String)>,
}
impl MqttConnection {
    /// Waits for a message to arrive.  Returns the topic and message.
    pub async fn next(&mut self) -> anyhow::Result<(String, String)> {
        self.msgrx
            .next()
            .await
            .ok_or(anyhow::anyhow!("Disconnected"))
    }
    pub fn subscribe(&mut self, topic: &str) -> anyhow::Result<()> {
        let id = self.id.borrow();
        let id = id.as_ref().ok_or(anyhow::anyhow!("Disconnected"))?;

        mqtt_subscribe(id.0, topic);
        Ok(())
    }
}

async fn rs_mqtt_connect(
    hostname: &str,
    port: u16,
    client_id: &str,
) -> anyhow::Result<MqttConnection> {
    let (tx, rx) = futures::channel::oneshot::channel();
    let tx = Rc::new(RefCell::new(Some(tx)));
    let on_connect = {
        let tx = tx.clone();
        Closure::new(move || {
            log!("got connect callback");
            tx.borrow_mut().take().unwrap().send(true).unwrap();
        })
    };
    let on_fail = Closure::new(move || {
        log!("Failed to connect");
        tx.borrow_mut().take().unwrap().send(false).unwrap();
    });

    let (msgtx, msgrx) = futures::channel::mpsc::channel(16);
    let msgtx = Rc::new(RefCell::new(msgtx));
    let on_message = Closure::new(move |topic: String, message: String| {
        log!("got message callback");
        msgtx.borrow_mut().try_send((topic, message)).unwrap();
        ()
    });

    // The connection id is an optional u32 where the optional state indicates connection
    let connect_id = Rc::new(RefCell::new(None));

    let on_disconnect = {
        let connect_id = connect_id.clone();
        Closure::new(move || {
            _ = connect_id.borrow_mut().take();
            log!("Disconnected");
        })
    };

    // Set our connection id
    *connect_id.borrow_mut() = Some(MqttID(mqtt_connect(
        hostname,
        port,
        client_id,
        &on_connect,
        &on_fail,
        &on_disconnect,
        &on_message,
    )));
    let success = rx.await?;

    // Explicitly drop so that the callbacks are held in the async closure
    drop(on_connect);
    drop(on_fail);

    if success {
        Ok(MqttConnection {
            id: connect_id,
            _on_disconnect: on_disconnect,
            _on_message: on_message,
            msgrx,
        })
    } else {
        Err(anyhow::anyhow!("Failed to connect"))
    }
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
                    <Route path="" view=HomePage/>
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

    create_local_resource(
        || (),
        |_| async move {
            let mut res = rs_mqtt_connect("localhost", 9002, "leptosclient").await?;
            res.subscribe("test").unwrap();
            let msg = res.next().await.unwrap();
            log!("got message: {:?}", msg);

            Ok(())
        },
    );

    view! {
        <h1>"Welcome to Leptos!"</h1>
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
