use crate::models::{Board, Move, Player, Point, CardSet};
use rand::prelude::*;

impl Board {
    pub fn legal_moves(&self) -> Vec<Move> {
        let mut moves = vec![];
        let pieces = self.player_pieces();
        let wind_spirit_pos = self.wind_spirit();
        let red_king_pos = self.red_king;
        let blue_king_pos = self.blue_king;
        let kings: Vec<Point> = [red_king_pos, blue_king_pos]    
             .iter()
            .filter_map(|&king| king)
            .collect();
        if self.extra_move_pending {
            let mut moves = vec![];
            if let Some(wind_spirit_pos) = self.wind_spirit() {
                let extra_card = self.extra_move_card.unwrap();
                for offset in extra_card.moves(false, true) {
                    let offset = match self.turn {
                        Player::Red => offset,
                        Player::Blue => -offset,
                    };
                    let dst = wind_spirit_pos + offset;

                    if dst.in_bounds()
                        && (!self.player_pieces().contains(&Some(dst)) || kings.contains(&dst))
                    {
                        // Prevent Wind Spirit from moving onto a King
                        if kings.contains(&dst) {
                            continue;
                        }

                        moves.push(Move::Move {
                            card: extra_card,
                            src: wind_spirit_pos,
                            dst,
                        });
                    }
                }
            }

            return moves;
        }
        
        for card in self.player_hand() {

            for src in pieces.iter().filter_map(|p| *p) {

                let is_wind_spirit = Some(src) == wind_spirit_pos;

                let is_king = self.player_king() == Some(src);

                let cached_moves: Vec<_> = card.moves(is_king, false);

                for offset in cached_moves {

                    if is_wind_spirit && CardSet::WayOfTheWind.cards().contains(&card) {
                        continue; // Skip this illegal move
                    }

                    let offset = match self.turn {
                        Player::Red => offset,
                        Player::Blue => -offset,
                    };
                    let dst = src + offset;

                    if dst.in_bounds() && (!pieces.contains(&Some(dst)) || is_wind_spirit) {

                        // Prevent Wind Spirit from moving onto a King
                        if is_wind_spirit && kings.contains(&dst) {
                            continue;
                        }

                        // Prevent pieces from moving onto Wind Spirit
                        if let Some(wind_spirit_pos) = wind_spirit_pos {
                            if dst == wind_spirit_pos {
                                continue;
                            }
                        }
                        
                        moves.push(Move::Move {
                            card: *card,
                            src,
                            dst,
                        });
                    }
                }
            }
        }
        if moves.len() > 0 {
            let opponent_pieces = self.opponent_pieces();
            let key = |game_move: &Move| match game_move {
                Move::Move { dst, .. } => match opponent_pieces.contains(&Some(*dst)) {
                    true => 0,
                    false => 1,
                },
                Move::Discard { .. } => 0,
            };
            moves.sort_by_cached_key(key);
            return moves;
        }
        // No moves, have to discard
        self
            .player_hand()
            .iter()
            .map(|&card| Move::Discard { card })
            .collect()
    }

    pub fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Move> {
        let mut moves = self.legal_moves();
    
        // Shuffle moves to randomize selection
        moves.shuffle(rng);
    
        // Validate each move using `try_move` before selecting
        for game_move in moves {
            if self.try_move(game_move).is_ok() {
                return Some(game_move); // Return the first valid move
            }
        }
    
        // Return None if no valid moves remain
        None
    }
    
}
