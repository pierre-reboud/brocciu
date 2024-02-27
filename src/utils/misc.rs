use chess::Color as ChessColor;
use lichess_api::model::Color as ApiColor;

pub fn api_to_chess_color(api_color: ApiColor) -> ChessColor {
    match api_color {
        ApiColor::Black => ChessColor::Black,
        ApiColor::White => ChessColor::White,
        _ => panic!("ApiColor Random received, how to handle??"),
    }
}
