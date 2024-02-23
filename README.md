<div style="display:flex; justify-content:center">
    <img src="assets/ab2d340d999142aab8fc88abb1ee9c14.jpg" alt="Brocciu" style="max-width:20%; height:auto;">
</div>

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://opensource.org/licenses/MIT)

# Brocciu
**Brocciu** is a simple interface to access the [Lichess-Api](https://lichess.org/api) programmatically and lets a custom bot engine intercept incoming challenges and react to them in parallel. Furthermore, it comes with a simple chess engine written in [Rust](https://www.rust-lang.org) which implements the [Monte-Carlo-Tree-Search](https://en.wikipedia.org/wiki/Monte_Carlo_tree_search) algorithm using the straight-forward but slow ```Rc<RefCell<Node>>``` data structure. For more performant (and complex/unsafe) graph data structure solutions, the discussion on [Graphs and arena allocation](https://github.com/nrc/r4cppp/blob/master/graphs/README.md) is worth a read.

### Usage
#### Api config setup
In the json file ```/configs/default_api.json```, enter your lichess username and token. The token can be obtained by following the [bot instructions](https://lichess.org/api#tag/Bot/operation/botAccountUpgrade). Subsequently, rename the file to ```/configs/api.json```. 

#### Simple Example
Use the provided example chess engine:
```rust
use brocciu;
use tokio;

// Create a runtime environment
#[tokio::main]
async fn main(){
    // Run brocciu
    brocciu::main().await;
}
```

#### TODO: Advanced Example
Use your own engine, by letting it implement the ```Engine``` trait:
```rust
use brocciu;
use tokio;

struct MyEngine{}

/* The Engine trait

pub trait Engine{
    fn new(game: Rc<RefCell<chess::Game>>) -> Self;
    fn get_next_move(&mut self, bot_color: chess::Color) -> Result<(String, bool), NoAvailableMoveError>;
}

*/

impl brocciu::mcts::search::Engine<MyEngine> for MyEngine{
    pub fn new(game: Rc<RefCell<chess::Game>>) -> MyEngine {
        // Your implementation
    }
    pub fn get_next_move() -> Result<(String, bool), brocciu::mcts::search::NoAvailableMoveError>{
        // Your implementation
    }
}

// Create a runtime environment
#[tokio::main]
async fn main(){
    // Run brocciu

    brocciu::run()
}
```

### Feature Tracking
This project only offers the most bare-bone features necessary for functionality. The following features have yet to be implemented
State | Comment 
---|---
:x: | **Selection Policy**: Currently only UCT -> Add more refined node selection policies
:x: | **Simulation Policy**: Currently random self-play -> Add more refined node simulation policies (e.g. with NNs)
:x: | **Simulation Time Dynamization**: Currently, each move generation takes a constant amount of time except when reaching fully explored tree states -> Allocate different search time budgets at different game stages
:x: | **Simulation Break Condition**: Currently constant depth break condition -> Break simulate step when position obviously leads to stalemate
:x: | **Challenge Initiation**: Currently, bot can only react to exogeneous challenges -> Initiate challenges against the computer
:x: | **Challenge Types**: Currently, only regular untimed challenge types supported. Non-standard (and timed) challenges result in undefined behavior -> Accept different challenge types;
:x: | **Tree Data Structure**: Current node data structure is ```Rc<RefCell<Node>>``` -> Use more efficient node data structure
:x: | Other 

### Contributing
Pull requests are welcome. Please open an issue to describe the desired feature. No ETA implied.