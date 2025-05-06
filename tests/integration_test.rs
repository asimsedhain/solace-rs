use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU32,
    sync::{mpsc, Arc, Barrier, Mutex},
    thread::{self, sleep},
    time::Duration,
};

use solace_rs::{
    endpoint_props::{EndpointId, EndpointPropsBuilder},
    flow::builder::{FlowAckMode, FlowBindEntityId},
    message::{
        inbound::{FlowInboundMessage, InboundMessageTrait},
        DeliveryMode, DestinationType, InboundMessage, Message, MessageDestination,
        OutboundMessageBuilder,
    },
    session::SessionEvent,
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
            None::<fn(SessionEvent)>,
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
            None::<fn(SessionEvent)>,
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

    let session = Arc::new(Mutex::new(
        solace_context
            .session(
                format!("tcp://{}:{}", host, port),
                "default",
                "default",
                "",
                Some(on_message),
                None::<fn(SessionEvent)>,
            )
            .expect("creating session"),
    ));

    session
        .lock()
        .unwrap()
        .subscribe(topic)
        .expect("multi_thread_publisher");

    // need to wait before publishing so that the client is properly subscribed
    sleep(SLEEP_TIME);

    let mut handles = vec![];

    for _ in 0..msg_multiplier {
        let session_clone = session.clone();
        let tx_msgs_clone = tx_msgs.clone();
        let thread_h = std::thread::spawn(move || {
            let session_clone_lock = session_clone.lock().unwrap();
            for msg in &tx_msgs_clone {
                let dest = MessageDestination::new(DestinationType::Topic, topic).unwrap();
                let outbound_msg = OutboundMessageBuilder::new()
                    .destination(dest)
                    .delivery_mode(DeliveryMode::Direct)
                    .payload(*msg)
                    .build()
                    .expect("building outbound msg");
                session_clone_lock
                    .publish(outbound_msg)
                    .expect("publishing message");
            }
            drop(session_clone_lock);
        });
        handles.push(thread_h);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    sleep(SLEEP_TIME);
    drop(session);
    drop(solace_context);

    let mut rx_msgs = vec![];

    while let Ok(msg) = rx.recv() {
        let str = String::from_utf8_lossy(&msg).to_string();
        rx_msgs.push(str);
    }

    assert!(rx_msgs.len() == tx_msgs.len() * msg_multiplier);

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

    let (tx, rx) = mpsc::channel();

    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let topic = "auto_generate_tx_rx_session_fields";
    let send_count = 1000;
    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
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
        // NOTE: there is bug in the solace lib where it does not copy over the message if there is
        // not enough space in the buffer. This can cause the TSan to trigger.
        .buffer_size_bytes(900_000)
        .generate_rcv_timestamps(true)
        .generate_sender_id(true)
        .generate_send_timestamp(true)
        .generate_sender_sequence_number(true)
        .build()
        .expect("creating session");

    session.subscribe(topic).expect("subscribing to topic");

    // need to wait before publishing so that the client is properly subscribed
    sleep(SLEEP_TIME);

    for msg in tx_msgs.clone().into_iter().cycle().take(send_count) {
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
    let _ = session.disconnect();

    drop(solace_context);

    let mut iter = tx_msgs.clone().into_iter().cycle();

    let mut rx_count = 0;
    while let Ok(msg) = rx.recv() {
        assert!(msg.get_payload().unwrap().unwrap() == iter.next().unwrap().as_bytes());
        assert!(msg.get_receive_timestamp().is_ok_and(|v| v.is_some()));
        assert!(msg.get_sender_id().is_ok_and(|v| v.is_some()));
        assert!(msg.get_sender_timestamp().is_ok_and(|v| v.is_some()));
        assert!(msg.get_sequence_number().is_ok_and(|v| v.is_some()));

        rx_count += 1;
    }

    assert!(rx_count == send_count);
}

#[test]
#[ignore]
fn request_and_reply() {
    let host = option_env!("SOLACE_HOST").unwrap_or(DEFAULT_HOST);
    let port = option_env!("SOLACE_PORT").unwrap_or(DEFAULT_PORT);
    let topic = "request_and_reply";

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let g_barrier = Arc::new(Barrier::new(2));

    thread::scope(|s| {
        let context = solace_context.clone();
        let barrier = g_barrier.clone();
        // requester
        let req = s.spawn(move || {
            let session = context
                .session(
                    format!("tcp://{}:{}", host, port),
                    "default",
                    "default",
                    "",
                    None::<fn(InboundMessage)>,
                    None::<fn(SessionEvent)>,
                )
                .unwrap();
            barrier.wait();
            sleep(SLEEP_TIME);

            let dest = MessageDestination::new(DestinationType::Topic, topic).unwrap();

            let request = OutboundMessageBuilder::new()
                .destination(dest)
                .delivery_mode(DeliveryMode::Direct)
                .payload("ping".to_string())
                .build()
                .expect("could not build message");
            let reply = session
                .request(request, NonZeroU32::new(5_000).unwrap())
                .unwrap();
            assert!(reply.get_payload().unwrap().unwrap() == b"pong");
        });

        let context = solace_context.clone();
        let res = s.spawn(move || {
            let (tx, rx) = mpsc::channel();
            let session = context
                .session(
                    format!("tcp://{}:{}", host, port),
                    "default",
                    "default",
                    "",
                    Some(move |message: InboundMessage| {
                        let _ = tx.send(message);
                    }),
                    None::<fn(SessionEvent)>,
                )
                .unwrap();
            session.subscribe(topic).unwrap();

            g_barrier.wait();

            let msg = rx.recv().unwrap();

            let reply_msg = OutboundMessageBuilder::new()
                .destination(msg.get_reply_to().unwrap().unwrap())
                .delivery_mode(DeliveryMode::Direct)
                .payload("pong".to_string())
                .is_reply(true)
                .correlation_id(msg.get_correlation_id().unwrap().unwrap())
                .build()
                .expect("could not build message");
            let _ = session.publish(reply_msg);
        });
        assert!(res.join().is_ok());
        assert!(req.join().is_ok());
    });
}

#[test]
#[ignore]
fn subscribe_and_publish_with_queue() {
    let host = option_env!("SOLACE_HOST").unwrap_or(DEFAULT_HOST);
    let port = option_env!("SOLACE_PORT").unwrap_or(DEFAULT_PORT);

    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx, rx) = mpsc::channel();
    let tx_msgs = vec!["helo", "hello2", "hello4", "helo5"];
    let queue_name = "subscribe_and_publish_with_queue";

    let on_message = move |message: FlowInboundMessage| {
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
            None::<fn(InboundMessage)>,
            None::<fn(SessionEvent)>,
        )
        .expect("creating session");

    // Provision a queue
    let endpoint_props = EndpointPropsBuilder::new()
        .id(EndpointId::Queue {
            name: queue_name.to_string(),
        })
        .build()
        .expect("building endpoint props");
    session
        .endpoint_provision(endpoint_props.clone(), true)
        .expect("provisioning queue");

    sleep(SLEEP_TIME);

    // Subscribe to the queue
    let flow = session
        .flow_builder()
        .bind_entity_id(FlowBindEntityId::Queue {
            queue_name: queue_name.to_string(),
        })
        .ack_mode(FlowAckMode::Auto)
        .on_message(on_message)
        .on_event(|_| {})
        .build()
        .expect("subscribing to queue");

    // need to wait before publishing so that the client is properly subscribed
    sleep(SLEEP_TIME);

    for msg in tx_msgs.clone() {
        let dest = MessageDestination::new(DestinationType::Queue, queue_name).unwrap();
        let outbound_msg = OutboundMessageBuilder::new()
            .destination(dest)
            .delivery_mode(DeliveryMode::Persistent)
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

    // Deprovision queue and cleanup
    drop(flow);
    session.endpoint_deprovision(endpoint_props, true).unwrap();
    sleep(SLEEP_TIME);

    assert_eq!(tx_msgs, rx_msgs);
}

#[test]
#[ignore]
fn flow_message_ack() {
    let host = option_env!("SOLACE_HOST").unwrap_or(DEFAULT_HOST);
    let port = option_env!("SOLACE_PORT").unwrap_or(DEFAULT_PORT);
    let queue_name = "flow_message_ack";
    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    let (tx, rx) = mpsc::channel();
    let tx_msgs = vec!["ack1", "ack2", "ack3"];

    // Provision a queue
    let endpoint_props = EndpointPropsBuilder::new()
        .id(EndpointId::Queue {
            name: queue_name.to_string(),
        })
        .build()
        .expect("building endpoint props");
    let session = solace_context
        .session(
            format!("tcp://{}:{}", host, port),
            "default",
            "default",
            "",
            None::<fn(InboundMessage)>,
            None::<fn(SessionEvent)>,
        )
        .expect("creating session");
    session
        .endpoint_provision(endpoint_props.clone(), true)
        .expect("provisioning queue");

    sleep(SLEEP_TIME);

    // Subscribe to the queue with client ack mode
    let flow = session
        .flow_builder()
        .bind_entity_id(FlowBindEntityId::Queue {
            queue_name: queue_name.to_string(),
        })
        .ack_mode(FlowAckMode::Client)
        .on_message({
            let tx = tx.clone();
            move |message: FlowInboundMessage| {
                let Ok(Some(payload)) = message.get_payload() else {
                    return;
                };
                // Ack the message
                message.try_ack().expect("acknowledging message");
                let _ = tx.send(payload.to_owned());
            }
        })
        .on_event(|_| {})
        .build()
        .expect("subscribing to queue");

    sleep(SLEEP_TIME);

    // Publish messages
    for msg in tx_msgs.clone() {
        let dest = MessageDestination::new(DestinationType::Queue, queue_name).unwrap();
        let outbound_msg = OutboundMessageBuilder::new()
            .destination(dest)
            .delivery_mode(DeliveryMode::Persistent)
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

    // Deprovision queue and cleanup
    drop(flow);
    session.endpoint_deprovision(endpoint_props, true).unwrap();
    sleep(SLEEP_TIME);

    assert_eq!(tx_msgs, rx_msgs);
}
