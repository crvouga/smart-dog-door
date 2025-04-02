use crate::image_classifier::test::fixture::Fixture;

#[test]
fn test_cat() {
    println!("test cat");
    let f = Fixture::new();
    println!("test cat");

    let image = include_bytes!("./images/cat_clear_front.jpeg").to_vec();

    let result = f.image_classifier.classify(vec![image]);

    println!("results: {:?}", result);

    // let result = f.image_classifier.classify_image(image);
    assert_eq!(1, 2);
}
