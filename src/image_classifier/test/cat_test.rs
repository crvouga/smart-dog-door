#[cfg(test)]
mod tests {
    use super::super::fixture::Fixture;

    #[test]
    fn test_cat() {
        let f = Fixture::new();

        let result = f.image_classifier.classify_image(image);
        assert_eq!(result, "cat");
    }
}
