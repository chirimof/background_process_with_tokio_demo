extern crate tokio;
extern crate futures;

use futures::sync::mpsc;
use futures::{Future, Stream, Sink, stream, lazy};

use tokio::net::{TcpListener};
use tokio::timer::Interval;
use tokio::io;

use std::time::Duration;


#[derive(PartialEq)]
enum Item {
    Value(usize),
    Tick,
    Done
}

fn bg_process(rx: mpsc::Receiver<usize>) -> impl Future<Item = (), Error = ()> {
    let interval = Interval::new_interval(Duration::from_secs(30))
        .map(|_| Item::Tick);
    rx.map(|num| Item::Value(num))
        .chain(stream::once(Ok(Item::Done)))
        .select(interval.map_err(|_| ()))
        .take_while(|item| Ok(*item != Item::Done))
        .fold(0, |num, item| {
            match item {
                Item::Value(n) => Ok(num + n),
                Item::Tick => {
                    println!("read {} bytes", num);
                    Ok(0)
                }
                _ => unreachable!()
            }
        })
        .map(|_| ())
}

fn main() {

    tokio::run(lazy(|| {
        let (tx, rx) = mpsc::channel(1024);
        tokio::spawn(bg_process(rx));

        let addr = "127.0.0.1:8000".parse().unwrap();
        let listener = TcpListener::bind(&addr).unwrap();

        listener.incoming()
            .for_each(move |sock| {
                let tx2 = tx.clone();
                let calc_task = io::read_to_end(sock, vec![])
                    .and_then(|(_, buf)| {
                        tx2.send(buf.len())
                            .map_err(|_| io::ErrorKind::Other.into())
                    })
                    .map(|_| ())
                    .map_err(|e| println!("{}", e));
                tokio::spawn(calc_task);
                Ok(())
            })
            .map_err(|e| {
                println!("{}", e);
            })

    }))

}
