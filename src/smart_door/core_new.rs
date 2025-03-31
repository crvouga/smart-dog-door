use crate::config::Config;
use crate::device_camera::interface::{DeviceCameraEvent, Frame};
use crate::device_door::interface::DeviceDoorEvent;
use crate::image_classifier::interface::Classification;
use std::time::Instant;

//
//
//

//
//
//

#[derive(Clone, Debug, PartialEq)]
pub enum Model {
    Connecting(ModelConnecting),
    Ready(ModelReady),
}

impl Default for Model {
    fn default() -> Self {
        Model::Connecting(ModelConnecting::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ModelConnecting {
    #[default]
    Connecting,
    OnlyCameraConnecting,
    OnlyDoorConnecting,
}

impl ModelConnecting {
    pub fn init() -> Self {
        ModelConnecting::Connecting
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ModelReady {
    camera: ModelCamera,
    door: ModelDoor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelCamera {
    Idle,
    Capturing,
    Classifying,
}

impl Default for ModelCamera {
    fn default() -> Self {
        ModelCamera::Idle
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelDoor {
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

impl Default for ModelDoor {
    fn default() -> Self {
        ModelDoor::Unlocked
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
    FramesCaptureDone(Result<Vec<Frame>, Box<dyn std::error::Error + Send + Sync>>),
    FramesClassifyDone(Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>>),
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

//
//
//

pub fn init() -> (Model, Vec<Effect>) {
    (
        Model::default(),
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

//
//
//

pub fn transition(config: &Config, state: Model, event: Event) -> (Model, Vec<Effect>) {
    match (state, event) {
        (_, Event::CameraEvent(DeviceCameraEvent::Disconnected)) => {
            (Model::Connecting(ModelConnecting::Connecting), vec![])
        }

        (_, Event::DoorEvent(DeviceDoorEvent::Disconnected)) => {
            (Model::Connecting(ModelConnecting::Connecting), vec![])
        }

        (Model::Connecting(child), event) => transition_connecting(config, child, event),

        (Model::Ready(child), event) => transition_ready(config, child, event),
    }
}

fn transition_connecting(
    _config: &Config,
    state: ModelConnecting,
    event: Event,
) -> (Model, Vec<Effect>) {
    match (state, event) {
        (ModelConnecting::Connecting, Event::CameraEvent(DeviceCameraEvent::Connected)) => (
            Model::Connecting(ModelConnecting::OnlyDoorConnecting),
            vec![Effect::StartCamera],
        ),

        (ModelConnecting::Connecting, Event::DoorEvent(DeviceDoorEvent::Connected)) => (
            Model::Connecting(ModelConnecting::OnlyCameraConnecting),
            vec![],
        ),

        (ModelConnecting::OnlyDoorConnecting, Event::CameraStartDone(Ok(()))) => {
            (Model::Ready(ModelReady::default()), vec![])
        }

        (ModelConnecting::OnlyCameraConnecting, Event::DoorEvent(DeviceDoorEvent::Connected)) => {
            (Model::Ready(ModelReady::default()), vec![])
        }

        (ModelConnecting::OnlyDoorConnecting, Event::CameraStartDone(Err(_))) => (
            Model::Connecting(ModelConnecting::OnlyDoorConnecting),
            vec![Effect::StartCamera],
        ),

        (_, Event::CameraEvent(DeviceCameraEvent::Disconnected)) => {
            (Model::Connecting(ModelConnecting::Connecting), vec![])
        }

        (_, Event::DoorEvent(DeviceDoorEvent::Disconnected)) => {
            (Model::Connecting(ModelConnecting::Connecting), vec![])
        }

        (state, _) => (Model::Connecting(state), vec![]),
    }
}

fn transition_ready(config: &Config, state: ModelReady, event: Event) -> (Model, Vec<Effect>) {
    let door_result = transition_door(state.door, &event);
    let camera_result = transition_camera(config, state.camera, event);

    let combined = ModelReady {
        camera: camera_result.0,
        door: door_result.0,
    };

    let mut combined_effects = door_result.1;
    combined_effects.extend(camera_result.1);

    (Model::Ready(combined), combined_effects)
}

fn transition_door(current: ModelDoor, event: &Event) -> (ModelDoor, Vec<Effect>) {
    match (current.clone(), event) {
        (ModelDoor::Locking { .. }, Event::DoorLockDone(Ok(_))) => (ModelDoor::Locked, vec![]),

        (ModelDoor::Unlocking { .. }, Event::DoorUnlockDone(Ok(_))) => {
            (ModelDoor::Unlocked, vec![])
        }

        (ModelDoor::Locking { .. }, Event::DoorLockDone(Err(_))) => {
            (ModelDoor::Locked, vec![Effect::LockDoor])
        }

        (ModelDoor::Unlocking { .. }, Event::DoorUnlockDone(Err(_))) => {
            (ModelDoor::Locked, vec![Effect::UnlockDoor])
        }

        _ => (current, vec![]),
    }
}

fn transition_camera(
    config: &Config,
    model: ModelCamera,
    event: Event,
) -> (ModelCamera, Vec<Effect>) {
    match (model.clone(), event) {
        (ModelCamera::Capturing { .. }, Event::FramesCaptureDone(Ok(frames))) => {
            if frames.is_empty() {
                return (model, vec![]);
            }

            (
                ModelCamera::Classifying,
                vec![Effect::ClassifyFrames {
                    frames: frames.clone(),
                }],
            )
        }

        (ModelCamera::Classifying { .. }, Event::FramesClassifyDone(Ok(classifications))) => {
            let outcome = to_classification_outcome(config, &classifications);

            let model_new = ModelCamera::Idle;

            match outcome {
                ClassificationOutcome::CatDetected => (model_new, vec![Effect::LockDoor]),
                ClassificationOutcome::DogDetected => (model_new, vec![Effect::UnlockDoor]),
                ClassificationOutcome::NoDetection => (model_new, vec![]),
            }
        }

        (ModelCamera::Classifying { .. }, Event::FramesClassifyDone(Err(_))) => {
            (ModelCamera::Capturing, vec![Effect::CaptureFrames])
        }

        (ModelCamera::Idle, Event::Tick(_)) => {
            (ModelCamera::Capturing, vec![Effect::CaptureFrames])
        }

        (current, _) => (current, vec![]),
    }
}

enum ClassificationOutcome {
    CatDetected,
    DogDetected,
    NoDetection,
}

fn to_classification_outcome(
    config: &Config,
    classifications: &Vec<Vec<Classification>>,
) -> ClassificationOutcome {
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

    if cat_detected {
        ClassificationOutcome::CatDetected
    } else if dog_detected {
        ClassificationOutcome::DogDetected
    } else {
        ClassificationOutcome::NoDetection
    }
}
