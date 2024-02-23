mod mcts;
mod game;
mod utils;
mod api;

use api::ApiHandler;
use tokio_stream::StreamExt;
use std::sync::{Arc, Mutex, MutexGuard};
use lichess_api::model::board::stream::events::{Event, GameEventInfo};
use lichess_api::model::bot::stream::game::{Event as BotGameEvent, GetQuery};
use lichess_api::model::bot::chat::{PostRequest as ChatPostRequest};
use lichess_api::client::LichessApi;
use std::boxed::Box;
use std::pin::Pin;
use log::{debug, LevelFilter};
use env_logger::Builder;
use crate::game::BotGame;
use crate::utils::parse_args::get_lorem_ipsum;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{

    // Debug
    std::env::set_var("RUST_BACKTRACE", "1");
    // std::env::set_var("RUST_BACKTRACE", "full");
    Builder::new()
        .filter(None, LevelFilter::Debug)
        .init();

    // Create processing queues
    let mut api_handle = Arc::new(api::ApiHandler::new().unwrap());

    // Launch event listening loop
    _ = streaming_loop(api_handle.clone()).await;

    // Get latest puzzle
    // let request = lichess_api::model::puzzles::daily::GetRequest::new();
    // let daily_puzzle = api.get_daily_puzzle(request).await?;
    // let pgn = daily_puzzle.game.pgn;

    // debug!("{}", pgn);
    Ok(())
}

async fn streaming_loop(api_handle: Arc<ApiHandler>) -> (){//Result<(), Box<dyn std::error::Error>>{
    // Request("error sending request for url (https://lichess.org/api/stream/event): error trying to connect: Connection reset by peer (os error 104)")
    let mut event_stream = api_handle.get_event_stream().await;
    while let Err(err) = event_stream{
        (event_stream) = api_handle.get_event_stream().await
    }
    let mut event_stream = event_stream.unwrap();
    // let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());
    debug!("Printing incoming events ... \n");
    // let rt = tokio::runtime::Builder::new_current_thread()
    //     .enable_all()
    //     .build()
    //     .unwrap();
    // std::thread::spawn( move || {
    //     let local_set = tokio::task::LocalSet::new();
    //     local_set.spawn_local( async move {
    //         while let Some(item) = event_stream.next().await{
    //             let event_api_handle = api_handle.clone();
    //             // let rt_handle = rt.clone();
    //             debug!("Received streaming loop event: {:?}", item);
    //             match item {
    //                 Ok(Event::Challenge {challenge: ref json}) => {
    //                     let mut accept_request = lichess_api::model::challenges::accept::PostRequest::new(json.base.id.to_string());
    //                     let accept_challenge_res = event_api_handle.lichess_api.accept_challenge(accept_request).await;
    //                     debug!("Challenge accepted");
    //                 }
    //                 Ok(Event::ChallengeDeclined {challenge: ref json}
    //                 | Event::ChallengeCanceled {challenge: ref json}) => {}
    //                 Ok(Event::GameStart {game}) => {
    //                     let mut game_handle = Arc::new(Mutex::new(BotGame::new_from_challenge(&game)));
    //                     let mut games_handle_guard = event_api_handle.game_handles.lock().unwrap();
    //                     games_handle_guard.insert(game.game_id.clone(), game_handle);
    //                     drop(games_handle_guard);
    //                     // event_api_handle.game_handles.lock().unwrap().insert(game.game_id, game_handle);
    //                     // let _ = bot_game_stream(event_api_handle.clone(), game.game_id);
    //                     tokio::task::spawn_local(
    //                             bot_game_stream(event_api_handle.clone(), game.game_id.clone())
    //                         );
    
    //                     // event_api_handle.thread_handles.insert(info.game_id, thread_handle);
    //                 }
    //                 Ok(Event::GameFinish {game: info}) => {
    //                     _ = event_api_handle.game_handles.lock().unwrap().remove(&info.game_id).unwrap();
    //                     // _ = tokio::join!(event_api_handle.thread_handles.remove(&info.game_id).unwrap())
    //                     // _ = tokio::join!(event_api_handle.thread_handles.remove(&info.game_id).unwrap().lock().unwrap());
    //                 }
    //                 Err(item) => {
    //                     panic!("Panic within stream handling: {:?}", item);
    //                 }
    //             }
    //         };
    //     });
    //     local_set.await;
    //     // rt.block_on(local_set);
    // });
// }
    // let local_set = tokio::task::LocalSet::new();
    // local_set.
    while let Some(item) = event_stream.next().await{
        let event_api_handle = api_handle.clone();
        // let rt_handle = rt.clone();
        debug!("Received streaming loop event: {:?}", item);
        match item {
            Ok(Event::Challenge {challenge: ref json}) => {
                let mut accept_request = lichess_api::model::challenges::accept::PostRequest::new(json.base.id.to_string());
                let accept_challenge_res = event_api_handle.lichess_api.accept_challenge(accept_request).await;
                debug!("Challenge accepted");
            }
            Ok(Event::ChallengeDeclined {challenge: ref json}
            | Event::ChallengeCanceled {challenge: ref json}) => {}
            Ok(Event::GameStart {game}) => {
                let mut game_handle = Arc::new(Mutex::new(BotGame::new_from_challenge(&game)));
                let mut games_handle_guard = event_api_handle.game_handles.lock().unwrap();
                games_handle_guard.insert(game.game_id.clone(), game_handle);
                drop(games_handle_guard);
                // event_api_handle.game_handles.lock().unwrap().insert(game.game_id, game_handle);
                // let _ = bot_game_stream(event_api_handle.clone(), game.game_id);
                let local_set = tokio::task::LocalSet::new();
                local_set.run_until(async move {
                        bot_game_stream(event_api_handle.clone(), game.game_id.clone())
                    }.await).await;

                // event_api_handle.thread_handles.insert(info.game_id, thread_handle);
            }
            Ok(Event::GameFinish {game: info}) => {
                _ = event_api_handle.game_handles.lock().unwrap().remove(&info.game_id).unwrap();
                // _ = tokio::join!(event_api_handle.thread_handles.remove(&info.game_id).unwrap())
                // _ = tokio::join!(event_api_handle.thread_handles.remove(&info.game_id).unwrap().lock().unwrap());
            }
            Err(item) => {
                panic!("Panic within stream handling: {:?}", item);
            }
        }
    };
}



