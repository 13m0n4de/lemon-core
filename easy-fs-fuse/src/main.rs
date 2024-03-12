use block_file::BlockFile;
use easy_fs::EasyFileSystem;

use clap::Parser;
use std::fs::{read_dir, File, OpenOptions};
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

mod block_file;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    source: String,

    #[arg(short, long)]
    target: String,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    let source_path = Path::new(&cli.source);
    let target_path = Path::new(&cli.target);
    let image_path = target_path.join("fs.img");

    println!("Initializing the easy-fs image...");
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&image_path)?;
        f.set_len(16 * 2048 * 512)?;
        f
    })));

    // 16 MiB, at most 4095 files
    let efs = EasyFileSystem::create(block_file, 16 * 2048, 1);
    let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));
    root_inode.set_default_dirent(root_inode.inode_id());

    // bin
    let bin_inode = root_inode.create_dir("bin").unwrap();
    bin_inode.set_default_dirent(root_inode.inode_id());

    println!(
        "Packing files from {:?} into the easy-fs image...",
        source_path
    );
    for entry in read_dir(source_path)? {
        let path = entry?.path();
        if path.is_file() {
            let file_stem = path.file_stem().unwrap().to_str().unwrap();
            let app_file_path = target_path.join(file_stem);
            println!("Processing file: {}", app_file_path.display());

            let mut app_file = File::open(app_file_path)?;
            let mut app_data = Vec::new();
            app_file.read_to_end(&mut app_data)?;

            // create a file in easy-fs
            let inode = bin_inode.create(file_stem).unwrap();
            // write data to easy-fs
            inode.write_at(0, app_data.as_slice());
        }
    }

    println!(
        "The easy-fs image has been saved to: {}",
        image_path.display()
    );

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
