//! The committed fixture pictures P1 and P2 are what the generator makes.

#![allow(clippy::unwrap_used)]

mod common;

fn check(name: &str, picture: &tracyo_core::picture::Picture) {
    let path = format!(
        "{}/tests/fixtures/pictures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("TRACYO_UPDATE_GOLDEN").is_some() {
        image::save_buffer(
            &path,
            &picture.rgba,
            picture.width,
            picture.height,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    let committed = tracyo_core::picture::Picture::decode(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        (committed.width, committed.height),
        (picture.width, picture.height)
    );
    assert!(
        committed.rgba == picture.rgba,
        "{name} differs from its generator"
    );
}

#[test]
fn p1_is_the_generated_picture() {
    check("p1.png", &common::p1());
}

#[test]
fn p2_is_the_generated_picture() {
    check("p2.png", &common::p2());
}
