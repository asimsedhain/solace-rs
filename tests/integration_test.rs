use std::{
    collections::{HashMap, HashSet},
    sync::{mpsc, Arc},
    thread::sleep,
    time::Duration,
};

use solace_rs::{
    event::SessionEvent,
    message::{
        DeliveryMode, DestinationType, InboundMessage, Message, MessageDestination,
        OutboundMessageBuilder,
    },
    Context, SolaceLogLevel,
};

enum TestMessage {
    SolaceMessage(Vec<u8>),
    TimerError,
}

#[test]
#[ignore]
fn subscribe_and_publish() {
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
        let _ = tx.send(TestMessage::SolaceMessage(payload.to_owned()));
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
            .send(TestMessage::TimerError)
            .expect("sending timer error");
    });

    let mut rx_msgs = vec![];
    while let Ok(msg) = rx.recv() {
        match msg {
            TestMessage::SolaceMessage(msg) => {
                let str = String::from_utf8_lossy(&msg).to_string();
                rx_msgs.push(str);
                if rx_msgs.len() == tx_msgs.len() {
                    break;
                }
            }
            TestMessage::TimerError => panic!(),
        }
    }

    assert_eq!(tx_msgs, rx_msgs);
}

#[test]
#[ignore]
fn multi_subscribe_and_publish() {
    let sleep_time = Duration::from_millis(500);

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx0, rx) = mpsc::channel();
    let tx1 = tx0.clone();
    let tx_clone = tx0.clone();
    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let topic = "multi_subscribe_and_publish";

    let session0 = solace_context
        .session(
            "tcp://localhost:55554",
            "default",
            "default",
            "",
            Some(move |message: InboundMessage| {
                let Ok(payload) = message.get_payload() else {
                    return;
                };
                let _ = tx0.send(TestMessage::SolaceMessage(payload.to_owned()));
            }),
            Some(|_: SessionEvent| {}),
        )
        .expect("creating session");
    session0.subscribe(topic).expect("subscribing to topic");

    let session1 = solace_context
        .session(
            "tcp://localhost:55554",
            "default",
            "default",
            "",
            Some(move |message: InboundMessage| {
                let Ok(payload) = message.get_payload() else {
                    return;
                };
                let _ = tx1.send(TestMessage::SolaceMessage(payload.to_owned()));
            }),
            Some(|_: SessionEvent| {}),
        )
        .expect("creating session");
    session1.subscribe(topic).expect("subscribing to topic");

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
        session0.publish(outbound_msg).expect("publishing message");
    }
    std::thread::spawn(move || {
        sleep(sleep_time);
        tx_clone
            .send(TestMessage::TimerError)
            .expect("sending timer error");
    });

    let mut rx_msgs = vec![];
    while let Ok(msg) = rx.recv() {
        match msg {
            TestMessage::SolaceMessage(msg) => {
                let str = String::from_utf8_lossy(&msg).to_string();
                rx_msgs.push(str);
                if rx_msgs.len() == tx_msgs.len() * 2 {
                    break;
                }
            }
            TestMessage::TimerError => panic!(),
        }
    }

    let mut rx_msg_map = HashMap::new();
    for msg in rx_msgs {
        *rx_msg_map.entry(msg).or_insert(0) += 1;
    }

    assert_eq!(
        tx_msgs.clone().into_iter().collect::<HashSet<_>>(),
        rx_msg_map
            .keys()
            .map(|v| v.as_str())
            .collect::<HashSet<_>>()
    );

    assert_eq!(
        tx_msgs.iter().map(|_| 2).collect::<Vec<_>>(),
        rx_msg_map.into_values().collect::<Vec<_>>()
    )
}

