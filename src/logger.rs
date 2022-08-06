pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if (!record.target().starts_with("wgpu") && !record.target().starts_with("naga"))
            || record.level() < log::LevelFilter::Info
        {
            eprintln!(
                "{:<8} {:<32} {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
