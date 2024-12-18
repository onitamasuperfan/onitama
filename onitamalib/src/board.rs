use crate::CardSet;
use enum_iterator::IntoEnumIterator;
use rand::prelude::*;
use std::collections::HashSet;

use crate::models::{Board, Card, GameSquare, GameSettings, GameState, Move, Player, Point};

impl Board {
    pub fn try_move(&self, game_move: Move) -> Result<GameState, String> {
        // Destructure only what we actually need
        let Board {
            wind_spirit,
            blue_king,
            blue_hand,
            red_king,
            red_hand,
            spare_card,
            extra_move_pending,
            turn,
            ..
        } = self;

        // If there's an extra move pending, route to try_extra_move
        if *extra_move_pending {
            return self.try_extra_move(game_move);
        }

        // Identify current and opponent kings
        let (player_king, opponent_king) = match turn {
            Player::Red => (red_king, blue_king),
            Player::Blue => (blue_king, red_king),
        };

        // Gather player's pieces (king + pawns + optional wind spirit)
        let player_pieces = self.player_pieces();

        // Parse the move
        let (card, src, dst) = match game_move {
            Move::Move { card, src, dst } => (card, src, dst),
            Move::Discard { card } => {
                // Only discard if no valid moves exist
                if self.can_move() {
                    return Err("Valid moves exist".to_string());
                }
                return self.discard_card(card);
            }
        };

        // Validations
        if !self.player_hand().contains(&card) {
            return Err("Card not in hand".to_string());
        }
        if !player_pieces.contains(&Some(src)) {
            return Err("No piece at source".to_string());
        }
        let move_wind_spirit = wind_spirit.map_or(false, |ws| ws == src);
        if move_wind_spirit && CardSet::WayOfTheWind.cards().contains(&card) {
            return Err("Wind Spirit cannot use a Way of the Wind card to move".to_string());
        }
        if out_of_bounds(dst) {
            return Err("Destination is out of bounds".to_string());
        }

        let raw_delta = dst - src;
        let delta = match turn {
            Player::Red => raw_delta,
            Player::Blue => -raw_delta,
        };

        let moving_king = *player_king == Some(src);
        let moves = card.moves(moving_king, false);
        if !moves.contains(&delta) {
            return Err("Move not valid for card".to_string());
        }

        if move_wind_spirit && (Some(dst) == *red_king || Some(dst) == *blue_king) {
            return Err("Wind Spirit cannot move onto a Master!".to_string());
        }

        // If a non-Wind Spirit piece tries to move onto your own piece, that's invalid
        if player_pieces.contains(&Some(dst)) && !(move_wind_spirit && self.player_pawns().contains(&Some(dst))) {
            return Err("Destination occupied by your piece".to_string());
        }

        // Set opponent's Temple Arch to goal_square
        let goal_square = match turn {
            Player::Red => Point { x: 2, y: 0 },
            Player::Blue => Point { x: 2, y: 4 },
        };

        // Move/swap pawns
        let mut player_pawns = self.player_pawns();
        let mut opponent_pawns = self.opponent_pawns();
        move_or_swap_pawns(&mut player_pawns, &mut opponent_pawns, src, dst, move_wind_spirit);

        // Check if we can enable extra move
        let extra_pending = self.enable_extra_move(card, src, dst);
        let em_card = if extra_pending { Some(card) } else { None };

        // If no extra move, we replace the used card with the spare
        let player_hand = if !extra_pending {
            replace_card(self.player_hand(), card, *spare_card)
        } else {
            *self.player_hand()
        };

        // Update king position if it moved
        let new_king_pos = if moving_king { Some(dst) } else { *player_king };

        // Wind spirit might have moved
        let new_wind_spirit = if move_wind_spirit { Some(dst) } else { *wind_spirit };

        // Build the updated board
        let updated_board = match turn {
            Player::Red => Board {
                wind_spirit: new_wind_spirit,
                blue_king: *blue_king,
                blue_pawns: opponent_pawns,
                blue_hand: *blue_hand,
                red_king: new_king_pos,
                red_pawns: player_pawns,
                red_hand: player_hand,
                spare_card: if extra_pending { *spare_card } else { card },
                extra_move_pending: extra_pending,
                extra_move_card: em_card,
                turn: if extra_pending { Player::Red } else { Player::Blue },
            },
            Player::Blue => Board {
                wind_spirit: new_wind_spirit,
                blue_king: new_king_pos,
                blue_pawns: player_pawns,
                blue_hand: player_hand,
                red_king: *red_king,
                red_pawns: opponent_pawns,
                red_hand: *red_hand,
                spare_card: if extra_pending { *spare_card } else { card },
                extra_move_pending: extra_pending,
                extra_move_card: em_card,
                turn: if extra_pending { Player::Blue } else { Player::Red },
            },
        };

        // Check if this move finishes the game
        if Some(dst) == *opponent_king || (moving_king && dst == goal_square) {
            return Ok(GameState::Finished {
                winner: *turn,
                board: updated_board,
            });
        }

        Ok(GameState::Playing { board: updated_board })
    }

