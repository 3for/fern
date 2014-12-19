use std::io;
use std::io::stdio;
use std::sync;

use api::{Logger, Level};
use config;

pub struct ConfigurationLogger {
    pub output: Vec<Box<Logger + Sync + Send>>,
    pub level: Level,
    pub format: Box<Fn(&str, &Level) -> String + Sync + Send>,
}

impl ConfigurationLogger {
    pub fn new(format: Box<Fn(&str, &Level) -> String + Sync + Send>, config_output: Vec<config::OutputConfig>, level: Level)
                    -> io::IoResult<ConfigurationLogger> {

        let output = try!(config_output.into_iter().fold(Ok(Vec::new()),
                                            |processed: io::IoResult<Vec<Box<Logger + Sync + Send>>>, next: config::OutputConfig| {
            // If an error has already been found, don't try to process any future outputs, just continue passing along the error.
            let mut processed_so_far = try!(processed);
            return match next.into_logger() {
                Err(e) => Err(e), // If this one errors, return the error instead of the Vec so far
                Ok(processed_value) => {
                    // If it's ok, add the processed logger to the vec, and pass the vec along
                    processed_so_far.push(processed_value);
                    Ok(processed_so_far)
                }
            };
        }));

        return Ok(ConfigurationLogger {
            output: output,
            level: level,
            format: format,
        });
    }
}

impl Logger for ConfigurationLogger {
    fn log(&self, level: &Level, msg: &str) -> io::IoResult<()> {
        if level.as_int() < self.level.as_int() {
            return Ok(());
        }
        let new_msg = self.format.call((msg, level));
        for logger in self.output.iter() {
            try!(logger.log(level, new_msg.as_slice()));
        }
        return Ok(());
    }
}

pub struct WriterLogger<T: io::Writer + Send> {
    writer: sync::Arc<sync::Mutex<T>>,
}

impl <T: io::Writer + Send> WriterLogger<T> {
    pub fn new(writer: T) -> WriterLogger<T> {
        return WriterLogger {
            writer: sync::Arc::new(sync::Mutex::new(writer)),
        };
    }

    pub fn with_stdout() -> WriterLogger<io::stdio::StdWriter> {
        return WriterLogger::new(stdio::stdout_raw());
    }

    pub fn with_stderr() -> WriterLogger<io::stdio::StdWriter> {
        return WriterLogger::new(stdio::stderr_raw());
    }

    pub fn with_file(path: &Path) -> io::IoResult<WriterLogger<io::File>> {
        return Ok(WriterLogger::new(try!(io::File::create(path))));
    }
}

impl <T: io::Writer + Send> Logger for WriterLogger<T> {
    fn log(&self, _level: &Level, message: &str) -> io::IoResult<()> {
        return self.writer.lock().write_line(message);
    }
}