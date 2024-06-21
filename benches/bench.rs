#![feature(test)]

use cueue::cueue;

extern crate test;
use self::test::Bencher;

#[bench]
fn bench_write(b: &mut Bencher) {
    let (mut w, mut r) = cueue(16).unwrap();

    let data = &b"123456789abcdefh"[..];
    let data_len = data.len();

    let rt = std::thread::spawn(move || {
        while !r.is_abandoned() {
            let _rr = r.read_chunk();
            r.commit();
        }
    });

    b.iter(move || {
        let buf = loop {
            let buf: &mut [u8] = w.write_chunk();
            if buf.len() >= data_len {
                break buf;
            }
            std::hint::spin_loop();
        };
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), buf.as_mut_ptr(), data_len);
        }
        w.commit(data_len);
    });

    rt.join().unwrap();
}
