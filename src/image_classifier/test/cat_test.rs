use crate::image_classifier::{interface::Classification, test::fixture::Fixture};

impl Classification {
    fn is_cat(&self) -> bool {
        self.label.to_lowercase().contains("cat")
    }
}

#[test]
fn test_cat_easy() {
    let f = Fixture::new();

    let frames =
        vec![image::open("./src/image_classifier/test/images/cat_clear_front.jpeg").unwrap()];

    let result = f.image_classifier.classify(frames);

    let classifications = result.unwrap().first().unwrap().clone();

    for classification in classifications.iter().take(5) {
        println!("classification: {:?}", classification);
    }

    let best = classifications
        .iter()
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
        .unwrap();

    assert!(best.is_cat());
}

#[test]
fn test_cat_hard_security_camera() {
    let f = Fixture::new();

    let frames =
        vec![image::open("./src/image_classifier/test/images/cat_security_footage.jpeg").unwrap()];

    let result = f.image_classifier.classify(frames);

    let classifications = result.unwrap().first().unwrap().clone();

    for classification in classifications.iter().take(5) {
        println!("classification: {:?}", classification);
    }

    assert!(false);
}

// #[test]
// fn test_not_cat() {
//     let f = Fixture::new();

//     let images =
//         vec![image::open("./src/image_classifier/test/images/person_clear_front.jpeg").unwrap()];

//     for image in images {
//         let result = f.image_classifier.classify(vec![image]);

//         let classifications = result.unwrap().into_iter().flatten().collect::<Vec<_>>();

//         let cat_classifications = classifications
//             .iter()
//             .filter(|c| c.label.to_lowercase().contains("cat") && c.confidence > 0.9)
//             .collect::<Vec<_>>();

//         println!("cat_classifications: {:?}", cat_classifications);

//         let has_cat = cat_classifications.len() > 0;

//         assert!(
//             !has_cat,
//             "Expected to not find a cat classification with confidence > 0.9"
//         );
//     }
// }
