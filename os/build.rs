use std::fs::{read_dir, File};
use std::io::{self, Write};

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=../user/src/");
    println!("cargo:rerun-if-changed={}", TARGET_PATH);

    insert_app_data()
}

const TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release/";

fn insert_app_data() -> io::Result<()> {
    let mut file = File::create("src/link_app.S")?;

    let apps = read_app_names("../user/src/bin")?;

    write_app_data_section(&mut file, &apps)?;

    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        write_app_include(&mut file, idx, app)?;
    }

    Ok(())
}

fn read_app_names(dir: &str) -> io::Result<Vec<String>> {
    let entries = read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry
                .file_name()
                .into_string()
                .ok()
                .and_then(|name| name.split('.').next().map(String::from))
        })
        .collect::<Vec<String>>();

    Ok(entries)
}

fn write_app_data_section(file: &mut File, apps: &[String]) -> io::Result<()> {
    writeln!(
        file,
        r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}"#,
        apps.len()
    )?;

    for i in 0..apps.len() {
        writeln!(file, r#"    .quad app_{}_start"#, i)?;
    }
    writeln!(file, r#"    .quad app_{}_end"#, apps.len() - 1)?;

    Ok(())
}

fn write_app_include(file: &mut File, idx: usize, app: &str) -> io::Result<()> {
    writeln!(
        file,
        r#"
    .section .data
    .global app_{0}_start
    .global app_{0}_end
app_{0}_start:
    .incbin "{2}{1}.bin"
app_{0}_end:"#,
        idx, app, TARGET_PATH
    )
}
