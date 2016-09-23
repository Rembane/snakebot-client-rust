use structs;
use serde_json::{ from_str, to_string, Error };
use target_info::Target;
use rustc_version;

// Inbound
pub const GAME_ENDED: &'static str =
    "se.cygni.snake.api.event.GameEndedEvent";
pub const TOURNAMENT_ENDED: &'static str =
    "se.cygni.snake.api.event.TournamentEndedEvent";
pub const MAP_UPDATE: &'static str =
    "se.cygni.snake.api.event.MapUpdateEvent";
pub const SNAKE_DEAD: &'static str =
    "se.cygni.snake.api.event.SnakeDeadEvent";
pub const GAME_STARTING: &'static str =
    "se.cygni.snake.api.event.GameStartingEvent";
pub const PLAYER_REGISTERED: &'static str =
    "se.cygni.snake.api.response.PlayerRegistered";
pub const INVALID_PLAYER_NAME: &'static str =
    "se.cygni.snake.api.exception.InvalidPlayerName";
pub const HEART_BEAT_RESPONSE: &'static str =
    "se.cygni.snake.api.response.HeartBeatResponse";
pub const GAME_LINK_EVENT: &'static str =
    "se.cygni.snake.api.event.GameLinkEvent";

// Outbound
const REGISTER_PLAYER_MESSAGE_TYPE: &'static str =
    "se.cygni.snake.api.request.RegisterPlayer";
const START_GAME: &'static str =
    "se.cygni.snake.api.request.StartGame";
const REGISTER_MOVE: &'static str =
    "se.cygni.snake.api.request.RegisterMove";
const HEART_BEAT_REQUEST: &'static str =
    "se.cygni.snake.api.request.HeartBeatRequest";
const CLIENT_INFO: &'static str =
    "se.cygni.snake.api.request.ClientInfo";

pub enum Inbound {
    GameEnded(structs::GameEnded),
    TournamentEnded(structs::TournamentEnded),
    MapUpdate(structs::MapUpdate),
    SnakeDead(structs::SnakeDead),
    GameStarting(structs::GameStarting),
    PlayerRegistered(structs::PlayerRegistered),
    InvalidPlayerName(structs::InvalidPlayerName),
    HeartBeatResponse(structs::HeartBeatResponse),
    GameLinkEvent(structs::GameLink),
    UnrecognizedMessage
}

pub fn parse_inbound_msg(msg: &String) -> Result<Inbound, Error> {
    let msg: Inbound =
        if msg.contains(GAME_ENDED) {
            Inbound::GameEnded(try!(from_str(msg)))
        } else if msg.contains(TOURNAMENT_ENDED) {
            Inbound::TournamentEnded(try!(from_str(msg)))
        } else if msg.contains(MAP_UPDATE) {
            Inbound::MapUpdate(try!(from_str(msg)))
        } else if msg.contains(SNAKE_DEAD) {
            Inbound::SnakeDead(try!(from_str(msg)))
        } else if msg.contains(GAME_STARTING) {
            Inbound::GameStarting(try!(from_str(msg)))
        } else if msg.contains(PLAYER_REGISTERED) {
            Inbound::PlayerRegistered(try!(from_str(msg)))
        } else if msg.contains(INVALID_PLAYER_NAME) {
            Inbound::InvalidPlayerName(try!(from_str(msg)))
        } else if msg.contains(HEART_BEAT_RESPONSE) {
            Inbound::HeartBeatResponse(try!(from_str(msg)))
        } else if msg.contains(GAME_LINK_EVENT) {
            Inbound::GameLinkEvent(try!(from_str(msg)))
        } else {
            Inbound::UnrecognizedMessage
        };

    Ok(msg)
}


pub fn create_play_registration_msg(name: String) -> Result<String, Error> {
    to_string(&structs::PlayRegistration {
        type_: String::from(REGISTER_PLAYER_MESSAGE_TYPE),
        playerName: name,
        gameSettings: default_gamesettings()
    })
}

pub fn create_start_game_msg() -> Result<String, Error> {
    to_string(&structs::StartGame {
        type_: String::from(START_GAME)
    })
}

pub fn create_register_move_msg(direction: String, request: structs::MapUpdate) -> Result<String, Error> {
    to_string(&structs::RegisterMove {
        type_: String::from(REGISTER_MOVE),
        direction: direction,
        gameTick: request.gameTick,
        receivingPlayerId: request.receivingPlayerId,
        gameId: request.gameId
    })
}

pub fn create_heart_beat_msg(id: String) -> Result<String, Error> {
    to_string(&structs::HeartBeatRequest {
        type_: String::from( HEART_BEAT_REQUEST ),
        receivingPlayerId: id
    })
}

pub fn create_client_info_msg() -> Result<String, Error> {
    to_string(&structs::ClientInfo {
        type_: String::from(CLIENT_INFO),
        language: String::from("rust"),
        languageVersion: format!("{}", rustc_version::version()),
        operatingSystem: String::from(Target::os()),
        operatingSystemVersion: String::from(""),
        clientVersion: String::from(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
    })
}

pub fn default_gamesettings() -> structs::GameSettings {
    structs::GameSettings {
        maxNoofPlayers: 5,
        startSnakeLength: 1,
        timeInMsPerTick: 250,
        obstaclesEnabled: true,
        foodEnabled: true,
        headToTailConsumes: true,
        tailConsumeGrows: false,
        addFoodLikelihood: 15,
        removeFoodLikelihood: 5,
        spontaneousGrowthEveryNWorldTick: 3,
        trainingGame: false,
        pointsPerLength: 1,
        pointsPerFood: 2,
        pointsPerCausedDeath: 5,
        pointsPerNibble: 10,
        noofRoundsTailProtectedAfterNibble: 3,
    }
}
