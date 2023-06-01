use std::fs::File;
use log::{error, info};
use notify::event::{DataChange, ModifyKind};
use notify::{recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::{Arc, RwLock};

pub fn watch_file_content(path: &str) -> (RecommendedWatcher, Arc<RwLock<Arc<String>>>) {
    let file_content = Arc::new(RwLock::new(Arc::new(
        std::fs::read_to_string(path).unwrap(),
    )));

    let file_content2 = file_content.clone();
    let path2 = path.to_string();

    let mut watcher = recommended_watcher(move |event: notify::Result<Event>| {
        match event {
            Ok(event) => {
                println!("Fuck Received event {event:?}");
                match event.kind {
                    EventKind::Modify(ModifyKind::Data(_)) => {
                        println!("Received modified file data event {event:?} for {path2}");
                        *file_content2.write().unwrap() = Arc::new(std::fs::read_to_string(&path2).unwrap());
                    }
                    _ => println!("Received event {event:?}"),
                }
            },
            _ => error!("Received error event {event:?}"),
        }
    })
    .unwrap();

    watcher
        .watch(path.as_ref(), RecursiveMode::Recursive)
        .unwrap();

    (watcher, file_content)
}

#[cfg(test)]
mod tests {
    use std::fmt::format;
    use std::fs::{File, OpenOptions};
    use super::*;

    use std::io::Write;
    use std::time::Duration;
    use log::LevelFilter;
    use simple_logger::SimpleLogger;
    use tempfile::NamedTempFile;
    use rand::prelude::*;

    #[test]
    fn test() -> anyhow::Result<()> {
        SimpleLogger::new()
            .with_level(LevelFilter::Info)
            .with_threads(true)
            .init()
            .unwrap();

        let path = "foo.txt";
        File::create(path)?;

        let (_watcher, file_content) = watch_file_content(path);

        assert_eq!(*file_content.read().unwrap().clone(), "");

        std::thread::sleep(Duration::from_millis(500));

        let mut file = OpenOptions::new().write(true).open(path)?;
        let string = format!("Hello, world! {}", random::<i32>());
        write!(file, "{}", string)?;

        std::thread::sleep(Duration::from_millis(500));

        assert_eq!(*file_content.read().unwrap().clone(), string);

        Ok(())
    }

    #[test]
    fn test2() {
        let lock = RwLock::new(Arc::new("123".to_string()));

        let data = lock.read().unwrap().clone();

        println!("{data}");

        assert_eq!(*lock.read().unwrap().clone(), "123");

        let data = lock.write().unwrap().clone();

        println!("{data}");
    }
}
