use std::{thread, time::Duration};

/**
Example showing how to create a solace context, session and provision/deprovision an endpoint using
the session.
*/
use solace_rs::{
    endpoint_props::{
        EndpointAccessType, EndpointDiscardBehavior, EndpointId, EndpointPermission,
        EndpointPropsBuilder,
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
        .durable(true)
        .permission(EndpointPermission::Consume)
        .access_type(EndpointAccessType::NonExclusive)
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

    let sleep_duration = Duration::from_secs(5);
    println!(
        "Sleeping for {:?} before deprovisioning, check queue created",
        sleep_duration
    );
    thread::sleep(sleep_duration);

    session.endpoint_deprovision(endpoint_props, true).unwrap();
    println!("Deprovisioned endpoint try-me");
}
