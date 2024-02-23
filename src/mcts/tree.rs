use std::borrow::BorrowMut;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::cell::RefCell;
use std::rc::Rc;
use std::thread::current;
use lichess_api::api::board;
use rand::thread_rng;
use tokio::select;
use rand::prelude::SliceRandom;
use std::time::{Instant};

use crate::utils::rc_wrapper::{NodeRef, HashableRcRefCell};
use std::fmt;
use log::debug;


pub struct Tree {
    nodes: HashSet<NodeRef>,
    head: NodeRef,
    max_search_depth: usize,
}

impl Tree {
    pub fn new(initial_board: chess::Board, max_search_depth: usize) -> Tree{
        let head = HashableRcRefCell::new(Node::new(None, initial_board)); 
        Tree {
            nodes: HashSet::new(),
            head,
            max_search_depth
        }
    }

    pub fn provide_opponent_move(&mut self, chess_move: chess::ChessMove){
        // Prune all unreachable nodes
        self._prune_tree_based_on_move_and_update_head(chess_move);
    }

    pub fn yield_best_move(&mut self, color_to_play: chess::Color) -> chess::ChessMove{
        // Calculates the upper confidence bounds for each tree node
        self._populate_tree();
        // Select the best move based on the current estimate
        self._yield_best_move(color_to_play)
    }
    
    fn _yield_best_move(&mut self, color_to_play: chess::Color) -> chess::ChessMove{
        // Score inverter flag: 
        //      White: choose child node based on highest score
        //      Black: choose child node based on lowest score
        let score_inverter_flag: f32;
        match color_to_play{
            chess::Color::White => score_inverter_flag = 1_f32,
            chess::Color::Black => score_inverter_flag = -1_f32,
        }
        // Compute argmax amongst children scores
        let best_child_index = (*self.head).borrow().children
            .iter()
            .map(| x| score_inverter_flag * (*x).borrow().white_wins/(*x).borrow().visits)
            .enumerate()
            .fold((0_usize,0_f32),|(id_max, score_max),(i, score)| {
                if score > score_max {(i, score)}
                else{(id_max, score_max)}
            }).0;
        // Select best child
        let best_child = (*self.head).borrow().children[best_child_index].clone();
        // Convert best child to chess move
        let chess_move = Node::_get_move_diff(self.head.clone(), best_child);
        // Remove all but the selected children (and their now unreachable children) from the tree
        self._prune_tree_based_on_move_and_update_head(chess_move);
        chess_move            
    }

    fn _prune_tree_based_on_move_and_update_head(&mut self, chess_move: chess::ChessMove){
        // Define the chosen child node
        let board = (*self.head).borrow().board.clone();
        let mut target_board = chess::Board::default();
        board.make_move(chess_move, &mut target_board);
        // Chosen child node
        let best_child = HashableRcRefCell::new(Node::new(Some(self.head.clone()), target_board));
        // Drop all but the chosen child nodes
        let borrowed_head = (*self.head).borrow();
        borrowed_head.children
            .iter()
            .filter(|x| **x != best_child)
            .for_each(|x| Self::drop_node(&mut self.nodes, &x));
        drop(borrowed_head);
        // Drop head from nodes
        self.nodes.remove(&self.head);
        // Update the tree's head
        self.head = best_child;
        // Clear the heads parents
        (*self.head).borrow_mut().parents.clear();
    }

    fn _populate_tree(&mut self){
        const MAX_SEARCH_TIME: f32 = 5_f32;
        debug!("Populate tree called");
        let now = Instant::now();
        let starting_node = self.head.clone();
        let mut n_iterations: usize = 0;
        loop{
            // debug!("Looping");
            let selected_node = self.select(starting_node.clone(),SelectionPolicy::UCT);
            // debug!("Selection done");
            let expanded_node = self.expand(selected_node.clone());
            drop(selected_node);
            // debug!("Expansion done");
            let result = self.simulate(expanded_node.clone(), SimulationPolicy::Random);
            // debug!("Simulation done");
            self.backpropagate(&expanded_node, result);
            // debug!("Backpropagation done");
            // Time limit
            n_iterations += 1;
            if now.elapsed().as_secs_f32() > MAX_SEARCH_TIME{ break;}
            // Tree already fully explored limit
            // @TODO
        }
        debug!("Populate tree left with {n_iterations} iterations");
    }

    fn select(&mut self, root: NodeRef, selection_policy: SelectionPolicy) -> NodeRef{
        let leaf: NodeRef;
        // If root node has children
        if (*root).borrow()._has_children(){
            let selected_child_id = (*root).borrow().children
                .iter()
                .map(|child_node| (**child_node).borrow()._get_score(selection_policy))
                .enumerate()
                .fold((0_usize,0_f32),|(id_max, score_max),(i, score)| {
                    if score > score_max{(i, score)}
                    else{(id_max, score_max)}
                }).0;
            let selected_child = (*root).borrow().children[selected_child_id].clone();
            leaf = self.select(selected_child, selection_policy);
        }
        // If root node has no children and is not terminal
        else if (*root).borrow()._is_not_terminal(){
            leaf = root;
        }
        // If root node has no children and is terminal
        else{
            leaf = root;
        }
        leaf.clone()
    }