    fn try_extra_move(&self, game_move: Move) -> Result<GameState, String> {
         let Board {
            wind_spirit,
            blue_king,
            blue_hand,
            red_king,
            red_hand,
            spare_card,
            extra_move_card,
            turn,
            ..
        } = self;

        // If no valid moves exist for extra move, auto-discard
        if !self.can_move() {
            let card = extra_move_card.unwrap();
            let player_hand = replace_card(self.player_hand(), card, *spare_card);
            let (red_hand, blue_hand) = match turn {
                Player::Red => (player_hand, *blue_hand),
                Player::Blue => (*red_hand, player_hand),
            };
    
            return Ok(GameState::Playing {
                board: Board {
                    wind_spirit: *wind_spirit,
                    blue_king: *blue_king,
                    blue_pawns: self.blue_pawns,
                    blue_hand,
                    red_king: *red_king,
                    red_pawns: self.red_pawns,
                    red_hand,
                    spare_card: card,
                    extra_move_pending: false,
                    extra_move_card: None,
                    turn: turn.invert(),
                },
            });
        }
    
        let (card, src, dst) = match game_move {
            Move::Move { card, src, dst } => (card, src, dst),
            Move::Discard { card } => {
                let mut updated_board = self.clone();
                updated_board.extra_move_pending = false;
                return updated_board.try_move(Move::Discard { card });
            }
        };

        let wind_spirit_pos = match wind_spirit {
            Some(pos) => pos,
            None => return Err("Wind Spirit is missing!".to_string()),
        };
        if src != *wind_spirit_pos {
            return Err("You must move the Wind Spirit".to_string());
        }
        if card != extra_move_card.unwrap() {
            return Err(format!("Must use {} to move", extra_move_card.unwrap()));
        }
    
        let (player_king, _opponent_king) = match turn {
            Player::Red => (red_king, blue_king),
            Player::Blue => (blue_king, red_king),
        };
    
        let goal_square = match turn {
            Player::Red => Point { x: 2, y: 0 },
            Player::Blue => Point { x: 2, y: 4 },
        };
    
        if out_of_bounds(dst) {
            return Err("Destination is out of bounds".to_string());
        }

        let raw_delta = dst - src;
        let delta = match turn {
            Player::Red => raw_delta,
            Player::Blue => -raw_delta,
        };
    
        let moves = card.moves(false, true);
        if !moves.contains(&delta) {
            return Err("Move not valid for card".to_string());
        }
        if Some(dst) == *red_king || Some(dst) == *blue_king {
            return Err("Wind Spirit cannot move onto a Master!".to_string());
        }
    
        let player_hand = replace_card(self.player_hand(), card, *spare_card);

        let mut player_pawns = self.player_pawns();
        let mut opponent_pawns = self.opponent_pawns();
        move_or_swap_pawns(&mut player_pawns, &mut opponent_pawns, src, dst, true);
    
        let wind_spirit = Some(dst);
        let player_king = *player_king;
  
        // Construct the updated board after the extra move
        let updated_board = match turn {
            Player::Red => Board {
                wind_spirit,
                blue_king: *blue_king,
                blue_pawns: opponent_pawns,
                blue_hand: *blue_hand,
                red_king: player_king,
                red_pawns: player_pawns,
                red_hand: player_hand,
                spare_card: card,
                extra_move_pending: false,
                extra_move_card: None,
                turn: Player::Blue,
            },
            Player::Blue => Board {
                wind_spirit,
                blue_king: player_king,
                blue_pawns: player_pawns,
                blue_hand: player_hand,
                red_king: *red_king,
                red_pawns: opponent_pawns,
                red_hand: *red_hand,
                spare_card: card,
                extra_move_pending: false,
                extra_move_card: None,
                turn: Player::Red,
            },
        };
    
        if player_king == Some(goal_square) {
            return Ok(GameState::Finished {
                winner: *turn,
                board: updated_board,
            });
        }
    
        Ok(GameState::Playing { board: updated_board })
    }

