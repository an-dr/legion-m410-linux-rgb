use std::sync::{Arc, Mutex};

use m410::{Connection, Effect, Lighting, Rgb};

use super::*;
use crate::{MouseError, Status};

const WAIT: Duration = Duration::from_secs(3);

struct Fake {
    stored: Arc<Mutex<Lighting>>,
}

impl Mouse for Fake {
    fn connection(&mut self) -> Result<Connection, MouseError> {
        Ok(Connection::Wired)
    }

    fn read(&mut self) -> Result<Lighting, MouseError> {
        Ok(*self.stored.lock().unwrap())
    }

    fn write(&mut self, lighting: &Lighting) -> Result<(), MouseError> {
        *self.stored.lock().unwrap() = *lighting;
        Ok(())
    }
}

fn lighting(brightness: u8) -> Lighting {
    Lighting {
        effect: Effect::Static,
        brightness,
        speed: 3,
        random: false,
        color: Rgb(0, 128, 255),
    }
}

#[test]
fn a_refresh_is_answered_without_the_caller_waiting_on_the_device() {
    let stored = Arc::new(Mutex::new(lighting(2)));
    let backend = Backend::spawn_polling(Fake { stored }, Duration::from_millis(20));
    backend.send(Command::Refresh);
    let snapshot = backend.recv_timeout(WAIT).expect("an answer");
    assert_eq!(snapshot.status, Status::Ready);
    assert_eq!(snapshot.lighting, Some(lighting(2)));
}

#[test]
fn an_applied_change_reaches_the_mouse_and_is_reported_twice() {
    let stored = Arc::new(Mutex::new(lighting(2)));
    let backend = Backend::spawn_polling(Fake { stored: stored.clone() }, Duration::from_millis(20));
    backend.send(Command::Apply(lighting(4)));

    let first = backend.recv_timeout(WAIT).expect("the optimistic snapshot");
    assert_eq!(first.lighting, Some(lighting(4)));
    let second = backend.recv_timeout(WAIT).expect("the confirmed snapshot");
    assert_eq!(second.status, Status::Ready);
    assert_eq!(*stored.lock().unwrap(), lighting(4));
}

#[test]
fn plugging_in_is_noticed_by_polling_alone() {
    let stored = Arc::new(Mutex::new(lighting(1)));
    let backend = Backend::spawn_polling(Fake { stored }, Duration::from_millis(20));
    let snapshot = backend.recv_timeout(WAIT).expect("the first poll finds the mouse");
    assert_eq!(snapshot.connection, Some(Connection::Wired));
}

#[test]
fn dropping_the_backend_ends_the_worker() {
    let stored = Arc::new(Mutex::new(lighting(1)));
    let watcher = stored.clone();
    let backend = Backend::spawn_polling(Fake { stored }, Duration::from_millis(10));
    assert!(backend.recv_timeout(WAIT).is_some());
    drop(backend);
    // The worker held the other handle to the lighting; it lets go when it exits.
    for _ in 0..200 {
        if Arc::strong_count(&watcher) == 1 {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("the worker thread is still running");
}
