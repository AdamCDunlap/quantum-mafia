#![allow(dead_code)]
extern crate ws;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate quantum_mafia;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak as RcWeak};
use std::cell::RefCell;
use quantum_mafia as qm;

#[derive(Deserialize, Debug)]
enum Command {
    SetName(String),
    EnterGame(usize),
    CreateGame(String),
}

#[derive(Deserialize, Debug)]
struct RecvMessage {
    cmd: Command,
}

#[derive(Serialize, Debug)]
enum SetClientState<'a> {
    SetNickname {},
    WaitingRoom {
        nickname: &'a str,
        waiting_room_people: &'a [String],
        open_games: &'a [(usize, String, Vec<String>)],
    },
    UnstartedGame {
        nickname: &'a str,
        game_name: &'a str,
        people_in_game: &'a [String],
        //gameid: usize,
        //waiting_room_people: &'a [String],
        //open_games: &'a [(usize, String, Vec<String>)],
    },
}

impl<'a> Into<ws::Message> for SetClientState<'a> {
    fn into(self) -> ws::Message {
        serde_json::to_string(&self).unwrap().into()
    }
}

#[derive(Debug)]
enum GameState {
    WaitingToStart,
    Playing(qm::QuantumMafia),
}

#[derive(Debug)]
struct Game {
    players: Vec<RcWeakRefCellWithHash<Player>>,
    state: GameState,
    name: String,
}

#[derive(Debug)]
struct Player {
    name: String,
    game: Option<RcWeakRefCellWithHash<Game>>,

    ws_sender: MySender,
}

