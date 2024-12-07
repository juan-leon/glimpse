use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::ws::Message;

pub type Clients = Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>>;
