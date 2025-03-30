use crate::config::Config;
use crate::device_camera::interface::{DeviceCamera, DeviceCameraEvent};
use crate::device_display::interface::DeviceDisplay;
use crate::device_door::interface::{DeviceDoor, DeviceDoorEvent};
use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::library::logger::interface::Logger;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default, Clone)]
pub struct DeviceStates {
    cameras: Vec<CameraState>,
    door: DoorState,
}

#[derive(Default, Clone)]
pub enum CameraState {
    #[default]
    Disconnected,
    Connected(Instant),
    Started,
}

#[derive(Default, Clone)]
pub enum DoorState {
    #[default]
    Disconnected,
    Connected(Instant),
    Initialized,
    Locked,
    Unlocked,
}

#[derive(Clone)]
pub enum State {
    DevicesInitializing {
        device_states: DeviceStates,
    },
    CapturingFrames {
        door_state: DoorState,
    },
    ClassifyingFrames {
        door_state: DoorState,
    },
    ControllingDoor {
        action: DoorAction,
        #[allow(dead_code)]
        door_state: DoorState,
        start_time: Instant,
    },
    Idle {
        #[allow(dead_code)]
        action: DoorAction,
        door_state: DoorState,
        message: String,
        message_time: Instant,
    },
    UnlockGracePeriod {
        door_state: DoorState,
    },
    LockCountdown {
        door_state: DoorState,
        countdown_start: Instant,
    },
}

#[derive(Clone)]
pub enum DoorAction {
    Locking,
    Unlocking,
}

