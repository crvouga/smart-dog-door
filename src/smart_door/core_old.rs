use crate::config::Config;
use crate::device_camera::interface::{DeviceCameraEvent, Frame};
use crate::device_door::interface::DeviceDoorEvent;
use crate::image_classifier::interface::Classification;
use std::time::{Duration, Instant};

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct DeviceStates {
    pub camera: CameraState,
    pub door: DoorState,
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum CameraState {
    #[default]
    Disconnected,
    Connected(Instant),
    Started,
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum DoorState {
    #[default]
    Disconnected,
    Connected(Instant),
    Initialized,
    Locked,
    Unlocked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Initial state while devices are connecting
    DevicesInitializing {
        device_states: DeviceStates,
        status: String,
    },
    /// Capturing frames from cameras for analysis
    AnalyzingFramesCapture {
        door_state: DoorState,
        status: String,
    },
    /// Classifying captured frames to detect animals
    AnalyzingFramesClassifying {
        door_state: DoorState,
        frames: Vec<Frame>,
        status: String,
    },
    /// Actively controlling the door (locking/unlocking)
    ControllingDoor {
        action: DoorAction,
        door_state: DoorState,
        start_time: Instant,
        status: String,
    },
    /// Waiting state between operations
    Idle {
        door_state: DoorState,
        status: String,
        last_activity: Instant,
    },
    /// Period after unlocking to allow dog to pass through
    UnlockedGracePeriod {
        door_state: DoorState,
        countdown_start: Instant,
        status: String,
    },
    /// Period before locking to ensure no dog is present
    LockingGracePeriod {
        door_state: DoorState,
        countdown_start: Instant,
        last_detection: Instant,
        status: String,
    },
    /// Emergency state if something goes wrong
    Error {
        error: String,
        door_state: DoorState,
        recovery_actions: Vec<Effect>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DoorAction {
    Locking,
    Unlocking,
}

#[derive(Debug)]
pub enum Event {
    Tick(Instant),
    CameraEvent(DeviceCameraEvent),
    CameraStartDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorEvent(DeviceDoorEvent),
    DoorLockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorUnlockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    FramesClassifyDone(Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>>),
    FramesCaptureDone(Result<Vec<Frame>, Box<dyn std::error::Error + Send + Sync>>),
    #[allow(dead_code)]
    ManualOverride(DoorAction),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    StartCamera,
    LockDoor,
    UnlockDoor,
    CaptureFrames,
    ClassifyFrames { frames: Vec<Frame> },
    SubscribeToCameraEvents,
    SubscribeToDoorEvents,
    SubscribeTick,
}

pub fn init() -> (State, Vec<Effect>) {
    (
        State::DevicesInitializing {
            device_states: DeviceStates::default(),
            status: "Initializing devices...".to_string(),
        },
        vec![
            Effect::SubscribeToDoorEvents,
            Effect::SubscribeToCameraEvents,
            Effect::SubscribeTick,
        ],
    )
}

pub fn transition(config: &Config, state: State, event: Event) -> (State, Vec<Effect>) {
    match (state.clone(), event) {
        // Device initialization
        (
            State::DevicesInitializing { device_states, .. },
            Event::CameraEvent(DeviceCameraEvent::Connected),
        ) => {
            let new_states = DeviceStates {
                camera: CameraState::Connected(Instant::now()),
                door: device_states.door,
            };
            (
                State::DevicesInitializing {
                    device_states: new_states,
                    status: "Camera connected - starting...".to_string(),
                },
                vec![Effect::StartCamera],
            )
        }

        (State::DevicesInitializing { device_states, .. }, Event::CameraStartDone(Ok(()))) => {
            let new_states = DeviceStates {
                camera: CameraState::Started,
                door: device_states.door.clone(),
            };

            let status = if device_states.door == DoorState::Initialized {
                "Camera started - ready to capture frames".to_string()
            } else {
                "Camera started - waiting for door".to_string()
            };

            if device_states.door == DoorState::Initialized {
                (
                    State::AnalyzingFramesCapture {
                        door_state: DoorState::Locked,
                        status,
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (
                    State::DevicesInitializing {
                        device_states: new_states,
                        status,
                    },
                    vec![],
                )
            }
        }

        (
            State::DevicesInitializing { device_states, .. },
            Event::DoorEvent(DeviceDoorEvent::Connected),
        ) => {
            let new_states = DeviceStates {
                camera: device_states.camera,
                door: DoorState::Connected(Instant::now()),
            };
            (
                State::DevicesInitializing {
                    device_states: new_states,
                    status: "Door connected - initializing...".to_string(),
                },
                vec![Effect::LockDoor],
            )
        }

        (State::DevicesInitializing { device_states, .. }, Event::DoorLockDone(Ok(()))) => {
            let new_states = DeviceStates {
                camera: device_states.camera.clone(),
                door: DoorState::Initialized,
            };

            let status = if device_states.camera == CameraState::Started {
                "Door initialized - starting capture".to_string()
            } else {
                "Door initialized - waiting for camera".to_string()
            };

            if device_states.camera == CameraState::Started {
                (
                    State::AnalyzingFramesCapture {
                        door_state: DoorState::Locked,
                        status,
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (
                    State::DevicesInitializing {
                        device_states: new_states,
                        status,
                    },
                    vec![],
                )
            }
        }

        // Main operation flow
        (
            State::AnalyzingFramesCapture { door_state, .. },
            Event::FramesCaptureDone(Ok(frames)),
        ) => {
            if frames.is_empty() {
                (
                    State::AnalyzingFramesCapture {
                        door_state,
                        status: "No frames captured - retrying...".to_string(),
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (
                    State::AnalyzingFramesClassifying {
                        door_state: door_state.clone(),
                        frames: frames.clone(),
                        status: format!("Analyzing {} frames...", frames.len()),
                    },
                    vec![Effect::ClassifyFrames { frames }],
                )
            }
        }

        (
            State::AnalyzingFramesClassifying { door_state, .. },
            Event::FramesClassifyDone(Ok(classifications)),
        ) => {
            let dog_detected = classifications.iter().any(|frame_class| {
                frame_class.iter().any(|c| {
                    config.unlock_list.iter().any(|unlock_config| {
                        c.label
                            .to_lowercase()
                            .contains(&unlock_config.label.to_lowercase())
                            && c.confidence >= unlock_config.min_confidence
                    })
                })
            });

            let cat_detected = classifications.iter().any(|frame_class| {
                frame_class.iter().any(|c| {
                    config.lock_list.iter().any(|lock_config| {
                        c.label
                            .to_lowercase()
                            .contains(&lock_config.label.to_lowercase())
                            && c.confidence >= lock_config.min_confidence
                    })
                })
            });

            match (dog_detected, cat_detected, door_state.clone()) {
                (true, false, _) => (
                    State::ControllingDoor {
                        action: DoorAction::Unlocking,
                        door_state: door_state.clone(),
                        start_time: Instant::now(),
                        status: "Dog detected - unlocking door".to_string(),
                    },
                    vec![Effect::UnlockDoor],
                ),
                (false, true, DoorState::Unlocked) => (
                    State::LockingGracePeriod {
                        door_state,
                        countdown_start: Instant::now(),
                        last_detection: Instant::now(),
                        status: "Cat detected - preparing to lock".to_string(),
                    },
                    vec![],
                ),
                (false, true, _) => (
                    State::Idle {
                        door_state,
                        status: "Cat detected - door remains locked".to_string(),
                        last_activity: Instant::now(),
                    },
                    vec![],
                ),
                _ => (
                    State::Idle {
                        door_state,
                        status: "No relevant animals detected".to_string(),
                        last_activity: Instant::now(),
                    },
                    vec![],
                ),
            }
        }

        // Grace period handling
        (
            State::LockingGracePeriod {
                door_state,
                countdown_start,
                last_detection,
                ..
            },
            Event::Tick(now),
        ) => {
            let grace_elapsed = now.duration_since(countdown_start);
            let since_last_detection = now.duration_since(last_detection);

            if grace_elapsed >= config.locking_grace_period {
                (
                    State::ControllingDoor {
                        action: DoorAction::Locking,
                        door_state,
                        start_time: Instant::now(),
                        status: "Locking grace period ended - locking door".to_string(),
                    },
                    vec![Effect::LockDoor],
                )
            } else if since_last_detection >= Duration::from_secs(5) {
                (
                    State::AnalyzingFramesCapture {
                        door_state,
                        status: "No recent detections - verifying...".to_string(),
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                let remaining = config.locking_grace_period - grace_elapsed;
                (
                    State::LockingGracePeriod {
                        door_state,
                        countdown_start,
                        last_detection,
                        status: format!("Locking in {} seconds", remaining.as_secs()),
                    },
                    vec![],
                )
            }
        }

        (
            State::UnlockedGracePeriod {
                door_state,
                countdown_start,
                ..
            },
            Event::Tick(now),
        ) => {
            if now.duration_since(countdown_start) >= config.unlock_grace_period {
                (
                    State::AnalyzingFramesCapture {
                        door_state,
                        status: "Unlock grace period ended".to_string(),
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                let elapsed = now.duration_since(countdown_start);
                (
                    State::UnlockedGracePeriod {
                        door_state,
                        countdown_start,
                        status: format!("Unlocked for {} seconds", elapsed.as_secs()),
                    },
                    vec![],
                )
            }
        }

        // Door control results
        (State::ControllingDoor { .. }, Event::DoorLockDone(Ok(_))) => (
            State::Idle {
                door_state: DoorState::Locked,
                status: "Door locked successfully".to_string(),
                last_activity: Instant::now(),
            },
            vec![],
        ),

        (State::ControllingDoor { .. }, Event::DoorUnlockDone(Ok(_))) => (
            State::UnlockedGracePeriod {
                door_state: DoorState::Unlocked,
                countdown_start: Instant::now(),
                status: "Door unlocked successfully".to_string(),
            },
            vec![],
        ),

        (State::ControllingDoor { door_state, .. }, Event::DoorLockDone(Err(e))) => (
            State::Error {
                error: format!("Failed to lock door: {}", e),
                door_state,
                recovery_actions: vec![Effect::LockDoor],
            },
            vec![],
        ),

        (State::ControllingDoor { door_state, .. }, Event::DoorUnlockDone(Err(e))) => (
            State::Error {
                error: format!("Failed to unlock door: {}", e),
                door_state,
                recovery_actions: vec![Effect::UnlockDoor],
            },
            vec![],
        ),

        // Manual override
        (state, Event::ManualOverride(DoorAction::Unlocking)) => match state {
            State::LockingGracePeriod { door_state, .. } | State::Idle { door_state, .. } => (
                State::ControllingDoor {
                    action: DoorAction::Unlocking,
                    door_state,
                    start_time: Instant::now(),
                    status: "Manual unlock initiated".to_string(),
                },
                vec![Effect::UnlockDoor],
            ),
            _ => (state, vec![]),
        },

        (state, Event::ManualOverride(DoorAction::Locking)) => match state {
            State::UnlockedGracePeriod { door_state, .. } => (
                State::ControllingDoor {
                    action: DoorAction::Locking,
                    door_state,
                    start_time: Instant::now(),
                    status: "Manual lock initiated".to_string(),
                },
                vec![Effect::LockDoor],
            ),
            _ => (state, vec![]),
        },

        // Error recovery
        (
            State::Error {
                recovery_actions, ..
            },
            Event::DoorEvent(DeviceDoorEvent::Connected),
        ) => (
            State::DevicesInitializing {
                device_states: DeviceStates {
                    camera: CameraState::Disconnected,
                    door: DoorState::Connected(Instant::now()),
                },
                status: "Door reconnected - recovering".to_string(),
            },
            recovery_actions,
        ),

        (
            State::Error {
                door_state,
                recovery_actions,
                ..
            },
            Event::CameraEvent(DeviceCameraEvent::Connected),
        ) => (
            State::DevicesInitializing {
                device_states: DeviceStates {
                    camera: CameraState::Connected(Instant::now()),
                    door: door_state,
                },
                status: "Camera reconnected - recovering".to_string(),
            },
            recovery_actions,
        ),

        // Periodic checks
        (
            State::Idle {
                door_state,
                last_activity,
                ..
            },
            Event::Tick(now),
        ) => {
            if now.duration_since(last_activity) >= config._analyze_rate {
                (
                    State::AnalyzingFramesCapture {
                        door_state,
                        status: "Periodic check".to_string(),
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (state, vec![])
            }
        }

        // Device disconnection
        (_, Event::CameraEvent(DeviceCameraEvent::Disconnected)) => (
            State::DevicesInitializing {
                device_states: DeviceStates::default(),
                status: "Camera disconnected".to_string(),
            },
            vec![],
        ),

        (_, Event::DoorEvent(DeviceDoorEvent::Disconnected)) => (
            State::DevicesInitializing {
                device_states: DeviceStates::default(),
                status: "Door disconnected".to_string(),
            },
            if matches!(state, State::UnlockedGracePeriod { .. }) {
                vec![Effect::LockDoor]
            } else {
                vec![]
            },
        ),

        // Default case - no state change
        (state, _) => (state, vec![]),
    }
}