fn main() {

    //let test = SetClientState::SetNickname {};
    //println!("Serialized is {}", serde_json::to_string(&test).unwrap());

    //let test = SetClientState::WaitingRoom {
    //    nickname: "Lisa",
    //    waiting_room_people: &["Lisa".to_string(), "Aaron".to_string(), "Adam".to_string()],
    //    open_games: &[(42, "Foobar",  vec!["Adam".to_string(),  "Joe".to_string() ] ),
    //                  (67, "Monkies", vec!["Aaron".to_string(), "Bob".to_string() ] )],
    //};
    //println!("Serialized is {}", serde_json::to_string(&test).unwrap());

    let players: RefCell<HashMap<ws::util::Token, Rc<RefCell<Player>>>> =
        RefCell::new(HashMap::new());
    //let waiting_room_people: RefCell<HashSet<ws::util::Token>> = RefCell::new(HashSet::new());
    let waiting_room_people: RefCell<HashSet<RcWeakRefCellWithHash<Player>>> =
        RefCell::new(HashSet::new());
    let games: RefCell<HashMap<usize, Rc<RefCell<Game>>>> = RefCell::new(HashMap::new());

    let players_ref = &players;
    let waiting_room_people_ref = &waiting_room_people;
    let games_ref = &games;

    ws::listen("0.0.0.0:3918", |out| {
        struct H<'a> {
            out: ws::Sender,
            players: &'a RefCell<HashMap<ws::util::Token, Rc<RefCell<Player>>>>,
            waiting_room_people: &'a RefCell<HashSet<RcWeakRefCellWithHash<Player>>>,
            games: &'a RefCell<HashMap<usize, Rc<RefCell<Game>>>>,
        }

        impl<'a> H<'a> {
            fn send_update_to_player(&self, player: &Player) -> Result<(), Error> {
                match player.game {
                    Some(ref game) => {
                        let game = game.upgrade();
                        let game = game.unwrap();
                        let game = game.borrow();

                        let ref people_in_game = game.players
                            .iter()
                            .map(|p| {
                                     let p = p.upgrade();
                                     let p = p.unwrap();
                                     let p = p.borrow();
                                     p.name.clone()
                                 })
                            .collect::<Vec<_>>();


                        player
                            .ws_sender
                            .send(SetClientState::UnstartedGame {
                                      nickname: &player.name,
                                      game_name: &game.name,
                                      people_in_game,
                                  })
                            .map_err(Error::WsError)?;
                    }

                    // Waiting room!
                    None => {

                        let ref waiting_room_people: Vec<_> = self.waiting_room_people
                            .borrow()
                            .iter()
                            .map(|p| {
                                     let p = p.upgrade().unwrap();
                                     let p = p.borrow();
                                     p.name.clone()
                                 })
                            .collect();
                        let ref open_games: Vec<_> = self.games
                            .borrow()
                            .iter()
                            .filter_map(|(id, game)| {
                                let game = game.borrow();
                                if let GameState::WaitingToStart = game.state {
                                } else {
                                    return None;
                                }
                                let player_names = game.players
                                    .iter()
                                    .map(|p| {
                                             let p = p
                                                 .upgrade()
                                                 .expect("Open game references dead player");
                                             let p = p.borrow();
                                             p.name.clone()
                                         })
                                    .collect();
                                Some((*id, game.name.clone(), player_names))
                            })
                            .collect();

                        player
                            .ws_sender
                            .send(SetClientState::WaitingRoom {
                                      nickname: &player.name,
                                      waiting_room_people,
                                      open_games,
                                  })
                            .map_err(Error::WsError)?;
                    }
                }
                Ok(())
            }
        }


        impl<'a> ws::Handler for H<'a> {
            fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
                match (move || {
                    if let ws::Message::Text(ref msg) = msg {
                        let msg: RecvMessage = serde_json::from_str(msg)
                            .map_err(Error::DecodeError)?;

                        //let remove_game = |game: RcWeakRefCellWithHash<Game>| {
                        //    let gid = game.0.upgrade().unwrap().as_ptr() as usize;
                        //};

                        let player_name = {
                            let players = self.players.borrow();
                            let player = players
                                .get(&self.out.token())
                                .expect("on_message called with token not in players map")
                                .borrow();
                            player.name.clone()
                        };

                        println!("Player infos = {:#?}", self.players);

                        let token = self.out.token();

                        println!("Got {:#?} from {:#?}.", msg, player_name);

                        let (update_lobby, update_game) = match msg.cmd {
                            Command::SetName(name) => {
                                let mut players = self.players.borrow_mut();
                                let player_rc =
                                    players
                                        .get_mut(&self.out.token())
                                        .expect("on_message called with token not in players map");
                                let mut player = player_rc.borrow_mut();
                                player.name = name.clone();

                                self.waiting_room_people
                                    .borrow_mut()
                                    .insert(RcWeakRefCellWithHash(Rc::downgrade(player_rc)));

                                (true, false)
                            }
                            Command::EnterGame(gameid) => {
                                println!("Adding {} to {}", player_name, gameid);
                                {

                                    let mut players = self.players.borrow_mut();
                                    let player_rc = &players
                                        .get_mut(&token)
                                        .expect("on_message called with token not in players map");
                                    let mut player = player_rc.borrow_mut();

                                    let mut games = self.games.borrow_mut();
                                    let game_rc = games
                                            .get_mut(&gameid)
                                            .ok_or(Error::InvalidReference(
                                                format!("Trying to enter nonexistant game {}",
                                                        gameid)))?;
                                    let mut game = game_rc.borrow_mut();
                                    self.waiting_room_people
                                        .borrow_mut()
                                        .remove(&RcWeakRefCellWithHash(Rc::downgrade(player_rc)));
                                    game.players
                                        .push(RcWeakRefCellWithHash(Rc::downgrade(player_rc)));
                                    player.game =
                                        Some(RcWeakRefCellWithHash(Rc::downgrade(&game_rc)));
                                }

                                let players = self.players.borrow();
                                let player_rc = &players[&token];
                                let player = player_rc.borrow();

                                // Tell the player that they're in a game.
                                self.send_update_to_player(&player)?;
                                (true, true)
                            }
                            Command::CreateGame(name) => {
                                println!("Creating game with name {}", name);
                                // TODO
                                {
                                    let mut games = self.games.borrow_mut();
                                    let mut players = self.players.borrow_mut();
                                    let player_rc = players
                                        .get_mut(&token)
                                        .expect("on_message called with token not \
                                                     in players map");
                                    let mut player = player_rc.borrow_mut();
                                    let game =
                                        Rc::new(RefCell::new(Game {
                                                                 players:
                                                                     vec![
                                                             RcWeakRefCellWithHash(
                                                                 Rc::downgrade(&player_rc))],
                                                                 state: GameState::WaitingToStart,
                                                                 name: name.clone(),
                                                             }));
                                    let id = game.as_ptr() as usize;
                                    player.game = Some(RcWeakRefCellWithHash(Rc::downgrade(&game)));
                                    games.insert(id, game);
                                    self.waiting_room_people
                                        .borrow_mut()
                                        .remove(&RcWeakRefCellWithHash(Rc::downgrade(player_rc)));
                                }

                                (true, true)
                            }
                        };

                        // Send updates to everyone in lobby
                        if update_lobby {
                            for player in self.waiting_room_people.borrow().iter() {
                                let player = player.upgrade().unwrap();
                                let player = player.borrow();
                                self.send_update_to_player(&player)?;
                            }
                        }

                        // Send updates to everyone in our game
                        if update_game {
                            let players = self.players.borrow();
                            let player_rc =
                                players
                                    .get(&token)
                                    .expect("on_message called with token not in players map");
                            let player = player_rc.borrow();

                            let game = player
                                .game
                                .as_ref()
                                .expect("Updating game but player not in game")
                                .upgrade()
                                .expect("Player references dead game");
                            let game = game.borrow();

                            for p in &game.players {
                                let p = p.upgrade().expect("Game references dead player");
                                self.send_update_to_player(&p.borrow())?;
                            }
                        }

                        Ok(())
                    } else {
                        Err(Error::WsError(ws::Error {
                                               kind: ws::ErrorKind::Protocol,
                                               details: Cow::from("Non-text data sent"),
                                           }))
                    }
                })() {
                    Ok(()) => Ok(()),
                    // Handle errors in dealing with the message. Usually, this
                    // means sending another message to the client to tell them
                    // there's an error.
                    Err(Error::WsError(e)) => Err(e),
                    Err(e) => {
                        println!("Error! {:?}", e);
                        Ok(())
                    }
                }
            }

            fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
                self.players
                    .borrow_mut()
                    .insert(self.out.token(),
                            Rc::new(RefCell::new(Player {
                                                     name: String::from("Noname"),
                                                     game: None,
                                                     ws_sender: MySender(self.out.clone()),
                                                 })));

                Ok(())
            }

            fn on_close(&mut self, _: ws::CloseCode, _: &str) {
                let token = self.out.token();
                let mut players = self.players.borrow_mut();

                {
                    let player_rc = players
                        .get(&token)
                        .expect("on_message called with token not in players map");
                    if let Some(ref game_weak) = player_rc.borrow().game {
                        let game_rc = game_weak.upgrade().expect("Player references dead game");
                        let game = game_rc.borrow();

                        let delete_game = if game.players.len() <= 1 {
                            // Delete the whole game
                            true
                        } else if let GameState::WaitingToStart = game.state {
                            // We can simply remove the player from the game
                            let playerpos =
                                game.players
                                    .iter()
                                    .position(|p| {
                                                  *p ==
                                                  RcWeakRefCellWithHash(Rc::downgrade(player_rc))
                                              })
                                    .expect("Player references game but not vice-versa");

                            // We need to mutably borrow here so drop the immutable borrow and then
                            // pick it back up after
                            std::mem::drop(game);
                            game_rc.borrow_mut().players.remove(playerpos);
                            let game = game_rc.borrow();

                            for player in &game.players {
                                let p = player.upgrade().expect("Game references dead player");
                                let p = p.borrow();
                                let _ = self.send_update_to_player(&p);
                            }

                            // Don't delete the game
                            false
                        } else {
                            // Game has started, have to get rid of entire game
                            true
                        };

                        if delete_game {
                            // TODO
                            // Drop our borrow so we can mutate the game vector
                            unimplemented!();
                        }
                    }
                }

                self.waiting_room_people
                    .borrow_mut()
                    .remove(&RcWeakRefCellWithHash(Rc::downgrade(&players
                                                                      .remove(&token)
                                                                      .unwrap())));
            }
        }

        H {
            out,
            players: players_ref,
            waiting_room_people: waiting_room_people_ref,
            games: games_ref,
        }
    })
            .unwrap();
}

