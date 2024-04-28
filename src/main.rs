use std::arch::x86_64::__rdtscp;
use std::env;
use std::sync::mpsc;
use std::thread;
use tokio::sync::mpsc as async_mpsc;
use tokio::task;

const N: usize = 10_000;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: taskset 1 cargo run --release -- <async|sync>");
        return;
    }

    let benchmark_type = &args[1];

    match benchmark_type.as_str() {
        "sync" => run_sync_benchmark(),
        "async" => run_async_benchmark().await,
        _ => {
            println!("Invalid benchmark type. Use 'sync' or 'async'.");
            return;
        }
    }
}

fn run_sync_benchmark() {
    let (tx1, rx1) = mpsc::sync_channel::<u64>(10);
    let (tx2, rx2) = mpsc::sync_channel::<u64>(10);

    let thread1 = thread::spawn(move || {
        for _ in 0..N {
            let start = unsafe { __rdtscp(&mut 0) };
            tx2.send(start).unwrap();
            let _ = rx1.recv().unwrap();
        }
    });

    let thread2 = thread::spawn(move || {
        let mut min_diff = u64::MAX;
        for _ in 0..N {
            let start = rx2.recv().unwrap();
            let end = unsafe { __rdtscp(&mut 0) };
            let diff = end.wrapping_sub(start);
            if diff < min_diff {
                min_diff = diff;
            }
            tx1.send(end).unwrap();
        }
        println!("Thread 2 min roundtrip time: {}", min_diff);
    });

    thread1.join().unwrap();
    thread2.join().unwrap();
}

async fn run_async_benchmark() {
    let (tx1, mut rx1) = async_mpsc::channel::<u64>(10);
    let (tx2, mut rx2) = async_mpsc::channel::<u64>(10);

    let task1 = task::spawn(async move {
        for _ in 0..N {
            let start = unsafe { __rdtscp(&mut 0) };
            tx2.send(start).await.unwrap();
            let _ = rx1.recv().await.unwrap();
        }
    });

    let task2 = task::spawn(async move {
        let mut min_diff = u64::MAX;
        for _ in 0..N {
            let start = rx2.recv().await.unwrap();
            let end = unsafe { __rdtscp(&mut 0) };
            let diff = end.wrapping_sub(start);
            if diff < min_diff {
                min_diff = diff;
            }
            tx1.send(end).await.unwrap();
        }
        println!("Task 2 min roundtrip time: {}", min_diff);
    });

    task1.await.unwrap();
    task2.await.unwrap();
}
