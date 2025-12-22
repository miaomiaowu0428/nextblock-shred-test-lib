use std::{env, path::PathBuf};

fn main() {
    // 1. 获取当前 Lib crate 的根目录（对应：~/solana/strategy/pumpswap-snipe/）
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // 2. 直接定位到 proto 子目录下的 stream.proto（核心修改点）
    // 路径：~/solana/strategy/pumpswap-snipe/proto/stream.proto
    let proto_file = manifest_dir.join("proto").join("stream.proto");

    // 3. 检查 proto 文件是否存在（友好报错）
    if !proto_file.exists() {
        panic!(
            "Proto 文件未找到：{}，请确认文件路径为 proto/stream.proto",
            proto_file.display()
        );
    }

    // 4. 指定生成的 Rust 代码输出目录（src/protos/）
    let out_dir = manifest_dir.join("src/protos");

    // 5. 监听 proto 文件和 proto 目录的变化（修改后自动重新编译）
    println!("cargo:rerun-if-changed={}", proto_file.display());
    println!("cargo:rerun-if-changed={}", manifest_dir.join("proto").display());

    // 6. 创建输出目录（若不存在）
    std::fs::create_dir_all(&out_dir).expect("创建 src/protos 目录失败");

    // 7. 编译 Proto 文件（核心：第二个参数是 proto 文件的根目录，即 proto/）
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .out_dir(out_dir)
        .compile_protos(
            // 第一个参数：要编译的 proto 文件列表
            &[proto_file.to_string_lossy().to_string()],
            // 第二个参数：proto 文件的根目录（用于解析 import 语句）
            &[manifest_dir.join("proto").to_string_lossy().to_string()],
        )
        .expect("编译 Proto 文件失败");
}