use cueue::*;

#[test]
fn test_capacity() {
    let pagesize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
    let (w, r) = cueue::<u8>(16).unwrap();
    assert_eq!(w.capacity(), r.capacity());
    assert_eq!(w.capacity(), pagesize);

    let (w, _r) = cueue::<u8>(pagesize + 1).unwrap();
    assert_eq!(w.capacity(), pagesize * 2);
}

#[test]
fn test_writer() {
    let (mut w, r) = cueue::<u8>(16).unwrap();

    let cap = w.capacity();

    let buf = w.write_chunk();
    assert_eq!(buf.len(), cap);
    w.commit(0);

    let buf = w.write_chunk();
    assert_eq!(buf.len(), cap);
    w.commit(3);

    let buf = w.write_chunk();
    assert_eq!(buf.len(), cap - 3);

    assert!(!w.is_abandoned());
    std::mem::drop(r);
    assert!(w.is_abandoned());
}

#[test]
fn test_reader() {
    let (mut w, mut r) = cueue(16).unwrap();

    let empty = r.read_chunk();
    assert_eq!(empty.len(), 0);
    r.commit();

    let buf = w.write_chunk();
    buf[..3].copy_from_slice(b"foo");
    w.commit(3);

    let foo = r.read_chunk();
    assert_eq!(foo, b"foo");
    r.commit();
    r.commit();

    assert!(!r.is_abandoned());
    std::mem::drop(w);
    assert!(r.is_abandoned());
}

#[test]
#[should_panic]
fn test_commit_read_panic() {
    let (mut w, mut r) = cueue(16).unwrap();

    let empty = r.read_chunk();
    assert_eq!(empty.len(), 0);
    r.commit();

    let buf = w.write_chunk();
    buf[..3].copy_from_slice(b"foo");
    w.commit(3);

    let foo = r.read_chunk();
    assert_eq!(foo, b"foo");
    r.commit();
    // this would trigger a panic because we have no data
    r.commit_read(1);
}

#[test]
fn test_commit_read() {
    let (mut w, mut r) = cueue(16).unwrap();

    let empty = r.read_chunk();
    assert_eq!(empty.len(), 0);
    r.commit();

    let buf = w.write_chunk();
    buf[..3].copy_from_slice(b"foo");
    w.commit(3);

    let foo = r.read_chunk();
    assert_eq!(foo, b"foo");
    r.commit_read(1);

    let foo = r.read_chunk();
    assert_eq!(foo, b"oo");
    r.commit_read(2);

    let empty = r.read_chunk();
    assert!(empty.is_empty());
}

#[test]
fn test_full() {
    let (mut w, mut r) = cueue::<u8>(16).unwrap();

    let buf = w.write_chunk();
    let buflen = buf.len();
    assert_eq!(buf.len(), w.capacity());
    w.commit(buflen);

    let empty = w.write_chunk();
    assert_eq!(empty.len(), 0);

    let full = r.read_chunk();
    assert_eq!(full.len(), buflen);
    assert_eq!(full.len(), r.capacity());
}

#[test]
fn test_reuse() {
    let (mut w, mut r) = cueue(16).unwrap();

    // fill the queue with strings
    let buf = w.write_chunk();
    for s in buf.iter_mut() {
        *s = "foobar";
    }
    let buflen = buf.len();
    w.commit(buflen);

    // consume everything
    let full = r.read_chunk();
    assert_eq!(full.len(), buflen);
    r.commit();

    // try writing again
    let buf = w.write_chunk();
    assert_eq!(buf[0], "foobar");
}

#[test]
fn test_push() {
    let (mut w, _) = cueue(16).unwrap();
    let cap = w.capacity();

    for i in 0..cap {
        assert_eq!(w.push(i), Ok(()));
    }

    assert_eq!(w.push(0), Err(0));
}

#[test]
fn test_pop() {
    let (mut w, mut r) = cueue(16).unwrap();
    let cap = w.capacity();

    for i in 0..cap {
        assert_eq!(w.push(i), Ok(()));
    }

    assert_eq!(w.push(0), Err(0));

    for i in 0..cap {
        assert_eq!(r.pop(), Some(i));
    }

    assert_eq!(r.pop(), None);
}

#[test]
fn test_chunk_size() {
    let (mut w, mut r) = cueue(16).unwrap();
    let cap = w.capacity();

    for i in 0..cap * 2 {
        w.push(i).unwrap();
        assert_eq!(w.write_chunk().len(), cap - 1);
        assert_eq!(r.pop(), Some(i));
    }
}

#[test]
fn test_push_string() {
    let (mut w, _) = cueue(16).unwrap();
    let cap = w.capacity();

    for i in 0..cap {
        assert_eq!(w.push(i.to_string()), Ok(()));
    }

    assert_eq!(w.push("foo".to_string()), Err("foo".to_string()));
}

#[test]
fn test_cueue_threaded_w_r() {
    let (mut w, mut r) = cueue(16).unwrap();
    let maxi = 1_000_000;

    let wt = std::thread::spawn(move || {
        let mut msg: u8 = 0;
        for _ in 0..maxi {
            let buf = loop {
                let buf = w.write_chunk();
                if !buf.is_empty() {
                    break buf;
                }
            };
            buf[0] = msg;
            w.commit(1);

            msg = msg.wrapping_add(1);
        }
    });

    let rt = std::thread::spawn(move || {
        let mut emsg: u8 = 0;
        let mut i = 0;
        while i < maxi {
            let rr = r.read_chunk();
            for msg in rr {
                assert_eq!(*msg, emsg);
                emsg = emsg.wrapping_add(1);
                i += 1;
            }
            r.commit();
        }
    });

    wt.join().unwrap();
    rt.join().unwrap();
}
