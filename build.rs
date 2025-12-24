#![allow(unused)]

use std::{env, path::PathBuf};
use std::fs;
use std::path::Path;

fn main() {
    // 1. 获取当前 Lib crate 的根目录
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // 2. 定位到 proto 目录（核心：遍历该目录下所有proto文件）
    let proto_dir = manifest_dir.join("proto");

    // 检查 proto 目录是否存在
    if !proto_dir.is_dir() {
        panic!(
            "Proto目录未找到：{}，请确认目录路径为 proto/",
            proto_dir.display()
        );
    }

    // 3. 遍历 proto 目录下所有 .proto 文件（支持子目录，若不需要可改为仅遍历当前目录）
    let mut proto_files = Vec::new();
    // 方案1：仅遍历当前目录（标准库）
    // for entry in fs::read_dir(&proto_dir).expect("读取proto目录失败") {
    //     let entry = entry.expect("获取目录项失败");
    //     let path = entry.path();
    //     // 筛选：是文件 + 后缀为 .proto
    //     if path.is_file() && path.extension().map_or(false, |ext| ext == "proto") {
    //         proto_files.push(path);
    //     }
    // }

    // 方案2：遍历当前目录 + 所有子目录（推荐，需添加 walkdir 依赖）
    // 若使用此方案，需在 Cargo.toml 的 [build-dependencies] 中添加：walkdir = "2"
    for entry in walkdir::WalkDir::new(&proto_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        // 筛选：是文件 + 后缀为 .proto
        if path.is_file() && path.extension().map_or(false, |ext| ext == "proto") {
            proto_files.push(path.to_path_buf());
        }
    }

    // 检查是否找到 proto 文件
    if proto_files.is_empty() {
        panic!(
            "在Proto目录 {} 中未找到任何 .proto 文件",
            proto_dir.display()
        );
    }

    // 4. 指定生成的 Rust 代码输出目录（src/protos/）
    let out_dir = manifest_dir.join("src/protos");

    // 5. 监听 proto 目录和所有 proto 文件的变化（修改后自动重新编译）
    println!("cargo:rerun-if-changed={}", proto_dir.display());
    for proto_file in &proto_files {
        println!("cargo:rerun-if-changed={}", proto_file.display());
    }

    // 6. 创建输出目录（若不存在）
    fs::create_dir_all(&out_dir).expect("创建 src/protos 目录失败");

    // 7. 批量编译所有 Proto 文件
    // 转换路径为 &str（tonic_build 需要的类型）
    let proto_files_str: Vec<&str> = proto_files
        .iter()
        .map(|p| p.to_str().expect("Proto文件路径包含非UTF-8字符"))
        .collect();
    let proto_dir_str = proto_dir.to_str().expect("Proto目录路径包含非UTF-8字符");

    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .out_dir(out_dir)
        .compile_protos(
            // 第一个参数：所有要编译的 proto 文件列表
            &proto_files_str,
            // 第二个参数：proto 文件的根目录（用于解析 import 语句）
            &[proto_dir_str],
        )
        .expect("编译 Proto 文件失败");
}
