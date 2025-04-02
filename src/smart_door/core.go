package smartdoor

import (
	"time"
)

type Config struct {
	MinimalDurationUnlocking time.Duration
	MinimalDurationLocking   time.Duration
	MinimalRateCameraProcess time.Duration
	ClassificationUnlockList []ClassificationConfig
	ClassificationLockList   []ClassificationConfig
}

type ClassificationConfig struct {
	Label         string
	MinConfidence float64
}

type DeviceCamera interface {
	Subscribe() <-chan DeviceCameraEvent
	CaptureFrames() ([]Frame, error)
}

type DeviceDoor interface {
	Subscribe() <-chan DeviceDoorEvent
	Lock() error
	Unlock() error
}

type ImageClassifier interface {
	ClassifyFrames(frames []Frame) ([][]Classification, error)
}

type DeviceCameraEvent int

const (
	CameraEventConnected DeviceCameraEvent = iota
	CameraEventDisconnected
)

type DeviceDoorEvent int

const (
	DoorEventConnected DeviceDoorEvent = iota
	DoorEventDisconnected
)

type Frame struct {
	// Frame data
}

type Classification struct {
	Label      string
	Confidence float64
}

type Detection int

const (
	DetectionNone Detection = iota
	DetectionCat
	DetectionDog
)

type SmartDoor struct {
	config           Config
	camera           DeviceCamera
	door             DeviceDoor
	classifier       ImageClassifier
	cameraEvents     <-chan DeviceCameraEvent
	doorEvents       <-chan DeviceDoorEvent
	classificationCh chan [][]Classification
	doorActionCh     chan DoorAction
}

type DoorAction int

const (
	ActionNone DoorAction = iota
	ActionLock
	ActionUnlock
)

func NewSmartDoor(
	config Config,
	camera DeviceCamera,
	door DeviceDoor,
	classifier ImageClassifier,
) *SmartDoor {
	return &SmartDoor{
		config:           config,
		camera:           camera,
		door:             door,
		classifier:       classifier,
		cameraEvents:     camera.Subscribe(),
		doorEvents:       door.Subscribe(),
		classificationCh: make(chan [][]Classification),
		doorActionCh:     make(chan DoorAction),
	}
}

func (sd *SmartDoor) Run() {
	// Start camera processing goroutine
	go sd.processCamera()

	// Start door control goroutine
	go sd.controlDoor()

	// Main event loop
	for {
		select {
		case event := <-sd.cameraEvents:
			sd.handleCameraEvent(event)
		case event := <-sd.doorEvents:
			sd.handleDoorEvent(event)
		}
	}
}

func (sd *SmartDoor) processCamera() {
	ticker := time.NewTicker(sd.config.MinimalRateCameraProcess)
	defer ticker.Stop()

	for range ticker.C {
		frames, err := sd.camera.CaptureFrames()
		if err != nil {
			continue
		}

		classifications, err := sd.classifier.ClassifyFrames(frames)
		if err != nil {
			continue
		}

		sd.classificationCh <- classifications
	}
}

func (sd *SmartDoor) controlDoor() {
	var lastDetection Detection
	var lastActionTime time.Time

	for classifications := range sd.classificationCh {
		detection := sd.toDetection(classifications)

		if detection == lastDetection {
			continue
		}

		now := time.Now()
		if now.Sub(lastActionTime) < sd.config.MinimalDurationUnlocking {
			continue
		}

		switch detection {
		case DetectionDog:
			if lastDetection != DetectionDog {
				sd.doorActionCh <- ActionUnlock
				lastActionTime = now
			}
		case DetectionCat:
			sd.doorActionCh <- ActionLock
			lastActionTime = now
		}

		lastDetection = detection
	}
}

func (sd *SmartDoor) toDetection(classifications [][]Classification) Detection {
	// Implementation similar to Rust version
	// Returns DetectionCat, DetectionDog, or DetectionNone
	return DetectionNone
}

func (sd *SmartDoor) handleCameraEvent(event DeviceCameraEvent) {
	// Handle camera connection/disconnection
}

func (sd *SmartDoor) handleDoorEvent(event DeviceDoorEvent) {
	// Handle door connection/disconnection
}
