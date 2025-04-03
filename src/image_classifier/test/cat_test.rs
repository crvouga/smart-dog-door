use crate::image_classifier::test::fixture::Fixture;

#[test]
fn test_cat() {
    let f = Fixture::new();

    let images = vec![
        image::open("./src/image_classifier/test/images/cat_clear_front.jpeg").unwrap(),
        image::open("./src/image_classifier/test/images/cat_security_footage.jpeg").unwrap(),
    ];

    for image in images {
        let result = f.image_classifier.classify(vec![image]);

        let classifications = result.unwrap();
        println!("classifications: {:?}", classifications);
        assert_eq!(classifications.len(), 1);
        let first_image_classifications = &classifications[0];
        assert!(!first_image_classifications.is_empty());

        // Check if any classification is for a cat with high confidence
        let has_cat = first_image_classifications
            .iter()
            .any(|c| c.label == "cat" && c.confidence > 0.5);
        assert!(
            has_cat,
            "Expected to find a cat classification with confidence > 0.5"
        );
    }
}
