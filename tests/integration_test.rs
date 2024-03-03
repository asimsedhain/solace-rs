use std::{
    collections::{HashMap, HashSet},
    sync::{mpsc, Arc},
    thread::sleep,
    time::Duration,
};

use solace_rs::{
    session::SessionEvent,
    message::{
        DeliveryMode, DestinationType, InboundMessage, Message, MessageDestination,
        OutboundMessageBuilder,
    },
    Context, SolaceLogLevel,
};

static SLEEP_TIME: std::time::Duration = Duration::from_millis(10);

const DEFAULT_HOST: &str = "worker-lenovo-yoga";
const DEFAULT_PORT: &str = "55555";

#[test]
#[ignore]
fn subscribe_and_publish() {
    let host = option_env!("SOLACE_HOST").unwrap_or(DEFAULT_HOST);
    let port = option_env!("SOLACE_PORT").unwrap_or(DEFAULT_PORT);

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx, rx) = mpsc::channel();
    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let topic = "publish_and_receive";

    let on_message = move |message: InboundMessage| {
        let Ok(Some(payload)) = message.get_payload() else {
            return;
        };
        let _ = tx.send(payload.to_owned());
    };

    let session = solace_context
        .session(
            format!("tcp://{}:{}", host, port),
            "default",
            "default",
            "",
            Some(on_message),
            Some(|_: SessionEvent| {}),
        )
        .expect("creating session");
    session.subscribe(topic).expect("subscribing to topic");

    // need to wait before publishing so that the client is properly subscribed
    sleep(SLEEP_TIME);

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
    sleep(SLEEP_TIME);

    let mut rx_msgs = vec![];

    loop {
        match rx.try_recv() {
            Ok(msg) => {
                let str = String::from_utf8_lossy(&msg).to_string();
                rx_msgs.push(str);
                if rx_msgs.len() == tx_msgs.len() {
                    break;
                }
            }
            _ => panic!(),
        }
    }

    assert_eq!(tx_msgs, rx_msgs);
}

#[test]
#[ignore]
fn multi_subscribe_and_publish() {
    let host = option_env!("SOLACE_HOST").unwrap_or(DEFAULT_HOST);
    let port = option_env!("SOLACE_PORT").unwrap_or(DEFAULT_PORT);
    let msg_multiplier = 2;

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx0, rx) = mpsc::channel();
    let tx1 = tx0.clone();
    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let topic = "multi_subscribe_and_publish";

    let session0 = solace_context
        .session(
            format!("tcp://{}:{}", host, port),
            "default",
            "default",
            "",
            Some(move |message: InboundMessage| {
                let Ok(Some(payload)) = message.get_payload() else {
                    return;
                };
                let _ = tx0.send(payload.to_owned());
            }),
            Some(|_: SessionEvent| {}),
        )
        .expect("creating session");
    session0.subscribe(topic).expect("subscribing to topic");

    let session1 = solace_context
        .session(
            format!("tcp://{}:{}", host, port),
            "default",
            "default",
            "",
            Some(move |message: InboundMessage| {
                let Ok(Some(payload)) = message.get_payload() else {
                    return;
                };
                let _ = tx1.send(payload.to_owned());
            }),
            Some(|_: SessionEvent| {}),
        )
        .expect("creating session");
    session1.subscribe(topic).expect("subscribing to topic");

    // need to wait before publishing so that the client is properly subscribed
    sleep(SLEEP_TIME);

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

    sleep(SLEEP_TIME);

    let mut rx_msgs = vec![];
    loop {
        match rx.try_recv() {
            Ok(msg) => {
                let str = String::from_utf8_lossy(&msg).to_string();
                rx_msgs.push(str);
                if rx_msgs.len() == tx_msgs.len() * msg_multiplier {
                    break;
                }
            }
            _ => panic!(),
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
        tx_msgs.iter().map(|_| msg_multiplier).collect::<Vec<_>>(),
        rx_msg_map.into_values().collect::<Vec<_>>()
    )
}

