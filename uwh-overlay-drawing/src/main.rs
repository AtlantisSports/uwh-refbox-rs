use std::sync::mpsc::channel;
use uwh_common::game_snapshot::GameSnapshot;

mod load_images;
mod network;
mod pages;
mod render;

fn main() {
    let (tx, rx) = channel::<GameSnapshot>();
    std::thread::spawn(|| network::networking_thread(tx).unwrap());
    render::rendering_thread(rx);
}

