use lichess_api::api::board;
use rand::prelude::SliceRandom;
use rand::{thread_rng, Rng};
use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};
use std::thread::current;
use std::time::Instant;
use tokio::select;

use crate::utils::rc_wrapper::{HashableRcRefCell, NodeRef, WNodeRef};
use log::debug;
use std::fmt;

struct TreeParams {
    max_search_depth: usize,
    n_cutoff_moves: usize,
    max_search_time: f32,
}

pub struct Tree {
    nodes: HashSet<NodeRef>,
    head: NodeRef,
    params: TreeParams,
    rng: rand::rngs::ThreadRng,
}

impl Tree {
    pub fn new(initial_board: chess::Board, max_search_depth: usize) -> Tree {
        // Define parameters
        const MAX_SEARCH_TIME: f32 = 5_f32;
        const CUTOFF_MOVES: usize = 200;
        // Instantiate tree's parameters
        let params = TreeParams {
            max_search_depth,
            n_cutoff_moves: CUTOFF_MOVES,
            max_search_time: MAX_SEARCH_TIME,
        };
        // Create node with starting game position
        let head = HashableRcRefCell::new(Node::new(None, initial_board));
        // Create hashset of all tree nodes and insert the head
        let mut nodes = HashSet::<NodeRef>::new();
        nodes.insert(head.clone());

        // Return the tree
        Tree {
            nodes,
            head,
            params,
            rng: rand::thread_rng(),
        }
    }

    pub fn provide_opponent_move(&mut self, chess_move: chess::ChessMove) {
        // Prune all unreachable nodes
        self._prune_tree_based_on_move_and_update_head(chess_move);
    }

    pub fn yield_best_move(&mut self, color_to_play: chess::Color) -> chess::ChessMove {
        // Calculates the upper confidence bounds for each tree node
        self._populate_tree();
        // Select the best move based on the current estimate
        let chess_move = self._yield_best_move(color_to_play);
        // debug!("Best move: {}", movee);
        // Plot graph in critical situation
        // crate::utils::graph_visualization::draw_graph(&self.nodes, &self.head, "Tree");
        // panic!("Breakpoint");
        chess_move
    }

    fn _yield_best_move(&mut self, color_to_play: chess::Color) -> chess::ChessMove {
        // Compute argmax amongst children scores
        let best_child_index = (*self.head)
            .borrow()
            .children
            .iter()
            .map(|x| {
                // Compute pure win ratio
                (**x).borrow().get_win_ratio(Some(color_to_play))
            })
            .enumerate()
            .fold((0_usize, 0_f32), |(id_max, score_max), (i, score)| {
                if score > score_max {
                    (i, score)
                } else {
                    (id_max, score_max)
                }
            })
            .0;
        // Select best child
        let best_child = (*self.head).borrow().children[best_child_index].clone();
        // Convert best child to chess move
        let chess_move = Node::_get_move_diff(self.head.clone(), best_child);
        // Remove all but the selected children (and their now unreachable children) from the tree
        self._prune_tree_based_on_move_and_update_head(chess_move);
        chess_move
    }

    fn _prune_tree_based_on_move_and_update_head(&mut self, chess_move: chess::ChessMove) {
        // Define the chosen child node
        let board = (*self.head).borrow().board.clone();
        let mut target_board = chess::Board::default();
        board.make_move(chess_move, &mut target_board);
        // Chosen child node
        let best_child = HashableRcRefCell::new(Node::new(Some(self.head.clone()), target_board));
        // Drop all but the chosen child nodes
        (*self.head)
            .borrow()
            .children
            .iter()
            .filter(|x| **x != best_child)
            .for_each(|x| Self::drop_node(&mut self.nodes, &x));
        // Drop head from nodes
        self.nodes.remove(&self.head);
        // Update the tree's head
        self.head = best_child;
        // Clear the heads parents
        (*self.head).borrow_mut().parents.clear();
    }

