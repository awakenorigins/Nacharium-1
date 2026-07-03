//! S2.5 — Multi-node gossip integration test.
//!
//! Spins up TWO libp2p swarms in-process, dials one from the other, subscribes
//! both to a Sanctum topic, and verifies a published message is delivered
//! peer-to-peer via gossipsub. This proves the "No King can Kill" fabric:
//! nodes talk directly, no central server.

use futures::StreamExt;
use libp2p::{
    gossipsub, identify, ping,
    swarm::SwarmEvent,
    Multiaddr, SwarmBuilder,
};
use nacharium_transport::{sanctum_topic, NacharuBehaviour};
use std::time::Duration;

fn build_swarm() -> libp2p::Swarm<NacharuBehaviour> {
    SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            Default::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .unwrap()
        .with_behaviour(|key| {
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub::Config::default(),
            )
            .unwrap();
            let identify = identify::Behaviour::new(identify::Config::new(
                "/nacharium/1.0".into(),
                key.public(),
            ));
            let ping = ping::Behaviour::new(ping::Config::new());
            NacharuBehaviour { gossipsub, identify, ping }
        })
        .unwrap()
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(10)))
        .build()
}

#[tokio::test]
async fn two_nodes_gossip_a_sanctum_message() {
    let topic = sanctum_topic("integration");

    let mut a = build_swarm();
    let mut b = build_swarm();

    a.behaviour_mut().gossipsub.subscribe(&topic).unwrap();
    b.behaviour_mut().gossipsub.subscribe(&topic).unwrap();

    // A listens on a random local TCP port.
    a.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();
    let listen_addr: Multiaddr = loop {
        if let SwarmEvent::NewListenAddr { address, .. } = a.select_next_some().await {
            break address;
        }
    };

    // B dials A.
    b.dial(listen_addr).unwrap();

    // Drive both swarms until B publishes and A receives, or timeout.
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        let mut published = false;
        loop {
            tokio::select! {
                ev = a.select_next_some() => {
                    if let SwarmEvent::Behaviour(nacharium_transport::NacharuBehaviourEvent::Gossipsub(
                        gossipsub::Event::Message { message, .. })) = ev {
                        return String::from_utf8_lossy(&message.data).to_string();
                    }
                }
                ev = b.select_next_some() => {
                    // Once B sees A subscribed to the topic, publish.
                    if let SwarmEvent::Behaviour(nacharium_transport::NacharuBehaviourEvent::Gossipsub(
                        gossipsub::Event::Subscribed { .. })) = ev {
                        if !published {
                            // small settle then publish
                            let _ = b.behaviour_mut().gossipsub.publish(topic.clone(), b"hello-peer".to_vec());
                            published = true;
                        }
                    }
                }
            }
        }
    })
    .await;

    assert!(result.is_ok(), "gossip message was not delivered within timeout");
    assert_eq!(result.unwrap(), "hello-peer");
}
