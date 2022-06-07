#![allow(unused_imports)]

pub fn use_async_await() {
    use futures::executor::block_on;
    use std::thread::sleep;
    use std::time::Duration;

    struct Song {
        author: String,
        name: String,
    }

    async fn learn_song() -> Song {
        println!("Learning a song...");
        //sleep(Duration::from_secs(2));

        Song {
            author: "The Beatles".to_string(),
            name: "Yesterday".to_string(),
        }
    }

    async fn sing_song(song: Song) {
        println!("Sing a song: {} - {}", song.author, song.name);
    }

    async fn dance() {
        println!("Dancing...");
        //sleep(Duration::from_secs(3));
        println!("Dance is over");
    }

    async fn learn_and_sing() {
        let song = learn_song().await;
        sing_song(song).await;
    }

    async fn async_main() {
        let f1 = learn_and_sing();
        let f2 = dance();

        futures::join!(f1, f2);
    }

    block_on(async_main());

}

pub fn use_async_basic() {
    use futures::executor::block_on;

    async fn do_something_async() {
        hello_cat().await;
        println!("go go go async");
    }

    async fn hello_cat() {
        println!("hello cat");
    }

    let future = do_something_async();
    block_on(future);
}