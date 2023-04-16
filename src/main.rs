use std::collections::HashMap;
use std::time::Duration;
use tokio::{
    sync::mpsc,
    sync::mpsc::{Receiver, Sender},
    time::sleep,
};

use strum_macros::{
    EnumString,
    self,
    Display
};
use strum::EnumProperty;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Hash, EnumString)]
pub enum ActorTypes {
    ActorA,
    ActorB,
}

#[derive(Default, Clone, Debug)]
pub struct ActorAMessage {
    pub data: String,
}

#[derive(Default, Clone, Debug)]
pub struct ActorBMessage {
    pub data: String,
}

#[derive(Default, Clone, Debug)]
pub struct AllActorsMessage {
    pub data: String,
}


trait MessageActor {
    fn actors(&self) -> Vec<ActorTypes>;
}

#[derive(Clone, strum_macros::EnumProperty, Display, Debug)]
pub enum Message {
    #[strum(props(actors = "ActorA"))]
    ActorAMessage(ActorAMessage),
    #[strum(props(actors = "ActorB"))]
    ActorBMessage(ActorBMessage),
    #[strum(props(actors = "ActorA,ActorB"))]
    AllActorsMessage(AllActorsMessage),

}

impl MessageActor for Message {
    fn actors(&self) -> Vec<ActorTypes> {
        if let Some(actors_as_string) = self.get_str("actors") {
            let mut actor_types = vec![];
            for actor_as_string in actors_as_string.split(",").clone() {
                if let Ok(actor) = ActorTypes::from_str(actor_as_string) {
                    actor_types.push(actor);
                } else {
                    // TODO add logging, this shouldn't fail
                }

            }
            return actor_types;
        }
        // TODO add logging, this shouldn't fail

        vec![]
    }
}

struct ActorA {}
impl ActorA {
    pub fn start(mut receiver: Receiver<Message>) {
        tokio::spawn(async move {
            while let Some(message_enum) = receiver.recv().await {
                match message_enum {
                    Message::ActorAMessage(message) => {
                        println!("ActorA Received message - message data: {}", message.data);
                    }
                    Message::AllActorsMessage(message) => {
                        println!("ActorA Received message - message data: {}", message.data);
                    }
                    _ => {}
                }
            }
        });
    }
}



struct ActorB {}
impl ActorB {
    pub fn start(mut receiver: Receiver<Message>, bus_sender: Sender<Message>) {
        tokio::spawn(async move {
            while let Some(message_enum) = receiver.recv().await {
                match message_enum {
                    Message::ActorBMessage(message) => {
                        println!("ActorB Received message - message data: {}", message.data);
                    }
                    Message::AllActorsMessage(message) => {
                        println!("ActorB Received message - message data: {}", message.data);
                    }
                    _ => {}
                }
            }
        });

        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(3)).await;
                let message = Message::ActorBMessage(ActorBMessage { data: "A, are you still theading? from B".to_string() });
                bus_sender.send(message).await;
            }

        });
    }
}

pub struct MessageBus {}

impl MessageBus {
    pub fn start(mut receiver: Receiver<Message>, subscriptions: HashMap<ActorTypes, Sender<Message>>) {
        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                for actor_type in message.actors() {
                    let message_to_resend = message.clone();
                    let option = subscriptions.get(&actor_type);
                    if let Some(sender) = option {
                        sender.send(message_to_resend).await;
                    }
                }
            }
        });
    }
}


pub fn create_channels() -> (Sender<Message>, Receiver<Message>) {
    let (tx, rx): (Sender<Message>, Receiver<Message>) = mpsc::channel(100);

    (tx, rx)
}

#[tokio::main]
async fn main() {

    let actor_a_channels = create_channels();
    let message_bus_channels = create_channels();
    let actor_b_channels = create_channels();

    let mut subscriptions = HashMap::new();
    subscriptions.insert(ActorTypes::ActorA, actor_a_channels.0);
    subscriptions.insert(ActorTypes::ActorB, actor_b_channels.0);


    MessageBus::start(message_bus_channels.1, subscriptions);
    ActorA::start(actor_a_channels.1);
    ActorB::start(actor_b_channels.1, message_bus_channels.0.clone());

    let message_to_a = Message::ActorAMessage(ActorAMessage { data: "hello ActorA".to_string() });
    message_bus_channels.0.send(message_to_a).await;

    let message_to_b = Message::ActorBMessage(ActorBMessage { data: "hello ActorB".to_string() });
    message_bus_channels.0.send(message_to_b).await;


    let message_to_all = Message::AllActorsMessage(AllActorsMessage { data: "hello both ActorA and ActorB".to_string() });
    message_bus_channels.0.send(message_to_all).await;


    loop {
        sleep(Duration::from_secs(1)).await;
        println!("tick")
    }
}
