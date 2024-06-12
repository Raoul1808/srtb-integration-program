mod app;
mod file;

pub use app::program;

#[derive(Debug)]
struct ReadFile {
    name: String,
    content: String,
}
