extern crate rand;

use std::{
    hint,
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc,
    },
    thread,
};

use rand::Rng;

const GUESTS: i32 = 150;

fn main() {
    let is_cupcake = Arc::new(AtomicBool::new(true));
    let invited_guest = Arc::new(AtomicI32::new(0));
    let mut threads = vec![];

    for i in 0..GUESTS {
        let is_cupcake = is_cupcake.clone();
        let invited_guest = invited_guest.clone();
        let mut has_eaten = false;
        if i == 0 {
            let mut counter = 0;
            threads.push(thread::spawn(move || loop {
                thread::park();
                if invited_guest.load(Ordering::SeqCst) == i {
                    if is_cupcake.load(Ordering::SeqCst) == false {
                        counter += 1;
                        is_cupcake.store(true, Ordering::SeqCst);
                    }
                    println!("Boss thread woke up: {counter} cupcakes have been eaten total!");
                    if !has_eaten {
                        if is_cupcake.load(Ordering::SeqCst) {
                            is_cupcake.store(false, Ordering::SeqCst);
                            has_eaten = true;
                        }
                    }
                    if counter == GUESTS {
                        invited_guest.store(-2, Ordering::SeqCst);
                    } else {
                        invited_guest.store(-1, Ordering::SeqCst);
                    }
                }
            }));
        } else {
            threads.push(thread::spawn(move || loop {
                thread::park();
                if invited_guest.load(Ordering::SeqCst) == i {
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