    pub fn new_with_settings(settings: GameSettings) -> Board { 
        let mut rng = thread_rng();
    
        // Decide if Wind Spirit is included
        let include_wind_spirit = !settings.disabled_card_sets.contains(&"WayOfTheWind".to_string())
            && (settings.force_wind_spirit_inclusion || rng.gen_bool(0.25));
  
        // Separate "Way of the Wind" cards
        let mut way_of_the_wind_cards = Vec::new();
        let mut other_cards = Vec::new();

        let disabled_card_sets: HashSet<CardSet> = settings
            .disabled_card_sets
            .iter()
            .filter_map(|s| std::str::FromStr::from_str(s).ok())
            .collect();

        for card_set in CardSet::into_enum_iter() {
            if !disabled_card_sets.contains(&card_set) {
                if card_set == CardSet::WayOfTheWind {
                    way_of_the_wind_cards.extend(card_set.cards());
                } else {
                    other_cards.extend(card_set.cards());
                }
            }
        }
    
        let num_wind_cards = if include_wind_spirit {
            settings.number_of_wind_cards.unwrap_or_else(|| {
                let chance = rng.gen_range(0.0..1.0);
                if chance < 0.10 {
                    0
                } else if chance < 0.25 {
                    1
                } else if chance < 0.60 {
                    2
                } else if chance < 0.75 {
                    3
                } else if chance < 0.90 {
                    4
                } else {
                    5
                }
            })
        } else {
            0
        };

        way_of_the_wind_cards.shuffle(&mut rng);
        other_cards.shuffle(&mut rng);

        // Distribute cards
        let (player_hand_red, player_hand_blue, spare_card) = match num_wind_cards {
            0 => {
                let red = [other_cards.pop().unwrap(), other_cards.pop().unwrap()];
                let blue = [other_cards.pop().unwrap(), other_cards.pop().unwrap()];
                let spare = other_cards.pop().unwrap();
                (red, blue, spare)
            }
            1 => {
                let red = [other_cards.pop().unwrap(), other_cards.pop().unwrap()];
                let blue = [other_cards.pop().unwrap(), other_cards.pop().unwrap()];
                let spare = way_of_the_wind_cards.pop().unwrap();
                (red, blue, spare)
            }
            2 => {
                let red = [way_of_the_wind_cards.pop().unwrap(), other_cards.pop().unwrap()];
                let blue = [way_of_the_wind_cards.pop().unwrap(), other_cards.pop().unwrap()];
                let spare = other_cards.pop().unwrap();
                (red, blue, spare)
            }
            3 => {
                let red = [way_of_the_wind_cards.pop().unwrap(), other_cards.pop().unwrap()];
                let blue = [way_of_the_wind_cards.pop().unwrap(), other_cards.pop().unwrap()];
                let spare = way_of_the_wind_cards.pop().unwrap();
                (red, blue, spare)
            }
            4 => {
                let red = [way_of_the_wind_cards.pop().unwrap(), way_of_the_wind_cards.pop().unwrap()];
                let blue = [way_of_the_wind_cards.pop().unwrap(), way_of_the_wind_cards.pop().unwrap()];
                let spare = other_cards.pop().unwrap();
                (red, blue, spare)
            }
            5 => {
                let red = [way_of_the_wind_cards.pop().unwrap(), way_of_the_wind_cards.pop().unwrap()];
                let blue = [way_of_the_wind_cards.pop().unwrap(), way_of_the_wind_cards.pop().unwrap()];
                let spare = way_of_the_wind_cards.pop().unwrap();
                (red, blue, spare)
            }
            _ => unreachable!(),
        };

        Board {
            wind_spirit: if include_wind_spirit {
                Some(Point { x: 2, y: 2 })
            } else {
                None
            },
            blue_king: Some(Point { x: 2, y: 0 }),
            blue_pawns: [
                Some(Point { x: 0, y: 0 }),
                Some(Point { x: 1, y: 0 }),
                Some(Point { x: 3, y: 0 }),
                Some(Point { x: 4, y: 0 }),
            ],
            blue_hand: player_hand_blue,
            red_king: Some(Point { x: 2, y: 4 }),
            red_pawns: [
                Some(Point { x: 0, y: 4 }),
                Some(Point { x: 1, y: 4 }),
                Some(Point { x: 3, y: 4 }),
                Some(Point { x: 4, y: 4 }),
            ],
            red_hand: player_hand_red,
            spare_card,
            extra_move_pending: false,
            extra_move_card: None,
            turn: Player::Red,
        }
    }

