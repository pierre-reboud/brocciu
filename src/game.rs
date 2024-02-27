use crate::api::ApiHandler;
use crate::mcts;
use crate::mcts::search::NoAvailableMoveError;
use crate::utils::misc::api_to_chess_color;
use chess::Color as ChessColor;
use lichess_api::model::challenges::ChallengeJson;
use rand::seq::SliceRandom;
use std::error::Error;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::mcts::search::Engine;
use lichess_api::model::board::stream::events::GameEventInfo;
use lichess_api::model::Color as ApiColor;
use log::debug;
use std::boxed::Box;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;

use std::fmt;

pub struct BotGame {
    id: String,
    game: Rc<RefCell<chess::Game>>,
    searcher: mcts::search::Searcher,
    pub bot_is: chess::Color,
}

impl BotGame {
    pub fn new_from_challenge(game_info: &GameEventInfo) -> BotGame {
        debug!("New game created with fen {:?}", &*game_info.fen);
        let board =
            chess::Board::from_str(&*game_info.fen).expect("Board could not be made from fen");
        let game = Rc::new(RefCell::new(chess::Game::new_with_board(board)));
        let searcher = mcts::search::Searcher::new(game.clone());
        BotGame {
            id: game_info.game_id.clone(),
            game: game,
            searcher: searcher,
            bot_is: api_to_chess_color(game_info.color.clone()),
        }
    }

    pub fn botgame_side_to_move(&mut self) -> chess::Color {
        let board = (*self.game).borrow().current_position();
        board.side_to_move()
    }

    pub fn enter_opponent_move(&mut self, move_chain: &str) -> () {
        let mut current_board = (*self.game).borrow().current_position();
        // Compute online board
        let mut default_board = chess::Board::default();
        let mut online_board = chess::Board::default();
        // Yield the online board by making all the chain moves from the default board
        move_chain
            .split(" ")
            .map(|mv| chess::ChessMove::from_str(mv).expect(&format!("{mv} is no valid ChessMove")))
            .for_each(|mv| {
                std::mem::swap(&mut default_board, &mut online_board);
                default_board.make_move(mv, &mut online_board);
            });
        // If both local and online boards different:
        if current_board != online_board {
            debug!("Board adjustment necesary, offline board is behind");
            let moves_iterator = chess::MoveGen::new_legal(&current_board);
            let last_opponent_move = move_chain
                .split(" ")
                .last()
                .expect(&format!("No last move found in {}", move_chain));
            let chess_move = chess::ChessMove::from_str(last_opponent_move).expect(&format!(
                "Unable to gather last opponent move {last_opponent_move} from chain: {move_chain}"
            ));
            // Assert opponent's move legality, panics if not the case
            assert!(
                current_board.legal(chess_move),
                "Move {:?} not in legal moves {:?}",
                chess_move,
                moves_iterator
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
            );
            debug!("Opponent Move entered: {:?}", chess_move);
            self.searcher.provide_opponent_move(chess_move);
        } else {
            debug!("Board adjustment unnecesary, both online/offline boards same");
        }
    }

    pub fn get_fen(&self) -> String {
        (*self.game).borrow().current_position().to_string().clone()
    }
}

pub fn yield_next_move(
    bot_game: Arc<Mutex<crate::game::BotGame>>,
    mut api: Arc<ApiHandler>,
) -> (String, bool) {
    debug!("Yield next move called");
    let next_move_receiver = api
        .pool
        .schedule_job(move || _yield_next_move(bot_game.clone()));
    let game_move = next_move_receiver.recv().unwrap();
    debug!("Next move {game_move} unwrapped from receiver");
    let offer_draw_flag = false;
    (game_move, offer_draw_flag)
}

fn _yield_next_move(bot_game: Arc<Mutex<BotGame>>) -> String {
    debug!("_yield_next_move_called");
    let mut game_guard = bot_game.lock().unwrap();
    let bot_color = game_guard.bot_is;
    let str_move = game_guard.searcher.get_next_move(bot_color);
    debug!("Next move generated");
    str_move.unwrap().0
    // let curr_pos = (*game_guard.game).borrow().current_position();
    // let moves_iterator = chess::MoveGen::new_legal(&curr_pos);
    // let moves = moves_iterator.collect::<Vec<chess::ChessMove>>();
    // let str_move = match moves.choose(&mut rand::thread_rng()){
    //     Some(mv) => {
    //         let str_move = format!("{}",mv);
    //         (*game_guard.game).borrow_mut().make_move(*mv);
    //         str_move
    //     }
    //     _ => "".to_string()
    // };
    // str_move
}

impl fmt::Debug for BotGame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BotGame: {}", self.id)
    }
}

unsafe impl Send for BotGame {}