    fn _populate_tree(&mut self) {
        debug!("Populate tree called");
        let now = Instant::now();
        let starting_node = self.head.clone();
        let mut n_iterations: usize = 0;
        loop {
            // debug!("Looping");
            let selected_node = self.select(starting_node.clone(), SelectionPolicy::UCT);
            // debug!("Selection done");
            let expanded_node = self.expand(&selected_node);
            // debug!("Expansion done");
            let result = self.simulate(&expanded_node, SimulationPolicy::Random);
            // debug!("Simulation done");
            self.backpropagate(&expanded_node.downgrade(), result);
            // debug!("Backpropagation done");
            // Time limit
            n_iterations += 1;
            if now.elapsed().as_secs_f32() > self.params.max_search_time {
                break;
            }
            // Tree already fully explored limit
            // @TODO
        }
        debug!("Populate tree left with {n_iterations} iterations");
        // Plot graph in critical situation
        // if n_iterations > 10000 { crate::utils::graph_visualization::draw_graph(&self.nodes, &self.head, "Tree");}
    }

    fn select(&mut self, root: NodeRef, selection_policy: SelectionPolicy) -> NodeRef {
        let leaf: NodeRef;
        // If root node has children
        let has_children = (*root).borrow()._has_children();
        let is_not_terminal = (*root).borrow()._is_not_terminal();
        if has_children {
            let selected_child_id = (*root)
                .borrow()
                .children
                .iter()
                // Gather score of each child node
                .map(|child_node| (**child_node).borrow().score)
                .enumerate()
                // Gather argmax of children scores
                .fold((0_usize, 0_f32), |(id_max, score_max), (i, score)| {
                    if score > score_max {
                        (i, score)
                    } else {
                        (id_max, score_max)
                    }
                })
                .0;
            let selected_child = (*root).borrow().children[selected_child_id].clone();
            leaf = self.select(selected_child, selection_policy);
        }
        // If root node has no children and is not terminal
        else if is_not_terminal {
            leaf = root;
        }
        // If root node has no children and is terminal
        else {
            leaf = root;
        }
        leaf.clone()
    }

    fn expand(&mut self, root: &NodeRef) -> NodeRef {
        // Asserts root hasno children
        assert!(!(**root).borrow()._has_children());
        // If target depth not yet reached
        if (**root).borrow().depth < self.params.max_search_depth + (*self.head).borrow().depth {
            // Get root board
            let current_board = (**root).borrow().board;
            // Get root legal moves
            let mut move_generator = chess::MoveGen::new_legal(&current_board);
            // let available_moves: Vec<chess::ChessMove> = move_generator.collect();
            let mut boards = move_generator.map(|x| {
                let mut target_board = chess::Board::default();
                current_board.make_move(x, &mut target_board);
                target_board
            });
            // Add each legal move to the tree
            self.add_children(root, boards);
        }
        // If expansion created new children
        if (**root).borrow()._has_children() {
            (**root)
                .borrow()
                .children
                .choose(&mut self.rng)
                .unwrap()
                .clone()
        }
        // Target depth reached or terminal board state reached
        else {
            root.clone()
        }
    }

    fn simulate(
        &mut self,
        root: &NodeRef,
        simulation_policy: SimulationPolicy,
    ) -> (chess::BoardStatus, chess::Color) {
        if let SimulationPolicy::Random = simulation_policy {
            // Tree leaf's board
            let mut board = (**root).borrow().board.clone();
            let mut target_board = chess::Board::default();
            for _ in 0..self.params.n_cutoff_moves {
                let mut move_generator = chess::MoveGen::new_legal(&board);
                let n_moves = move_generator.len();
                if n_moves > 0 {
                    let move_index = self.rng.gen_range(0, n_moves);
                    board.make_move(move_generator.nth(move_index).unwrap(), &mut target_board);
                    std::mem::swap(&mut board, &mut target_board);
                } else {
                    break;
                }
            }
            // Asserts that the game is indeed terminated
            // assert!(status == chess::BoardStatus::Checkmate || status == chess::BoardStatus::Stalemate);
            (board.status(), board.side_to_move())
        } else {
            panic!(
                "{} simulation policy not yet implemented.",
                simulation_policy
            );
        }
    }

