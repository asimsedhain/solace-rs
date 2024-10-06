/**
Example showing how to create do request-reply.
*/
use std::{
    num::NonZeroU32,
    sync::mpsc,
    thread::{self, sleep},
    time::Duration,
};

use solace_rs::{
    message::{
        DeliveryMode, DestinationType, InboundMessage, Message, MessageDestination,
        OutboundMessageBuilder,
    },
    session::SessionEvent,
    Context, SolaceLogLevel,
};

const HOST: &str = "tcp://localhost:55554";
const VPN: &str = "default";
const USER: &str = "default";
const TOPIC: &str = "ping";

fn main() {
    let context = Context::new(SolaceLogLevel::Warning).unwrap();
    println!("Context created");
    {
        let context = context.clone();
        thread::spawn(move || responder(context));
    }
    sleep(Duration::new(1, 0));
    requester(context);
    sleep(Duration::new(1, 0));
}

fn requester(context: Context) {
    println!("Starting Requester...");
    let sender = context
        .session(
            HOST,
            VPN,
            USER,
            "", // password
            Some(|message: InboundMessage| {
                println!("on_message handler got: {:#?} ", message);
            }),
            Some(|e: SessionEvent| {
                println!("on_event handler got: {}", e);
            }),
        )
        .expect("Could not create session");

    let dest = MessageDestination::new(DestinationType::Topic, TOPIC).unwrap();

    let request = OutboundMessageBuilder::new()
        .destination(dest)
        .delivery_mode(DeliveryMode::Direct)
        .payload("request from rust".to_string())
        .build()
        .expect("could not build message");

    println!("Requester: Sending request");
    let reply = sender.request(request, NonZeroU32::new(30_000).unwrap());

    println!("Got reply: {:?}", reply);
    println!("Ending Requester...");
}

fn responder(context: Context) {
    println!("Starting Responder...");
    let (tx, rx) = mpsc::channel();

    let replier = context
        .session(
            HOST,
            VPN,
            USER,
            "", // password
            Some(move |message: InboundMessage| {
                let _ = tx.send(message);
            }),
            Some(|e: SessionEvent| {
                println!("replier on_event handler got: {}", e);
            }),
        )
        .expect("Could not create responder");

    replier.subscribe(TOPIC).unwrap();

    if let Ok(msg) = rx.recv() {
        if let Ok(Some(reply_dest)) = msg.get_reply_to() {
            let reply_payload =
                String::from_utf8_lossy(msg.get_payload().unwrap_or(Some(&[])).unwrap_or(&[]));

            let reply_msg = OutboundMessageBuilder::new()
                .destination(reply_dest)
                .delivery_mode(DeliveryMode::Direct)
                .payload(format!("pong to : {}", reply_payload))
                .is_reply(true)
                .correlation_id(msg.get_correlation_id().unwrap().unwrap())
                .build()
                .expect("could not build message");

            let _ = replier.publish(reply_msg);
        } else {
            println!("Got message without reply to address")
        }
    }

    println!("Ending Responder...");
}