#[derive(Debug)]
pub enum Event {
    CameraEvent(usize, DeviceCameraEvent),
    // CameraStarting(usize),
    CameraStartDone(usize, Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorEvent(DeviceDoorEvent),
    DoorLockStart,
    DoorLockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorUnlockStart,
    DoorUnlockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    FrameClassifyStart,
    FrameClassifyDone(Result<Vec<Classification>, Box<dyn std::error::Error + Send + Sync>>),
    FramesCaptureStart,
    FramesCaptureDone(Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>),
    DelayStart,
    DelayDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    GracePeriodDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    CountdownTick,
}

impl Event {
    fn to_display_string(&self) -> String {
        match self {
            Event::FramesCaptureDone(Ok(_)) => {
                format!("{:?}", Event::FramesCaptureDone(Ok(vec![])))
            }
            event => format!("{:?}", event),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Effect {
    StartCamera(usize),
    LockDoor,
    UnlockDoor,
    CaptureFrames,
    ClassifyFrames { frames: Vec<Vec<u8>> },
    Delay,
    GracePeriodDelay,
    SubscribeToCameraEvents(usize),
    SubscribeToDoorEvents,
    StartCountdown,
}

impl Effect {
    fn to_display_string(&self) -> String {
        match self {
            Effect::ClassifyFrames { .. } => {
                format!("{:?}", Effect::ClassifyFrames { frames: vec![] })
            }
            effect => format!("{:?}", effect),
        }
    }
}

#[derive(Clone)]
pub struct SmartDoor {
    config: Config,
    logger: Arc<dyn Logger + Send + Sync>,
    device_cameras: Vec<Arc<dyn DeviceCamera + Send + Sync>>,
    device_door: Arc<dyn DeviceDoor + Send + Sync>,
    device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
    image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
    event_sender: Sender<Event>,
    event_receiver: Arc<Mutex<Receiver<Event>>>,
}

impl SmartDoor {
    pub fn new(
        config: Config,
        logger: Arc<dyn Logger + Send + Sync>,
        device_cameras: Vec<Arc<dyn DeviceCamera + Send + Sync>>,
        device_door: Arc<dyn DeviceDoor + Send + Sync>,
        device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
        image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
    ) -> Self {
        let (sender, receiver) = channel();
        Self {
            config,
            logger,
            device_cameras,
            device_door,
            device_display,
            image_classifier,
            event_sender: sender,
            event_receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    fn init(&self) -> (State, Vec<Effect>) {
        let mut effects = vec![Effect::SubscribeToDoorEvents];
        for i in 0..self.device_cameras.len() {
            effects.push(Effect::SubscribeToCameraEvents(i));
        }

        (
            State::DevicesInitializing {
                device_states: DeviceStates {
                    cameras: vec![CameraState::default(); self.device_cameras.len()],
                    door: DoorState::default(),
                },
            },
            effects,
        )
    }

    fn transition(&self, state: State, event: Event) -> (State, Vec<Effect>) {
        match (state.clone(), event) {
            // Device connection handling
            (
                State::DevicesInitializing { mut device_states },
                Event::CameraEvent(camera_idx, DeviceCameraEvent::Connected),
            ) => {
                device_states.cameras[camera_idx] = CameraState::Connected(Instant::now());
                (
                    State::DevicesInitializing { device_states },
                    vec![Effect::StartCamera(camera_idx)],
                )
            }
            (
                State::DevicesInitializing { mut device_states },
                Event::CameraStartDone(camera_idx, Ok(())),
            ) => {
                device_states.cameras[camera_idx] = CameraState::Started;

                // Check if all cameras are started
                let all_cameras_started = device_states
                    .cameras
                    .iter()
                    .all(|state| matches!(state, CameraState::Started));

                if all_cameras_started && matches!(device_states.door, DoorState::Initialized) {
                    (
                        State::CapturingFrames {
                            door_state: DoorState::Locked,
                        },
                        vec![Effect::CaptureFrames],
                    )
                } else {
                    (State::DevicesInitializing { device_states }, vec![])
                }
            }
            (
                State::DevicesInitializing { device_states },
                Event::DoorEvent(DeviceDoorEvent::Connected),
            ) => {
                let new_states = DeviceStates {
                    cameras: device_states.cameras,
                    door: DoorState::Connected(Instant::now()),
                };
                (
                    State::DevicesInitializing {
                        device_states: new_states,
                    },
                    vec![Effect::LockDoor],
                )
            }
            (State::DevicesInitializing { mut device_states }, Event::DoorLockDone(Ok(()))) => {
                device_states.door = DoorState::Initialized;

                let all_cameras_started = device_states
                    .cameras
                    .iter()
                    .all(|state| matches!(state, CameraState::Started));

                if all_cameras_started {
                    (
                        State::CapturingFrames {
                            door_state: DoorState::Locked,
                        },
                        vec![Effect::CaptureFrames],
                    )
                } else {
                    (State::DevicesInitializing { device_states }, vec![])
                }
            }

            // Main loop
            (State::CapturingFrames { door_state }, Event::FramesCaptureDone(Ok(frames))) => (
                State::ClassifyingFrames { door_state },
                vec![Effect::ClassifyFrames { frames }],
            ),
            (
                State::ClassifyingFrames { door_state },
                Event::FrameClassifyDone(Ok(classifications)),
            ) => {
                let should_unlock = classifications.iter().any(|c| {
                    self.config
                        .classification_unlock_list
                        .iter()
                        .any(|unlock_config| {
                            c.label
                                .to_lowercase()
                                .contains(&unlock_config.label.to_lowercase())
                                && c.confidence >= unlock_config.min_confidence
                        })
                });

                let should_lock = classifications.iter().any(|c| {
                    self.config
                        .classification_lock_list
                        .iter()
                        .any(|lock_config| {
                            c.label
                                .to_lowercase()
                                .contains(&lock_config.label.to_lowercase())
                                && c.confidence >= lock_config.min_confidence
                        })
                });

                if should_unlock && !should_lock {
                    (
                        State::ControllingDoor {
                            action: DoorAction::Unlocking,
                            door_state,
                            start_time: Instant::now(),
                        },
                        vec![Effect::UnlockDoor],
                    )
                } else {
                    let message = if should_lock {
                        format!(
                            "{} detected",
                            self.config
                                .classification_lock_list
                                .iter()
                                .map(|c| c.label.as_str())
                                .collect::<Vec<_>>()
                                .join("/")
                        )
                    } else {
                        format!(
                            "No {} detected",
                            self.config
                                .classification_unlock_list
                                .iter()
                                .map(|c| c.label.as_str())
                                .collect::<Vec<_>>()
                                .join("/")
                        )
                    };

                    match door_state {
                        DoorState::Unlocked => (
                            State::LockCountdown {
                                door_state,
                                countdown_start: Instant::now(),
                            },
                            vec![Effect::StartCountdown],
                        ),
                        _ => (
                            State::Idle {
                                action: DoorAction::Locking,
                                door_state,
                                message,
                                message_time: Instant::now(),
                            },
                            vec![Effect::Delay],
                        ),
                    }
                }
            }
            (
                State::LockCountdown {
                    door_state,
                    countdown_start,
                },
                Event::CountdownTick,
            ) => {
                let elapsed = countdown_start.elapsed();
                if elapsed >= Duration::from_secs(5) {
                    (
                        State::ControllingDoor {
                            action: DoorAction::Locking,
                            door_state,
                            start_time: Instant::now(),
                        },
                        vec![Effect::LockDoor],
                    )
                } else {
                    (
                        State::LockCountdown {
                            door_state,
                            countdown_start,
                        },
                        vec![Effect::StartCountdown],
                    )
                }
            }
            (State::ControllingDoor { action, .. }, Event::DoorLockDone(_)) => (
                State::Idle {
                    action,
                    door_state: DoorState::Locked,
                    message: "Door locked".to_string(),
                    message_time: Instant::now(),
                },
                vec![Effect::Delay],
            ),
            (State::ControllingDoor { action, .. }, Event::DoorUnlockDone(result)) => {
                match result {
                    Ok(_) => (
                        State::UnlockGracePeriod {
                            door_state: DoorState::Unlocked,
                        },
                        vec![Effect::GracePeriodDelay],
                    ),
                    Err(_) => (
                        State::Idle {
                            action,
                            door_state: DoorState::Locked,
                            message: "Door locked".to_string(),
                            message_time: Instant::now(),
                        },
                        vec![Effect::Delay],
                    ),
                }
            }
            (State::UnlockGracePeriod { door_state }, Event::GracePeriodDone(result)) => {
                match result {
                    Ok(_) | Err(_) => (
                        State::CapturingFrames { door_state },
                        vec![Effect::CaptureFrames],
                    ),
                }
            }
            (State::Idle { door_state, .. }, Event::DelayDone(result)) => match result {
                Ok(_) | Err(_) => (
                    State::CapturingFrames { door_state },
                    vec![Effect::CaptureFrames],
                ),
            },

            (_, Event::CameraEvent(camera_idx, DeviceCameraEvent::Disconnected)) => {
                let mut device_states = DeviceStates {
                    cameras: vec![CameraState::default(); self.device_cameras.len()],
                    door: DoorState::default(),
                };
                device_states.cameras[camera_idx] = CameraState::Disconnected;
                (State::DevicesInitializing { device_states }, vec![])
            }
            (_, Event::DoorEvent(DeviceDoorEvent::Disconnected)) => {
                let mut effects = vec![];
                if matches!(state, State::UnlockGracePeriod { .. }) {
                    effects.push(Effect::LockDoor);
                }
                (
                    State::DevicesInitializing {
                        device_states: DeviceStates::default(),
                    },
                    effects,
                )
            }

            // Default case
            _ => (state, vec![]),
        }
    }

    fn run_effect(&self, effect: Effect, event_queue: Sender<Event>) {
        let _ = self
            .logger
            .info(&format!("Running effect: {:?}", effect.to_display_string()));

        match effect {
            Effect::SubscribeToDoorEvents => {
                let events = self.device_door.events();
                loop {
                    match events.recv() {
                        Ok(event) => {
                            if event_queue.send(Event::DoorEvent(event)).is_err() {
                                continue;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Effect::SubscribeToCameraEvents(camera_idx) => {
                let events = self.device_cameras[camera_idx].events();
                loop {
                    match events.recv() {
                        Ok(event) => {
                            if event_queue
                                .send(Event::CameraEvent(camera_idx, event))
                                .is_err()
                            {
                                continue;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Effect::StartCamera(camera_idx) => {
                // let _ = event_queue.send(Event::CameraStarting(camera_idx));
                let started = self.device_cameras[camera_idx].start();
                let _ = event_queue.send(Event::CameraStartDone(camera_idx, started));
            }
            Effect::LockDoor => {
                let _ = event_queue.send(Event::DoorLockStart);
                let locked = self.device_door.lock();
                let _ = event_queue.send(Event::DoorLockDone(locked));
            }
            Effect::UnlockDoor => {
                let _ = event_queue.send(Event::DoorUnlockStart);
                let unlocked = self.device_door.unlock();
                let _ = event_queue.send(Event::DoorUnlockDone(unlocked));
            }
            Effect::CaptureFrames => {
                let _ = event_queue.send(Event::FramesCaptureStart);
                let mut frames = Vec::new();
                for camera in &self.device_cameras {
                    match camera.capture_frame() {
                        Ok(frame) => frames.push(frame),
                        Err(e) => {
                            let _ = event_queue.send(Event::FramesCaptureDone(Err(e)));
                            return;
                        }
                    }
                }
                let _ = event_queue.send(Event::FramesCaptureDone(Ok(frames)));
            }
            Effect::ClassifyFrames { frames } => {
                let _ = event_queue.send(Event::FrameClassifyStart);
                let mut all_classifications = Vec::new();
                for frame in frames {
                    match self.image_classifier.classify(&frame) {
                        Ok(classifications) => all_classifications.extend(classifications),
                        Err(e) => {
                            let _ = event_queue.send(Event::FrameClassifyDone(Err(e)));
                            return;
                        }
                    }
                }
                let _ = event_queue.send(Event::FrameClassifyDone(Ok(all_classifications)));
            }
            Effect::Delay => {
                let _ = event_queue.send(Event::DelayStart);
                std::thread::sleep(self.config.classification_rate);
                let _ = event_queue.send(Event::DelayDone(Ok(())));
            }
            Effect::GracePeriodDelay => {
                std::thread::sleep(self.config.unlock_grace_period);
                let _ = event_queue.send(Event::GracePeriodDone(Ok(())));
            }
            Effect::StartCountdown => {
                std::thread::sleep(Duration::from_secs(1));
                let _ = event_queue.send(Event::CountdownTick);
            }
        }
    }

    fn render(&self, state: &State) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let mut device_display = self.device_display.lock().unwrap();
        device_display.write_line(0, "")?;
        device_display.write_line(1, "")?;

        match state {
            State::DevicesInitializing { device_states } => {
                let mut camera_status = String::new();
                for (i, camera_state) in device_states.cameras.iter().enumerate() {
                    match camera_state {
                        CameraState::Disconnected => {
                            camera_status.push_str(&format!("Camera {} connecting... ", i + 1));
                        }
                        CameraState::Connected(time) => {
                            if time.elapsed() > Duration::from_secs(2) {
                                camera_status.push_str(&format!("Camera {} connected ", i + 1));
                            } else {
                                camera_status.push_str(&format!("Camera {} connecting... ", i + 1));
                            }
                        }
                        CameraState::Started => {
                            camera_status.push_str(&format!("Camera {} connected ", i + 1));
                        }
                    }
                }
                device_display.write_line(0, &camera_status)?;

                match device_states.door {
                    DoorState::Disconnected => {
                        device_display.write_line(1, "Door connecting...")?;
                    }
                    DoorState::Connected(time) => {
                        if time.elapsed() > Duration::from_secs(2) {
                            device_display.write_line(1, "Door connected")?;
                        } else {
                            device_display.write_line(1, "Door connecting...")?;
                        }
                    }
                    _ => {
                        device_display.write_line(1, "Door connected")?;
                    }
                }
            }
            State::CapturingFrames { .. } | State::ClassifyingFrames { .. } => {
                device_display.write_line(0, "Analyzing...")?;
            }
            State::ControllingDoor {
                action, start_time, ..
            } => match action {
                DoorAction::Locking => {
                    if start_time.elapsed() > Duration::from_secs(2) {
                        device_display.write_line(0, "Door locked")?;
                    } else {
                        device_display.write_line(0, "Locking door...")?;
                    }
                }
                DoorAction::Unlocking => {
                    if start_time.elapsed() > Duration::from_secs(2) {
                        device_display.write_line(0, "Door unlocked")?;
                    } else {
                        device_display.write_line(0, "Unlocking door...")?;
                    }
                }
            },
            State::UnlockGracePeriod { .. } => {
                device_display.write_line(0, "Door unlocked")?;
            }
            State::LockCountdown {
                countdown_start, ..
            } => {
                let remaining = 5 - countdown_start.elapsed().as_secs();
                device_display.write_line(0, &format!("Locking in {}...", remaining))?;
            }
            State::Idle {
                message,
                message_time,
                ..
            } => {
                if message_time.elapsed() > Duration::from_secs(2) {
                    device_display.write_line(0, "Analyzing...")?;
                } else {
                    device_display.write_line(0, message)?;
                }
            }
        }

        Ok(())
    }

    fn spawn_effects(&self, effects: Vec<Effect>) {
        for effect in effects {
            let effect_sender = self.event_sender.clone();
            let effect_clone = effect.clone();
            let self_clone = self.clone();
            std::thread::spawn(move || self_clone.run_effect(effect_clone, effect_sender));
        }
    }

    pub fn run(&self) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let (mut current_state, effects) = self.init();

        self.spawn_effects(effects);

        loop {
            match self.event_receiver.lock().unwrap().recv() {
                Ok(event) => {
                    let _ = self.logger.info(&format!(
                        "Processing event: {:?}",
                        event.to_display_string()
                    ));

                    let (new_state, new_effects) = self.transition(current_state.clone(), event);
                    current_state = new_state;
                    self.render(&current_state)?;

                    self.spawn_effects(new_effects);
                }
                Err(e) => {
                    return Err(Arc::new(e));
                }
            }
        }
    }
}