    fn backpropagate(&mut self, leaf: &WNodeRef, status: (chess::BoardStatus, chess::Color)) {
        // Stop backpropagation is dead parent reached
        if leaf.upgrade().is_none() {
            return;
        }
        // Stop backpropagation if node already borrowed in DAG
        if let Ok(mut mut_leaf_node_ref) = leaf.upgrade().unwrap().try_borrow_mut() {
            // let leaf_ref = leaf.upgrade().unwrap();
            // let mut mut_leaf_node_ref = (*leaf_ref).borrow_mut();
            if let chess::BoardStatus::Checkmate = status.0 {
                // White won
                if let chess::Color::White = status.1 {
                    mut_leaf_node_ref.white_wins += 1_f32;
                }
                // Black won
                else {
                    mut_leaf_node_ref.white_wins += 0_f32;
                }
            }
            // Stalemate or simulation bound condition exceeded, ignores status.1
            else {
                mut_leaf_node_ref.white_wins += 0.5_f32;
            }
            // Update visit count
            mut_leaf_node_ref.visits += 1;
            // Update score
            mut_leaf_node_ref.score = mut_leaf_node_ref.get_score(SelectionPolicy::default());
            //Recursively backpropagate on parent nodes
            drop(mut_leaf_node_ref);
            let node_ref = leaf.upgrade().unwrap();
            for parent in &(*node_ref).borrow().parents {
                self.backpropagate(parent, status); // Backpropagate to each parent
            }
        }
    }

    fn add_children(&mut self, parent: &NodeRef, boards: impl Iterator<Item = chess::Board>) {
        // Collect iterator into vector
        let boards: Vec<chess::Board> = boards.collect();
        // Create vector of child nodes
        let mut child_nodes = Vec::<NodeRef>::with_capacity(boards.len());
        for board in boards {
            let cand_node = NodeRef::new(Node::new(Some(parent.clone()), board));
            // If board is already tracked, update target node's parents
            if let Some(existing_cand_node) = self.nodes.get(&cand_node) {
                (**existing_cand_node)
                    .borrow_mut()
                    .add_parent_node(parent.clone().downgrade());
            }
            // Else, update hashset with the newly created node
            else {
                self.nodes.insert(cand_node.clone());
            }
            // Append to children vector
            child_nodes.push(cand_node);
        }
        // Assign children vector to parent
        (**parent).borrow_mut().children = child_nodes;
    }

    fn drop_node(nodes: &mut HashSet<NodeRef>, root: &NodeRef) {
        //
        {
            if let Ok(root_ref) = (*root).try_borrow() {
                root_ref.children.iter().for_each(|x| {
                    let childs_parents_len = (**x).borrow().parents.len();
                    // Drop all child nodes that have root as sole parent
                    if childs_parents_len == 1 {
                        Self::drop_node(nodes, &x)
                    }
                    // Prune root from children having root and other parents
                    else if childs_parents_len > 1 {
                        if let Ok(mut mut_child_ref) = (**x).try_borrow_mut() {
                            mut_child_ref.parents.retain(|x| x.upgrade().is_some());
                            mut_child_ref
                                .parents
                                .retain(|x| *x.upgrade().unwrap() != **root);
                            if mut_child_ref.parents.is_empty() {
                                Self::drop_node(nodes, &x)
                            }
                        }
                    }
                });
            }
        }
        {
            if let Ok(mut mut_root_ref) = (**root).try_borrow_mut() {
                mut_root_ref.children.clear();
                mut_root_ref.parents.clear();
            }
        }
        // Removes hashset reference
        nodes.remove(root);
        // Memory should now be free as no more references point to the nodes
    }
}

pub struct Node {
    parents: Vec<WNodeRef>,
    pub children: Vec<NodeRef>,
    board: chess::Board,
    pub depth: usize,
    visits: usize,
    white_wins: f32,
    score: f32,
}

impl Node {
    pub fn new(parent: Option<NodeRef>, board: chess::Board) -> Node {
        let depth: usize;
        let mut parents: Vec<WNodeRef> = Vec::new();
        if let Some(parent) = parent {
            depth = (*parent).borrow_mut().depth + 1;
            parents.push(parent.downgrade() as WNodeRef);
        } else {
            depth = 0;
        }

        Node {
            parents: parents,
            children: Vec::<NodeRef>::new(),
            board: board,
            depth: depth,
            visits: 0,
            white_wins: 0.,
            score: 0.,
        }
    }

    fn add_child_node(&mut self, child: NodeRef) {
        self.children.push(child);
    }

    fn add_parent_node(&mut self, parent: WNodeRef) {
        self.parents.push(parent);
    }

    fn _has_children(&self) -> bool {
        self.children.len() > 0
    }

