use rand::prelude::*;

use std::io::Write;
use std::process::Command;
use std::time::Duration;

use notify::event::{CreateKind, DataChange, MetadataKind, ModifyKind};
use notify::{recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock};
use tempfile::NamedTempFile;

pub fn watch_file_content_channel(
    path: &str,
) -> (RecommendedWatcher, Receiver<notify::Result<Event>>) {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = recommended_watcher(move |event: notify::Result<Event>| {
        println!("{event:?}");
        tx.send(event).unwrap();
    })
    .unwrap();

    watcher
        .watch(path.as_ref(), RecursiveMode::Recursive)
        .unwrap();

    (watcher, rx)
}

pub fn watch_file_content(path: &str) -> (RecommendedWatcher, Arc<RwLock<Arc<String>>>) {
    let file_content = Arc::new(RwLock::new(Arc::new(
        std::fs::read_to_string(path).unwrap(),
    )));

    let file_content2 = file_content.clone();
    let path2 = path.to_string();

    let mut watcher = recommended_watcher(move |event: notify::Result<Event>| match event {
        Ok(event) => match event.kind {
            EventKind::Modify(ModifyKind::Data(_)) => {
                println!("Received modified file data event {event:?} for {path2}");
                *file_content2.write().unwrap() =
                    Arc::new(std::fs::read_to_string(&path2).unwrap());
            }
            _ => println!("Received event {event:?}"),
        },
        _ => println!("Received error event {event:?}"),
    })
    .unwrap();

    watcher
        .watch(path.as_ref(), RecursiveMode::Recursive)
        .unwrap();

    (watcher, file_content)
}

#[test]
fn test_arc() -> anyhow::Result<()> {
    let mut file = NamedTempFile::new()?;

    let (_watcher, file_content) = watch_file_content(file.path().to_str().unwrap());

    assert_eq!(*file_content.read().unwrap().clone(), "");

    std::thread::sleep(Duration::from_millis(500));

    let string = format!("Hello, world! {}", random::<i32>());
    write!(file, "{}", string)?;

    std::thread::sleep(Duration::from_millis(500));

    assert_eq!(*file_content.read().unwrap().clone(), string);

    Ok(())
}

#[test]
#[cfg(target_os = "macos")]
fn test_channel() -> anyhow::Result<()> {
    let mut file = NamedTempFile::new()?;

    let (_watcher, rx) = watch_file_content_channel(file.path().to_str().unwrap());

    let event = rx.recv().unwrap().unwrap();
    assert_eq!(event.kind, EventKind::Create(CreateKind::File));

    std::thread::sleep(Duration::from_millis(500));

    write!(file, "{}", format!("Hello, world! {}", random::<i32>()))?;

    std::thread::sleep(Duration::from_millis(500));

    // let event = rx.recv().unwrap().unwrap();
    // assert_eq!(event.kind, EventKind::Create(CreateKind::File));

    println!("{:?}", event);

    Ok(())
}

#[test]
#[cfg(target_os = "linux")]
fn test_channel_linux() -> anyhow::Result<()> {
    let mut file = NamedTempFile::new()?;

    let (_watcher, rx) = watch_file_content_channel(file.path().to_str().unwrap());

    write!(file, "{}", format!("Hello, world! {}", random::<i32>()))?;

    let event = rx.recv().unwrap().unwrap();
    assert_eq!(
        event.kind,
        EventKind::Modify(ModifyKind::Data(DataChange::Any))
    );

    println!("{:?}", event);

    Ok(())
}

#[test]
#[cfg(target_os = "macos")]
fn test_channel_normal_file() -> anyhow::Result<()> {
    let path = "foo.txt";

    let (_watcher, rx) = watch_file_content_channel(path);

    std::thread::spawn(|| {
        Command::new("./update_foo.sh").output().unwrap();
    });

    let event = rx.recv().unwrap().unwrap();
    assert_eq!(
        event.kind,
        EventKind::Modify(ModifyKind::Data(DataChange::Content))
    );

    Ok(())
}

#[test]
#[cfg(feature = "macos_kqueue")]
fn test_channel_normal_file_kqueue() -> anyhow::Result<()> {
    let path = "foo.txt";

    let (_watcher, rx) = watch_file_content_channel(path);

    Command::new("./update_foo.sh").output().unwrap();

    // Sometimes it receives all 3 events, but always at least 2
    receive(&rx);
    receive(&rx);

    fn receive(rx: &Receiver<notify::Result<Event>>) {
        match rx.recv().unwrap().unwrap().kind {
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any)) => {}
            EventKind::Modify(ModifyKind::Data(DataChange::Any)) => {}
            EventKind::Modify(ModifyKind::Data(DataChange::Size)) => {}
            _ => unreachable!(),
        }
    }

    Ok(())
}

#[test]
#[cfg(target_os = "linux")]
fn test_channel_normal_file_linux() -> anyhow::Result<()> {
    let path = "foo.txt";
    let mut file = File::create(path)?;

    let (_watcher, rx) = watch_file_content_channel(path);

    write!(file, "{}", format!("Hello, world! {}", random::<i32>()))?;

    let event = rx.recv().unwrap().unwrap();
    assert_eq!(
        event.kind,
        EventKind::Modify(ModifyKind::Data(DataChange::Any))
    );

    Ok(())
}

#[test]
fn test_lock() {
    let lock = RwLock::new(Arc::new("123".to_string()));

    let data = lock.read().unwrap().clone();

    println!("{data}");

    assert_eq!(*lock.read().unwrap().clone(), "123");

    let data = lock.write().unwrap().clone();

    println!("{data}");
}
