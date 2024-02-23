pub struct Node<'a>{
    hash: &'a str,
    visits: u32,
    wins: u32,
    draws: u32,
    children: Vec<Node<'a>>
}

impl<'a> Node<'a>{
    pub fn new(hash: &'a str) -> Node{
        Node {
            hash: hash,
            visits: 0,
            wins: 0,
            draws: 0,
            children: Vec::new()
        }
    }

    pub fn update_statistics(&mut self, outcome: chess::GameResult) -> () {
        match outcome {
            chess::GameResult::WhiteCheckmates | chess::GameResult::BlackResigns => (*self).wins += 1,
            chess::GameResult::Stalemate | chess::GameResult::DrawAccepted | chess::GameResult::DrawDeclared => (*self).draws +=1,
            chess::GameResult::WhiteResigns | chess::GameResult::BlackCheckmates => {}
        }
        (*self).visits+=1;
    }

    pub fn append(&mut self, child: Node<'a>){
        (*self).children.push(child);
    }
}