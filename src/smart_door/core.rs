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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelCameraState {
    Idle { start_time: Instant },
    Capturing { start_time: Instant },
    Classifying { start_time: Instant },
}

impl Default for ModelCameraState {
    fn default() -> Self {
        ModelCameraState::Idle {
            start_time: Instant::now(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelDoor {
    Locked,
    WillUnlock { start_time: Instant },
    Unlocked,
    WillLock { start_time: Instant },
}

impl Default for ModelDoor {
    fn default() -> Self {
        ModelDoor::Locked
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

pub fn transition(config: &Config, model: Model, msg: Msg) -> (Model, Vec<Effect>) {
    match (model.clone(), msg) {
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

        _ => {
            let (model_new, effects) = transition_ready_main(config, model, &msg);
            (Model::Ready(model_new), effects)
        }
    }
}

fn transition_ready_main(
    config: &Config,
    model: ModelReady,
    msg: &Msg,
) -> (ModelReady, Vec<Effect>) {
    let mut effects = vec![];
    let camera = transition_ready_camera(config, model.camera.clone(), &msg);
    effects.extend(camera.1);

    let detection_before = to_detection(&model.camera, config);
    let detection_after = to_detection(&camera.0, config);

    let door = transition_door_on_detection_change(model.door, detection_before, detection_after);
    effects.extend(door.1);

    let door = transition_door_on_tick(config, door.0, &msg);
    effects.extend(door.1);

    let model_new = ModelReady {
        camera: camera.0,
        door: door.0,
    };

    (model_new, effects)
}

fn transition_door_on_detection_change(
    model: ModelDoor,
    detection_before: Detection,
    detection_after: Detection,
) -> (ModelDoor, Vec<Effect>) {
    if detection_before == detection_after {
        return (model, vec![]);
    }

    match (model, detection_after) {
        (ModelDoor::Locked, Detection::Dog) => (
            ModelDoor::WillUnlock {
                start_time: Instant::now(),
            },
            vec![],
        ),
        (ModelDoor::WillUnlock { .. }, Detection::Cat) => {
            (ModelDoor::Locked, vec![Effect::LockDoor])
        }
        (ModelDoor::Unlocked, Detection::None) => (
            ModelDoor::WillLock {
                start_time: Instant::now(),
            },
            vec![],
        ),
        (ModelDoor::Unlocked, Detection::Cat) => (
            ModelDoor::WillLock {
                start_time: Instant::now(),
            },
            vec![],
        ),
        (door, _) => (door, vec![]),
    }
}

fn transition_door_on_tick(
    config: &Config,
    door: ModelDoor,
    msg: &Msg,
) -> (ModelDoor, Vec<Effect>) {
    match (door.clone(), msg) {
        (ModelDoor::WillUnlock { start_time }, Msg::Tick(now)) => {
            if now.duration_since(start_time) >= config.minimal_duration_unlocking {
                (ModelDoor::Unlocked, vec![Effect::UnlockDoor])
            } else {
                (door, vec![])
            }
        }
        (ModelDoor::WillLock { start_time }, Msg::Tick(now)) => {
            if now.duration_since(start_time) >= config.minimal_duration_locking {
                (ModelDoor::Locked, vec![Effect::LockDoor])
            } else {
                (door, vec![])
            }
        }
        (ModelDoor::WillUnlock { .. }, Msg::DoorUnlockDone(Ok(_))) => (ModelDoor::Unlocked, vec![]),
        (ModelDoor::WillLock { .. }, Msg::DoorLockDone(Ok(_))) => (ModelDoor::Locked, vec![]),
        _ => (door, vec![]),
    }
}

fn transition_ready_camera(
    config: &Config,
    model: ModelCamera,
    msg: &Msg,
) -> (ModelCamera, Vec<Effect>) {
    match (model.clone(), msg) {
        (
            ModelCamera {
                state: ModelCameraState::Idle { start_time },
                ..
            },
            Msg::Tick(now),
        ) => {
            let should_process =
                now.duration_since(start_time) >= config.minimal_rate_camera_process;

            if should_process {
                (
                    ModelCamera {
                        state: ModelCameraState::Capturing { start_time: *now },
                        latest_classifications: model.latest_classifications,
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (model, vec![])
            }
        }

        (
            ModelCamera {
                state: ModelCameraState::Capturing { start_time },
                ..
            },
            Msg::FramesCaptureDone(Ok(frames)),
        ) => {
            if frames.is_empty() {
                return (
                    ModelCamera {
                        state: ModelCameraState::Idle { start_time },
                        latest_classifications: model.latest_classifications,
                    },
                    vec![],
                );
            }

            (
                ModelCamera {
                    state: ModelCameraState::Classifying { start_time },
                    latest_classifications: model.latest_classifications,
                },
                vec![Effect::ClassifyFrames {
                    frames: frames.clone(),
                }],
            )
        }

        (
            ModelCamera {
                state: ModelCameraState::Capturing { start_time },
                ..
            },
            Msg::FramesCaptureDone(Err(_)),
        ) => (
            ModelCamera {
                state: ModelCameraState::Idle { start_time },
                latest_classifications: model.latest_classifications,
            },
            vec![],
        ),

        (
            ModelCamera {
                state: ModelCameraState::Classifying { start_time },
                ..
            },
            Msg::FramesClassifyDone(Ok(classifications)),
        ) => (
            ModelCamera {
                state: ModelCameraState::Idle { start_time },
                latest_classifications: classifications.clone(),
            },
            vec![],
        ),

        (
            ModelCamera {
                state: ModelCameraState::Classifying { start_time },
                ..
            },
            Msg::FramesClassifyDone(Err(_)),
        ) => (
            ModelCamera {
                state: ModelCameraState::Idle { start_time },
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

pub fn to_detection(camera: &ModelCamera, config: &Config) -> Detection {
    let dog_detected = camera.latest_classifications.iter().any(|frame_class| {
        frame_class.iter().any(|c| {
            config
                .classification_unlock_list
                .iter()
                .any(|unlock_config| {
                    c.label
                        .to_lowercase()
                        .contains(&unlock_config.label.to_lowercase())
                        && c.confidence >= unlock_config.min_confidence
                })
        })
    });

    let cat_detected = camera.latest_classifications.iter().any(|frame_class| {
        frame_class.iter().any(|c| {
            config.classification_lock_list.iter().any(|lock_config| {
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
