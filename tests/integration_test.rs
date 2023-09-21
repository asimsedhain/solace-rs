use std::{sync::mpsc, thread::sleep, time::Duration};

use solace_rs::{
    event::SessionEvent,
    message::{
        DeliveryMode, DestinationType, InboundMessage, Message, MessageDestination,
        OutboundMessageBuilder,
    },
    Context, SolaceLogLevel,
};

#[test]
#[ignore]
fn subscribe_and_publish() {
    enum Message {
        SolaceMessage(Vec<u8>),
        TimerError,
    }
    let sleep_time = Duration::from_millis(500);

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx, rx) = mpsc::channel();
    let tx_clone = tx.clone();
    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let topic = "publish_and_receive";

    let on_message = move |message: InboundMessage| {
        let Ok(payload) = message.get_payload() else {
            return;
        };
        let _ = tx.send(Message::SolaceMessage(payload.to_owned()));
    };

    let session = solace_context
        .session(
            "tcp://localhost:55554",
            "default",
            "default",
            "",
            Some(on_message),
            Some(|_: SessionEvent| {}),
        )
        .expect("creating session");
    session.subscribe(topic).expect("subscribing to topic");

    // need to wait before publishing so that the client is properly subscribed
    sleep(sleep_time);

    for msg in tx_msgs.clone() {
        let dest = MessageDestination::new(DestinationType::Topic, topic).unwrap();
        let outbound_msg = OutboundMessageBuilder::new()
            .destination(dest)
            .delivery_mode(DeliveryMode::Direct)
            .payload(msg)
            .build()
            .expect("building outbound msg");
        session.publish(outbound_msg).expect("publishing message");
    }
    std::thread::spawn(move || {
        sleep(sleep_time);
        tx_clone
            .send(Message::TimerError)
            .expect("sending timer error");
    });

    let mut rx_msgs = vec![];
    while let Ok(msg) = rx.recv() {
        match msg {
            Message::SolaceMessage(msg) => {
                let str = String::from_utf8_lossy(&msg).to_string();
                rx_msgs.push(str);
                if rx_msgs.len() == tx_msgs.len() {
                    break;
                }
            }
            Message::TimerError => panic!(),
        }
    }

    assert_eq!(tx_msgs, rx_msgs);
}