    pub fn new() -> Board {
        let settings = GameSettings::default();
        Board::new_with_settings(settings)
    }

    pub fn can_move(&self) -> bool {
        // If an extra move is pending, only the Wind Spirit can move using the extra_move_card
        if self.extra_move_pending {
            if let Some(wind_spirit_pos) = self.wind_spirit() {
                if let Some(extra_card) = self.extra_move_card {
                    // Attempt all wind moves
                    for &raw_delta in extra_card.moves(false, true).iter() {
                        let delta = match self.turn {
                            Player::Red => raw_delta,
                            Player::Blue => -raw_delta,
                        };
                        let dst = wind_spirit_pos + delta;
                        if dst.in_bounds() && ![self.red_king, self.blue_king].contains(&Some(dst)) {
                            return true;
                        }
                    }
                }
            }
            return false;
        }

        // Normal move check
        let player_pieces = self.player_pieces();
        let kings = [self.red_king, self.blue_king];

        for src in player_pieces.iter().filter_map(|&p| p) {
            for &card in self.player_hand() {
                let is_king = self.player_king() == Some(src);
                let is_spirit = self.wind_spirit() == Some(src);

                // Wind Spirit cannot use WayOfTheWind card
                if is_spirit && CardSet::WayOfTheWind.cards().contains(&card) {
                    continue;
                }
                for &raw_delta in card.moves(is_king, false).iter() {
                    let delta = match self.turn {
                        Player::Red => raw_delta,
                        Player::Blue => -raw_delta,
                    };
                    let dst = src + delta;

                    if dst.in_bounds() {
                        // Can't move onto the wind spirit position
                        if let Some(ws) = self.wind_spirit() {
                            if dst == ws {
                                continue;
                            }
                        }
                        // Can't move onto your own piece, unless Wind Spirit is swapping
                        if !player_pieces.contains(&Some(dst)) || is_spirit {
                            // Wind Spirit cant move onto a Master
                            if !(is_spirit && kings.contains(&Some(dst))) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    pub fn to_grid(&self) -> [[GameSquare; 5]; 5] {
        let mut grid = [[GameSquare::Empty; 5]; 5];
        for Point { x, y } in self.blue_pawns.iter().filter_map(|p| *p) {
            grid[y as usize][x as usize] = GameSquare::BluePawn;
        }
        for Point { x, y } in self.red_pawns.iter().filter_map(|p| *p) {
            grid[y as usize][x as usize] = GameSquare::RedPawn;
        }
        if let Some(Point { x, y }) = self.red_king {
            grid[y as usize][x as usize] = GameSquare::RedKing;
        }
        if let Some(Point { x, y }) = self.blue_king {
            grid[y as usize][x as usize] = GameSquare::BlueKing;
        }
        if let Some(Point { x, y }) = self.wind_spirit {
            grid[y as usize][x as usize] = GameSquare::WindSpirit;
        }
        grid
    }

    pub fn player_hand(&self) -> &[Card; 2] {
        match self.turn {
            Player::Red => &self.red_hand,
            Player::Blue => &self.blue_hand,
        }
    }

    pub fn opponent_hand(&self) -> &[Card; 2] {
        match self.turn.invert() {
            Player::Red => &self.red_hand,
            Player::Blue => &self.blue_hand,
        }
    }

    pub fn player_pawns(&self) -> [Option<Point>; 4] {
        match self.turn {
            Player::Red => self.red_pawns,
            Player::Blue => self.blue_pawns,
        }
    }

    pub fn opponent_pawns(&self) -> [Option<Point>; 4] {
        match self.turn.invert() {
            Player::Red => self.red_pawns,
            Player::Blue => self.blue_pawns,
        }
    }

    pub fn player_king(&self) -> Option<Point> {
        match self.turn {
            Player::Red => self.red_king,
            Player::Blue => self.blue_king,
        }
    }

    pub fn opponent_king(&self) -> Option<Point> {
        match self.turn.invert() {
            Player::Red => self.red_king,
            Player::Blue => self.blue_king,
        }
    }

    pub fn player_pieces(&self) -> Vec<Option<Point>> {
        let mut pieces = vec![self.player_king()];
        pieces.extend(self.player_pawns().iter().copied());
        if let Some(ws) = self.wind_spirit {
            pieces.push(Some(ws));
        }
        pieces
    }

    pub fn opponent_pieces(&self) -> Vec<Option<Point>> {
        let mut pieces = vec![self.opponent_king()];
        pieces.extend(self.opponent_pawns().iter().copied());
        pieces
    }

    pub fn wind_spirit(&self) -> Option<Point> {
        self.wind_spirit
    }

    fn enable_extra_move(&self, card: Card, moved_src: Point, moved_dst: Point) -> bool {
        if !CardSet::WayOfTheWind.cards().contains(&card) {
            return false;
        }

        let mut temp_board = self.clone();
        for pawn in temp_board.player_pawns().iter_mut().chain(temp_board.opponent_pawns().iter_mut()) {
            if let Some(pos) = pawn {
                if *pos == moved_src {
                    *pos = moved_dst;
                }
            }
        }

        // Update the King if it moved
        if temp_board.player_king() == Some(moved_src) {
            if self.turn == Player::Red {
                temp_board.red_king = Some(moved_dst);
            } else {
                temp_board.blue_king = Some(moved_dst);
            }
        }

        temp_board.extra_move_pending = true;
        temp_board.extra_move_card = Some(card);
        temp_board.can_move()
    }

    // Helper that discards a card
    fn discard_card(&self, card: Card) -> Result<GameState, String> {
        let player_hand = replace_card(self.player_hand(), card, self.spare_card);
        let (red_hand, blue_hand) = match self.turn {
            Player::Red => (player_hand, self.blue_hand),
            Player::Blue => (self.red_hand, player_hand),
        };

        Ok(GameState::Playing {
            board: Board {
                wind_spirit: self.wind_spirit,
                blue_king: self.blue_king,
                blue_pawns: self.blue_pawns,
                blue_hand,
                red_king: self.red_king,
                red_pawns: self.red_pawns,
                red_hand,
                spare_card: card,
                extra_move_pending: false,
                extra_move_card: None,
                turn: self.turn.invert(),
            },
        })
    }
}

impl GameState {
    pub fn new() -> GameState {
        log::info!("GameState::new() called");
        GameState::Playing {
            board: Board::new_with_settings(GameSettings::default()),
        }
    }

    pub fn new_with_settings(settings: GameSettings) -> GameState {
        log::info!("GameState::new_with_settings() called with settings: {:?}", settings);
        GameState::Playing {
            board: Board::new_with_settings(settings),
        }
    }

    pub fn finished(&self) -> bool {
        matches!(self, GameState::Finished { .. })
    }

    pub fn try_move(&self, game_move: Move) -> Result<GameState, String> {
        match self {
            GameState::Playing { board } => board.try_move(game_move),
            GameState::Finished { .. } => Err("Game already finished".to_string()),
        }
    }
}

fn replace_card(hand: &[Card; 2], used: Card, spare: Card) -> [Card; 2] {
    let [c1, c2] = hand;
    [
        if *c1 == used { spare } else { *c1 },
        if *c2 == used { spare } else { *c2 },
    ]
}

fn move_or_swap_pawns(
    player_pawns: &mut [Option<Point>; 4],
    opponent_pawns: &mut [Option<Point>; 4],
    src: Point,
    dst: Point,
    wind_spirit_moving: bool
) {
    for pawn in player_pawns.iter_mut() {
        match pawn {
            None => {}
            Some(pawn_pos) if *pawn_pos == src => {
                *pawn_pos = dst;
            }
            Some(pawn_pos) if *pawn_pos == dst && wind_spirit_moving => {
                *pawn_pos = src;
            }
            _ => {}
        }
    }

    for pawn in opponent_pawns.iter_mut() {
        match pawn {
            None => {}
            Some(pawn_pos) if *pawn_pos == dst => {
                if wind_spirit_moving {
                    *pawn_pos = src;
                } else {
                    *pawn = None;
                }
            }
            _ => {}
        }
    }
}

fn out_of_bounds(point: Point) -> bool {
    point.x < 0 || point.x > 4 || point.y < 0 || point.y > 4
}
