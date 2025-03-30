pub trait DogDoor {
    fn lock(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn unlock(&self) -> Result<(), Box<dyn std::error::Error>>;
}
