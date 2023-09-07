# Solace-rs

The Unofficial Solace PubSub+ Rust Client Library.
Focuses on providing a safe and idiomatic API over the C Solace library.
It can be used to access the services of a Solace PubSub+ Event Broker.
The C library is not included.


## Features

* Publishing and subscribing
    * Direct
    * Persistent
* Solcache - TODO
* Request Reply - TODO
* Async - TODO


## Simple Example


```rust
let solace_context = Context::new(SolaceLogLevel::Warning)
    .map_err(|_| SessionError::InitializationFailure)
    .unwrap();
println!("Context created");

let (tx, rx) = mpsc::channel();

let on_message = move |message: InboundMessage| {
    let Ok(payload) = message.get_payload()else {
        return;
    };
    println!("Got message, sending it over the channel");
    let _ = tx.send(payload.to_owned());
};

let session = solace_context
    .session(
        "tcp://localhost:55554",
        "default",
        "default",
        "",
        Some(on_message),
        Some(|e: SessionEvent| {
            println!("on_event handler got: {}", e);
        }),
    )
    .expect("Could not create session");

session
    .subscribe("try-me")
    .expect("Could not subscribe to topic");
println!("Subscribed to try-me topic");

while let Ok(msg) = rx.recv() {
    let Ok(payload) = std::str::from_utf8(&msg)else{
        break;
    };
    println!("Got on channel: {}", payload);
}

session
    .unsubscribe("try-me")
    .expect("Could not unsubscribe to topic");
println!("Unsubscribed from try-me topic");
```