    fn expand(&mut self, root: NodeRef) -> NodeRef{
        // Asserts root hasno children
        assert!(!(*root).borrow()._has_children());
        // If target depth not yet reached
        if (*root).borrow().depth < self.max_search_depth + (*self.head).borrow().depth{
            // Get root board
            let current_board = (*root).borrow().board.clone();
            // Get root legal moves
            let move_generator = chess::MoveGen::new_legal(&current_board);
            let available_moves: Vec<chess::ChessMove> = move_generator.collect();
            // Add each legal move to the tree as a new node
            for legal_move in available_moves {
                let mut target_board = chess::Board::default();
                current_board.make_move(legal_move, &mut target_board);
                self.add_child(root.clone(), target_board);
            }
        }
        // If expansion created new children
        if (*root).borrow()._has_children(){
            let mut rng = rand::thread_rng();
            (*root).borrow().children.choose(&mut rng).unwrap().clone()
        }
        else{
            root
        }
    }

    fn simulate(&mut self, root: NodeRef, simulation_policy: SimulationPolicy, ) -> (chess::BoardStatus, chess::Color){
        if let SimulationPolicy::Random = simulation_policy {
            let mut board = (*root).borrow().board.clone();
            let mut rng = rand::thread_rng();
            const CUTOFF_MOVES: usize = 1000;
            let mut moves_count: usize = 0;
            loop{
                let mut target_board = chess::Board::default();
                let move_generator = chess::MoveGen::new_legal(&board);
                let legal_moves: Vec<chess::ChessMove> = move_generator.collect();
                if legal_moves.is_empty() || moves_count > CUTOFF_MOVES {
                    break;
                }
                let rand_move = legal_moves.choose(&mut rng).unwrap();
                board.make_move(*rand_move, &mut target_board);
                std::mem::swap(&mut board, &mut target_board);
                moves_count +=1;
                // debug!("Move {} simulated, board fen = {}", *rand_move, board.to_string())
            }
            // Asserts that the game is indeed terminated
            // assert!(status == chess::BoardStatus::Checkmate || status == chess::BoardStatus::Stalemate);
            (board.status(), board.side_to_move())
        }
        else{
            panic!("{} simulation policy not yet implemented.", simulation_policy);
        }
    }

    fn backpropagate(&mut self, leaf: &NodeRef, status: (chess::BoardStatus, chess::Color)){
        // If stop backpropagation if head node reached 
        if self.head == *leaf {return;}
        // debug!("N° parent {}", (*leaf).borrow().parents.len());
        // (*leaf).borrow().parents.iter().for_each(|x| debug!("Leaf's parents {:?}", (*x).borrow().board.to_string()));
        // debug!("Leaf depth {}, Head depth {}, Leaf board {}",leaf.borrow().depth, self.head.borrow().depth, leaf.borrow().board);
        if let Err(err) = (*leaf).try_borrow_mut() {
            debug!("error node \n depth {}, parents {}, children {}\n", (*leaf).borrow().depth, (*leaf).borrow().parents.len(), (*leaf).borrow().children.len() );
            (*leaf).borrow().parents.iter().enumerate().for_each(
                |(i,x)| debug!("Leaf's parent {i}: depth {}, parents {}, children {}, is in heads children {}\n\n", (*x).borrow().depth, (*x).borrow().parents.len(), (*x).borrow().children.len(), (*self.head).borrow().children.iter().any(|y| *x == *y))
            );
            debug!("head depth \n depth {}, parents {}, children {}", (*self.head).borrow().depth, (*self.head).borrow().parents.len(), (*self.head).borrow().children.len());    
            return;
        }
        {
            let mut leaf_node = (**leaf).borrow_mut();
            if let chess::BoardStatus::Checkmate = status.0 {
                // White won
                if let chess::Color::White = status.1 {
                    leaf_node.white_wins += 1_f32;
                }
                // Black won
                else{
                    leaf_node.white_wins += 0_f32;
                }
            }
            // Stalemate or too long to simulate, ignore status.1
            else{
                leaf_node.white_wins += 0.5_f32;
            }
            // Update visit count
            leaf_node.visits +=1_f32;
        }

        // Backpropagate result to all parents
        let leaf_ref = (*leaf).borrow();
        leaf_ref.parents.iter()
            .for_each(|x| {
                assert!(*x != *leaf); // Parent ref should never point to leaf
                self.backpropagate(x, status); // Backpropagate to each parent
            });
        drop(leaf_ref);
        // Backpropagate result to all parents
        // for parent in &leaf_node.parents{
        //     assert!(*parent != leaf);
        //     self.backpropagate(parent.clone(), status)
        // }
    }

