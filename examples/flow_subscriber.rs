use std::{thread, time::Duration};

/**
Example showing how to create a solace context, session and flow using
the session.
This example creates a flow to a queue and consumes messages from it.
*/
use solace_rs::{
    endpoint_props::{
        EndpointDiscardBehavior, EndpointId, EndpointPermission, EndpointPropsBuilder,
    },
    flow::{
        builder::{FlowAckMode, FlowBindEntityDurable, FlowBindEntityId},
        event::FlowEvent,
    },
    message::InboundMessage,
    session::SessionEvent,
    Context, SolaceLogLevel,
};

fn main() {
    let solace_context = Context::new(SolaceLogLevel::Warning).unwrap();
    println!("Context created");

    let session = solace_context
        .session_builder()
        .host_name("tcp://localhost:55554")
        .vpn_name("default")
        .username("default")
        .password("")
        .client_name("Sol Client")
        .application_description("This is a library")
        .on_message(move |message: InboundMessage| {
            println!("on_message handler got: {:#?} ", message);
        })
        .on_event(|e: SessionEvent| {
            println!("on_event handler got: {}", e);
        })
        .build()
        .unwrap();

    let endpoint_props = EndpointPropsBuilder::new()
        .id(EndpointId::Queue {
            name: "try-me".to_string(),
        })
        .permission(EndpointPermission::Consume)
        .quota_mb(10)
        .max_msg_size(1024)
        .respects_msg_ttl(true)
        .discard_behavior(EndpointDiscardBehavior::DiscardNotifySenderOn)
        .max_msg_redelivery(5)
        .build()
        .unwrap();

    session
        .endpoint_provision(endpoint_props.clone(), true)
        .unwrap();
    println!("Provisioned endpoint try-me");

    // Note: flow will be destroyed when it dropped
    let _flow = session
        .flow_builder()
        .bind_timeout_ms(5000)
        .bind_entity_id(FlowBindEntityId::Queue {
            queue_name: "try-me".to_string(),
        })
        .bind_entity_durable(FlowBindEntityDurable::Durable)
        .window_size(100)
        .ack_mode(FlowAckMode::Client)
        .topic("try-me".to_string())
        .max_bind_tries(5)
        .ack_timer_ms(100)
        .ack_threshold(50)
        .start_state(true)
        // .selector()
        .no_local(true)
        .max_unacked_messages(10)
        .browser(false)
        .active_flow_ind(true)
        // .replay_start_location()
        .max_reconnect_tries(5)
        .reconnect_retry_interval_ms(500)
        .required_outcome_failed(false)
        .required_outcome_rejected(false)
        .on_message(move |message: InboundMessage| {
            println!("on_message handler in flow got: {:#?} ", message);
        })
        .on_event(|event: FlowEvent| {
            println!("on_event handler in flow got: {:#?}", event);
        })
        .build()
        .unwrap();
    println!("Flow created");

    let sleep_duration = Duration::from_secs(5);
    println!("Sleeping for {:?} before close flow", sleep_duration);
    thread::sleep(sleep_duration);

    session.endpoint_deprovision(endpoint_props, true).unwrap();
    println!("Deprovisioned endpoint try-me");
}