async fn bot_game_stream(lichess_api: Arc<ApiHandler>, id: String) -> () {
    let request = lichess_api::model::bot::stream::game::GetRequest::new(id.as_str());
    let mut events_stream = lichess_api.lichess_api.bot_stream_board_state(request).await.unwrap();
    let mut game_id = String::from("");
    let mut last_sent_line = String::from("");
    while let Some(event) = events_stream.next().await{
        debug!("Received game loop event: {:?}", event);
        match event {
            Ok(BotGameEvent::GameFull {game_full}) => {
                game_id = game_full.id.clone();
                let game_state = game_full.state.unwrap();
                let games_hashmap = lichess_api.game_handles.lock().unwrap();
                let game = games_hashmap.get(&game_id).expect(&format!("Game id {} not found in {:?}", game_id, games_hashmap)).clone();
                let mut game_guard = game.lock().unwrap();
                let black_play_condition = chess::Color::Black == game_guard.bot_is && (game_state.moves.len() > 1 && game_state.moves.split(" ").count()%2 == 1); 
                let white_play_condition = chess::Color::White == game_guard.bot_is && (game_state.moves.len() == 0 || game_state.moves.split(" ").count()%2 == 0);
                if black_play_condition {
                    game_guard.enter_opponent_move(game_state.moves.as_str());
                }
                if !(white_play_condition | black_play_condition) {debug!("Waiting for opponent's move");}
                else {
                    // Make a move
                    drop(game_guard);
                    let (next_move, offer_draw) = crate::game::yield_next_move(game.clone(), lichess_api.clone());
                    let request = lichess_api::model::bot::r#move::PostRequest::new(id.as_str(), next_move.as_str(), offer_draw);
                    let make_move_res = lichess_api.lichess_api.bot_make_move(request).await; 
                }
            }
            Ok(BotGameEvent::GameState {game_state, ..}) => {
                match game_state.status.as_str(){
                    "started" => {}
                    _ => {debug!("Received game state event with status: {:?} => leaving game loop", game_state.status); break;} 
                }
                let games_hashmap = lichess_api.game_handles.lock().unwrap();
                let game = games_hashmap.get(&game_id).expect(&format!("Game id {} not found in {:?}", game_id, games_hashmap)).clone();
                let mut game_guard = game.lock().unwrap();
                let bot_is = game_guard.bot_is;
                // Infer side to play from string move sequence
                let side_to_play: chess::Color;
                let moves_count = game_state.moves.split(" ").filter(|x| *x!= "").count() % 2;
                match moves_count {
                    0 => side_to_play = chess::Color::White,
                    1 => side_to_play = chess::Color::Black,
                    _ => unreachable!()
                }
                debug!("GameState: bot_is {:?}, side to  play {:?}, moves {}, game fen {}", game_guard.bot_is, side_to_play, game_state.moves, game_guard.get_fen());
                if bot_is != side_to_play {
                    // Do nothing
                }
                else{
                    // Plays a move if offline board is behind.
                    game_guard.enter_opponent_move(&game_state.moves);
                    drop(game_guard);
                    // Make a move
                    let (next_move, offer_draw) = crate::game::yield_next_move(game.clone(), lichess_api.clone());
                    debug!("BotGameStream: 1. Request scheduled: move {next_move}; offer draw:{offer_draw}");
                    let request = lichess_api::model::bot::r#move::PostRequest::new(id.as_str(), next_move.as_str(), offer_draw);
                    let make_move_res = lichess_api.lichess_api.bot_make_move(request).await; //board_make_move(request).await;
                } 
                
            }
            Ok(BotGameEvent::ChatLine {chat_line}) => {}
            //     if chat_line.text.clone() != last_sent_line {
            //         // "Io ùn sò ancu capace di parlà. Per piacè, scusate stu disagiamentu."
            //         let new_line = get_lorem_ipsum();
            //         let chat_request = ChatPostRequest::new(&game_id, chat_line.room.clone(), &new_line);
            //         let chat_res = lichess_api.lichess_api.bot_write_in_chat(chat_request).await;
            //         last_sent_line = new_line;
            //         debug!("Chat Post request {:?}; last line {}", chat_res, last_sent_line);
            //     }
            // }
            Ok(BotGameEvent::OpponentGone {..}) => {break;}
            Err(item) => {
                panic!("Panic within stream handling: {:?}", item);
            }
        }
    }
}