    fn drop_node(nodes: &mut HashSet<NodeRef>, root: &NodeRef){
        unsafe {
            let pointer = (**root).as_ptr(); 
            (*pointer).children.iter()
                .for_each(|x| {
                    let childs_parents = &mut (**x).borrow_mut().parents;
                    // Drop all child nodes that have root as sole parent
                    if childs_parents.len() == 1 {Self::drop_node(nodes, x)}
                    // Prune root from children having root and other parents
                    else if childs_parents.len() > 1 {childs_parents.retain(|x| *x != *root);}
                    else {panic!("Drop called on a head node. Should not happen")}
            });
            (*pointer).children.clear();
            (*pointer).parents.clear();
        }
        // Removes hashset reference
        nodes.remove(&root);
        // Memory should now be free as no more references point to the nodes
    }

    fn add_child(&mut self, parent: NodeRef, board: chess::Board) {
        let node = Node::new(Some(parent.clone()), board);
        let mut node_ref = NodeRef::new(node);
        let node_already_tracked = !self.nodes.insert(node_ref.clone());
        // If board already tracked
        if node_already_tracked{
            // Update target node with already existing reference
            node_ref = self.nodes.get(&node_ref).unwrap().clone();
            // Append to node's upstream connections 
            (*node_ref).borrow_mut().add_parent_node(parent.clone());
        }
        // Add parent's downstream connection
        // let assertions = (*parent).borrow().parents.iter().for_each(|x| (*x) != node_ref).any();
        // debug_assert!(assertions);
        (*parent).borrow_mut().add_child_node(node_ref);
        
    }
}


pub struct Node{
    parents: Vec<NodeRef>,
    children: Vec<NodeRef>,
    board: chess::Board,
    depth: usize,
    visits: f32,
    white_wins: f32,
}

impl Node{
    pub fn new(parent: Option<NodeRef>, board: chess::Board) -> Node{
        let depth: usize;
        let mut parents: Vec<NodeRef> = Vec::new();
        if let Some(parent) = parent {
            depth = (*parent).borrow_mut().depth + 1;
            parents.push(parent);
        }
        else{
            depth = 0;
        }
         
        Node{
            parents: parents,
            children: Vec::<NodeRef>::new(),
            board: board,
            depth:  depth,
            visits: 0.,
            white_wins: 0.,
        }
    }

    fn add_child_node(&mut self, child: NodeRef){
        self.children.push(child);
    }

    fn add_parent_node(&mut self, parent: NodeRef){
        self.parents.push(parent);
    }

    fn _has_children(&self) -> bool{
        self.children.len() > 0
    }

    fn _get_score(&self, selection_policy: SelectionPolicy) -> f32{
        let parent_visits = self.parents.iter().map(|x| (**x).borrow().visits).sum();
        // Score reverser flag: 1 if white to play, -1 if black to play
        let score_reverser_flag = (-2_f32 * (self.depth%2_usize) as f32) + 1_f32; 
        const C: f32 = 1.;
        if let SelectionPolicy::UCT = selection_policy {
            let score: f32;
            if self.visits > 0.{
                score = score_reverser_flag*self.white_wins/self.visits + C * (f32::sqrt(f32::ln(parent_visits)/self.visits));
            }
            else{
                // Unvisited children should be explored with high priority
                score = f32::INFINITY;
                // panic!("Children visits must be > 0, but has value {}", self.visits);
            }
            score
        }
        else{
            panic!("{} selection policy not yet implemented.", selection_policy);
        }
    }

    fn _is_fully_expanded(&self) -> bool{
        let move_generator = chess::MoveGen::new_legal(&self.board);
        let legal_moves: Vec<chess::ChessMove> = move_generator.collect();
        // TODO Check if children are 1. legal and 2. unique
        legal_moves.len() == self.children.len()
    }

    fn _is_not_terminal(&self) -> bool{
        self.board.status() == chess::BoardStatus::Ongoing
    }

    fn _get_move_diff(parent_node: NodeRef, child_node: NodeRef) -> chess::ChessMove {
        let current_board = (*parent_node).borrow().board;
        let mut move_generator = chess::MoveGen::new_legal(&current_board);
        let potential_move_ids = (*parent_node).borrow().children
            .iter()
            .map(|x| (**x).borrow().board)
            .enumerate()
            .filter(|(_, board)| *board == (*child_node).borrow().board)
            .collect::<Vec<(usize, chess::Board)>>();
        assert!(potential_move_ids.len()==1);
        let move_id = potential_move_ids[0].0;
        move_generator.nth(move_id).unwrap()
    }
    // #![cfg(debug_assertions)]  
    // fn _check_move_tracking_mismatch(&self, available_moves: Vec<chess::ChessMove>, node_children: &Vec<NodeRef>){
    //     for (mv, node) in available_moves.iter().zip(node_children.iter()){
    //         assert!(mv == &(**node).borrow().board.get_move());

    // }
}

impl Hash for Node{
    fn hash<H: Hasher>(&self, hasher_state: &mut H) {
        self.board.get_hash().hash(hasher_state);
    }
}

impl PartialEq for Node{
    fn eq(&self, other: &Self) -> bool {
        self.board.get_hash() == other.board.get_hash()
    }

    fn ne(&self, other: &Self) -> bool {
        self.board.get_hash() != other.board.get_hash()
    }
}

impl Eq for Node{}

#[derive(Debug, Default, Copy, Clone)]
pub enum SelectionPolicy {
    #[default]
    UCT,
    AlphaZero
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