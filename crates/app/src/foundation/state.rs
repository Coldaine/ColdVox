use std::sync::Arc;
use parking_lot::RwLock;
use crossbeam_channel::{Sender, Receiver};
use crate::foundation::error::AppError;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Initializing,
    Running,
    Recovering { from_error: String },
    Stopping,
    Stopped,
}

pub struct StateManager {
    state: Arc<RwLock<AppState>>,
    state_tx: Sender<AppState>,
    state_rx: Receiver<AppState>,
}

impl StateManager {
    pub fn new() -> Self {
        let (state_tx, state_rx) = crossbeam_channel::unbounded();
        Self {
            state: Arc::new(RwLock::new(AppState::Initializing)),
            state_tx,
            state_rx,
        }
    }
    
    pub fn transition(&self, new_state: AppState) -> Result<(), AppError> {
        let mut current = self.state.write();
        
        // Validate state transitions
        let valid = match (&*current, &new_state) {
            (AppState::Initializing, AppState::Running) => true,
            (AppState::Running, AppState::Recovering { .. }) => true,
            (AppState::Running, AppState::Stopping) => true,
            (AppState::Recovering { .. }, AppState::Running) => true,
            (AppState::Recovering { .. }, AppState::Stopping) => true,
            (AppState::Stopping, AppState::Stopped) => true,
            _ => false,
        };
        
        if !valid {
            return Err(AppError::Fatal(
                format!("Invalid state transition: {:?} -> {:?}", *current, new_state)
            ));
        }
        
        tracing::info!("State transition: {:?} -> {:?}", *current, new_state);
        *current = new_state.clone();
        let _ = self.state_tx.send(new_state);
        Ok(())
    }
    
    pub fn current(&self) -> AppState {
        self.state.read().clone()
    }
    
    pub fn subscribe(&self) -> Receiver<AppState> {
        self.state_rx.clone()
    }
}