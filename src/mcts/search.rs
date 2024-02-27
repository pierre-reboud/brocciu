use chess::ChessMove;
use core::fmt;
use rand::prelude::SliceRandom;
use rand::Error;
use std::boxed::Box;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::{borrow::BorrowMut, io::Read};

pub trait Engine<T> {
    fn new(game: Rc<RefCell<chess::Game>>) -> T;
    fn get_next_move(
        &mut self,
        bot_color: chess::Color,
    ) -> Result<(String, bool), NoAvailableMoveError>;
}

pub struct Searcher {
    tree: super::tree::Tree,
    game: Rc<RefCell<chess::Game>>,
}

impl Engine<Searcher> for Searcher {
    fn new(game: Rc<RefCell<chess::Game>>) -> Self {
        const MAX_SEARCH_DEPTH: usize = 15;
        let current_board = (*game).borrow().current_position();
        let tree = crate::mcts::tree::Tree::new(current_board, MAX_SEARCH_DEPTH);
        Searcher { tree, game }
    }

    fn get_next_move(
        &mut self,
        bot_color: chess::Color,
    ) -> Result<(String, bool), NoAvailableMoveError> {
        self._get_next_move_mcts(bot_color)
    }
}

impl Searcher {
    // #[test]
    fn _get_next_move_random(&mut self) -> Result<(String, bool), NoAvailableMoveError> {
        let curr_pos = (*self.game).borrow().current_position();
        let moves_iterator = chess::MoveGen::new_legal(&curr_pos);
        let moves: Vec<chess::ChessMove> = moves_iterator.enumerate().map(|(_, x)| x).collect();
        let str_move = match moves.choose(&mut rand::thread_rng()) {
            Some(mv) => {
                let str_move = format!("{}", mv);
                let offer_draw = false;
                (*self.game)
                    .borrow_mut()
                    .current_position()
                    .make_move_new(mv.clone());
                Ok((str_move, offer_draw))
            }
            _ => Err(NoAvailableMoveError {}),
        };
        str_move
    }

    fn _get_next_move_mcts(
        &mut self,
        my_color: chess::Color,
    ) -> Result<(String, bool), NoAvailableMoveError> {
        let best_move = self.tree.yield_best_move(my_color);
        (*self.game).borrow_mut().make_move(best_move);
        let offer_draw = false;
        Ok((best_move.to_string(), offer_draw))
    }

    pub fn provide_opponent_move(&mut self, chess_move: chess::ChessMove) {
        // Make the move within the game's board
        (*self.game).borrow_mut().make_move(chess_move);
        // Propagate move to tree
        self.tree.provide_opponent_move(chess_move);
    }
}

#[derive(Debug)]
pub struct NoAvailableMoveError {}

impl fmt::Display for NoAvailableMoveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An error occurred")
    }
}

impl std::error::Error for NoAvailableMoveError {}