    pub fn get_score(&self, selection_policy: SelectionPolicy) -> f32 {
        let mut parent_visits: usize = self
            .parents
            .iter()
            // Remove dropped parents
            .filter(|x| x.upgrade().is_some())
            // Gather sum of all parent visits
            .map(|x| (*(*x).upgrade().unwrap()).borrow().visits)
            .sum();
        // Case where parent (n_parents > 1) is unexplored
        if parent_visits == 0 {
            parent_visits = 1;
        }
        // Pure win ratio score
        let win_score = self.get_win_ratio(None);
        let C: f32 = f32::sqrt(2.0);
        if let SelectionPolicy::UCT = selection_policy {
            let score: f32;
            if self.visits > 0 {
                score = win_score + C * ((parent_visits as f32).ln() / self.visits as f32).sqrt();
            } else {
                // Unvisited children should be explored with high priority
                score = f32::INFINITY;
            }
            if score.is_nan() {
                panic!("This should never happen: Win score {win_score}, parent_visits {parent_visits}, child visits {}, ln(parent_visits){}.", self.visits, (parent_visits as f32).ln());
            }
            score
        } else {
            panic!("{} selection policy not yet implemented.", selection_policy);
        }
    }

    fn get_win_ratio(&self, color_to_play: Option<chess::Color>) -> f32 {
        debug_assert!(
            self.board.side_to_move() == color_to_play.unwrap_or(self.board.side_to_move())
        );
        if self.board.side_to_move() == chess::Color::White {
            self.white_wins / (self.visits as f32)
        } else {
            1_f32 - (self.white_wins / self.visits as f32)
        }
    }

    fn _is_fully_expanded(&self) -> bool {
        let move_generator = chess::MoveGen::new_legal(&self.board);
        let legal_moves: Vec<chess::ChessMove> = move_generator.collect();
        // TODO Check if children are 1. legal and 2. unique
        legal_moves.len() == self.children.len()
    }

    fn _is_not_terminal(&self) -> bool {
        self.board.status() == chess::BoardStatus::Ongoing
    }

    fn _get_move_diff(parent_node: NodeRef, child_node: NodeRef) -> chess::ChessMove {
        let current_board = (*parent_node).borrow().board;
        let mut move_generator = chess::MoveGen::new_legal(&current_board);
        let potential_move_ids = (*parent_node)
            .borrow()
            .children
            .iter()
            .map(|x| (**x).borrow().board)
            .enumerate()
            .filter(|(_, board)| *board == (*child_node).borrow().board)
            .collect::<Vec<(usize, chess::Board)>>();
        assert!(potential_move_ids.len() == 1);
        let move_id = potential_move_ids[0].0;
        move_generator.nth(move_id).unwrap()
    }
    // #![cfg(debug_assertions)]
    // fn _check_move_tracking_mismatch(&self, available_moves: Vec<chess::ChessMove>, node_children: &Vec<NodeRef>){
    //     for (mv, node) in available_moves.iter().zip(node_children.iter()){
    //         assert!(mv == &(**node).borrow().board.get_move());

    // }
}

impl Hash for Node {
    fn hash<H: Hasher>(&self, hasher_state: &mut H) {
        self.board.get_hash().hash(hasher_state);
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.board.get_hash() == other.board.get_hash()
    }

    fn ne(&self, other: &Self) -> bool {
        self.board.get_hash() != other.board.get_hash()
    }
}

impl Eq for Node {}

#[derive(Debug, Default, Copy, Clone)]
pub enum SelectionPolicy {
    #[default]
    UCT,
    AlphaZero,
}

impl fmt::Display for SelectionPolicy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SelectionPolicy::UCT => write!(f, "UCT"),
            SelectionPolicy::AlphaZero => write!(f, "AlphaZero"),
        }
    }
}

#[derive(Debug, Default)]
pub enum ExpansionPolicy {
    #[default]
    Random,
}

impl fmt::Display for ExpansionPolicy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExpansionPolicy::Random => write!(f, "Random"),
        }
    }
}

#[derive(Debug, Default)]
pub enum SimulationPolicy {
    #[default]
    Random,
}

impl fmt::Display for SimulationPolicy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SimulationPolicy::Random => write!(f, "Random"),
        }
    }
}
