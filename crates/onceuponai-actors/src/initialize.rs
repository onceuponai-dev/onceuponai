use anyhow::Result;
use dirs::cache_dir;
use reqwest;
use std::env;
use std::fs::{self, File};
use std::io::BufReader;
#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::symlink;
#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file as symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;
use tar::Archive;
use xz2::read::XzDecoder;

const DEVELOPER_NVIDIA_URL: &str = "https://developer.download.nvidia.com/compute/cuda/redist";
const CUBLAS_LIB: &str = "libcublas";
const CUBLAS_VERSION: &str = "12.3.4.1";
const CURAND_LIB: &str = "libcurand";
const CURAND_VERSION: &str = "10.3.7.37";
const CUDART_LIB: &str = "cuda_cudart";
const CUDART_VERSION: &str = "12.3.101";

pub async fn initialize() -> Result<()> {
    if is_initialized().await? {
        set_library_path();
        return Ok(());
    }

    cleanup_cache_path()?;
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    println!("Detected OS: {}, Architecture: {}", os, arch);

    match gpu_info() {
        Some((gpu_name, driver_version)) => {
            println!("Detected GPU: {}", gpu_name);
            println!("Driver Version: {}", driver_version);

            if let Some(cuda_version) = determine_cuda_version(&driver_version) {
                println!("Determined CUDA Version: {}", cuda_version);
                println!("Please read nvidia license agreements:");
                println!("  https://developer.download.nvidia.com/compute/cuda/redist/cuda_cudart/LICENSE.txt");
                println!("  https://developer.download.nvidia.com/compute/cuda/redist/libcurand/LICENSE.txt");
                println!("  https://developer.download.nvidia.com/compute/cuda/redist/libcublas/LICENSE.txt");

                setup_cuda_libraries(CUDART_LIB, CUDART_VERSION).await?;
                setup_cuda_libraries(CUBLAS_LIB, CUBLAS_VERSION).await?;
                setup_cuda_libraries(CURAND_LIB, CURAND_VERSION).await?;
            } else {
                println!(
                    "No compatible CUDA version found for driver version {}.",
                    driver_version
                );
            }
        }
        None => {
            println!("No NVIDIA GPU detected or could not retrieve GPU info.");
        }
    }

    set_library_path();
    Ok(())
}

pub async fn is_initialized() -> Result<bool> {
    match gpu_info() {
        Some((_gpu_name, _driver_version)) => {
            let cache_path = get_cache_dir();
            let cublas_lib = CUBLAS_LIB;
            let cublas_version = CUBLAS_VERSION;
            let curand_lib = CURAND_LIB;
            let curand_version = CURAND_VERSION;
            let cudart_lib = "libcudart";
            let cudart_version = CUDART_VERSION;

            let cublas_path = cache_path.join(format!("{cublas_lib}.so.{cublas_version}"));
            let curand_path = cache_path.join(format!("{curand_lib}.so.{curand_version}"));
            let cudart_path = cache_path.join(format!("{cudart_lib}.so.{cudart_version}"));

            Ok(cublas_path.exists() && curand_path.exists() && cudart_path.exists())
        }
        None => Ok(true),
    }
}

pub fn gpu_info() -> Option<(String, String)> {
    // Run `nvidia-smi` with specific query flags
    let output = Command::new("nvidia-smi")
        .arg("--query-gpu=name,driver_version")
        .arg("--format=csv,noheader")
        .output()
        .ok()?; // Use `ok()?` to handle possible errors

    if output.status.success() {
        // Parse the output as UTF-8 text
        let result = str::from_utf8(&output.stdout).ok()?;
        // Split the output by commas to extract GPU name and driver version
        let mut parts = result.trim().split(", ");
        let gpu_name = parts.next()?.to_string();
        let driver_version = parts.next()?.to_string();

        Some((gpu_name, driver_version))
    } else {
        None
    }
}

fn determine_cuda_version(driver_version: &str) -> Option<&'static str> {
    match driver_version {
        version if version.starts_with("470") => Some("11.3"),
        version if version.starts_with("510") => Some("11.6"),
        version if version.starts_with("520") => Some("11.8"),
        version if version.starts_with("546") => Some("12.0"),
        _ => Some("12.0"),
    }
}

fn get_cache_dir() -> PathBuf {
    let mut cache_path = cache_dir().expect("Failed to locate cache directory");
    cache_path.push("onceuponai/cuda");
    if !cache_path.exists() {
        fs::create_dir_all(&cache_path).expect("Failed to create cache directory");
    }
    cache_path
}

