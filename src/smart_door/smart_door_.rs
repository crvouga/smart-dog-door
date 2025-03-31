use crate::config::Config;
use crate::device_camera::interface::{DeviceCamera, DeviceCameraEvent};
use crate::device_display::interface::DeviceDisplay;
use crate::device_door::interface::{DeviceDoor, DeviceDoorEvent};
use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::library::logger::interface::Logger;
use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default, Clone)]
pub struct DeviceStates {
    camera: CameraState,
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
    AnalyzingFramesCapture {
        door_state: DoorState,
    },
    AnalyzingFramesClassifying {
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
        last_classification: Instant,
    },
    UnlockedGracePeriod {
        door_state: DoorState,
        countdown_start: Instant,
    },
    LockingGracePeriod {
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
    Tick(Instant),
    CameraEvent(DeviceCameraEvent),
    CameraStartDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorEvent(DeviceDoorEvent),
    DoorLockStart,
    DoorLockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorUnlockStart,
    DoorUnlockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    FramesClassifyStart,
    FramesClassifyDone(Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>>),
    FramesCaptureStart,
    FramesCaptureDone(Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>),
}

impl Event {
    fn to_display_string(&self) -> String {
        match self {
            Event::FramesCaptureDone(Ok(_frames)) => {
                format!("{:?}", Event::FramesCaptureDone(Ok(vec![])))
            }
            event => format!("{:?}", event),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Effect {
    StartCamera,
    LockDoor,
    UnlockDoor,
    CaptureFrames,
    ClassifyFrames { frames: Vec<Vec<u8>> },
    SubscribeToCameraEvents,
    SubscribeToDoorEvents,
    SubscribeTick,
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
    device_camera: Arc<dyn DeviceCamera + Send + Sync>,
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
        device_camera: Arc<dyn DeviceCamera + Send + Sync>,
        device_door: Arc<dyn DeviceDoor + Send + Sync>,
        device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
        image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
    ) -> Self {
        let (sender, receiver) = channel();
        Self {
            config,
            logger,
            device_camera,
            device_door,
            device_display,
            image_classifier,
            event_sender: sender,
            event_receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    fn init(&self) -> (State, Vec<Effect>) {
        (
            State::DevicesInitializing {
                device_states: DeviceStates::default(),
            },
            vec![
                Effect::SubscribeToDoorEvents,
                Effect::SubscribeToCameraEvents,
                Effect::SubscribeTick,
            ],
        )
    }

    fn transition(&self, state: State, event: Event) -> (State, Vec<Effect>) {
        match (state.clone(), event) {
            // Device connection handling
            (
                State::DevicesInitializing { mut device_states },
                Event::CameraEvent(DeviceCameraEvent::Connected),
            ) => {
                device_states.camera = CameraState::Connected(Instant::now());
                (
                    State::DevicesInitializing { device_states },
                    vec![Effect::StartCamera],
                )
            }
            (State::DevicesInitializing { mut device_states }, Event::CameraStartDone(Ok(()))) => {
                device_states.camera = CameraState::Started;

                if matches!(device_states.door, DoorState::Initialized) {
                    (
                        State::AnalyzingFramesCapture {
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
                    camera: device_states.camera,
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

                if matches!(device_states.camera, CameraState::Started) {
                    (
                        State::AnalyzingFramesCapture {
                            door_state: DoorState::Locked,
                        },
                        vec![Effect::CaptureFrames],
                    )
                } else {
                    (State::DevicesInitializing { device_states }, vec![])
                }
            }

            // Main loop
            (
                State::AnalyzingFramesCapture { door_state },
                Event::FramesCaptureDone(Ok(frames)),
            ) => {
                if !frames.is_empty() {
                    (
                        State::AnalyzingFramesClassifying { door_state },
                        vec![Effect::ClassifyFrames { frames }],
                    )
                } else {
                    (
                        State::AnalyzingFramesCapture { door_state },
                        vec![Effect::CaptureFrames],
                    )
                }
            }
            (
                State::AnalyzingFramesClassifying { door_state },
                Event::FramesClassifyDone(Ok(classifications)),
            ) => {
                let should_unlock = classifications.iter().any(|frame_class| {
                    frame_class.iter().any(|c| {
                        self.config.unlock_list.iter().any(|unlock_config| {
                            c.label
                                .to_lowercase()
                                .contains(&unlock_config.label.to_lowercase())
                                && c.confidence >= unlock_config.min_confidence
                        })
                    })
                });

                let should_lock = classifications.iter().any(|frame_class| {
                    frame_class.iter().any(|c| {
                        self.config.lock_list.iter().any(|lock_config| {
                            c.label
                                .to_lowercase()
                                .contains(&lock_config.label.to_lowercase())
                                && c.confidence >= lock_config.min_confidence
                        })
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
                        "Lock entity detected".to_string()
                    } else {
                        "Unlock entity not detected".to_string()
                    };

                    match door_state {
                        DoorState::Unlocked => (
                            State::LockingGracePeriod {
                                door_state,
                                countdown_start: Instant::now(),
                            },
                            vec![],
                        ),
                        _ => (
                            State::Idle {
                                action: DoorAction::Locking,
                                door_state,
                                message,
                                message_time: Instant::now(),
                                last_classification: Instant::now(),
                            },
                            vec![],
                        ),
                    }
                }
            }
            (
                State::LockingGracePeriod {
                    door_state,
                    countdown_start,
                },
                Event::Tick(now),
            ) => {
                let elapsed = now.duration_since(countdown_start);
                if elapsed >= self.config.locking_grace_period {
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
                        State::LockingGracePeriod {
                            door_state,
                            countdown_start,
                        },
                        vec![],
                    )
                }
            }
            (State::ControllingDoor { action, .. }, Event::DoorLockDone(_)) => (
                State::Idle {
                    action,
                    door_state: DoorState::Locked,
                    message: "Door locked".to_string(),
                    message_time: Instant::now(),
                    last_classification: Instant::now(),
                },
                vec![],
            ),
            (State::ControllingDoor { action, .. }, Event::DoorUnlockDone(result)) => {
                match result {
                    Ok(_) => (
                        State::UnlockedGracePeriod {
                            door_state: DoorState::Unlocked,
                            countdown_start: Instant::now(),
                        },
                        vec![],
                    ),
                    Err(_) => (
                        State::Idle {
                            action,
                            door_state: DoorState::Locked,
                            message: "Door locked".to_string(),
                            message_time: Instant::now(),
                            last_classification: Instant::now(),
                        },
                        vec![],
                    ),
                }
            }
            (
                State::UnlockedGracePeriod {
                    door_state,
                    countdown_start,
                },
                Event::Tick(now),
            ) => {
                let elapsed = now.duration_since(countdown_start);
                if elapsed >= self.config.unlock_grace_period {
                    (
                        State::AnalyzingFramesCapture { door_state },
                        vec![Effect::CaptureFrames],
                    )
                } else {
                    (
                        State::UnlockedGracePeriod {
                            door_state,
                            countdown_start,
                        },
                        vec![],
                    )
                }
            }
            (
                State::Idle {
                    door_state,
                    last_classification,
                    ..
                },
                Event::Tick(now),
            ) => {
                if now.duration_since(last_classification) >= self.config.analyze_rate {
                    (
                        State::AnalyzingFramesCapture { door_state },
                        vec![Effect::CaptureFrames],
                    )
                } else {
                    (state, vec![])
                }
            }

            (_, Event::CameraEvent(DeviceCameraEvent::Disconnected)) => (
                State::DevicesInitializing {
                    device_states: DeviceStates::default(),
                },
                vec![],
            ),
            (_, Event::DoorEvent(DeviceDoorEvent::Disconnected)) => {
                let mut effects = vec![];
                if matches!(state, State::UnlockedGracePeriod { .. }) {
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
            Effect::SubscribeToCameraEvents => {
                let events = self.device_camera.events();
                loop {
                    match events.recv() {
                        Ok(event) => {
                            if event_queue.send(Event::CameraEvent(event)).is_err() {
                                continue;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Effect::SubscribeTick => loop {
                std::thread::sleep(self.config.tick_rate);
                if event_queue.send(Event::Tick(Instant::now())).is_err() {
                    continue;
                }
            },
            Effect::StartCamera => {
                let started = self.device_camera.start();
                let _ = event_queue.send(Event::CameraStartDone(started));
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
                let frames = self.device_camera.capture_frame();
                let _ = event_queue.send(Event::FramesCaptureDone(frames));
            }
            Effect::ClassifyFrames { frames } => {
                let _ = event_queue.send(Event::FramesClassifyStart);
                let classifications = self.image_classifier.classify(frames.clone());
                let _ = event_queue.send(Event::FramesClassifyDone(classifications));
            }
        }
    }

    fn render(&self, state: &State) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let mut device_display = self.device_display.lock().unwrap();

        device_display.clear()?;

        match state {
            State::DevicesInitializing { device_states } => {
                match device_states.camera {
                    CameraState::Disconnected => {
                        device_display.write_line(0, "Camera connecting...")?;
                    }
                    CameraState::Connected(time) => {
                        if time.elapsed() > Duration::from_secs(2) {
                            device_display.write_line(0, "Camera connected")?;
                        } else {
                            device_display.write_line(0, "Camera connecting...")?;
                        }
                    }
                    CameraState::Started => {
                        device_display.write_line(0, "Camera connected")?;
                    }
                }

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
            State::AnalyzingFramesCapture { .. } | State::AnalyzingFramesClassifying { .. } => {
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
            State::UnlockedGracePeriod {
                countdown_start, ..
            } => {
                let remaining = (self.config.unlock_grace_period.as_secs() as i64
                    - countdown_start.elapsed().as_secs() as i64)
                    .max(0);
                device_display.write_line(0, &format!("Door unlocked ({}s)", remaining))?;
            }
            State::LockingGracePeriod {
                countdown_start, ..
            } => {
                let remaining = (self.config.locking_grace_period.as_secs() as i64
                    - countdown_start.elapsed().as_secs() as i64)
                    .max(0);
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
                    // Split message into lines of max 16 chars
                    let mut line = String::new();
                    let mut first = true;
                    for word in message.split_whitespace() {
                        if line.len() + word.len() + 1 <= 16 {
                            if !line.is_empty() {
                                line.push(' ');
                            }
                            line.push_str(word);
                        } else {
                            if first {
                                device_display.write_line(0, &line)?;
                                first = false;
                            } else {
                                device_display.write_line(1, &line)?;
                            }
                            line = word.to_string();
                        }
                    }
                    if first {
                        device_display.write_line(0, &line)?;
                    } else {
                        device_display.write_line(1, &line)?;
                    }
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

    fn run_loop(&self) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
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
                    if let Err(e) = self.render(&current_state) {
                        return Err::<(), Arc<dyn std::error::Error + Send + Sync>>(Arc::new(
                            io::Error::new(io::ErrorKind::Other, e.to_string()),
                        ));
                    }

                    self.spawn_effects(new_effects);
                }
                Err(e) => {
                    return Err::<(), Arc<dyn std::error::Error + Send + Sync>>(Arc::new(
                        io::Error::new(io::ErrorKind::Other, e.to_string()),
                    ));
                }
            }
        }
    }

    pub fn run(&self) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        self.run_loop()?;

        Ok(())
    }
}
