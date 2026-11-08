use bones_messages::{DecodeError, DecodeMessage, EncodeMessage};

use crate::{Command, Connection, Effect, Lighting, State, Status, BRIGHTNESS_MAX, SPEED_MAX};

fn lighting() -> Lighting {
    Lighting {
        effect: Effect::Breathing,
        brightness: 3,
        speed: 4,
        random: false,
        color: (255, 128, 0),
    }
}

#[test]
fn every_command_round_trips() {
    for command in [Command::Refresh, Command::Apply(lighting())] {
        assert_eq!(Command::decode(&command.encode()), Ok(command));
    }
}

#[test]
fn every_effect_round_trips() {
    for effect in Effect::ALL {
        let command = Command::Apply(Lighting { effect, ..lighting() });
        assert_eq!(Command::decode(&command.encode()), Ok(command));
    }
}

#[test]
fn state_with_lighting_round_trips() {
    let state = State {
        connection: Connection::Wireless,
        status: Status::Ready,
        lighting: Some(lighting()),
        detail: "",
    };
    assert_eq!(State::decode(&state.encode()), Ok(state));
}

#[test]
fn state_without_lighting_keeps_its_detail() {
    let state = State {
        connection: Connection::None,
        status: Status::NotFound,
        lighting: None,
        detail: "Plug in the mouse or its receiver.",
    };
    assert_eq!(State::decode(&state.encode()), Ok(state));
}

#[test]
fn every_status_and_connection_round_trips() {
    for connection in [Connection::None, Connection::Wired, Connection::Wireless] {
        for status in [
            Status::Ready,
            Status::Waiting,
            Status::NotFound,
            Status::NoAccess,
            Status::Failed,
        ] {
            let state = State {
                connection,
                status,
                lighting: None,
                detail: "x",
            };
            assert_eq!(State::decode(&state.encode()), Ok(state));
        }
    }
}

#[test]
fn out_of_range_values_are_clamped_on_decode() {
    let mut payload = Command::Apply(lighting()).encode();
    payload[2] = 200; // brightness
    payload[3] = 0; // speed
    let Ok(Command::Apply(decoded)) = Command::decode(&payload) else {
        panic!("decode failed")
    };
    assert_eq!(decoded.brightness, BRIGHTNESS_MAX);
    assert_eq!(decoded.speed, 1);
    assert!(decoded.speed <= SPEED_MAX);
}

#[test]
fn bad_input_is_an_error_not_a_panic() {
    assert!(matches!(Command::decode(&[]), Err(DecodeError::Truncated)));
    assert!(matches!(Command::decode(&[9]), Err(DecodeError::InvalidTag { .. })));
    assert!(matches!(Command::decode(&[0, 0]), Err(DecodeError::TrailingBytes)));
    assert!(matches!(
        Command::decode(&[1, 7, 3, 3, 0, 0, 0, 0]),
        Err(DecodeError::InvalidTag { .. })
    ));
    assert!(matches!(State::decode(&[1]), Err(DecodeError::Truncated)));
}
