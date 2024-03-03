/**
Example showing how to create a solace context, session and subscribing to a topic using
the session.
*/
use std::{thread::sleep, time::Duration};

use solace_rs::{event::SessionEvent, message::InboundMessage, Context, SolaceLogLevel};

fn main() {
    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    println!("Context created");

    let on_message = move |message: InboundMessage| {
        println!("on_message handler got: {:#?} ", message);
    };

    let session = solace_context
        .session(
            "tcp://localhost:55554", // host
            "default",               // vpn
            "default",               // username
            "",                      // password
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

    let sleep_duration = Duration::new(10, 0);
    println!("Sleeping for {:?} before exiting", sleep_duration);
    sleep(sleep_duration);

    session
        .unsubscribe("try-me")
        .expect("Could not unsubscribe to topic");
    println!("Unsubscribed from try-me topic");
}
