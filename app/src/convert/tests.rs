use m410_messages::{Connection, Effect};

use super::*;

#[test]
fn lighting_survives_the_round_trip_for_every_effect() {
    for effect in Effect::ALL {
        let wire = Lighting {
            effect,
            brightness: 3,
            speed: 2,
            random: true,
            color: (1, 2, 3),
        };
        assert_eq!(lighting_to_wire(&lighting_from_wire(&wire)), wire);
    }
}

#[test]
fn connections_map_one_to_one() {
    assert_eq!(connection(m410::Connection::Wired), Connection::Wired);
    assert_eq!(connection(m410::Connection::Wireless), Connection::Wireless);
}

#[test]
fn a_missing_reply_means_the_mouse_is_asleep() {
    assert_eq!(
        mouse_error(m410::Error::NoReply(m410::Connection::Wireless)),
        MouseError::Asleep
    );
    assert_eq!(mouse_error(m410::Error::NotFound), MouseError::NotFound);
}

#[test]
fn a_permission_error_carries_the_device_path() {
    let error = m410::Error::PermissionDenied("/dev/hidraw5".into());
    assert_eq!(mouse_error(error), MouseError::NoAccess("/dev/hidraw5".into()));
}
