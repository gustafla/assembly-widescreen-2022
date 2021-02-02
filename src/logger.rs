pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        eprintln!("{:<8} {:<16} {}", record.level(), record.target(), record.args());
    }

    fn flush(&self) {}
}
