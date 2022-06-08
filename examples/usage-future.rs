use std::time::Duration;
use rust_playground::usage_future::{TimerFuture, new_executor_and_spawner};

fn main() {
    let (executor, spawner) = new_executor_and_spawner();

    spawner.spawn(async {
        println!("show time!");
        TimerFuture::new(Duration::new(2, 0)).await;
        println!("time over!");
    });

    spawner.spawn(async {
        println!("[2] show time!");
        TimerFuture::new(Duration::new(3, 0)).await;
        println!("[2] time over!");
    });

    spawner.spawn(async {
        println!("[3] show time!");
        println!("[3] time over!");
    });

    drop(spawner);

    executor.run();
}