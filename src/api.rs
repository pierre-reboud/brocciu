use crate::utils;
use crate::utils::threadpool::ThreadPool;
use lichess_api::model::board::stream::events::Event;
use reqwest::Client;
use std::sync::{Arc,Mutex};
use std::collections::HashMap;
use std::thread::JoinHandle;
use tokio::task::JoinHandle as tJoinHandle;
use std::error::Error;
use std::pin::Pin;

pub struct ApiHandler{
    pub lichess_api: lichess_api::client::LichessApi<reqwest::Client>,
    pub queues: Mutex<Queues>,
    pub game_handles: Mutex<HashMap<String, Arc<Mutex<crate::game::BotGame>>>>,
    pub pool: utils::threadpool::ThreadPool
}

pub struct Queues{
    pub wait_queue: Vec<Event>,
    pub processing_queue: Vec<Event>,
}

impl Queues{
    pub fn new() -> Mutex<Queues>{
        Mutex::new(Queues{
            wait_queue: Vec::<Event>::new(),
            processing_queue: Vec::<Event>::new()
        })
    }
}

impl ApiHandler{
    pub fn new() -> Result<ApiHandler, Box<dyn std::error::Error>>{
        // Load API Information from JSON file
        let api_info = utils::parse_args::get_api_tokens();
        // Extract token string
        let token = api_info.map(|x| x.token).ok();
        // // Build API client
        let client: Client = reqwest::ClientBuilder::new().build().unwrap();
        let lichess_api = lichess_api::client::LichessApi::new(client, token);
        let queues = Queues::new();
        // let thread_handles = HashMap::<String, Mutex<tJoinHandle<()>>>::new();
        let game_handles = Mutex::new(HashMap::<String, Arc<Mutex<crate::game::BotGame>>>::new());
        let pool = ThreadPool::new(2);
        let api = ApiHandler{
            lichess_api,
            queues,
            game_handles,
            pool
        };
        Ok(api)
    }

    pub async fn get_event_stream(&self) -> Result<impl tokio_stream::StreamExt<Item = Result<Event, lichess_api::error::Error>>, lichess_api::error::Error>{
        let mut game_ids = Vec::<String>::new();
        // Stream incoming events
        let request = lichess_api::model::board::stream::events::GetRequest::new();
        let mut event_stream = self.lichess_api.bot_stream_incoming_events(request).await;
        event_stream
    }

    // pub async fn handle_event(&self, event: Event) -> Result<(), Box<dyn std::error::Error>>{
    //     // if lichess_api.processing_queue.push(event);
    //     let mut queues = self.queues.lock().unwrap();
    //     Ok(())
    // }



}

unsafe impl Send for ApiHandler{}
unsafe impl Sync for ApiHandler{}
