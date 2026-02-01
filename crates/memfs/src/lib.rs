use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use tokio::sync::RwLock;

use storage::SpanStore;
use trace::{SpanId, SpanStatus, TraceId};

const TTL: Duration = Duration::from_secs(1);
const ROOT_INO: u64 = 1;
const TRACES_INO: u64 = 2;

pub struct TraceFs {
    store: Arc<RwLock<SpanStore>>,
    trace_inos: HashMap<TraceId, u64>,
    span_inos: HashMap<SpanId, u64>,
    next_ino: u64, 
}

impl TraceFs {
    pub fn new(store: Arc<RwLock<SpanStore>>) -> Self {
        Self {
            store,
            trace_inos: HashMap::new(),
            span_inos: HashMap::new(),
            next_ino: 100,
        }
    }

    fn dir_attr(ino: u64) -> FileAttr {
        FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: SystemTime::UNIX_EPOCH,
            mtime: SystemTime::UNIX_EPOCH,
            ctime: SystemTime::UNIX_EPOCH,
            crtime: SystemTime::UNIX_EPOCH,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }

    fn file_attr(ino: u64, size: u64) -> FileAttr {
        FileAttr {
            ino, 
            size,
            blocks: 1,
            mtime: SystemTime::UNIX_EPOCH,
            ctime: SystemTime::UNIX_EPOCH,
            crtime: SystemTime::UNIX_EPOCH,
            kind: FileType::RegularFile,
            perm: 0o444,
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }
}

impl Filesystem for TraceFs {
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            ROOT_INO | TRACES_INO => reply.attr(&TTL, &Self::dir_attr(ino)),
            _ => reply.error(libc::ENOENT),
        }
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == ROOT_INO && name == "traces" {
            reply.entry(&TTL, &Self::dir_attr(TRACES_INO), 0);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let entries: Vec<(u64, FileType, &str)> = match ino {
            ROOT_INO => vec![
                (ROOT_INO, FileType::Directory, "."),
                (ROOT_INO, FileType::Directory, ".."),
                (TRACES_INO, FileType::Directory, "traces"),
            ],
            TRACES_INO => vec![
                (TRACES_INO, FileType::Directory, "."),
                (ROOT_INO, FileType::Directory, "..")
            ],
            _ => {
                reply.error(libc:ENOENT);
                return;
            } 
        };

        for (i, (ino, kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(ino, (i + 1) as i64, kind, name) {
                break;
            }
        }
        reply.ok();
    }

    fn read(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _size: u32, 
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        reply.error(libc::ENOENT);
    }
}

pub fn mount(store: Arc<RwLock<SpanStore>>, mountpoint: &str) -> std::io::Result<()> {
    let fs = TraceFs::new(store);
    let options = vec![
        fuser::MountOption::RO,
        fuser::MountOption::FSName("tracefs".to_string()),
        
    ];
    fuser::mount2(fs, mountpoint, &options)?;
    Ok(())
}
