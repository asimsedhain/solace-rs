extern crate bindgen;
use std::{env, io::Write, path::PathBuf};

#[cfg(target_os = "windows")]
const SOLCLIENT_GZ_PATH: &str = "solclient_Win_vs2015_7.26.1.8.tar.gz";

#[cfg(target_os = "macos")]
const SOLCLIENT_GZ_PATH: &str = "solclient_Darwin-universal2_opt_7.26.1.8.tar.gz";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const SOLCLIENT_GZ_PATH: &str = "solclient_Linux26-x86_64_opt_7.26.1.8.tar.gz";

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const SOLCLIENT_GZ_PATH: &str = "solclient_Linux-aarch64_opt_7.26.1.8.tar.gz";

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
const SOLCLIENT_GZ_PATH: &str = "solclient_Linux_musl-x86_64_opt_7.26.1.8.tar.gz";

fn download_and_unpack(url: &str, tarball_path: PathBuf, tarball_unpack_path: PathBuf) {
    let resp = reqwest::blocking::get(url).unwrap();
    let content = resp.bytes().unwrap();

    let mut file_gz = std::fs::File::create(tarball_path.clone()).unwrap();
    file_gz.write_all(&content).unwrap();
    file_gz.sync_data().unwrap();

    let file_gz = std::fs::File::open(tarball_path).unwrap();
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file_gz));
    archive
        .entries()
        .unwrap()
        .filter_map(|r| r.ok())
        .map(|mut entry| -> std::io::Result<PathBuf> {
            let strip_path = entry.path()?.iter().skip(1).collect::<std::path::PathBuf>();
            let path = tarball_unpack_path.join(strip_path);
            entry.unpack(&path)?;
            Ok(path)
        })
        .filter_map(|e| e.ok())
        .for_each(|x| println!("> {}", x.display()));
}

fn main() {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "windows")] {
            panic!("Windows currently not supported");
        }
    }

    // do nothing if we are just building the docs
    if std::env::var("DOCS_RS").is_ok() {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let solclient_folder_name = "solclient-7.26.1.8";
    let solclient_folder_path = out_dir.join(solclient_folder_name);

    let solclient_tarball_default_url = format!(
        "https://github.com/asimsedhain/solace-rs/releases/download/0.0.0.0/{SOLCLIENT_GZ_PATH}"
    );

    let lib_dir = if env::var("SOLCLIENT_LIB_PATH").is_ok() {
        PathBuf::from(env::var("SOLCLIENT_LIB_PATH").unwrap())
    } else {
        let solclient_tarball_url =
            env::var("SOLCLIENT_TARBALL_URL").unwrap_or(solclient_tarball_default_url);

        let solclient_tarball_path = out_dir.join(format!("{solclient_folder_name}.tar.gz"));

        if !solclient_folder_path.is_dir() {
            eprintln!(
                "Solclient not found. Downloading from {}",
                solclient_tarball_url
            );
            download_and_unpack(
                &solclient_tarball_url,
                solclient_tarball_path,
                solclient_folder_path.clone(),
            );
        }

        solclient_folder_path.join("lib")
    };

    println!(
        "cargo:rustc-link-search=native={}",
        lib_dir.as_path().display()
    );

    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
            println!("cargo:rustc-link-lib=dylib=gssapi_krb5");
        }
    }

    println!("cargo:rustc-link-lib=static=crypto");
    println!("cargo:rustc-link-lib=static=ssl");
    println!("cargo:rustc-link-lib=static=solclient");
    println!("cargo:rustc-link-lib=static=solclientssl");
}
