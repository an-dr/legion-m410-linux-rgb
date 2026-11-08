use std::sync::{Arc, Mutex};

use m410::{Effect, Lighting, Rgb};

use super::*;

/// What the fake mouse did, shared with the test after the controller owns it.
#[derive(Default)]
struct Log {
    writes: Vec<Lighting>,
    reads: usize,
}

struct Fake {
    connection: Result<Connection, MouseError>,
    asleep: bool,
    stored: Lighting,
    log: Arc<Mutex<Log>>,
}

fn lighting(brightness: u8) -> Lighting {
    Lighting {
        effect: Effect::Static,
        brightness,
        speed: 3,
        random: false,
        color: Rgb(255, 0, 0),
    }
}

fn fake() -> (Fake, Arc<Mutex<Log>>) {
    let log = Arc::new(Mutex::new(Log::default()));
    let mouse = Fake {
        connection: Ok(Connection::Wireless),
        asleep: false,
        stored: lighting(4),
        log: log.clone(),
    };
    (mouse, log)
}

impl Mouse for Fake {
    fn connection(&mut self) -> Result<Connection, MouseError> {
        self.connection.clone()
    }

    fn read(&mut self) -> Result<Lighting, MouseError> {
        self.log.lock().unwrap().reads += 1;
        if self.asleep {
            return Err(MouseError::Asleep);
        }
        Ok(self.stored)
    }

    fn write(&mut self, lighting: &Lighting) -> Result<(), MouseError> {
        if self.asleep {
            return Err(MouseError::Asleep);
        }
        self.stored = *lighting;
        self.log.lock().unwrap().writes.push(*lighting);
        Ok(())
    }
}

#[test]
fn refresh_reports_what_the_mouse_holds() {
    let (mouse, _) = fake();
    let mut controller = Controller::new(mouse);
    let snapshots = controller.handle(vec![Command::Refresh]);
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].status, Status::Ready);
    assert_eq!(snapshots[0].connection, Some(Connection::Wireless));
    assert_eq!(snapshots[0].lighting, Some(lighting(4)));
}

#[test]
fn an_applied_change_is_shown_at_once_and_then_confirmed() {
    let (mouse, log) = fake();
    let mut controller = Controller::new(mouse);
    let snapshots = controller.handle(vec![Command::Apply(lighting(2))]);
    assert_eq!(snapshots.len(), 2, "one optimistic snapshot, one after the write");
    assert_eq!(snapshots[0].lighting, Some(lighting(2)));
    assert_eq!(snapshots[1].status, Status::Ready);
    assert_eq!(log.lock().unwrap().writes, vec![lighting(2)]);
}

#[test]
fn a_burst_of_changes_costs_one_write_of_the_newest() {
    let (mouse, log) = fake();
    let mut controller = Controller::new(mouse);
    controller.handle(vec![
        Command::Apply(lighting(1)),
        Command::Refresh,
        Command::Apply(lighting(3)),
        Command::Apply(lighting(2)),
    ]);
    assert_eq!(log.lock().unwrap().writes, vec![lighting(2)]);
}

#[test]
fn a_sleeping_mouse_gets_the_change_when_it_wakes() {
    let (mut mouse, log) = fake();
    mouse.asleep = true;
    let mut controller = Controller::new(mouse);

    let snapshots = controller.handle(vec![Command::Apply(lighting(1))]);
    let last = snapshots.last().unwrap();
    assert_eq!(last.status, Status::Waiting);
    assert_eq!(
        last.lighting,
        Some(lighting(1)),
        "the panel keeps showing what the user asked for"
    );
    assert!(log.lock().unwrap().writes.is_empty());

    assert_eq!(controller.poll(), None, "still asleep, nothing new to report");

    controller.mouse.asleep = false;
    let woke = controller.poll().expect("waking is a change");
    assert_eq!(woke.status, Status::Ready);
    assert_eq!(log.lock().unwrap().writes, vec![lighting(1)]);
}

#[test]
fn the_waiting_notice_only_promises_a_change_when_there_is_one() {
    let (mut mouse, _) = fake();
    mouse.asleep = true;
    let mut controller = Controller::new(mouse);

    let idle = &controller.handle(vec![Command::Refresh])[0];
    assert_eq!(idle.status, Status::Waiting);
    assert!(!idle.detail.contains("change"), "{}", idle.detail);

    let busy = controller.handle(vec![Command::Apply(lighting(1))]);
    assert!(
        busy.last().unwrap().detail.contains("your change applies"),
        "{}",
        busy.last().unwrap().detail
    );
}

#[test]
fn a_mouse_that_never_wakes_is_given_up_on() {
    let (mut mouse, log) = fake();
    mouse.asleep = true;
    let mut controller = Controller::new(mouse);
    controller.handle(vec![Command::Apply(lighting(1))]);

    let mut last = None;
    for _ in 0..=MAX_WAITS {
        last = controller.poll().or(last);
    }
    let last = last.expect("giving up is reported");
    assert_eq!(last.status, Status::Failed);
    assert!(last.detail.contains("Refresh"));
    assert!(log.lock().unwrap().writes.is_empty());

    controller.mouse.asleep = false;
    assert_eq!(controller.poll(), None, "an abandoned change is not applied later");
    assert!(log.lock().unwrap().writes.is_empty());
}

#[test]
fn plugging_the_mouse_in_is_noticed_without_a_command() {
    let (mut mouse, _) = fake();
    mouse.connection = Err(MouseError::NotFound);
    let mut controller = Controller::new(mouse);
    assert_eq!(controller.poll(), None, "nothing connected, nothing changed");

    controller.mouse.connection = Ok(Connection::Wired);
    let snapshot = controller.poll().expect("a new connection is a change");
    assert_eq!(snapshot.connection, Some(Connection::Wired));
    assert_eq!(snapshot.status, Status::Ready);
}

#[test]
fn unplugging_drops_a_pending_change() {
    let (mut mouse, log) = fake();
    mouse.asleep = true;
    let mut controller = Controller::new(mouse);
    controller.handle(vec![Command::Apply(lighting(1))]);

    controller.mouse.connection = Err(MouseError::NotFound);
    let snapshot = controller.poll().expect("unplugging is a change");
    assert_eq!(snapshot.status, Status::NotFound);

    controller.mouse.connection = Ok(Connection::Wired);
    controller.mouse.asleep = false;
    controller.poll();
    assert!(
        log.lock().unwrap().writes.is_empty(),
        "the old change must not come back"
    );
}

#[test]
fn missing_permission_says_how_to_fix_it() {
    let (mut mouse, _) = fake();
    mouse.connection = Err(MouseError::NoAccess("/dev/hidraw5".into()));
    let mut controller = Controller::new(mouse);
    let snapshot = &controller.handle(vec![Command::Refresh])[0];
    assert_eq!(snapshot.status, Status::NoAccess);
    assert!(snapshot.detail.contains("/dev/hidraw5") && snapshot.detail.contains("udev"));
}

#[test]
fn an_unchanged_state_publishes_nothing_on_poll() {
    let (mouse, _) = fake();
    let mut controller = Controller::new(mouse);
    controller.handle(vec![Command::Refresh]);
    assert_eq!(controller.poll(), None);
    assert_eq!(controller.poll(), None);
}
