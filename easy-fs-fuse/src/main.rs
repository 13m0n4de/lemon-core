#![deny(clippy::all)]
#![deny(clippy::pedantic)]

use block_file::BlockFile;
use clap::Parser;
use easy_fs::{BlockDevice, EasyFileSystem, Inode};
use std::fs::{read_dir, File, OpenOptions};
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

mod block_file;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "easy-fs-root")]
    root: String,

    #[arg(short, long, default_value = "fs.img")]
    output: String,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    let root_path = Path::new(&cli.root);
    let output_path = Path::new(&cli.output);

    let image_path = if output_path.is_dir() {
        output_path.join("fs.img")
    } else {
        output_path.to_path_buf()
    };

    println!("Initializing the easy-fs image...");
    let block_file: Arc<dyn BlockDevice> = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&image_path)?;
        f.set_len(32 * 2048 * 512)?;
        f
    })));

    // 16 MiB, at most 4095 files
    let efs = EasyFileSystem::create(&block_file, 32 * 2048, 1);
    let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));
    root_inode.set_default_dirent(root_inode.inode_id());

    println!("Packing files from {root_path:?} into the easy-fs image...");
    pack_directory(&root_inode, root_path)?;

    println!(
        "The easy-fs image has been saved to: {}",
        image_path.display()
    );

    Ok(())
}

fn pack_directory(parent_inode: &Arc<Inode>, path: &Path) -> std::io::Result<()> {
    for entry in read_dir(path)? {
        let entry_path = entry?.path();
        let entry_name = entry_path.file_name().unwrap().to_str().unwrap();

        if entry_name.starts_with('.') {
            continue;
        }

        if entry_path.is_dir() {
            let dir_inode = parent_inode.create_dir(entry_name).unwrap();
            pack_directory(&dir_inode, &entry_path)?;
        } else if entry_path.is_file() {
            let mut file = File::open(&entry_path)?;
            let inode = parent_inode.create(entry_name).unwrap();

            let mut buffer = vec![0; 65536];
            let mut offset = 0;
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                inode.write_at(offset, &buffer[..bytes_read]);
                offset += bytes_read;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use easy_fs::BLOCK_SIZE;

    #[test]
    fn efs_test() -> std::io::Result<()> {
        // create a block device
        let block_file = Arc::new(BlockFile(Mutex::new({
            let f = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
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
