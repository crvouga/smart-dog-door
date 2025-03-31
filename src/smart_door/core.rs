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
pub struct ModelConnecting {
    pub camera: ModelDeviceConnection,
    pub door: ModelDeviceConnection,
}

impl ModelConnecting {
    pub fn init() -> Self {
        ModelConnecting {
            camera: ModelDeviceConnection::default(),
            door: ModelDeviceConnection::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ModelDeviceConnection {
    #[default]
    Connecting,
    Connected,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ModelReady {
    pub camera: ModelCamera,
    pub door: ModelDoor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelCamera {
    pub state: ModelCameraState,
    pub latest_classifications: Vec<Vec<Classification>>,
}
impl Default for ModelCamera {
    fn default() -> Self {
        ModelCamera {
            state: ModelCameraState::default(),
            latest_classifications: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ModelCameraState {
    #[default]
    Idle,
    Capturing,
    Classifying,
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
pub enum Msg {
    Tick(Instant),
    CameraEvent(DeviceCameraEvent),
    DoorEvent(DeviceDoorEvent),
    DoorLockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorUnlockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    FramesCaptureDone(Result<Vec<Frame>, Box<dyn std::error::Error + Send + Sync>>),
    FramesClassifyDone(Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    LockDoor,
    UnlockDoor,
    CaptureFrames,
    ClassifyFrames { frames: Vec<Frame> },
    SubscribeCamera,
    SubscribeDoor,
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
            Effect::SubscribeDoor,
            Effect::SubscribeCamera,
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

pub fn transition(config: &Config, model: Model, msg: Msg) -> (Model, Vec<Effect>) {
    match (model, msg) {
        (Model::Connecting(child), event) => transition_connecting(config, child, event),

        (Model::Ready(child), msg) => transition_ready(config, child, msg),
    }
}

fn transition_connecting(
    _config: &Config,
    state: ModelConnecting,
    event: Msg,
) -> (Model, Vec<Effect>) {
    let model_new = ModelConnecting {
        camera: transition_connecting_camera(state.camera, &event),
        door: transition_connecting_door(state.door, &event),
    };

    if is_all_devices_connected(&model_new) {
        (Model::Ready(ModelReady::default()), vec![])
    } else {
        (Model::Connecting(model_new), vec![])
    }
}

fn is_all_devices_connected(model: &ModelConnecting) -> bool {
    model.camera == ModelDeviceConnection::Connected
        && model.door == ModelDeviceConnection::Connected
}

fn transition_connecting_camera(
    model: ModelDeviceConnection,
    event: &Msg,
) -> ModelDeviceConnection {
    match (model.clone(), event) {
        (ModelDeviceConnection::Connecting, Msg::CameraEvent(DeviceCameraEvent::Connected)) => {
            ModelDeviceConnection::Connected
        }
        (ModelDeviceConnection::Connecting, Msg::CameraEvent(DeviceCameraEvent::Disconnected)) => {
            ModelDeviceConnection::Connecting
        }
        (ModelDeviceConnection::Connected, Msg::CameraEvent(DeviceCameraEvent::Connected)) => {
            ModelDeviceConnection::Connected
        }
        _ => model,
    }
}

fn transition_connecting_door(model: ModelDeviceConnection, event: &Msg) -> ModelDeviceConnection {
    match (model.clone(), event) {
        (ModelDeviceConnection::Connecting, Msg::DoorEvent(DeviceDoorEvent::Connected)) => {
            ModelDeviceConnection::Connected
        }
        (ModelDeviceConnection::Connecting, Msg::DoorEvent(DeviceDoorEvent::Disconnected)) => {
            ModelDeviceConnection::Connecting
        }
        (ModelDeviceConnection::Connected, Msg::DoorEvent(DeviceDoorEvent::Connected)) => {
            ModelDeviceConnection::Connected
        }
        _ => model,
    }
}

fn transition_ready(config: &Config, model: ModelReady, msg: Msg) -> (Model, Vec<Effect>) {
    match (model.clone(), &msg) {
        (_, Msg::CameraEvent(DeviceCameraEvent::Disconnected)) => (
            Model::Connecting(ModelConnecting {
                camera: ModelDeviceConnection::Connecting,
                door: ModelDeviceConnection::Connected,
            }),
            vec![],
        ),

        (_, Msg::DoorEvent(DeviceDoorEvent::Disconnected)) => (
            Model::Connecting(ModelConnecting {
                camera: ModelDeviceConnection::Connected,
                door: ModelDeviceConnection::Connecting,
            }),
            vec![],
        ),

        _ => transition_ready_main(config, model, &msg),
    }
}

fn transition_ready_main(config: &Config, model: ModelReady, msg: &Msg) -> (Model, Vec<Effect>) {
    let camera_result = transition_ready_camera(config, model.camera.clone(), &msg);

    let detection_before = model.camera.to_detection(config);
    let detection_after = camera_result.0.to_detection(config);

    if detection_before != detection_after {
        println!("Detection changed: {:?}", detection_after);
    }

    let door_result = transition_ready_door(model.door, &msg);

    let combined = ModelReady {
        camera: camera_result.0,
        door: door_result.0,
    };

    let mut combined_effects = door_result.1;
    combined_effects.extend(camera_result.1);

    (Model::Ready(combined), combined_effects)
}

fn transition_door_detection_changed(model: ModelDoor, msg: &Msg) -> (ModelDoor, Vec<Effect>) {
    match (model.clone(), msg) {
        (ModelDoor::Locking { .. }, Msg::DoorLockDone(Ok(_))) => (ModelDoor::Locked, vec![]),
        _ => (model, vec![]),
    }
}

fn transition_ready_door(model: ModelDoor, msg: &Msg) -> (ModelDoor, Vec<Effect>) {
    match (model.clone(), msg) {
        (ModelDoor::Locking { .. }, Msg::DoorLockDone(Ok(_))) => (ModelDoor::Locked, vec![]),

        (ModelDoor::Unlocking { .. }, Msg::DoorUnlockDone(Ok(_))) => (ModelDoor::Unlocked, vec![]),

        (ModelDoor::Locking { .. }, Msg::DoorLockDone(Err(_))) => {
            (ModelDoor::Locked, vec![Effect::LockDoor])
        }

        (ModelDoor::Unlocking { .. }, Msg::DoorUnlockDone(Err(_))) => {
            (ModelDoor::Locked, vec![Effect::UnlockDoor])
        }

        _ => (model, vec![]),
    }
}

fn transition_ready_camera(
    _config: &Config,
    model: ModelCamera,
    msg: &Msg,
) -> (ModelCamera, Vec<Effect>) {
    match (model.clone(), msg) {
        (
            ModelCamera {
                state: ModelCameraState::Idle,
                ..
            },
            Msg::Tick(_),
        ) => (
            ModelCamera {
                state: ModelCameraState::Capturing,
                latest_classifications: model.latest_classifications,
            },
            vec![Effect::CaptureFrames],
        ),

        (
            ModelCamera {
                state: ModelCameraState::Capturing,
                ..
            },
            Msg::FramesCaptureDone(Ok(frames)),
        ) => {
            if frames.is_empty() {
                return (
                    ModelCamera {
                        state: ModelCameraState::Idle,
                        latest_classifications: model.latest_classifications,
                    },
                    vec![],
                );
            }

            (
                ModelCamera {
                    state: ModelCameraState::Classifying,
                    latest_classifications: model.latest_classifications,
                },
                vec![Effect::ClassifyFrames {
                    frames: frames.clone(),
                }],
            )
        }

        (
            ModelCamera {
                state: ModelCameraState::Capturing,
                ..
            },
            Msg::FramesCaptureDone(Err(_)),
        ) => (
            ModelCamera {
                state: ModelCameraState::Idle,
                latest_classifications: model.latest_classifications,
            },
            vec![],
        ),

        (
            ModelCamera {
                state: ModelCameraState::Classifying,
                ..
            },
            Msg::FramesClassifyDone(Ok(classifications)),
        ) => (
            ModelCamera {
                state: ModelCameraState::Idle,
                latest_classifications: classifications.clone(),
            },
            vec![],
        ),

        (
            ModelCamera {
                state: ModelCameraState::Classifying,
                ..
            },
            Msg::FramesClassifyDone(Err(_)),
        ) => (
            ModelCamera {
                state: ModelCameraState::Idle,
                latest_classifications: model.latest_classifications,
            },
            vec![],
        ),

        (model, _) => (model, vec![]),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Detection {
    Cat,
    Dog,
    None,
}

impl ModelCamera {
    pub fn to_detection(&self, config: &Config) -> Detection {
        let dog_detected = self.latest_classifications.iter().any(|frame_class| {
            frame_class.iter().any(|c| {
                config.unlock_list.iter().any(|unlock_config| {
                    c.label
                        .to_lowercase()
                        .contains(&unlock_config.label.to_lowercase())
                        && c.confidence >= unlock_config.min_confidence
                })
            })
        });

        let cat_detected = self.latest_classifications.iter().any(|frame_class| {
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
            Detection::Cat
        } else if dog_detected {
            Detection::Dog
        } else {
            Detection::None
        }
    }
}
