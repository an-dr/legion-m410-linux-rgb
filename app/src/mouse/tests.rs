use super::*;

#[test]
fn a_missing_reply_means_the_mouse_is_asleep() {
    assert_eq!(error(m410::Error::NoReply(Connection::Wireless)), MouseError::Asleep);
    assert_eq!(error(m410::Error::NotFound), MouseError::NotFound);
}

#[test]
fn a_permission_error_carries_the_device_path() {
    let denied = m410::Error::PermissionDenied("/dev/hidraw5".into());
    assert_eq!(error(denied), MouseError::NoAccess("/dev/hidraw5".into()));
}