#[test]
#[ignore]
fn unsubscribe_and_publish() {
    let host = option_env!("SOLACE_HOST").unwrap_or(DEFAULT_HOST);
    let port = option_env!("SOLACE_PORT").unwrap_or(DEFAULT_PORT);

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx, rx) = mpsc::channel();
    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let topic = "unsubscribe_and_publish";

    let on_message = move |message: InboundMessage| {
        let Ok(Some(payload)) = message.get_payload() else {
            return;
        };
        let _ = tx.send(payload.to_owned());
    };

    let session = solace_context
        .session(
            format!("tcp://{}:{}", host, port),
            "default",
            "default",
            "",
            Some(on_message),
            Some(|_: SessionEvent| {}),
        )
        .expect("creating session");
    session.subscribe(topic).expect("subscribing to topic");

    sleep(SLEEP_TIME);

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

    sleep(SLEEP_TIME);

    let mut rx_msgs = vec![];

    loop {
        match rx.try_recv() {
            Ok(msg) => {
                let str = String::from_utf8_lossy(&msg).to_string();
                rx_msgs.push(str);
                if rx_msgs.len() == tx_msgs.len() {
                    break;
                }
            }
            _ => panic!(),
        }
    }

    assert_eq!(tx_msgs, rx_msgs);

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

    sleep(SLEEP_TIME);

    if rx.try_recv().is_ok() {
        panic!()
    }
}

#[test]
#[ignore]
fn multi_thread_publisher() {
    let host = option_env!("SOLACE_HOST").unwrap_or(DEFAULT_HOST);
    let port = option_env!("SOLACE_PORT").unwrap_or(DEFAULT_PORT);

    let msg_multiplier = 3;

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx, rx) = mpsc::channel();
    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let topic = "multi_thread_publisher";

    let on_message = move |message: InboundMessage| {
        let Ok(Some(payload)) = message.get_payload() else {
            return;
        };
        let _ = tx.send(payload.to_owned());
    };

    let session = Arc::new(
        solace_context
            .session(
                format!("tcp://{}:{}", host, port),
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
    sleep(SLEEP_TIME);

    for _ in 0..msg_multiplier {
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

    sleep(SLEEP_TIME);

    let mut rx_msgs = vec![];

    loop {
        match rx.try_recv() {
            Ok(msg) => {
                let str = String::from_utf8_lossy(&msg).to_string();
                rx_msgs.push(str);
                if rx_msgs.len() == tx_msgs.len() * msg_multiplier {
                    break;
                }
            }
            _ => panic!(),
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
        tx_msgs.iter().map(|_| msg_multiplier).collect::<Vec<_>>(),
        rx_msg_map.into_values().collect::<Vec<_>>()
    )
}

#[test]
#[ignore]
fn no_local_session() {
    let host = option_env!("SOLACE_HOST").unwrap_or(DEFAULT_HOST);
    let port = option_env!("SOLACE_PORT").unwrap_or(DEFAULT_PORT);

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx, rx) = mpsc::channel();
    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let topic = "no_local_session";

    let on_message = move |message: InboundMessage| {
        let _ = tx.send(message);
    };

    let session = solace_context
        .session_builder()
        .host_name(format!("tcp://{}:{}", host, port))
        .vpn_name("default")
        .username("default")
        .password("")
        .on_message(on_message)
        .on_event(|_: SessionEvent| {})
        .no_local(true)
        .build()
        .expect("creating session");

    session.subscribe(topic).expect("subscribing to topic");

    // need to wait before publishing so that the client is properly subscribed
    sleep(SLEEP_TIME);

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
    sleep(SLEEP_TIME * 2);

    assert!(rx.try_recv().is_err());
}

#[test]
#[ignore]
fn auto_generate_tx_rx_session_fields() {
    let host = option_env!("SOLACE_HOST").unwrap_or(DEFAULT_HOST);
    let port = option_env!("SOLACE_PORT").unwrap_or(DEFAULT_PORT);

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx, rx) = mpsc::channel();
    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let topic = "auto_generate_tx_rx_session_fields";

    let on_message = move |message: InboundMessage| {
        let _ = tx.send(message);
    };

    let session = solace_context
        .session_builder()
        .host_name(format!("tcp://{}:{}", host, port))
        .vpn_name("default")
        .username("default")
        .password("")
        .on_message(on_message)
        .on_event(|_: SessionEvent| {})
        .generate_rcv_timestamps(true)
        .generate_sender_id(true)
        .generate_send_timestamp(true)
        .generate_sender_sequence_number(true)
        .build()
        .expect("creating session");

    session.subscribe(topic).expect("subscribing to topic");

    // need to wait before publishing so that the client is properly subscribed
    sleep(SLEEP_TIME);

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
    sleep(SLEEP_TIME);

    let mut rx_count = 0;
    loop {
        match rx.try_recv() {
            Ok(msg) => {
                assert!(msg.get_receive_timestamp().is_ok_and(|v| v.is_some()));
                assert!(msg.get_sender_id().is_ok_and(|v| v.is_some()));
                assert!(msg.get_sender_timestamp().is_ok_and(|v| v.is_some()));
                assert!(msg.get_sequence_number().is_ok_and(|v| v.is_some()));

                rx_count += 1;
                if rx_count == tx_msgs.len() {
                    break;
                }
            }
            _ => panic!(),
        }
    }
}
