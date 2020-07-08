
use super::*;

#[derive(Debug)]
pub struct Handler {}

impl Handler {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn handle(self, event: Event) -> Self {
        log::info!("EVENT: {:#?}", event);
        self
    }
}