#[test]
#[ignore]
fn unsubscribe_and_publish() {
    let sleep_time = Duration::from_millis(500);

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx, rx) = mpsc::channel();
    let tx_clone0 = tx.clone();
    let tx_clone1 = tx.clone();
    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let topic = "unsubscribe_and_publish";

    let on_message = move |message: InboundMessage| {
        let Ok(payload) = message.get_payload() else {
            return;
        };
        let _ = tx.send(TestMessage::SolaceMessage(payload.to_owned()));
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

    session.unsubscribe(topic).expect("unsubscribing to topic");

    std::thread::spawn(move || {
        sleep(sleep_time);
        tx_clone0
            .send(TestMessage::TimerError)
            .expect("sending timer error");
    });

    let mut rx_msgs = vec![];
    while let Ok(msg) = rx.recv() {
        match msg {
            TestMessage::SolaceMessage(msg) => {
                let str = String::from_utf8_lossy(&msg).to_string();
                rx_msgs.push(str);
                if rx_msgs.len() == tx_msgs.len() {
                    break;
                }
            }
            TestMessage::TimerError => panic!(),
        }
    }
    assert_eq!(tx_msgs, rx_msgs);

    // discard the timer error
    let _ = rx.recv();

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
        tx_clone1
            .send(TestMessage::TimerError)
            .expect("sending timer error");
    });

    while let Ok(msg) = rx.recv() {
        match msg {
            TestMessage::SolaceMessage(_) => panic!(),
            TestMessage::TimerError => break,
        }
    }
}

#[test]
#[ignore]
fn multi_thread_publisher() {
    let sleep_time = Duration::from_millis(500);

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx, rx) = mpsc::channel();
    let tx_clone = tx.clone();
    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let topic = "multi_thread_publisher";

    let on_message = move |message: InboundMessage| {
        let Ok(payload) = message.get_payload() else {
            return;
        };
        let _ = tx.send(TestMessage::SolaceMessage(payload.to_owned()));
    };

    let session = Arc::new(
        solace_context
            .session(
                "tcp://localhost:55554",
                "default",
                "default",
                "",
                Some(on_message),
                Some(|_: SessionEvent| {}),
            )
            .expect("creating session"),
    );

    session.subscribe(topic).expect("multi_thread_publisher");

    // need to wait before publishing so that the client is properly subscribed
    sleep(sleep_time);

    for _ in 0..3 {
        let session_clone = session.clone();
        let tx_msgs_clone = tx_msgs.clone();
        std::thread::spawn(move || {
            for msg in tx_msgs_clone {
                let dest = MessageDestination::new(DestinationType::Topic, topic).unwrap();
                let outbound_msg = OutboundMessageBuilder::new()
                    .destination(dest)
                    .delivery_mode(DeliveryMode::Direct)
                    .payload(msg)
                    .build()
                    .expect("building outbound msg");
                session_clone
                    .publish(outbound_msg)
                    .expect("publishing message");
            }
        });
    }

    std::thread::spawn(move || {
        sleep(sleep_time);
        tx_clone
            .send(TestMessage::TimerError)
            .expect("sending timer error");
    });

    let mut rx_msgs = vec![];
    while let Ok(msg) = rx.recv() {
        match msg {
            TestMessage::SolaceMessage(msg) => {
                let str = String::from_utf8_lossy(&msg).to_string();
                rx_msgs.push(str);
                if rx_msgs.len() == tx_msgs.len() * 3 {
                    break;
                }
            }
            TestMessage::TimerError => panic!(),
        }
    }

    let mut rx_msg_map = HashMap::new();
    for msg in rx_msgs {
        *rx_msg_map.entry(msg).or_insert(0) += 1;
    }

    assert_eq!(
        tx_msgs.clone().into_iter().collect::<HashSet<_>>(),
        rx_msg_map
            .keys()
            .map(|v| v.as_str())
            .collect::<HashSet<_>>()
    );

    assert_eq!(
        tx_msgs.iter().map(|_| 3).collect::<Vec<_>>(),
        rx_msg_map.into_values().collect::<Vec<_>>()
    )
}
