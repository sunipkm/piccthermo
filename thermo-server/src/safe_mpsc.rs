#![allow(dead_code)]
use std::sync::{
    Arc,
    atomic::AtomicBool,
    mpsc::{self, Receiver, Sender},
};

#[derive(Debug, Clone)]
pub struct SafeSender<T> {
    sender: Sender<T>,
    ready: Arc<AtomicBool>,
}

#[derive(Debug)]
pub struct SafeReceiver<T> {
    receiver: Receiver<T>,
    ready: Arc<AtomicBool>,
}

pub fn channel<T>() -> (SafeSender<T>, SafeReceiver<T>) {
    let (tx, rx) = mpsc::channel();
    let ready = Arc::new(AtomicBool::new(true));
    (
        SafeSender {
            sender: tx,
            ready: ready.clone(),
        },
        SafeReceiver {
            receiver: rx,
            ready,
        },
    )
}

impl<T> SafeSender<T> {
    pub fn send(&self, value: T) -> Result<(), SafeSendError<T>> {
        if self.ready.load(std::sync::atomic::Ordering::Relaxed) {
            self.sender.send(value).map_err(SafeSendError::from)
        } else {
            Err(mpsc::SendError(value).into())
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn sender(&self) -> &Sender<T> {
        &self.sender
    }
}

impl<T> SafeReceiver<T> {
    pub fn set_ready(&self, ready: bool) {
        self.ready
            .store(ready, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn receiver(&self) -> &Receiver<T> {
        &self.receiver
    }
}

#[derive(Debug)]
pub enum SafeSendError<T> {
    SendError(mpsc::SendError<T>),
    NotReady,
}

impl<T> From<mpsc::SendError<T>> for SafeSendError<T> {
    fn from(err: mpsc::SendError<T>) -> Self {
        SafeSendError::SendError(err)
    }
}
