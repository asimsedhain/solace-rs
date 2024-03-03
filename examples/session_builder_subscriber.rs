/**
Example showing how to create a solace context, session and subscribing to a topic using
the session.
*/
use std::{thread::sleep, time::Duration};

use solace_rs::{
    event::SessionEvent,
    message::{InboundMessage, Message},
    Context, SolaceLogLevel,
};

fn main() {
    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    println!("Context created");

    let on_message = move |message: InboundMessage| {
        let Ok(Some(payload)) = message.get_payload() else {
            println!("on_message handler could not decode bytes");
            return;
        };
        let Ok(payload) = std::str::from_utf8(payload) else {
            println!("on_message handler could not decode");
            return;
        };

        let Ok(Some(dest)) = message.get_destination() else {
            println!("on_message handler could not get destination");
            return;
        };
        println!(
            "on_message handler got: {} on topic: {:?}",
            payload, dest.dest
        );
    };

    let builder = solace_context.get_session_builder();

    let session = builder
        .host_name("tcp://localhost:55554")
        .vpn_name("default")
        .username("default")
        .password("")
        .on_message(on_message)
        .on_event(|e: SessionEvent| {
            println!("on_event handler got: {}", e);
        })
        .client_name("Sol Client")
        .application_description("This is a library")
        .build()
        .unwrap();

    session
        .subscribe("try-me")
        .expect("Could not subscribe to topic");
    println!("Subscribed to try-me topic");

    let sleep_duration = Duration::new(100, 0);
    println!("Sleeping for {:?} before exiting", sleep_duration);
    sleep(sleep_duration);

    session
        .unsubscribe("try-me")
        .expect("Could not unsubscribe to topic");
    println!("Unsubscribed from try-me topic");
}
