use async_trait::async_trait;

use crate::bot::qbot::MessageWrapper;
use crate::bot::messages::TelegramBackend;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct RutebotMock {
    inner: Arc<RwLock<InnerRutebotMock>>
}

#[derive(Default)]
struct InnerRutebotMock {
    messages: Vec<MessageWrapper>
}

impl RutebotMock {
    pub fn assert_last(&self, wants: &str) {
        assert_eq!(wants, self.inner.read().unwrap().messages.iter().last().unwrap().text)
    }
}

#[async_trait]
impl TelegramBackend for RutebotMock {
    async fn send_message(&self, _: i64, message: MessageWrapper) {
        self.inner.write().unwrap().messages.push(message);
    }
}
