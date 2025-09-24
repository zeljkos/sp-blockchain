use libp2p::{
    gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode},
    identify,
    kad::{self, store::MemoryStore},
    mdns,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm, Transport,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use futures::StreamExt;
use tokio::sync::mpsc;
use async_trait::async_trait;

use super::{NetworkMessage, NetworkResult, PeerInfo};

// Network behavior combining all protocols
#[derive(NetworkBehaviour)]
pub struct SpBlockchainBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub identify: identify::Behaviour,
}

pub struct P2PNetwork {
    swarm: Swarm<SpBlockchainBehaviour>,
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
    message_receiver: mpsc::UnboundedReceiver<NetworkMessage>,
    peers: std::collections::HashMap<PeerId, PeerInfo>,
    node_id: String,
    message_callback: Option<mpsc::UnboundedSender<NetworkMessage>>,
}

impl P2PNetwork {
    pub async fn new(node_id: String, listen_port: u16) -> NetworkResult<Self> {
        // Generate a random PeerId
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        println!("🔗 P2P Node {} starting with PeerID: {}", node_id, local_peer_id);

        // Set up the transport
        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::Config::new(&local_key)?)
            .multiplex(yamux::Config::default())
            .boxed();

        // Create gossipsub topic for SP blockchain
        let gossipsub_topic = IdentTopic::new("sp-blockchain-settlement");
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(ValidationMode::Strict)
            .message_id_fn(|message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                s.finish().to_string().into()
            })
            .build()
            .expect("Valid gossipsub config");

        let mut gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )?;
        gossipsub.subscribe(&gossipsub_topic)?;

        // Set up mDNS for local peer discovery
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;

        // Set up Kademlia for DHT
        let kademlia = kad::Behaviour::new(
            local_peer_id,
            MemoryStore::new(local_peer_id),
        );

        // Set up identify protocol
        let identify = identify::Behaviour::new(identify::Config::new(
            "/sp-blockchain/1.0.0".into(),
            local_key.public(),
        ));

        let behaviour = SpBlockchainBehaviour {
            gossipsub,
            mdns,
            kademlia,
            identify,
        };

        let swarm_config = libp2p::swarm::Config::with_tokio_executor();
        let mut swarm = Swarm::new(transport, behaviour, local_peer_id, swarm_config);

        // Listen on all interfaces
        let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", listen_port).parse()?;
        swarm.listen_on(listen_addr)?;

        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            swarm,
            message_sender,
            message_receiver,
            peers: std::collections::HashMap::new(),
            node_id,
            message_callback: None,
        })
    }

    /// Set the callback for forwarding received messages to blockchain
    pub fn set_message_callback(&mut self, callback: mpsc::UnboundedSender<NetworkMessage>) {
        self.message_callback = Some(callback);
        println!("🔗 P2P message callback connected to blockchain");
    }

    pub async fn start(&mut self) -> NetworkResult<()> {
        println!("🚀 Starting P2P network for node: {}", self.node_id);

        loop {
            tokio::select! {
                // Handle swarm events
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await?;
                }

                // Handle outgoing messages
                Some(message) = self.message_receiver.recv() => {
                    self.handle_outgoing_message(message).await?;
                }
            }
        }
    }

    async fn handle_swarm_event(&mut self, event: SwarmEvent<<SpBlockchainBehaviour as NetworkBehaviour>::ToSwarm>) -> NetworkResult<()> {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("🎧 Listening on: {}", address);
            }

            SwarmEvent::Behaviour(event) => match event {
                SpBlockchainBehaviourEvent::Mdns(mdns::Event::Discovered(list)) => {
                    for (peer_id, multiaddr) in list {
                        println!("🔍 Discovered peer: {} at {}", peer_id, multiaddr);
                        self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        self.swarm.behaviour_mut().kademlia.add_address(&peer_id, multiaddr.clone());

                        // Add to peer list
                        self.peers.insert(peer_id, PeerInfo {
                            node_id: peer_id.to_string(),
                            peer_id,
                            addresses: vec![multiaddr],
                            last_seen: std::time::SystemTime::now(),
                        });
                    }
                }

                SpBlockchainBehaviourEvent::Mdns(mdns::Event::Expired(list)) => {
                    for (peer_id, _) in list {
                        println!("🚫 Peer expired: {}", peer_id);
                        self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                        self.peers.remove(&peer_id);
                    }
                }

                SpBlockchainBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: _,
                    message_id: _,
                    message,
                }) => {
                    self.handle_gossip_message(message).await?;
                }

                SpBlockchainBehaviourEvent::Identify(identify::Event::Received {
                    peer_id,
                    info,
                    ..
                }) => {
                    println!("🆔 Identified peer: {} - Agent: {}", peer_id, info.agent_version);
                    for addr in info.listen_addrs {
                        self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                    }
                }

                _ => {}
            }

            _ => {}
        }
        Ok(())
    }

    async fn handle_gossip_message(&mut self, message: gossipsub::Message) -> NetworkResult<()> {
        // Deserialize the network message
        if let Ok(network_msg) = serde_json::from_slice::<NetworkMessage>(&message.data) {
            println!("📨 Received network message: {:?}", network_msg);

            // Forward to blockchain layer
            match network_msg.clone() {
                NetworkMessage::NewBlock { block_hash, .. } => {
                    println!("🆕 New block received: {}", hex::encode(block_hash.as_bytes()));
                    // Forward to blockchain message handler
                    if let Some(ref callback) = self.message_callback {
                        if let Err(e) = callback.send(network_msg) {
                            println!("❌ Failed to forward block to blockchain: {}", e);
                        }
                    }
                }
                NetworkMessage::Vote { block_hash, validator_id, approve, .. } => {
                    println!("🗳️  Vote received from {}: {} for block {}",
                             validator_id, if approve { "APPROVE" } else { "REJECT" },
                             hex::encode(block_hash.as_bytes()));
                    // Forward to blockchain message handler
                    if let Some(ref callback) = self.message_callback {
                        if let Err(e) = callback.send(network_msg) {
                            println!("❌ Failed to forward vote to blockchain: {}", e);
                        }
                    }
                }
                NetworkMessage::Ping => {
                    // Respond with pong
                    self.broadcast_message(NetworkMessage::Pong).await?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn handle_outgoing_message(&mut self, message: NetworkMessage) -> NetworkResult<()> {
        let serialized = serde_json::to_vec(&message)?;

        // Publish to gossipsub topic
        if let Err(e) = self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(IdentTopic::new("sp-blockchain-settlement"), serialized) {
            println!("❌ Failed to publish message: {:?}", e);
        }

        Ok(())
    }

    pub async fn broadcast_message(&mut self, message: NetworkMessage) -> NetworkResult<()> {
        self.message_sender.send(message)?;
        Ok(())
    }

    pub fn add_peer(&mut self, peer_addr: Multiaddr) -> NetworkResult<()> {
        self.swarm.dial(peer_addr)?;
        Ok(())
    }

    pub fn get_connected_peers(&self) -> Vec<&PeerInfo> {
        self.peers.values().collect()
    }

    pub fn get_node_id(&self) -> &str {
        &self.node_id
    }

    /// Get a sender for outbound P2P messages
    pub fn get_message_sender(&self) -> mpsc::UnboundedSender<NetworkMessage> {
        self.message_sender.clone()
    }
}

#[async_trait]
pub trait BlockchainNetwork {
    async fn broadcast_block(&mut self, block_hash: crate::hash::Blake2bHash, block_data: Vec<u8>) -> NetworkResult<()>;
    async fn request_block(&mut self, block_hash: crate::hash::Blake2bHash) -> NetworkResult<()>;
    async fn send_vote(&mut self, block_hash: crate::hash::Blake2bHash, approve: bool) -> NetworkResult<()>;
    async fn request_chain_state(&mut self) -> NetworkResult<()>;
}

#[async_trait]
impl BlockchainNetwork for P2PNetwork {
    async fn broadcast_block(&mut self, block_hash: crate::hash::Blake2bHash, block_data: Vec<u8>) -> NetworkResult<()> {
        let message = NetworkMessage::NewBlock { block_hash, block_data };
        self.broadcast_message(message).await
    }

    async fn request_block(&mut self, block_hash: crate::hash::Blake2bHash) -> NetworkResult<()> {
        let message = NetworkMessage::RequestBlock { block_hash };
        self.broadcast_message(message).await
    }

    async fn send_vote(&mut self, block_hash: crate::hash::Blake2bHash, approve: bool) -> NetworkResult<()> {
        let message = NetworkMessage::Vote {
            block_hash,
            validator_id: self.node_id.clone(),
            signature: vec![0u8; 32], // TODO: Real signature
            approve,
        };
        self.broadcast_message(message).await
    }

    async fn request_chain_state(&mut self) -> NetworkResult<()> {
        let message = NetworkMessage::RequestChainState;
        self.broadcast_message(message).await
    }
}