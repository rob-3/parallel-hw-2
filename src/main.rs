extern crate queues;
extern crate rand;

use std::sync::Mutex;
use std::thread::JoinHandle;
use std::{
    hint,
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc,
    },
    thread,
};

use queues::{queue, IsQueue, Queue};
use rand::Rng;
use rand::prelude::SliceRandom;

// set this to true to print out extra output
const OUTPUT: bool = false;

fn main() {
    // comment these out if you only wish to test one problem at a time
    problem1();
    problem2();
}

const GUESTS: i32 = 150;
const REQUEUE_RATE: f64 = 0.75;

fn problem1() {
    print_in_box("Problem 1");
    let is_cupcake = Arc::new(AtomicBool::new(true));
    let invited_guest = Arc::new(AtomicI32::new(0));
    let mut threads = vec![];

    for i in 0..GUESTS {
        // we need reference clones to make Rust's reference counting happy
        let is_cupcake = is_cupcake.clone();
        let invited_guest = invited_guest.clone();
        let mut has_eaten = false;
        // if we are the boss
        if i == 0 {
            let mut counter = 0;
            threads.push(thread::spawn(move || loop {
                // block until we are woken up
                thread::park();
                // check to ensure no spurious wakeup
                if invited_guest.load(Ordering::SeqCst) == i {
                    // check for missing cupcake
                    if is_cupcake.load(Ordering::SeqCst) == false {
                        counter += 1;
                        // request new cupcake
                        is_cupcake.store(true, Ordering::SeqCst);
                    }
                    if OUTPUT {
                        println!("Boss thread woke up: {counter} cupcakes have been eaten total!");
                    }
                    // eat cupcake if we haven't eaten yet
                    if !has_eaten {
                        if is_cupcake.load(Ordering::SeqCst) {
                            is_cupcake.store(false, Ordering::SeqCst);
                            has_eaten = true;
                            counter += 1;
                            is_cupcake.store(true, Ordering::SeqCst);
                        }
                    }
                    if counter == GUESTS {
                        // send completion signal if every has eaten
                        invited_guest.store(-2, Ordering::SeqCst);
                    } else {
                        invited_guest.store(-1, Ordering::SeqCst);
                    }
                }
            }));
        } else {
            // regular guest
            threads.push(thread::spawn(move || loop {
                thread::park();
                if invited_guest.load(Ordering::SeqCst) == i {
                    // eat if we haven't
                    if !has_eaten {
                        if is_cupcake.load(Ordering::SeqCst) {
                            is_cupcake.store(false, Ordering::SeqCst);
                            has_eaten = true;
                        }
                    }
                    invited_guest.store(-1, Ordering::SeqCst);
                }
            }));
        }
    }

    let mut rng = rand::thread_rng();
    loop {
        let thread_id = rng.gen_range(0, GUESTS);
        invited_guest.store(thread_id, Ordering::SeqCst);
        let thread = threads[thread_id as usize].thread();
        thread.unpark();
        let mut signal = invited_guest.load(Ordering::SeqCst);
        while signal != -1 && signal != -2 {
            signal = invited_guest.load(Ordering::SeqCst);
            hint::spin_loop();
        }
        if signal == -2 {
            break;
        }
    }
    println!("All guests have eaten!");
}

fn problem2() {
    print_in_box("Problem 2");
    /*
     * I suggest the queuing strategy. This strategy reduces contention by making sure only a
     * single guest is even trying to enter the room. It also has an element of "fairness" in that
     * guests cannot get unlucky and never get access to the vase if others keep beating them to
     * the punch.
     *
     * The disadvantage of strategy 3 is that a queue is required to exist and be managed, and the
     * guests are forced to wait because they might reach the front of the queue at any time. With
     * strategy 1 or 2, guests might be able to go do other things if their attempt to get access
     * to the showroom fails.
     */
    let threads: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::new(Mutex::new(vec![]));
    // fill queue with everyone to start
    let queue = Arc::new(Mutex::new({
        let mut q = queue![];
        let mut rng = rand::thread_rng();
        // don't include first guest, we'll manually let them in
        // set up guests in a random initial order
        let random_guests = {
            let mut random_guests = vec![];
            for i in 1..GUESTS {
                random_guests.push(i);
            }
            random_guests.shuffle(&mut rng);
            random_guests
        };
        for i in random_guests {
            if let Err(e) = q.add(i) {
                println!("{e}");
                return;
            }
        }
        q
    }));
    let current_guest = Arc::new(AtomicI32::new(0));

    // we represent the showroom with an int we can increment
    let showroom = Arc::new(Mutex::new(0));

    let mut thread_setup = threads.lock().unwrap();
    for i in 0..GUESTS {
        let queue = queue.clone();
        let current_guest = current_guest.clone();
        let my_threads = threads.clone();
        let showroom = showroom.clone();
        thread_setup.push(thread::spawn(move || loop {
            thread::park();
            if current_guest.load(Ordering::SeqCst) == i {
                // we don't have to do this, but it's the easiest way to show that a thread got
                // access to something
                let mut showroom = showroom.lock().unwrap();
                *showroom += 1;
                if OUTPUT {
                    let showroom = *showroom;
                    println!("Guest #{i} visited the showroom for a total of {showroom} visits");
                }
                // unlock early just to be sure
                drop(showroom);
                // notify next guest and possibly get back in line
                let mut queue = queue.lock().unwrap();
                if let Ok(next_guest) = queue.remove() {
                    current_guest.store(next_guest, Ordering::SeqCst);
                    my_threads.lock().unwrap()[next_guest as usize]
                        .thread()
                        .unpark();
                    if rand::random::<f64>() < REQUEUE_RATE {
                        if let Err(e) = queue.add(i) {
                            println!("{e}");
                            return;
                        }
                    }
                } else {
                    current_guest.store(-1, Ordering::SeqCst);
                }
            }
        }));
    }
    // make sure we unlock
    drop(thread_setup);
    threads.lock().unwrap()[0].thread().unpark();
    while current_guest.load(Ordering::SeqCst) != -1 {
        hint::spin_loop();
    }
    println!("Queue is empty!");
    let showroom = *showroom.lock().unwrap();
    println!("Total visitors: {showroom}");
}

fn print_in_box(s: &str) {
    print!("┏");
    for _ in 0..s.len() {
        print!("━");
    }
    println!("┓");
    println!("┃{s}┃");
    print!("┗");
    for _ in 0..s.len() {
        print!("━");
    }
    println!("┛");
}
