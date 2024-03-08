use easy_fs::{BlockDevice, EasyFileSystem};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

const BLOCK_SIZE: usize = 512;

struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SIZE, "Not a complete block!");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SIZE) as u64))
            .expect("Error when seeking!");
        assert_eq!(
            file.write(buf).unwrap(),
            BLOCK_SIZE,
            "Not a complete block!"
        );
    }
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn efs_test() -> std::io::Result<()> {
        // create a block device
        let block_file = Arc::new(BlockFile(Mutex::new({
            let f = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open("target/fs.img")?;
            f.set_len(8192 * 512)?;
            f
        })));
        EasyFileSystem::create(block_file.clone(), 4096, 1);

        // open the file system from the block device
        let efs = EasyFileSystem::open(block_file.clone());

        // get the Inode of the root directory
        let root_inode = EasyFileSystem::root_inode(&efs);

        // test create and ls
        root_inode.create("filea");
        root_inode.create("fileb");
        assert_eq!(root_inode.ls(), vec!["filea", "fileb"]);

        // test find
        let filea = root_inode.find("filea").unwrap();

        // test write and read
        let greet_str = "Hello, world!";
        filea.write_at(0, greet_str.as_bytes());
        let mut buffer = [0u8; 233];
        let len = filea.read_at(0, &mut buffer);
        assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap());

        // test clear
        filea.clear();
        let len = filea.read_at(0, &mut buffer);
        assert!(len == 0);

        // test random string
        let random_str_test = |len: usize| {
            filea.clear();

            let mut str = String::new();
            // random char
            for _ in 0..len {
                str.push(char::from(rand::random::<u8>() % 10));
            }
            filea.write_at(0, str.as_bytes());

            let mut read_buffer = [0u8; 127];
            let mut offset = 0usize;
            let mut read_str = String::new();
            loop {
                let len = filea.read_at(offset, &mut read_buffer);
                if len == 0 {
                    break;
                }
                offset += len;
                read_str.push_str(core::str::from_utf8(&read_buffer[..len]).unwrap());
            }
            assert_eq!(str, read_str);
        };

        random_str_test(4 * BLOCK_SIZE);
        random_str_test(8 * BLOCK_SIZE + BLOCK_SIZE / 2);
        random_str_test(100 * BLOCK_SIZE);
        random_str_test(70 * BLOCK_SIZE + BLOCK_SIZE / 7);
        random_str_test((12 + 128) * BLOCK_SIZE);
        random_str_test(400 * BLOCK_SIZE);
        random_str_test(1000 * BLOCK_SIZE);
        random_str_test(2000 * BLOCK_SIZE);

        Ok(())
    }
}
