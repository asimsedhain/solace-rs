/**
Example showing how to create a solace context, session and publishing to a topic using
the session.
*/
use std::{thread::sleep, time::Duration};

use solace_rs::{
    event::SessionEvent,
    message::{
        DeliveryMode, DestinationType, InboundMessage, MessageDestination, OutboundMessageBuilder,
    },
    Context, SolaceLogLevel,
};

fn main() {
    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    println!("Context created");

    let session = solace_context
        .session(
            "tcp://localhost:55554", // host
            "default",               // vpn
            "default",               // username
            "",                      // password
            Some(|message: InboundMessage| {
                println!("on_message handler got: {:#?} ", message);
            }),
            Some(|e: SessionEvent| {
                println!("on_event handler got: {}", e);
            }),
        )
        .expect("Could not create session");

    let topic = "try-me";

    for i in 0..10 {
        let message = {
            let dest = MessageDestination::new(DestinationType::Topic, topic).unwrap();

            OutboundMessageBuilder::new()
                .destination(dest)
                .delivery_mode(DeliveryMode::Direct)
                .payload(format!("hello from rust: {}", i))
                .build()
                .expect("could not build message")
        };
        session.publish(message).expect("message to be sent");
        sleep(Duration::new(1, 0));
    }

    let sleep_duration = Duration::new(10, 0);
    println!("Sleeping for {:?} before exiting", sleep_duration);
    sleep(sleep_duration);
}
