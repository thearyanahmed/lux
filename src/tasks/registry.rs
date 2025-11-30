use super::{HttpServerTask, Task};
use once_cell::sync::Lazy;

static TASKS: Lazy<Vec<Box<dyn Task>>> = Lazy::new(|| vec![Box::new(HttpServerTask::new())]);

pub fn get_task(id: &str) -> Option<&'static dyn Task> {
    TASKS
        .iter()
        .find(|t| t.id() == id)
        .map(|boxed| &**boxed as &dyn Task)
}
