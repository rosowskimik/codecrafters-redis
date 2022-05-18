use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

use tokio::sync::oneshot::Sender;

pub type Db = Arc<Mutex<HashMap<String, DbEntry>>>;

#[derive(Debug)]
pub struct DbEntry {
    pub value: String,
    pub timeout_channel: Option<Sender<()>>,
}

impl DbEntry {
    pub fn new(value: String) -> Self {
        Self {
            value,
            timeout_channel: None,
        }
    }

    pub fn with_timeout(value: String, tx: Sender<()>) -> Self {
        Self {
            value,
            timeout_channel: Some(tx),
        }
    }
}

impl Deref for DbEntry {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for DbEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
