use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use objc2_foundation::{NSDate, NSDateFormatter, NSString};

const LIMIT: u64 = 1 << 20;

macro_rules! note {
    ($($arg:tt)*) => {
        $crate::log::write(&format!($($arg)*))
    };
}

pub(crate) use note;

pub(crate) fn write(message: &str) {
    eprintln!("combe: {message}");
    let Some(path) = path() else { return };
    let Some(dir) = path.parent() else { return };
    if fs::create_dir_all(dir).is_err() {
        return;
    }
    if fs::metadata(&path).is_ok_and(|meta| meta.len() > LIMIT) {
        let _ = fs::rename(&path, path.with_extension("log.1"));
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let _ = writeln!(file, "{} {message}", stamp());
}

fn path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Logs/Combe/combe.log"))
}

fn stamp() -> String {
    let formatter = NSDateFormatter::new();
    formatter.setDateFormat(Some(&NSString::from_str("yyyy-MM-dd HH:mm:ss")));
    formatter.stringFromDate(&NSDate::now()).to_string()
}