// Make a wrapper around ws::Sender because it doesn't implement Debug :(
#[derive(Clone)]
struct MySender(ws::Sender);
impl std::fmt::Debug for MySender {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Since ws::Sender doesn't implement Debug, it's hard to say anything
        // meaningful here, we just want the trait to be implemented.
        write!(f, "<Sender>")
    }
}
impl std::ops::Deref for MySender {
    type Target = ws::Sender;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for MySender {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
enum ServerError {
    WsError(ws::Error),
    DecodeError(serde_json::Error),
    InvalidReference(String),
}

use ServerError as Error;

#[derive(Debug)]
struct RcWeakRefCellWithHash<T>(RcWeak<RefCell<T>>);

impl<T> std::ops::Deref for RcWeakRefCellWithHash<T> {
    type Target = RcWeak<RefCell<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> std::hash::Hash for RcWeakRefCellWithHash<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.0.upgrade().unwrap().as_ptr() as usize).hash(state);
    }
}
impl<T> PartialEq for RcWeakRefCellWithHash<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0
            .upgrade()
            .expect("Comparing equality of unupgradable weak")
            .as_ptr() ==
        other
            .0
            .upgrade()
            .expect("Comparing equality of unupgradable weak")
            .as_ptr()
    }
}
impl<T> Eq for RcWeakRefCellWithHash<T> {}
