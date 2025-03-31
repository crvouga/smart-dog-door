#[cfg(test)]
mod core_test {

    use std::time::{Duration, Instant};

    use crate::config::{ClassificationConfig, Config};
    use crate::device_camera::interface::DeviceCameraEvent;
    use crate::device_door::interface::DeviceDoorEvent;
    use crate::image_classifier::interface::Classification;
    use crate::smart_door::core::{
        init, transition, CameraState, DeviceStates, DoorAction, DoorState, Effect, Event, State,
    };

    #[test]
    fn test_init() {
        let (state, effects) = init();

        assert!(matches!(state, State::DevicesInitializing { .. }));
        assert_eq!(effects.len(), 3);
        assert!(effects.contains(&Effect::SubscribeToDoorEvents));
        assert!(effects.contains(&Effect::SubscribeToCameraEvents));
        assert!(effects.contains(&Effect::SubscribeTick));
    }

    #[test]
    fn test_camera_connection_flow() {
        let config = Config::default();
        let (initial_state, _) = init();

        // Test camera connects
        let (state, effects) = transition(
            &config,
            initial_state,
            Event::CameraEvent(DeviceCameraEvent::Connected),
        );

        match state.clone() {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.camera, CameraState::Connected(_)));
                assert!(matches!(device_states.door, DoorState::Disconnected));
            }
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::StartCamera]);

        // Test camera start completes
        let (state, effects) = transition(&config, state, Event::CameraStartDone(Ok(())));

        match state {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.camera, CameraState::Started));
            }
            _ => panic!("Unexpected state"),
        }
        assert!(effects.is_empty());
    }

    #[test]
    fn test_door_connection_flow() {
        let config = Config::default();
        let (initial_state, _) = init();

        // Test door connects
        let (state, effects) = transition(
            &config,
            initial_state,
            Event::DoorEvent(DeviceDoorEvent::Connected),
        );

        match state.clone() {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.door, DoorState::Connected(_)));
                assert!(matches!(device_states.camera, CameraState::Disconnected));
            }
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::LockDoor]);

        // Test door lock completes
        let (state, effects) = transition(&config, state, Event::DoorLockDone(Ok(())));

        match state {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.door, DoorState::Initialized));
            }
            _ => panic!("Unexpected state"),
        }
        assert!(effects.is_empty());
    }

    #[test]
    fn test_analyzing_frames_flow() {
        let mut config = Config::default();
        config.unlock_list = vec![ClassificationConfig {
            label: "dog".to_string(),
            min_confidence: 0.8,
        }];

        let state = State::AnalyzingFramesCapture {
            door_state: DoorState::Locked,
        };

        // Test empty frames
        let frames: Vec<Vec<u8>> = vec![];
        let (state, effects) =
            transition(&config, state.clone(), Event::FramesCaptureDone(Ok(frames)));

        match state {
            State::AnalyzingFramesCapture { .. } => (),
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::CaptureFrames]);

        // Test with frames
        let frames = vec![vec![1, 2, 3]];
        let (state, effects) =
            transition(&config, state, Event::FramesCaptureDone(Ok(frames.clone())));

        match state {
            State::AnalyzingFramesClassifying { .. } => (),
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::ClassifyFrames { frames }]);
    }

    #[test]
    fn test_unlocking_flow() {
        let mut config = Config::default();
        config.unlock_list = vec![ClassificationConfig {
            label: "dog".to_string(),
            min_confidence: 0.8,
        }];

        let state = State::AnalyzingFramesClassifying {
            door_state: DoorState::Locked,
            frames: vec![],
        };

        let classifications = vec![vec![Classification {
            label: "dog".to_string(),
            confidence: 0.9,
        }]];

        let (state, effects) = transition(
            &config,
            state,
            Event::FramesClassifyDone(Ok(classifications)),
        );

        match state {
            State::ControllingDoor {
                action: DoorAction::Unlocking,
                ..
            } => (),
            _ => panic!("Unexpected state"),
        }
        assert!(effects.contains(&Effect::UnlockDoor));
    }

    #[test]
    fn test_device_disconnection() {
        let config = Config::default();
        let state = State::AnalyzingFramesCapture {
            door_state: DoorState::Locked,
        };

        // Test camera disconnection
        let (state, effects) = transition(
            &config,
            state.clone(),
            Event::CameraEvent(DeviceCameraEvent::Disconnected),
        );

        match state.clone() {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.camera, CameraState::Disconnected));
                assert!(matches!(device_states.door, DoorState::Disconnected));
            }
            _ => panic!("Unexpected state"),
        }
        assert!(effects.is_empty());

        // Test door disconnection
        let (state, effects) = transition(
            &config,
            state,
            Event::DoorEvent(DeviceDoorEvent::Disconnected),
        );

        match state {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.camera, CameraState::Disconnected));
                assert!(matches!(device_states.door, DoorState::Disconnected));
            }
            _ => panic!("Unexpected state"),
        }
        assert!(effects.is_empty());
    }

    #[test]
    fn test_device_initialization() {
        let (initial_state, initial_effects) = init();

        match initial_state {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.camera, CameraState::Disconnected));
                assert!(matches!(device_states.door, DoorState::Disconnected));
            }
            _ => panic!("Unexpected state"),
        }

        assert_eq!(
            initial_effects,
            vec![
                Effect::SubscribeToDoorEvents,
                Effect::SubscribeToCameraEvents,
                Effect::SubscribeTick
            ]
        );
    }

    #[test]
    fn test_device_connection_sequence() {
        let config = Config::default();
        let state = State::DevicesInitializing {
            device_states: DeviceStates::default(),
        };

        // Test camera connects first
        let (state, effects) = transition(
            &config,
            state,
            Event::CameraEvent(DeviceCameraEvent::Connected),
        );

        match state.clone() {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.camera, CameraState::Connected(_)));
                assert!(matches!(device_states.door, DoorState::Disconnected));
            }
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::StartCamera]);

        // Then door connects
        let (state, effects) =
            transition(&config, state, Event::DoorEvent(DeviceDoorEvent::Connected));

        match state {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.camera, CameraState::Connected(_)));
                assert!(matches!(device_states.door, DoorState::Connected(_)));
            }
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::LockDoor]);
    }

    #[test]
    fn test_frame_analysis_sequence() {
        let config = Config::default();
        let state = State::AnalyzingFramesCapture {
            door_state: DoorState::Locked,
        };

        let frames = vec![vec![0u8; 100]];
        let (state, effects) =
            transition(&config, state, Event::FramesCaptureDone(Ok(frames.clone())));

        match state {
            State::AnalyzingFramesClassifying { door_state, .. } => {
                assert!(matches!(door_state, DoorState::Locked));
            }
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::ClassifyFrames { frames }]);
    }

    #[test]
    fn test_grace_period_transitions() {
        let mut config = Config::default();
        config.unlock_grace_period = Duration::from_secs(5);
        let start_time = Instant::now();

        let state = State::UnlockedGracePeriod {
            door_state: DoorState::Unlocked,
            countdown_start: start_time,
        };

        // Test grace period expires
        let (state, effects) = transition(
            &config,
            state,
            Event::Tick(start_time + config.unlock_grace_period + Duration::from_secs(1)),
        );

        match state {
            State::AnalyzingFramesCapture { door_state } => {
                assert!(matches!(door_state, DoorState::Unlocked));
            }
            _ => panic!("Unexpected state: {:?}", state),
        }
        assert_eq!(effects, vec![Effect::CaptureFrames]);
    }

    #[test]
    fn test_continuous_monitoring_while_unlocked() {
        let mut config = Config::default();
        config.unlock_list = vec![ClassificationConfig {
            label: "dog".to_string(),
            min_confidence: 0.8,
        }];

        let state = State::UnlockedGracePeriod {
            door_state: DoorState::Unlocked,
            countdown_start: Instant::now(),
        };

        // Test that we keep monitoring frames while unlocked
        let (state, effects) = transition(
            &config,
            state,
            Event::FramesCaptureDone(Ok(vec![vec![1, 2, 3]])),
        );

        match state {
            State::UnlockedGracePeriod { door_state, .. } => {
                assert!(matches!(door_state, DoorState::Unlocked));
            }
            _ => panic!("Unexpected state: {:?}", state),
        }
        assert_eq!(effects, vec![]);
    }
}
