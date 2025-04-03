use crate::image_classifier::test::fixture::Fixture;

#[test]
fn test_cat() {
    let f = Fixture::new();

    let images = vec![
        image::open("./src/image_classifier/test/images/cat_clear_front.jpeg").unwrap(),
        // image::open("./src/image_classifier/test/images/cat_security_footage_big.jpeg").unwrap(),
    ];

    for image in images {
        let result = f.image_classifier.classify(vec![image]);

        let classifications = result.unwrap().into_iter().flatten().collect::<Vec<_>>();

        println!("classifications: {:?}", classifications);

        let has_cat = classifications
            .iter()
            .any(|c| c.label == "cat" && c.confidence > 0.1);

        assert!(
            has_cat,
            "Expected to find a cat classification with confidence > 0.1"
        );
    }
}
