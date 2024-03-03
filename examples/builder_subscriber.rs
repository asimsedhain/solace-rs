/**
Example showing how to create a solace session using the builder api and subscribing to a topic using
the session.
*/
use std::{thread::sleep, time::Duration};

use solace_rs::{message::InboundMessage, session::SessionEvent, Context, SolaceLogLevel};

fn main() {
    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    println!("Context created");

    let on_message = move |message: InboundMessage| {
        println!("on_message handler got: {:#?} ", message);
    };

    let session = solace_context
        .session_builder()
        .host_name("tcp://localhost:55554")
        .vpn_name("default")
        .username("default")
        .password("")
        .client_name("Sol Client")
        .application_description("This is a library")
        .keep_alive_interval_ms(600)
        .keep_alive_limit(5)
        .generate_sender_id(true)
        .generate_send_timestamp(true)
        .generate_rcv_timestamps(true)
        .generate_sender_sequence_number(true)
        .on_message(on_message)
        .on_event(|e: SessionEvent| {
            println!("on_event handler got: {}", e);
        })
        .build()
        .unwrap();

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
