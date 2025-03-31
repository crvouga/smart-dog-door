use crate::config::Config;
use crate::device_camera::interface::{DeviceCameraEvent, Frame};
use crate::device_door::interface::DeviceDoorEvent;
use crate::image_classifier::interface::Classification;
use std::time::Instant;

//
//
//
// Model
//
//
//

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Connecting(StateConnecting),
    Connected(StateConnected),
}

impl Default for State {
    fn default() -> Self {
        State::Connecting(StateConnecting::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum StateConnecting {
    #[default]
    Connecting,
    OnlyCameraConnecting,
    OnlyDoorConnecting,
}

impl StateConnecting {
    pub fn init() -> Self {
        StateConnecting::Connecting
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct StateConnected {
    camera: StateCamera,
    door: StateDoor,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct StateCamera {
    frames: StateCameraFrames,
    classifications: Vec<Vec<Classification>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateCameraFrames {
    Idle { start_time: Instant },
    Capturing,
    Classifying(Vec<Frame>),
}

impl Default for StateCameraFrames {
    fn default() -> Self {
        StateCameraFrames::Idle {
            start_time: Instant::now(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateDoor {
    LockingGracePeriod {
        start_time: Instant,
        countdown_start: Instant,
    },
    Locking,
    Locked,
    UnlockingGracePeriod {
        start_time: Instant,
        countdown_start: Instant,
    },
    Unlocking,
    Unlocked,
}

impl Default for StateDoor {
    fn default() -> Self {
        StateDoor::Unlocked
    }
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

//
//
//
// Init
//
//
//

pub fn init() -> (State, Vec<Effect>) {
    (
        State::default(),
        vec![
            Effect::SubscribeToDoorEvents,
            Effect::SubscribeToCameraEvents,
            Effect::SubscribeTick,
        ],
    )
}

//
//
//
// Transition
//
//
//

pub fn transition(config: &Config, state: State, event: Event) -> (State, Vec<Effect>) {
    // Handle other state transitions
    match (state, event) {
        (_, Event::CameraEvent(DeviceCameraEvent::Disconnected)) => {
            (State::Connecting(StateConnecting::Connecting), vec![])
        }

        (_, Event::DoorEvent(DeviceDoorEvent::Disconnected)) => {
            (State::Connecting(StateConnecting::Connecting), vec![])
        }

        (State::Connecting(child), event) => transition_connecting(config, child, event),

        (State::Connected(child), event) => transition_connected(config, child, event),
    }
}

fn transition_connecting(
    _config: &Config,
    state: StateConnecting,
    event: Event,
) -> (State, Vec<Effect>) {
    match (state, event) {
        // Initial connection of both devices
        (StateConnecting::Connecting, Event::CameraEvent(DeviceCameraEvent::Connected)) => (
            State::Connecting(StateConnecting::OnlyDoorConnecting),
            vec![Effect::StartCamera],
        ),

        (StateConnecting::Connecting, Event::DoorEvent(DeviceDoorEvent::Connected)) => (
            State::Connecting(StateConnecting::OnlyCameraConnecting),
            vec![],
        ),

        // Camera connection handling
        (StateConnecting::OnlyDoorConnecting, Event::CameraStartDone(Ok(()))) => (
            State::Connected(StateConnected {
                camera: StateCamera {
                    frames: StateCameraFrames::Capturing,
                    classifications: vec![],
                },
                door: StateDoor::Unlocked,
            }),
            vec![],
        ),

        // Door connection handling
        (StateConnecting::OnlyCameraConnecting, Event::DoorEvent(DeviceDoorEvent::Connected)) => (
            State::Connected(StateConnected {
                camera: StateCamera {
                    frames: StateCameraFrames::Capturing,
                    classifications: vec![],
                },
                door: StateDoor::Unlocked,
            }),
            vec![],
        ),

        // Error cases
        (StateConnecting::OnlyDoorConnecting, Event::CameraStartDone(Err(_))) => (
            State::Connecting(StateConnecting::OnlyDoorConnecting),
            vec![Effect::StartCamera], // Retry
        ),

        // Disconnection handling
        (_, Event::CameraEvent(DeviceCameraEvent::Disconnected)) => {
            (State::Connecting(StateConnecting::Connecting), vec![])
        }

        (_, Event::DoorEvent(DeviceDoorEvent::Disconnected)) => {
            (State::Connecting(StateConnecting::Connecting), vec![])
        }

        // Default case - no state change
        (state, _) => (State::Connecting(state), vec![]),
    }
}

fn transition_connected(
    config: &Config,
    state: StateConnected,
    event: Event,
) -> (State, Vec<Effect>) {
    // Process door state transitions
    let door_result = transition_door(state.door, &event);

    // Process frame analysis state transitions
    let frame_result = transition_frame(config, state.camera, event);

    // Combine results
    let combined_state = StateConnected {
        camera: frame_result.0,
        door: door_result.0,
    };

    let mut combined_effects = door_result.1;
    combined_effects.extend(frame_result.1);

    (State::Connected(combined_state), combined_effects)
}

fn transition_door(current: StateDoor, event: &Event) -> (StateDoor, Vec<Effect>) {
    match (current.clone(), event) {
        (StateDoor::Locking { .. }, Event::DoorLockDone(Ok(_))) => (StateDoor::Locked, vec![]),

        (StateDoor::Unlocking { .. }, Event::DoorUnlockDone(Ok(_))) => {
            (StateDoor::Unlocked, vec![])
        }

        (StateDoor::Locking { .. }, Event::DoorLockDone(Err(_))) => (
            StateDoor::Locked,
            vec![Effect::LockDoor], // Retry
        ),

        (StateDoor::Unlocking { .. }, Event::DoorUnlockDone(Err(_))) => (
            StateDoor::Locked,
            vec![Effect::UnlockDoor], // Retry
        ),

        // No change for other events
        _ => (current, vec![]),
    }
}

fn transition_frame(
    config: &Config,
    current: StateCamera,
    event: Event,
) -> (StateCamera, Vec<Effect>) {
    match (current, event) {
        // Frame capture transitions
        (
            StateCamera {
                frames: StateCameraFrames::Capturing,
                ..
            },
            Event::FramesCaptureDone(Ok(frames)),
        ) => {
            if frames.is_empty() {
                (
                    StateCamera {
                        frames: StateCameraFrames::Capturing,
                        classifications: vec![],
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (
                    StateCamera {
                        frames: StateCameraFrames::Classifying(frames.clone()),
                        classifications: vec![],
                    },
                    vec![Effect::ClassifyFrames {
                        frames: frames.clone(),
                    }],
                )
            }
        }

        // Frame classification transitions
        (
            StateCamera {
                frames: StateCameraFrames::Classifying(..),
                ..
            },
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

            // Note: Door effects are handled separately now
            if dog_detected && !cat_detected {
                (
                    StateCamera {
                        frames: StateCameraFrames::Capturing,
                        classifications: vec![],
                    },
                    vec![Effect::UnlockDoor],
                )
            } else if cat_detected {
                (
                    StateCamera {
                        frames: StateCameraFrames::Capturing,
                        classifications: vec![],
                    },
                    vec![Effect::LockDoor],
                )
            } else {
                (
                    StateCamera {
                        frames: StateCameraFrames::Capturing,
                        classifications: vec![],
                    },
                    vec![Effect::CaptureFrames],
                )
            }
        }

        // Error cases
        (
            StateCamera {
                frames: StateCameraFrames::Capturing,
                classifications,
            },
            Event::FramesCaptureDone(Err(_)),
        ) => (
            StateCamera {
                frames: StateCameraFrames::Capturing,
                classifications,
            },
            vec![Effect::CaptureFrames], // Retry
        ),

        (
            StateCamera {
                frames: StateCameraFrames::Classifying(..),
                classifications,
            },
            Event::FramesClassifyDone(Err(_)),
        ) => (
            StateCamera {
                frames: StateCameraFrames::Capturing,
                classifications,
            },
            vec![Effect::CaptureFrames], // Retry with new frames
        ),
        // Periodic checks
        (
            StateCamera {
                frames: StateCameraFrames::Capturing,
                classifications,
            },
            Event::Tick(_),
        ) => (
            StateCamera {
                frames: StateCameraFrames::Capturing,
                classifications,
            },
            vec![Effect::CaptureFrames],
        ),

        // No change for other events
        (current, _) => (current, vec![]),
    }
}
