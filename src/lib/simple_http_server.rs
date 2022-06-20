use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::marker::Unpin;

use async_std::task;
use async_std::net as async_net;
use async_std::io as async_io;
use async_std::io::{ReadExt, WriteExt}; 
use futures::stream::StreamExt;

#[cfg(test)]
mod tests {
    use super::*;
    use futures::task::{Poll, Context};

    use pretty_assertions::assert_eq;
    use std::cmp::min;
    use std::pin::Pin;
    use std::marker::Unpin;

    struct MockTcpStream {
        read_buffer: Vec<u8>,
        write_buffer: Vec<u8>,
    }

    impl async_io::Read for MockTcpStream {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<async_io::Result<usize>> {
            let size: usize = min(self.read_buffer.len(), buf.len());
            buf[..size].copy_from_slice(&self.read_buffer[..size]);

            Poll::Ready(Ok(size))
        }
    }

    impl async_io::Write for MockTcpStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<async_io::Result<usize>> {
            self.write_buffer = Vec::from(buf);

            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<async_io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<async_io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    impl Unpin for MockTcpStream { }

    #[async_std::test]
    async fn test_handle_connection_async() {
        let input_bytes = b"GET / HTTP/1.1\r\n";
        let mut contents = vec![0u8; 1024];
        contents[..input_bytes.len()].clone_from_slice(input_bytes);

        let mut stream = MockTcpStream {
            read_buffer: contents,
            write_buffer: vec![],
        };

        handle_connection_async(&mut stream).await;
        let mut buf = [0u8; 1024];
        stream.read(&mut buf).await.unwrap();

        let expected_contents = fs::read_to_string("resource/hello.html").unwrap();
        let expected_response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                                                expected_contents.len(),
                                                expected_contents);
        //println!("[1] {}", expected_response);
        //println!("[2] {}", String::from_utf8_lossy(&stream.write_buffer));
        assert_eq!(expected_response.as_bytes(), stream.write_buffer);
    }

}

pub fn start_server() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming().take(2) {
        let stream = stream.unwrap();

        println!("Connection established!");

        // single thread version
        //handle_connection(stream);

        // thread pool version
        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];

    Read::read(&mut stream, &mut buffer).unwrap();
    //println!("Request: \n{}", String::from_utf8_lossy(&buffer[..]));

    let get = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";

    let (status_line, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK", "resource/hello.html")
    } else if buffer.starts_with(sleep) {
        thread::sleep(Duration::from_secs(5));
        ("HTTP/1.1 200 OK", "resource/hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "resource/404.html")
    };

    let contents = fs::read_to_string(filename).unwrap();

    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    );

    Write::write(&mut stream, response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

pub async fn start_server_async() {
    let listener = async_net::TcpListener::bind("127.0.0.1:8080").await.unwrap();

    listener
        .incoming()
        .for_each_concurrent(None, |tcp_stream| async move {
            println!("Connection established!");
            let tcp_stream = tcp_stream.unwrap();

            handle_connection_async(tcp_stream).await;

            // muti-thread with async
            //task::spawn(handle_connection_async(tcp_stream));
        })
        .await;

    // can not currently run
    /*
    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        println!("Connection established!");
        let stream = stream.unwrap();
        handle_connection_async(stream).await;
    }
    */

    println!("Shutting down.");
}

async fn handle_connection_async(mut stream: impl async_io::Read + async_io::Write + Unpin) {
    let mut buffer = [0; 1024];

    stream.read(&mut buffer).await.unwrap();
    //println!("Request: \n{}", String::from_utf8_lossy(&buffer[..]));

    let get = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";

    let (status_line, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK", "resource/hello.html")
    } else if buffer.starts_with(sleep) {
        //thread::sleep(Duration::from_secs(5));
        task::sleep(Duration::from_secs(5)).await;
        ("HTTP/1.1 200 OK", "resource/hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "resource/404.html")
    };

    let contents = fs::read_to_string(filename).unwrap();

    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    );

    stream.write(response.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();

}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => {
                    println!("Worker {} got a job; executing.", id);
                    job();
                    println!("Worker {} done.", id);
                }
                Message::Terminate => {
                    println!("Worker {} was told to terminate.", id);
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