async fn download_lib(url: &str, output_path: &Path) {
    let response = reqwest::get(url).await.expect("Failed to download file");
    let body = response.bytes().await.expect("body invalid");
    let mut file = File::create(output_path).expect("Failed to create file");
    std::io::copy(&mut body.as_ref(), &mut file).expect("Failed to write file");
}

fn decompress_and_extract_xz_tar(xz_path: &Path, extract_to: &Path) {
    let xz_file = File::open(xz_path).expect("Failed to open .xz file");
    let decompressed = XzDecoder::new(BufReader::new(xz_file));

    let mut archive = Archive::new(decompressed);
    archive
        .unpack(extract_to)
        .expect("Failed to unpack tar archive");
}

async fn setup_cuda_libraries(lib: &str, version: &str) -> Result<()> {
    let cache_path = get_cache_dir();

    // Check for required libraries or download if not present
    let lib_path = cache_path.join(format!("{lib}.so"));
    if !&lib_path.exists() {
        let os = env::consts::OS;
        let arch = env::consts::ARCH;
        println!("Downloading {lib}...");

        let cuda_url = format!(
            "{DEVELOPER_NVIDIA_URL}/{lib}/{os}-{arch}/{lib}-{os}-{arch}-{version}-archive.tar.xz"
        );
        let xz_path = cache_path.join(format!("{lib}.tar.xz"));
        download_lib(&cuda_url, &xz_path).await;

        println!("Decompressing and extracting {lib}...");
        decompress_and_extract_xz_tar(&xz_path, &cache_path);
        copy_and_link_lib(lib, version, os, arch)?;
        fs::remove_file(xz_path)?;
        fs::remove_dir_all(cache_path.join(format!("{lib}-{os}-{arch}-{version}-archive")))?;
    }

    println!("CUDA libraries setup completed.");
    Ok(())
}

fn cleanup_cache_path() -> Result<()> {
    let cache_path = get_cache_dir();
    if cache_path.exists() {
        fs::remove_dir_all(cache_path)?;
    }
    Ok(())
}

fn set_library_path() {
    let cache_path = get_cache_dir();
    // Set environment variable for LD_LIBRARY_PATH
    env::set_var(
        "LD_LIBRARY_PATH",
        format!(
            "{}:{}",
            cache_path.display(),
            env::var("LD_LIBRARY_PATH").unwrap_or_default()
        ),
    );
}

fn copy_and_link_lib(lib: &str, version: &str, os: &str, arch: &str) -> Result<()> {
    let cache_path = get_cache_dir();
    let lib_file = if lib == CUDART_LIB { "libcudart" } else { lib };
    let from_path = cache_path.join(format!(
        "{lib}-{os}-{arch}-{version}-archive/lib/{lib_file}.so.{version}"
    ));
    let to_path = cache_path.join(format!("{lib_file}.so.{version}"));
    let from_path_lt = cache_path.join(format!(
        "{lib}-{os}-{arch}-{version}-archive/lib/{lib_file}Lt.so.{version}"
    ));
    let to_path_lt = cache_path.join(format!("{lib_file}Lt.so.{version}"));

    if from_path.exists() {
        fs::rename(&from_path, &to_path)?;
        let to_path_sym = cache_path.join(format!("{lib_file}.so"));
        symlink(&to_path, &to_path_sym)?;
        let to_path_sym = cache_path.join(format!("{lib_file}.so.12"));
        symlink(&to_path, &to_path_sym)?;
    }

    if from_path_lt.exists() {
        fs::rename(&from_path_lt, &to_path_lt)?;
        let to_path_lt_sym = cache_path.join(format!("{lib_file}Lt.so"));
        symlink(&to_path_lt, &to_path_lt_sym)?;
        let to_path_lt_sym = cache_path.join(format!("{lib_file}Lt.so.12"));
        symlink(&to_path_lt, &to_path_lt_sym)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_decode_and_deserialize_json() -> Result<()> {
        let (gpu_name, driver_version) = gpu_info().unwrap();
        println!("HAS GPU {gpu_name} {driver_version}");
        let cache_dir = get_cache_dir();
        println!("CACHE DIR {cache_dir:?}");
        setup_cuda_libraries(CUDART_LIB, CUDART_VERSION).await?;
        // setup_cuda_libraries(CUBLAS_LIB, CUBLAS_VERSION).await?;
        // setup_cuda_libraries(CURAND_LIB, CURAND_VERSION).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_os_arch() -> Result<()> {
        let os = env::consts::OS;
        let arch = env::consts::ARCH;

        println!("OS {os}");
        println!("ARCH {arch}");
        Ok(())
    }
}
